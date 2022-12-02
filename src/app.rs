use crate::{
    commands::{info, Poll, Push},
    config::{PollConfig, PushConfig},
    database::Database,
    Config,
};
use reqwest::Client;

/// Application entry.
pub struct App {
    database: Database,
    client: Client,
    config: Config,
}

impl App {
    pub fn new(config: Config) -> Result<Self, String> {
        let database = Database::open(config.rocksdb_path.as_path());
        let client = Client::new();
        Ok(Self {
            database,
            client,
            config,
        })
    }

    pub async fn poll(&mut self) -> Result<(), String> {
        Poll::new(self.config.twitter_token.take(), self.poll_config()?)?
            .run(&self.client, &self.database)
            .await
    }

    pub async fn push(&mut self) -> Result<(), String> {
        Push::new(self.config.telegram_token.take(), self.push_config()?)?
            .run(&self.client, &mut self.database)
            .await
    }

    pub fn info(&self) -> Result<(), String> {
        info(&self.database).map_err(|err| format!("Error displaying database data: {:?}", err))
    }

    /// Returns poll configs that are included.
    fn poll_config(&mut self) -> Result<Vec<PollConfig>, String> {
        self.config
            .poll
            .take()
            .map(|cfg| cfg.into_iter().filter(|cfg| cfg.included).collect())
            .ok_or_else(|| "Empty poll config".into())
    }

    fn push_config(&mut self) -> Result<Vec<PushConfig>, String> {
        self.config
            .push
            .take()
            .map(|cfg| cfg.into_iter().filter(|cfg| cfg.included).collect())
            .ok_or_else(|| "Empty poll config".into())
    }
}
