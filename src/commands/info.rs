use std::str;
use tabled::{Table, Tabled};

use crate::{database::Database, twitter::Tweet};

pub(crate) fn info(database: &Database) -> anyhow::Result<()> {
    display_state(database)?;
    display_timeline(database)
}

fn display_state(database: &Database) -> anyhow::Result<()> {
    println!("Data in column family state:");
    let mut overview = vec![];
    for entry in database.iterator_cf("state").unwrap() {
        let (key, value) = entry?;
        let key_str = str::from_utf8(&key)?;
        let value_str = str::from_utf8(&value)?;
        overview.push(StateInfo {
            twitter_username: key_str.into(),
            last_tweet_datetime: value_str.into(),
        });
    }
    println!("{}", Table::new(overview));
    Ok(())
}

fn display_timeline(database: &Database) -> anyhow::Result<()> {
    // Start with en empty line.
    println!("\nData in column family timeline:");
    let timeline = database.iterator_cf("timeline").unwrap();
    for entry in timeline {
        let (key, value) = entry?;
        let key_str = str::from_utf8(&key)?;
        let value_str: Tweet = serde_json::from_slice(&value)?;
        println!("  {} = {:?}", key_str, value_str);
    }
    Ok(())
}

#[derive(Tabled)]
struct StateInfo {
    twitter_username: String,
    last_tweet_datetime: String,
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use rocksdb::{Options, DB};

    use super::info;
    use crate::{commands::Poll, config::PollConfig, database::Database};

    // To test this function:
    // RUST_LOG=debug cargo test get_info -- --ignored --show-output '[auth_token]'
    #[test_log::test(tokio::test)]
    #[ignore = "require command line input"]
    async fn get_info() {
        let mut args = std::env::args().rev();
        let auth_token = args.next();

        let rocksdb_path = "test";
        let database = Database::open(rocksdb_path);
        let client = Client::new();
        let poll_config = vec![PollConfig {
            included: true,
            username: "TwitterDev".into(),
            max_results: Some(5),
            start_time: Some("2022-10-25T00:00:00.000Z".into()),
            end_time: Some("2022-10-30T00:00:00.000Z".into()),
            since_id: None,
        }];

        let mut poll = Poll::new(auth_token, poll_config, &client, &database).unwrap();
        poll.run().await.unwrap();
        info(&database).unwrap();

        drop(database);
        DB::destroy(&Options::default(), rocksdb_path).unwrap();
    }
}
