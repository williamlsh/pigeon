use reqwest::{Client, Response};
use serde::Serialize;
use url::Url;

/// A message sent by Telegram bot.
#[derive(Debug, Serialize)]
pub(crate) struct Message {
    /// Telegram channel username.
    pub(crate) chat_id: String,
    /// Message text body.
    pub(crate) text: String,
}

impl Message {
   pub(crate) async fn send(&self, client: &Client, telegram_token: &str) -> Result<Response, String> {
        client
            .post(endpoint(telegram_token)?)
            .json(self)
            .send()
            .await
            .map_err(|err| format!("Error sending post request: {:?}", err))
    }
}

/// An endpoint for sending messages by Telegram bot.
/// See: https://core.telegram.org/bots/api#sendmessage
fn endpoint(token: &str) -> Result<Url, String> {
    let api = Url::parse("https://api.telegram.org/")
        .map_err(|error| format!("could not parse telegram api base endpoint: {}", error))?;
    let url = Url::options()
        .base_url(Some(&api))
        .parse(format!("/bot{token}/sendMessage").as_str())
        .map_err(|error| format!("could not parse path :{}", error))?;
    Ok(url)
}
