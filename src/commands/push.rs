use log::{info, warn};
use reqwest::{Client, StatusCode};
use std::{collections::HashMap, str, time::Duration};
use tokio::time;

use crate::{config::PushConfig, database::Database, telegram::Message, twitter::Tweet};

/// Push command entry.
///
/// The `first_entry` and `last_entry` fields are used to mark
/// entries range in timeline column family that successfully
/// pushed to Telegram channel(s). So that we can delete them
/// from database after pushing.
pub(crate) struct Push {
    telegram_token: String,
    config: Vec<PushConfig>,
    /// The first entry when reading timeline column family for pushing.
    first_entry: Option<Box<[u8]>>,
    /// The last entry when reading timeline column family for pushing.
    last_entry: Option<Box<[u8]>>,
}

impl Push {
    pub(crate) fn new(
        telegram_token: Option<String>,
        config: Vec<PushConfig>,
    ) -> Result<Self, String> {
        let telegram_token = telegram_token.ok_or("Empty Telegram token")?;
        Ok(Self {
            telegram_token,
            config,
            first_entry: None,
            last_entry: None,
        })
    }

    pub(crate) async fn run(
        &mut self,
        client: &Client,
        database: &mut Database,
    ) -> Result<(), String> {
        let user_map = self.user_map();
        // Read timeline column family from database.
        // Note: we're sure there's a timeline iterator, so just unwrap it directly.
        for (i, entry) in database.iterator_cf("timeline").unwrap().enumerate() {
            let (key, value) = entry?;
            if i == 0 {
                // Keep the first entry key.
                self.first_entry = Some(key.clone());
            }

            let (twitter_username, tweet) = {
                let key_str = str::from_utf8(&key)
                    .map_err(|err| format!("could not convert string from bytes: {:?}", err))?;
                let tweet: Tweet = serde_json::from_slice(&value)
                    .map_err(|err| format!("could not decode data from bytes: {:?}", err))?;
                // Unwrap it directly since we're sure it's Some(&str).
                let (twitter_username, _) = key_str.split_once(':').unwrap();
                (twitter_username, tweet)
            };
            if let Some(telegram_channel) = user_map.get(twitter_username) {
                let message = Message {
                    chat_id: format!("@{}", telegram_channel),
                    text: tweet.text,
                };
                let response = message.send(client, &self.telegram_token).await?;
                match response.status() {
                    // Note: Telegram bot api applies requests rate limit.
                    StatusCode::OK => time::sleep(Duration::from_secs(3)).await,
                    other => {
                        warn!(
                            "request not successful, got response status: {} and body: {}",
                            other,
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
        // Tidy database.
        match (self.first_entry.take(), self.last_entry.take()) {
            (Some(first_entry), Some(last_entry)) => {
                database.delete_range_cf("timeline", first_entry, last_entry)
            }
            (Some(_), None) => {
                info!("Finished pushing all timeline.");
                database.drop_cf("timeline")
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
