use crate::{
    cli::Sync,
    database::{self, Database},
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
    cfs.push(database::COLUMN_FAMILY_SYNC_CURSOR.to_string());
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    let cf_handle_sync_cursor = db.cf_handle(database::COLUMN_FAMILY_SYNC_CURSOR).unwrap();

    // A slice of tuples containing Twitter username and Telegram channel username.
    let pairs: Vec<(&str, &str)> = usernames
        .iter()
        .copied()
        .zip(args.channel_usernames.split(','))
        .collect();
    // Loop all Twitter users.
    for (twitter_username, channel_username) in pairs {
        info!(
            "Sync {}'s tweets to Telegram channel {}.",
            twitter_username, channel_username
        );

        // Get the position stopped at last time sync.
        let last_timeline =
            db.0.get_cf(cf_handle_sync_cursor, twitter_username)
                .unwrap();

        let cf_handle = db
            .cf_handle(Database::cf_timeline_from_username(twitter_username).as_str())
            .unwrap();
        // Loop all timeline.
        for (i, (key, value)) in db
            .iter_cf_since(cf_handle, last_timeline.as_deref())
            .enumerate()
        {
            // Get the position stopped at last time sync.
            // It must be initiated here in order to be updated on every loop
            // to make sure the following loop skipping check reads the new value
            // for every timeline.
            let last_timeline_index = match last_timeline.as_deref() {
                Some(key) => db.0.get_cf(cf_handle_sync_cursor, key).unwrap(),
                None => None,
            };

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
                            db.0.delete_cf(cf_handle_sync_cursor, twitter_username)
                                .unwrap();
                        }
                        if last_timeline_index.is_some() {
                            db.0.delete_cf(cf_handle_sync_cursor, &key).unwrap();
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
                        db.0.put_cf(cf_handle_sync_cursor, twitter_username, &key)
                            .unwrap();
                        db.0.put_cf(cf_handle_sync_cursor, &key, ii.to_string())
                            .unwrap();

                        return;
                    }
                }
            }
            info!("Syncing finished for this timeline.")
        }
        info!(
            "Syncing finished for Twitter user {} to Telegram channel {}",
            channel_username, twitter_username
        );

        // Once sync successfully for a Twitter user, update the last position.
        let (key, _) = db
            .last_kv_in_cf(&Database::cf_timeline_from_username(twitter_username))
            .unwrap();
        db.0.put_cf(cf_handle_sync_cursor, twitter_username, &key)
            .unwrap();
        db.0.put_cf(cf_handle_sync_cursor, key, 100.to_string())
            .unwrap();
    }
}
