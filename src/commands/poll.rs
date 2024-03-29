use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration};
use reqwest::Client;
use std::{collections::HashMap, str};
use tracing::{info, trace};
use url::Url;

use crate::{
    config::PollConfig,
    database::Database,
    twitter::{PaginationToken, Timeline, Tweet, UrlBuilder, Users},
};

/// Poll command entry.
pub(crate) struct Poll<'a> {
    twitter_token: String,
    config: Vec<PollConfig>,
    client: &'a Client,
    database: &'a Database,
}

impl<'a> Poll<'a> {
    pub(crate) fn new(
        twitter_token: Option<String>,
        poll_config: Vec<PollConfig>,
        client: &'a Client,
        database: &'a Database,
    ) -> Result<Self> {
        let twitter_token = twitter_token.ok_or_else(|| anyhow!("Empty twitter token"))?;
        Ok(Self {
            twitter_token,
            config: poll_config,
            client,
            database,
        })
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let user_map = self.user_map(self.client).await?;

        // Loop Twitter users in poll configs.
        for cfg in &mut self.config {
            let start_time = Self::fetch_state(self.database, &cfg.username)?;
            // Note: `start_time` in persistent state has higher priority than that in poll config.
            cfg.insert_start_time(start_time);
            info!("Polling timeline with config: {cfg:?}",);

            let endpoint = Self::endpoint(cfg, &user_map)?;
            // Note: `since_id` takes higher priority than `start_time` in request query parameters.
            let since_id = cfg.since_id.take().map(PaginationToken::TweetID);
            let mut timeline = Timeline::new(self.client, endpoint, &self.twitter_token, since_id);

            // Poll first tweet. The first tweet is the latest one in timeline.
            // Extract `create_at` from tweet, and upsert it to persistent state.
            // So we can continually poll user's timeline from last time.
            if let Some(tweet) = timeline.try_next().await? {
                Self::upsert_state(self.database, &cfg.username, &tweet.created_at)?;
                Self::insert_tweet(self.database, &cfg.username, &tweet)?;
            }
            // Poll remaining tweets.
            while let Some(tweet) = timeline.try_next().await? {
                Self::insert_tweet(self.database, &cfg.username, &tweet)?;
            }
        }
        info!("Finished polling all timeline.");
        Ok(())
    }

    // Gets `create_at` of a latest tweet in persistent state, then adds one second to it
    // to be used as `start_time` in timeline request query. This is necessary to deduplicate
    // a tweet when polling.
    fn fetch_state(database: &Database, username: &str) -> Result<Option<String>> {
        if let Some(value) = database.get_cf("state", username)? {
            let value_str = str::from_utf8(&value)?;
            Ok(DateTime::parse_from_rfc3339(value_str)?
                .checked_add_signed(Duration::seconds(1))
                .map(|datetime| datetime.to_rfc3339()))
        } else {
            Ok(None)
        }
    }

    fn upsert_state(database: &Database, username: &str, created_at: &str) -> Result<()> {
        trace!("Upsert state: key: {username}, value: {created_at}");
        database.put_cf("state", username, created_at)
    }

    fn insert_tweet(database: &Database, username: &str, tweet: &Tweet) -> Result<()> {
        let key = format!("{username}:{}", tweet.id);
        let value =
            serde_json::to_vec(&tweet).with_context(|| "could not serialize tweet data to json")?;
        trace!("Insert tweet: key: {key}, value: {tweet:?}");
        database.put_cf("timeline", key, value)
    }

    fn endpoint(config: &PollConfig, user_map: &HashMap<String, String>) -> Result<Url> {
        // Unwrap it directly since we are sure it's not None.
        let user_id = user_map.get(config.username.as_str()).unwrap();
        Ok(UrlBuilder::new(user_id)?
            .tweet_fields(vec!["created_at"])
            // Set default `max_results` value: 100.
            .max_results(config.max_results.unwrap_or(100))
            .start_time(config.start_time.as_deref())
            .end_time(config.end_time.as_deref())
            .build())
    }

    /// Returns a username to user_id map.
    async fn user_map(&self, client: &Client) -> Result<HashMap<String, String>> {
        let usernames = self
            .config
            .iter()
            .map(|cfg| cfg.username.as_str())
            .collect();
        Users::fetch(client, usernames, &self.twitter_token)
            .await?
            .ok_or_else(|| anyhow!("No Twitter users found"))
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use rocksdb::{Options, DB};

    use super::Poll;
    use crate::{config::PollConfig, database::Database};

    // To test this function:
    // RUST_LOG=debug cargo test poll -- --ignored '[auth_token]'
    #[test_log::test(tokio::test)]
    #[ignore = "require command line input"]
    async fn poll() {
        let mut args = std::env::args().rev();
        let auth_token = args.next();

        let rocksdb_path = "test";
        let database = Database::open(rocksdb_path);
        let client = Client::new();
        let mut poll_config = vec![PollConfig {
            included: true,
            username: "TwitterDev".into(),
            max_results: Some(5),
            start_time: Some("2022-10-25T00:00:00.000Z".into()),
            end_time: Some("2022-10-30T00:00:00.000Z".into()),
            since_id: None,
        }];
        {
            let mut poll =
                Poll::new(auth_token.clone(), poll_config.clone(), &client, &database).unwrap();
            poll.run().await.unwrap();
        }
        {
            // Remove `start_time` and `end_time` fields.
            poll_config.iter_mut().for_each(|cfg| {
                cfg.start_time.take();
                cfg.end_time.replace("2022-12-01T00:00:00.000Z".into());
            });
            let mut poll = Poll::new(auth_token, poll_config, &client, &database).unwrap();
            // Poll again from last time.
            poll.run().await.unwrap();
        }

        drop(database);
        DB::destroy(&Options::default(), rocksdb_path).unwrap();
    }
}
