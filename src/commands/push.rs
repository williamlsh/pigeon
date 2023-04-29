use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use std::{collections::HashMap, str, time::Duration};
use tokio::{
    signal,
    sync::oneshot::{self, Receiver},
    time,
};
use tracing::{debug, info, warn};

use crate::{config::PushConfig, database::Database, telegram::Message, twitter::Tweet};

/// Push command entry.
///
/// The `first_entry` and `last_entry` fields are used to mark
/// entries range in timeline column family that successfully
/// pushed to Telegram channel(s). So that we can delete them
/// from database after pushing.
pub(crate) struct Push<'a> {
    telegram_token: String,
    config: Vec<PushConfig>,
    client: &'a Client,
    database: &'a mut Database,
    /// The first entry when reading timeline column family for pushing.
    first_entry: Option<Box<[u8]>>,
    /// The last entry when reading timeline column family for pushing.
    last_entry: Option<Box<[u8]>>,
    /// Shutdown signal.
    signal: Receiver<()>,
}

impl<'a> Push<'a> {
    pub(crate) fn new(
        telegram_token: Option<String>,
        config: Vec<PushConfig>,
        client: &'a Client,
        database: &'a mut Database,
    ) -> Result<Self> {
        let telegram_token = telegram_token.ok_or_else(|| anyhow!("Empty Telegram token"))?;
        let signal = shutdown_signal();
        Ok(Self {
            telegram_token,
            config,
            client,
            database,
            first_entry: None,
            last_entry: None,
            signal,
        })
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let user_map = self.user_map();
        // Read timeline column family from database.
        // Note: we're sure there's a timeline iterator, so just unwrap it directly.
        for (i, entry) in self.database.iterator_cf("timeline").unwrap().enumerate() {
            let (key, value) = entry?;
            if i == 0 {
                // Keep the first entry key.
                self.first_entry = Some(key.clone());
            }

            // Check shutdown signal first.
            if self.signal.try_recv().is_ok() {
                self.last_entry = Some(key);
                break;
            }

            let (twitter_username, tweet) = {
                let key_str = str::from_utf8(&key)?;
                let tweet: Tweet = serde_json::from_slice(&value)?;
                // Unwrap it directly since we're sure it's Some(&str).
                let (twitter_username, _) = key_str.split_once(':').unwrap();
                (twitter_username, tweet)
            };
            debug!("Read {twitter_username}'s tweet.");
            if let Some(telegram_channel) = user_map.get(twitter_username) {
                debug!("Push tweet to {telegram_channel}");
                let response = Message::new(telegram_channel, tweet)
                    .send(self.client, &self.telegram_token)
                    .await
                    .with_context(|| "Failed to send message to Telegram channel")
                    .map_err(|err| {
                        // This error check is necessary in order to tidy database despite error or panic.
                        info!("An error happened when requesting, will stop pushing and delete pushed tweets in database.");
                        self.last_entry = Some(key.clone());
                        err
                    })?;
                match response.status() {
                    // Note: Telegram bot api applies requests rate limit.
                    StatusCode::OK => time::sleep(Duration::from_secs(3)).await,
                    other => {
                        warn!(
                            "Request not successful, channel: {telegram_channel}, response status: {other}, body: {}",
                            response.text().await.unwrap_or_else(|_| "".to_string())
                        );
                        info!("Stop pushing and deleting pushed tweets in database.");
                        // Keep the last entry key.
                        self.last_entry = Some(key);
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn tidy_database(&mut self) -> Result<()> {
        match (self.first_entry.take(), self.last_entry.take()) {
            (Some(first_entry), Some(last_entry)) => {
                self.database
                    .delete_range_cf("timeline", first_entry, last_entry)
            }
            (Some(_), None) => {
                info!("Finished pushing all timeline.");
                self.database.drop_cf("timeline")
            }
            _ => {
                info!("No tweets to push.");
                Ok(())
            }
        }
    }

    /// Returns a Twitter username to Telegram channel map.
    fn user_map(&mut self) -> HashMap<String, String> {
        self.config
            .drain(..)
            .map(|cfg| (cfg.from, cfg.username))
            .collect()
    }
}

impl<'a> Drop for Push<'a> {
    fn drop(&mut self) {
        let _ = self.tidy_database();
    }
}

/// Handles user interrupt signal.
fn shutdown_signal() -> Receiver<()> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("failed to listen for event");
        info!("received ctrl-c event");

        let _ = tx.send(());
    });
    rx
}
