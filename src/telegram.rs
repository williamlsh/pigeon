use serde::Serialize;
use url::Url;

const API_ENDPOINT_BASE: &str = "https://api.telegram.org/";

pub fn url_from_method(token: &str, method: &str) -> Result<Url, String> {
    let api = Url::parse(API_ENDPOINT_BASE)
        .map_err(|error| format!("could not parse telegram api base endpoint: {}", error))?;
    let url = Url::options()
        .base_url(Some(&api))
        .parse(format!("/bot{token}/{method}").as_str())
        .map_err(|error| format!("could not parse path :{}", error))?;
    Ok(url)
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub chat_id: String,
    pub text: String,
}
