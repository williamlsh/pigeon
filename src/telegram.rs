use anyhow::{Context, Result};
use reqwest::{Client, Response};
use serde::Serialize;
use url::Url;

use crate::twitter::Tweet;

/// A message sent by Telegram bot.
#[derive(Debug, Serialize)]
pub(crate) struct Message {
    /// Telegram channel username.
    chat_id: String,
    /// Message text body.
    text: String,
}

impl Message {
    pub(crate) fn new(channel: &str, tweet: Tweet) -> Self {
        Self {
            chat_id: format!("{channel}"),
            text: format!("{}\n\n{}", tweet.text, tweet.created_at),
        }
    }

    pub(crate) async fn send(&self, client: &Client, telegram_token: &str) -> Result<Response> {
        Ok(client
            .post(endpoint(telegram_token)?)
            .json(self)
            .send()
            .await?)
    }
}

/// An endpoint for sending messages by Telegram bot.
/// See: https://core.telegram.org/bots/api#sendmessage
fn endpoint(token: &str) -> Result<Url> {
    let api = Url::parse("https://api.telegram.org/")
        .with_context(|| "Could not parse Telegram api base endpoint")?;
    Url::options()
        .base_url(Some(&api))
        .parse(format!("/bot{token}/sendMessage").as_str())
        .with_context(|| "could not parse Telegram api path")
}
