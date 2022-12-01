use crate::{database::Database, twitter::Tweet};
use std::str;

pub(crate) fn info(database: &Database) -> anyhow::Result<()> {
    display_state(database)?;
    display_timeline(database)
}

fn display_state(database: &Database) -> anyhow::Result<()> {
    println!("Data in column family state:");
    if let Some(state) = database.iterator_cf("state") {
        for entry in state {
            let (key, value) = entry?;
            let key_str = str::from_utf8(&key)?;
            let value_str = str::from_utf8(&value)?;
            println!("{} = {}", key_str, value_str);
        }
    }
    Ok(())
}

fn display_timeline(database: &Database) -> anyhow::Result<()> {
    println!("Data in column family timeline:");
    if let Some(timeline) = database.iterator_cf("timeline") {
        for entry in timeline {
            let (key, value) = entry?;
            let key_str = str::from_utf8(&key)?;
            let value_str: Tweet = serde_json::from_slice(&value)?;
            println!("{} = {:?}", key_str, value_str);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use rocksdb::{Options, DB};

    use super::info;
    use crate::{commands::Poll, config::PollConfig, database::Database};

    // To test this function:
    // RUST_LOG=debug cargo test get_info -- --ignored --show-output '[auth_token]'
    #[tokio::test]
    #[ignore = "require command line input"]
    async fn get_info() {
        init();

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

        let mut poll = Poll::new(auth_token, poll_config).unwrap();
        poll.run(&client, &database).await.unwrap();
        info(&database).unwrap();

        drop(database);
        DB::destroy(&Options::default(), rocksdb_path).unwrap();
    }

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
}
