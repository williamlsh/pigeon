use log::{info, warn};
use reqwest::{Client, StatusCode};
use std::{collections::HashMap, str, time::Duration};
use tokio::time;

use crate::{config::PushConfig, database::Database, telegram::Message, twitter::Tweet};

pub(crate) struct Push {
    telegram_token: String,
    config: Vec<PushConfig>,
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
        })
    }

    pub(crate) async fn run(&mut self, client: &Client, database: &Database) -> Result<(), String> {
        let user_map = self.user_map();

        if let Some(timeline) = database.iterator_cf("timeline") {
            for entry in timeline {
                let (key, value) = entry?;
                let (twitter_username, tweet) = {
                    let key_str = str::from_utf8(&key)
                        .map_err(|err| format!("could not convert string from bytes: {:?}", err))?;
                    let tweet: Tweet = serde_json::from_slice(&value)
                        .map_err(|err| format!("could not decode data from bytes: {:?}", err))?;
                    // Unwrap it directly since we're sure it's Some(&str).
                    let (twitter_username, _) = key_str.split_once(':').unwrap();
                    (twitter_username, tweet)
                };
                let telegram_channel = user_map.get(twitter_username);
                if let Some(telegram_channel) = telegram_channel {
                    let message = Message {
                        chat_id: telegram_channel.to_string(),
                        text: tweet.text,
                    };
                    let response = message.send(client, &self.telegram_token).await?;
                    match response.status() {
                        StatusCode::OK => time::sleep(Duration::from_secs(3)).await,
                        other => {
                            warn!(
                                "request not successful, got response status: {} and body: {}",
                                other,
                                response.text().await.unwrap_or_else(|_| "".to_string())
                            );
                            info!("Stop pushing and deleting pushed tweets in database.");
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns a Twitter username to Telegram channel map.
    fn user_map(&mut self) -> HashMap<String, String> {
        self.config
            .drain(..)
            .map(|cfg| (cfg.from, cfg.username))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn x() {}
}
