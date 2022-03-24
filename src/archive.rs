use crate::{
    cli::Archive,
    database::Database,
    twitter::{
        timeline::{PaginatedTimeline, Timeline, UrlBuilder},
        user::User,
        API_ENDPOINT_BASE,
    },
    utils,
};
use log::{debug, info};
use reqwest::blocking::Client;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use url::Url;

pub fn archive(args: Archive) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Reusable HTTP client.
    let client = Client::new();

    let base_url =
        Url::parse(API_ENDPOINT_BASE).expect("could not parse twitter base api endpoint");

    let user_lookup_endpoint = User::url_from_usernames_query(&base_url, &args.twitter_usernames)
        .expect("could not construct url from given usernames");
    let user_ids = User::get_user_ids(&client, user_lookup_endpoint, &args.twitter_api_token)
        .expect("could not get user_ids")
        .unwrap();

    let usernames: Vec<&str> = args.twitter_usernames.split(',').collect();
    let username_id_pairs: Vec<(String, String)> = usernames
        .iter()
        .map(|&s| s.to_string())
        .zip(user_ids)
        .collect();

    // Column families formed from usernames.
    let mut cfs: Vec<String> = usernames
        .iter()
        .map(Database::cf_timeline_from_username)
        .collect();
    let mut cfs_existed = Database::list_cf(&args.rocksdb_path).unwrap();
    cfs.append(&mut cfs_existed);
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    'loop_username_id_pairs: for (username, user_id) in username_id_pairs {
        info!("Starting to archive timeline for user: {}", username);

        // We just need default fields in returning object.
        let timeline_endpoint = UrlBuilder::new(&base_url, &user_id)
            .tweet_fields(vec!["created_at"])
            .max_results(100)
            .build();

        // Get column family name.
        let cf = Database::cf_timeline_from_username(&username);
        // Get column family handle corresponds to this user id.
        let cf_handle = db.cf_handle(&cf).unwrap();
        // Continue from last pagination token.
        let pagination_token = match db.last_kv_in_cf(&cf) {
            Some((_, value)) => {
                utils::deserialize_from_bytes::<Timeline>(value.to_vec())
                    .unwrap()
                    .unwrap()
                    .meta
                    .next_token
            }
            None => None,
        };
        if pagination_token.is_some() {
            info!(
                "Previous pages found in database, last pagination token: {}",
                pagination_token.as_ref().unwrap()
            );
        }

        let paginated_timeline = PaginatedTimeline::new(
            &client,
            timeline_endpoint,
            &args.twitter_api_token,
            pagination_token,
        );

        for (i, timeline) in paginated_timeline.enumerate() {
            debug!("Current page: {}", i);

            // Handle signal.
            if !running.load(Ordering::SeqCst) {
                info!("Received ctl-c signal, stopping task now");
                break 'loop_username_id_pairs;
            }

            let key = utils::timestamp();
            db.put_cf(cf_handle, &key.to_string(), &timeline).unwrap();
        }
    }
}
