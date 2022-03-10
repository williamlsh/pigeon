use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub twitter_api_token: String,
    /// usernames are comma separated string.
    #[clap(short, long)]
    pub usernames: String,
    #[clap(short, long)]
    pub rocksdb_path: PathBuf,
}
