use crate::{
    commands::{info, Poll, Push},
    config::{PollConfig, PushConfig},
    database::Database,
    Config,
};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use tracing::{info, instrument};

/// Application entry.
pub struct App {
    database: Database,
    client: Client,
    config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        let database = Database::open(config.rocksdb_path.as_path());
        let client = Client::new();
        Self {
            database,
            client,
            config,
        }
    }

    #[instrument(skip_all)]
    pub async fn poll(&mut self) -> Result<()> {
        info!("Starting to poll Twitter timeline from config.");
        Poll::new(
            self.config.twitter_token.take(),
            self.poll_config()?,
            &self.client,
            &self.database,
        )?
        .run()
        .await
        .with_context(|| "Failed to execute poll command")
    }

    #[instrument(skip_all)]
    pub async fn push(&mut self) -> Result<()> {
        info!("Starting to push timeline to Telegram channel(s) from config.");
        Push::new(
            self.config.telegram_token.take(),
            self.push_config()?,
            &self.client,
            &mut self.database,
        )?
        .run()
        .await
        .with_context(|| "Failed to execute push command")
    }

    pub fn info(&self) -> Result<()> {
        info!("Overview info of database.");
        info(&self.database).with_context(|| "Failed to execute info command")
    }

    /// Returns poll configs that are included.
    fn poll_config(&mut self) -> Result<Vec<PollConfig>> {
        self.config
            .poll
            .take()
            .map(|cfg| cfg.into_iter().filter(|cfg| cfg.included).collect())
            .ok_or_else(|| anyhow!("Empty poll config"))
    }

    /// Returns push configs that are included.
    fn push_config(&mut self) -> Result<Vec<PushConfig>> {
        self.config
            .push
            .take()
            .map(|cfg| cfg.into_iter().filter(|cfg| cfg.included).collect())
            .ok_or_else(|| anyhow!("Empty push config"))
    }
}
