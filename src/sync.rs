use crate::{
    cli::Sync,
    database::{self, Database},
    telegram,
    twitter::timeline::Timeline,
    utils,
};
use log::{debug, info};
use reqwest::{blocking::Client, StatusCode};

// A key to record last index synced in a timeline of Twitter user,
// in the format, timeline_index_cursor:<twitter_user>.
const TIMELINE_INDEX_CURSOR_KEY: &str = "timeline_index_cursor";

pub fn sync(args: Sync) {
    let client = Client::new();
    let send_message_endpoint =
        telegram::url_from_method(&args.telegram_bot_api_token, "sendMessage").unwrap();

    let usernames: Vec<&str> = args.twitter_usernames.split(',').collect();
    let mut cfs: Vec<String> = usernames
        .iter()
        .map(Database::cf_timeline_from_username)
        .collect();
    // A column family to record last key to sync in database.
    cfs.push(database::COLUMN_FAMILY_SYNC_CURSOR.to_string());
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    let cf_handle_last_key_to_sync = db.cf_handle(database::COLUMN_FAMILY_SYNC_CURSOR).unwrap();

    // A slice of tuples containing Twitter username and Telegram channel username.
    let pairs: Vec<(&str, &str)> = usernames
        .iter()
        .copied()
        .zip(args.channel_usernames.split(','))
        .collect();
    for (twitter_username, channel_username) in pairs {
        info!(
            "Sync {}'s tweets to Telegram channel {}.",
            twitter_username, channel_username
        );

        let last_key_to_sync =
            db.0.get_cf(cf_handle_last_key_to_sync, twitter_username)
                .unwrap();
        let last_timeline_index_to_sync =
            db.0.get_cf(
                cf_handle_last_key_to_sync,
                format!("{}:{}", TIMELINE_INDEX_CURSOR_KEY, twitter_username),
            )
            .unwrap();

        let cf_handle = db
            .cf_handle(Database::cf_timeline_from_username(twitter_username).as_str())
            .unwrap();
        db.iter_cf_since(cf_handle, last_key_to_sync.as_deref())
            .enumerate()
            .for_each(|(i, (key, value))| {
                let timeline: Timeline = utils::deserialize_from_bytes(value.to_vec())
                    .unwrap()
                    .unwrap();
                timeline
                    .data
                    .unwrap()
                    .iter().rev()
                    .enumerate()
                    .for_each(|(ii, data)| {
                        if let Some(index) = last_timeline_index_to_sync.as_deref() {
                            let index = String::from_utf8(index.to_vec()).unwrap();
                            if (ii as u8) < index.parse::<u8>().unwrap() {
                                debug!("Skip items already synced before index: {}", ii);
                                // Skip items that already synced.
                                return;
                            }
                        }
                        info!("Current item: {} at current page: {} of Twitter user {}, ", ii, i, twitter_username);

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
                            // Limit request rate, 1s per request.
                            StatusCode::OK => {}
                            StatusCode::TOO_MANY_REQUESTS => {
                                info!("Save this un-synced key in database");
                                // A record to mark sync position, user -> key -> timeline index.
                                // Record this key as value in with its column family as key to database.
                                db.0.put_cf(cf_handle_last_key_to_sync, twitter_username, &key)
                                    .unwrap();
                                    // We also need to record the index of this timeline.
                                    db.0.put_cf(cf_handle_last_key_to_sync, format!("{}:{}", TIMELINE_INDEX_CURSOR_KEY, twitter_username), ii.to_string()).unwrap();

                                panic!(
                                    "Telegram bot api rate limit reached, please retry after a while: {}",
                                    response.text().unwrap_or_else(|_| "".to_string())
                                );
                            }
                            x => panic!(
                                "request not successful, got response status: {} and body: {}",
                                x,
                                response.text().unwrap_or_else(|_| "".to_string())
                            ),
                        }


                    });
            });
        info!("Syncing finished for Twitter user: {}", twitter_username);

        // Once sync successfully for a Twitter user, record the last cursor.
        let (key, _) = db
            .last_kv_in_cf(&Database::cf_timeline_from_username(twitter_username))
            .unwrap();
        db.0.put_cf(cf_handle_last_key_to_sync, twitter_username, key)
            .unwrap();
        db.0.put_cf(
            cf_handle_last_key_to_sync,
            format!("{}:{}", TIMELINE_INDEX_CURSOR_KEY, twitter_username),
            100.to_string(),
        )
        .unwrap();
    }
}
