use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub(crate) rocksdb_path: PathBuf,
    pub(crate) twitter_token: Option<String>,
    pub(crate) telegram_token: Option<String>,
    pub(crate) poll: Option<Vec<PollConfig>>,
    pub(crate) push: Option<Vec<PushConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct PollConfig {
    pub(crate) included: bool,
    pub(crate) username: String,
    pub(crate) max_results: Option<u8>,
    pub(crate) start_time: Option<String>,
    pub(crate) end_time: Option<String>,
    pub(crate) since_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PushConfig {
    pub(crate) included: bool,
    pub(crate) from: String,
    pub(crate) username: String,
}

impl PollConfig {
    pub(crate) fn insert_start_time(&mut self, start_time: Option<String>) {
        start_time.map(|start_time| self.start_time.insert(start_time));
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn decode() {
        let toml_str = r#"
        rocksdb_path = "rocksdb"
        twitter_token = "xxx"
        telegram_token = "xxx"

        [[poll]]
        included = true
        username = "TwitterDev"
        max_results = 5
        start_time = "2022-10-25T00:00:00.000Z"
        end_time = "2022-11-01T00:00:00.000Z"
        since_id = "xyz"

        [[push]]
        included = true
        from = "TwitterDev"
        username = "some_bot"
        "#;
        let decoded = toml::from_str::<Config>(toml_str);
        assert!(decoded.is_ok());
    }
}
