use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    rocksdb_path: PathBuf,
    twitter_token: Option<String>,
    telegram_token: Option<String>,
    poll: Option<Vec<PollConfig>>,
    push: Option<Vec<PushConfig>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PollConfig {
    included: bool,
    username: String,
    max_results: Option<u8>,
    start_time: Option<String>,
    end_time: Option<String>,
    since_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PushConfig {
    included: bool,
    from: String,
    username: String,
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
