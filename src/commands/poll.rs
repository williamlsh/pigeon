use chrono::{DateTime, Duration};
use reqwest::Client;
use std::{collections::HashMap, str};
use url::Url;

use crate::{
    config::PollConfig,
    database::Database,
    twitter::{PaginationToken, Timeline, Tweet, UrlBuilder, Users},
};

/// Poll command entry.
pub(crate) struct Poll {
    twitter_token: String,
    config: Vec<PollConfig>,
}

impl Poll {
    pub(crate) fn new(
        twitter_token: Option<String>,
        poll_config: Vec<PollConfig>,
    ) -> Result<Self, String> {
        let twitter_token = twitter_token.ok_or("Empty twitter token")?;
        Ok(Self {
            twitter_token,
            config: poll_config,
        })
    }

    pub(crate) async fn run(&mut self, client: &Client, database: &Database) -> Result<(), String> {
        let user_map = self.user_map(client).await?;

        // Loop Twitter users in poll configs.
        for cfg in &mut self.config {
            let start_time = Self::fetch_state(database, &cfg.username)?;
            // Note: `start_time` in poll config has higher priority than that in persistent state.
            cfg.insert_start_time(start_time);

            let endpoint = Self::endpoint(cfg, &user_map);
            // Note: `since_id` takes higher priority than `start_time` in request query parameters.
            let since_id = cfg.since_id.take().map(PaginationToken::TweetID);
            let mut timeline = Timeline::new(client, endpoint, &self.twitter_token, since_id);

            // Poll first tweet. The first tweet is the latest one in timeline.
            // Extract `create_at` from tweet, and upsert it to persistent state.
            // So we can continually poll user's timeline from last time.
            if let Some(tweet) = timeline.try_next().await? {
                Self::upsert_state(database, &cfg.username, &tweet.created_at)?;
                Self::insert_tweet(database, &cfg.username, &tweet)?;
            }
            // Poll remaining tweets.
            while let Some(tweet) = timeline.try_next().await? {
                Self::insert_tweet(database, &cfg.username, &tweet)?;
            }
        }
        Ok(())
    }

    // Gets `create_at` of a latest tweet in persistent state, then adds one second to it
    // to be used as `start_time` in timeline request query. This is necessary to deduplicate
    // a tweet when polling.
    fn fetch_state(database: &Database, username: &str) -> Result<Option<String>, String> {
        if let Some(value) = database.get_cf("state", username)? {
            let value_str = str::from_utf8(&value)
                .map_err(|err| format!("could not convert string from bytes: {:?}", err))?;
            Ok(DateTime::parse_from_rfc3339(value_str)
                .map_err(|err| format!("could not parse date time from string: {:?}", err))?
                .checked_add_signed(Duration::seconds(1))
                .map(|datetime| datetime.to_rfc3339()))
        } else {
            Ok(None)
        }
    }

    fn upsert_state(database: &Database, username: &str, created_at: &str) -> Result<(), String> {
        database.put_cf("state", username, created_at)
    }

    fn insert_tweet(database: &Database, username: &str, tweet: &Tweet) -> Result<(), String> {
        let key = format!("{}:{}", username, tweet.id);
        let value = serde_json::to_vec(&tweet)
            .map_err(|err| format!("could not serialize tweet data to json: {:?}", err))?;
        database.put_cf("timeline", key, value)
    }

    fn endpoint(config: &PollConfig, user_map: &HashMap<String, String>) -> Url {
        // Unwrap it directly since we are sure it's not None.
        let user_id = user_map.get(config.username.as_str()).unwrap();
        UrlBuilder::new(user_id)
            .tweet_fields(vec!["created_at"])
            .max_results(config.max_results)
            .start_time(config.start_time.as_deref())
            .end_time(config.end_time.as_deref())
            .build()
    }

    /// Returns a username to user_id map.
    async fn user_map(&self, client: &Client) -> Result<HashMap<String, String>, String> {
        let usernames = self
            .config
            .iter()
            .map(|cfg| cfg.username.as_str())
            .collect();
        Users::fetch(client, usernames, &self.twitter_token)
            .await?
            .ok_or_else(|| "No Twitter users found".into())
    }
}
