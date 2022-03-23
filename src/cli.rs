use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Archive users' Twitter timeline raw data to RocksDB.
    Archive(Archive),
    /// Sync tweets extracted from RocksDB raw timeline data to Telegram channel.
    Sync(Sync),
    /// Export all raw data from RocksDB.
    Export(Export),
}

#[derive(Debug, Args)]
pub struct Archive {
    /// Twitter api auth token.
    #[clap(long)]
    pub twitter_api_token: String,
    /// Path to RocksDB.
    #[clap(long)]
    pub rocksdb_path: PathBuf,
    /// Twitter usernames, it's a comma separated string.
    #[clap(long)]
    pub twitter_usernames: String,
}

#[derive(Debug, Args)]
pub struct Sync {
    /// Telegram bot api auth token.
    #[clap(long)]
    pub telegram_bot_api_token: String,
    /// Path to RocksDB.
    #[clap(long)]
    pub rocksdb_path: PathBuf,
    /// Twitter usernames, it's a comma separated string.
    #[clap(long)]
    pub twitter_usernames: String,
    /// Telegram channel usernames, it's a comma separated string.
    /// The channel username's order corresponds to that in the value of `twitter_usernames`, that is to say,
    /// one Twitter user to one Telegram channel.
    #[clap(long)]
    pub channel_usernames: String,
}

#[derive(Debug, Args)]
pub struct Export {
    /// Path to RocksDB.
    #[clap(long)]
    pub rocksdb_path: PathBuf,
}
