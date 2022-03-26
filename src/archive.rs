use crate::{
    cli::Archive,
    database::{Database, COLUMN_FAMILY_NEWEST_TWEET_ID},
    twitter::{
        timeline::{PaginatedTimeline, PaginationToken, Timeline, UrlBuilder},
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
    let username_id_pairs: Vec<(&&str, String)> = usernames.iter().zip(user_ids).collect();

    // Column families formed from usernames.
    let cfs_existed = Database::list_cf(&args.rocksdb_path).unwrap();
    let mut cfs: Vec<String> = usernames
        .iter()
        .map(Database::cf_timeline_from_username)
        .chain(cfs_existed)
        .collect();
    cfs.push(COLUMN_FAMILY_NEWEST_TWEET_ID.to_string());
    let db = Database::open_with_cfs(&args.rocksdb_path, &cfs);

    'loop_username_id_pairs: for (&username, user_id) in username_id_pairs {
        info!("Starting to archive timeline for user: {}", username);

        // We just need default fields in returning object.
        let timeline_endpoint = UrlBuilder::new(&base_url, &user_id)
            .tweet_fields(vec!["created_at"])
            .max_results(100)
            .build();

        // Get column family name.
        let cf = Database::cf_timeline_from_username(&username);
        // Continue from next pagination token.
        let pagination_token = db.last_kv_in_cf(&cf).and_then(|(_, value)| {
            utils::deserialize_from_bytes::<Timeline>(value.to_vec())
                .unwrap()
                .unwrap()
                .meta
                .next_token
                .map(PaginationToken::NextToken)
        });

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
            db.put_cf(&cf, key.to_string(), &timeline).unwrap();
        }

        // Record newest tweet id of this user.
        let (_, value) = db.first_kv_in_cf(&cf).unwrap();
        let newest_id = utils::deserialize_from_bytes::<Timeline>(value.to_vec())
            .unwrap()
            .unwrap()
            .meta
            .newest_id;
        debug!("newest tweet id: {}", newest_id);

        db.put_cf_bytes(COLUMN_FAMILY_NEWEST_TWEET_ID, &username, newest_id)
            .unwrap();
    }
}
