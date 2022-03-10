use clap::StructOpt;
use log::{debug, info};
use pigeon::{
    args::Args,
    client::ReusableBlockingClient,
    database::Database,
    twitter::{
        timeline::{PaginatedTimeline, Timeline, UrlBuilder},
        user::User,
        API_ENDPOINT_BASE,
    },
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use url::Url;

fn main() {
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let args = Args::parse();

    let client = ReusableBlockingClient::new(&args.twitter_api_token);

    let base_url =
        Url::parse(API_ENDPOINT_BASE).expect("could not parse twitter base api endpoint");

    let user_lookup_endpoint = User::url_from_usernames_query(&base_url, &args.usernames)
        .expect("could not construct url from given usernames");
    let user_ids = User::get_user_ids(&client, user_lookup_endpoint)
        .expect("could not get user_ids")
        .unwrap();

    // Column families formed from user_ids.
    let cfs: Vec<String> = user_ids
        .iter()
        .map(|user_id| Database::cf_timeline_from_user_id(user_id))
        .collect();
    let db = Database::open(&args.rocksdb_path, &cfs);
    // Column family handles map to user_ids.
    let cf_handles_map = db.get_cf_handles(&cfs);
    let last_items_in_cfs_map: HashMap<&str, Option<Timeline>> = db.last_items_in_cfs(&cfs);

    'loop_user_ids: for user_id in user_ids {
        info!("starting to archive timeline for user of id: {}", user_id);

        let timeline_endpoint = UrlBuilder::new(&base_url, &user_id)
            .tweet_fields(vec!["created_at", "attachments", "referenced_tweets"])
            .expansions(vec!["attachments.media_keys", "referenced_tweets.id"])
            .media_fields(vec!["height", "url", "width"])
            .max_results(100)
            .build();

        // Get column family name.
        let cf = Database::cf_timeline_from_user_id(&user_id);
        let pagination_token = match last_items_in_cfs_map.get(cf.as_str()).unwrap() {
            Some(timeline) => timeline.next_token(),
            None => None,
        };
        if pagination_token.is_some() {
            info!(
                "previous pages found in database, last pagination token: {}",
                pagination_token.as_ref().unwrap()
            );
        }

        let paginated_timeline =
            PaginatedTimeline::new(&client, timeline_endpoint, pagination_token);
        let mut page: u32 = 0;
        for timeline in paginated_timeline {
            page += 1;
            debug!("current page: {}", page);

            // Handle signal.
            if !running.load(Ordering::SeqCst) {
                info!("received ctl-c signal, stop task now");
                break 'loop_user_ids;
            }

            let cf_handle = cf_handles_map.get(cf.as_str()).unwrap().unwrap();
            db.put_cf(cf_handle, &page.to_string(), &timeline).unwrap();
        }
    }
}
