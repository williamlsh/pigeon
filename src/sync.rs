use crate::{
    cli::Sync,
    database::{Database, COLUMN_FAMILY_SYNC_CURSOR},
    telegram,
    twitter::timeline::Timeline,
    utils,
};
use log::{debug, info, warn};
use reqwest::{blocking::Client, StatusCode};
use std::{thread, time::Duration};

pub fn sync(args: Sync) {
    let client = Client::new();
    let send_message_endpoint =
        telegram::url_from_method(&args.telegram_bot_api_token, "sendMessage").unwrap();

    let usernames: Vec<&str> = args.twitter_usernames.split(',').collect();
    let mut cfs: Vec<String> = Database::list_cf(&args.rocksdb_path).unwrap();
    // A column family to record last position to sync in database.
    cfs.push(COLUMN_FAMILY_SYNC_CURSOR.to_string());
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    // A slice of tuples containing Twitter username and Telegram channel username.
    let pairs: Vec<(&&str, &str)> = usernames
        .iter()
        .zip(args.channel_usernames.split(','))
        .collect();
    // Loop all Twitter users.
    for (&twitter_username, channel_username) in pairs {
        info!(
            "Sync {}'s tweets to Telegram channel {}.",
            twitter_username, channel_username
        );

        // Get the position stopped at last time sync.
        let last_timeline = db
            .get_cf_bytes(COLUMN_FAMILY_SYNC_CURSOR, twitter_username)
            .unwrap();

        // Loop all timeline.
        for (i, item) in db
            .iter_cf_since(
                &Database::cf_timeline_from_username(twitter_username),
                last_timeline.as_deref(),
            )
            .unwrap()
            .enumerate()
        {
            let (key, value) = item.unwrap();
            // Get the position stopped at last time sync.
            // It must be initiated here in order to be updated on every loop
            // to make sure the following loop skipping check reads the new value
            // for every timeline.
            let last_timeline_index = last_timeline
                .as_deref()
                .and_then(|key| db.get_cf_bytes(COLUMN_FAMILY_SYNC_CURSOR, key).unwrap());

            let timeline: Timeline = utils::deserialize_from_bytes(value.to_vec())
                .unwrap()
                .unwrap();
            // Loop all tweets in timeline.
            // Reverse the iterator to adjust time order of tweets from old to new.
            for (ii, data) in
                timeline
                    .data
                    .unwrap()
                    .iter()
                    .rev()
                    .enumerate()
                    .skip_while(|(ii, _)| match last_timeline_index.as_deref() {
                        Some(index) => {
                            let index = String::from_utf8(index.to_vec()).unwrap();
                            (*ii as u8) < index.parse::<u8>().unwrap()
                        }
                        None => false,
                    })
            {
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
                    StatusCode::OK => {
                        // Delete last sync position on request success.
                        // So the next timeline iteration (not tweets iteration inside timeline) won't be based on old position
                        // which is supposed to be starting at 0.
                        if last_timeline.is_some() {
                            db.delete_cf(COLUMN_FAMILY_SYNC_CURSOR, twitter_username)
                                .unwrap();
                        }
                        if last_timeline_index.is_some() {
                            db.delete_cf(COLUMN_FAMILY_SYNC_CURSOR, &key).unwrap();
                        }

                        thread::sleep(Duration::from_secs(3));
                    }
                    other => {
                        warn!(
                            "request not successful, got response status: {} and body: {}",
                            other,
                            response.text().unwrap_or_else(|_| "".to_string())
                        );

                        info!("Save current position in database, index: {}", ii);
                        // Mark sync position, Twitter user -> timeline -> timeline index.
                        db.put_cf(COLUMN_FAMILY_SYNC_CURSOR, twitter_username, &key)
                            .unwrap();
                        db.put_cf(COLUMN_FAMILY_SYNC_CURSOR, &key, &ii.to_string())
                            .unwrap();

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

        // Once sync successfully for a Twitter user, update the last position.
        let (key, _) = db
            .last_kv_in_cf(&Database::cf_timeline_from_username(twitter_username))
            .unwrap()
            .unwrap();
        db.put_cf(COLUMN_FAMILY_SYNC_CURSOR, twitter_username, &key)
            .unwrap();
        db.put_cf(COLUMN_FAMILY_SYNC_CURSOR, key, &100.to_string())
            .unwrap();
    }
}
