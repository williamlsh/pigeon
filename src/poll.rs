use crate::{
    cli::Poll,
    database::{Database, COLUMN_FAMILY_NEWEST_TWEET_ID},
    telegram,
    twitter::{
        self,
        timeline::{self, PaginatedTimeline, PaginationToken, Timeline},
        user::User,
    },
    utils,
};
use log::{debug, info, warn};
use reqwest::{blocking::Client, StatusCode};
use std::{thread, time::Duration};
use url::Url;

pub fn poll(args: Poll) {
    // Reusable HTTP client.
    let client = Client::new();
    let send_message_endpoint =
        telegram::url_from_method(&args.telegram_bot_api_token, "sendMessage").unwrap();

    let twitter_api_base_url =
        Url::parse(twitter::API_ENDPOINT_BASE).expect("could not parse twitter base api endpoint");

    let twitter_user_lookup_endpoint =
        User::url_from_usernames_query(&twitter_api_base_url, &args.twitter_usernames)
            .expect("could not construct url from given usernames");
    let twitter_user_ids = User::get_user_ids(
        &client,
        twitter_user_lookup_endpoint,
        &args.twitter_api_token,
    )
    .expect("could not get user_ids")
    .unwrap();

    let timestamp = utils::timestamp();
    let twitter_usernames: Vec<&str> = args.twitter_usernames.split(',').collect();
    let cfs: Vec<String> = twitter_usernames
        .iter()
        .map(|username| Database::cf_poll_from_username(username, timestamp))
        .chain(Database::list_cf(&args.rocksdb_path).unwrap())
        .collect();
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    let twitter_username_id_pairs: Vec<(&&str, String)> =
        twitter_usernames.iter().zip(twitter_user_ids).collect();
    'loop_user: for (&twitter_username, twitter_user_id) in twitter_username_id_pairs {
        info!("Starting to poll timeline for user: {}", twitter_username);

        let newest_tweet_id = String::from_utf8(
            db.get_cf_bytes(COLUMN_FAMILY_NEWEST_TWEET_ID, twitter_username)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        debug!("newest tweet id: {}", newest_tweet_id);

        // We just need default fields in returning object.
        let timeline_endpoint = timeline::UrlBuilder::new(&twitter_api_base_url, &twitter_user_id)
            .tweet_fields(vec!["created_at"])
            .max_results(100)
            .build();

        // Get column family name.
        let cf = Database::cf_poll_from_username(twitter_username, timestamp);
        // Continue from last tweet id in database.
        let paginated_timeline = PaginatedTimeline::new(
            &client,
            timeline_endpoint,
            &args.twitter_api_token,
            Some(PaginationToken::TweetID(newest_tweet_id.clone())),
        );

        for (i, timeline) in paginated_timeline.enumerate() {
            debug!("Current page: {}", i);

            let key = utils::timestamp();
            db.put_cf(&cf, key.to_string(), &timeline).unwrap();

            if timeline
                .data
                .as_deref()
                .unwrap()
                .iter()
                .any(|tweet| tweet.id.eq(&newest_tweet_id))
            {
                info!("Timeline hits newest tweet id, stop");
                continue 'loop_user;
            }
        }
    }

    let twitter_username_channel_pairs: Vec<(&&str, &str)> = twitter_usernames
        .iter()
        .zip(args.channel_usernames.split(','))
        .collect();
    for (&twitter_username, channel_username) in twitter_username_channel_pairs {
        info!(
            "Sync {}'s tweets to Telegram channel {}.",
            twitter_username, channel_username
        );

        let newest_tweet_id = String::from_utf8(
            db.get_cf_bytes(COLUMN_FAMILY_NEWEST_TWEET_ID, twitter_username)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        debug!("newest tweet id: {}", newest_tweet_id);
        let mut hit_newest_tweet_id = false;

        let cf = Database::cf_poll_from_username(twitter_username, timestamp);
        for (i, (_, value)) in db.iter_cf_since(&cf, None).unwrap().enumerate() {
            let timeline: Timeline = utils::deserialize_from_bytes(value.to_vec())
                .unwrap()
                .unwrap();

            for (ii, data) in timeline.data.unwrap().iter().rev().enumerate() {
                if !hit_newest_tweet_id {
                    if data.id.eq(&newest_tweet_id) {
                        debug!("Hit newest tweet id");
                        hit_newest_tweet_id = true;
                    }
                    continue;
                }

                info!(
                    "Current syncing tweet {} at current page {} of Twitter user {}, ",
                    ii, i, twitter_username
                );

                let message = telegram::Message {
                    chat_id: format!("@{}", channel_username),
                    text: format!("{}\n\n{}", data.text, data.created_at),
                };
                debug!("Post message: {:?}", message);

                let response = client
                    .post(send_message_endpoint.as_str())
                    .json(&message)
                    .send()
                    .unwrap();
                match response.status() {
                    StatusCode::OK => thread::sleep(Duration::from_secs(3)),
                    other => {
                        warn!(
                            "request not successful, got response status: {} and body: {}",
                            other,
                            response.text().unwrap_or_else(|_| "".to_string())
                        );
                        return;
                    }
                }
            }
            info!("Syncing finished for this timeline.");
        }
        info!(
            "Syncing finished for Twitter user {} to Telegram channel {}",
            twitter_username, channel_username
        );

        if !hit_newest_tweet_id {
            continue;
        }

        let (_, value) = db.first_kv_in_cf(&cf).expect("No new timeline to poll");
        let newest_id = utils::deserialize_from_bytes::<Timeline>(value.to_vec())
            .unwrap()
            .unwrap()
            .meta
            .newest_id;
        db.put_cf_bytes(COLUMN_FAMILY_NEWEST_TWEET_ID, twitter_username, newest_id)
            .unwrap();
    }
}
