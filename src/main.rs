use clap::{Parser, Subcommand};
use pigeon::{App, Config};
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

#[derive(Parser, Debug)]
struct Cli {
    /// Activate debug mode
    #[arg(short, long, action)]
    debug: bool,

    /// Config file path
    #[arg(short, long, value_name = "config.toml")]
    config_path: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Poll Twitter users' timeline
    Poll,
    /// Push timeline to Telegram channel(s)
    Push,
    /// Display overview information about Database
    Info,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.debug {
        true => setup_logging("debug"),
        false => setup_logging("info"),
    }
    let config = load_config(cli.config_path).await?;
    let mut app = App::new(config);
    match cli.command {
        Command::Poll => app.poll().await?,
        Command::Push => app.push().await?,
        Command::Info => app.info()?,
    }
    Ok(())
}

async fn load_config(path: PathBuf) -> anyhow::Result<Config> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    let config: Config = toml::from_slice(&buf)?;
    Ok(config)
}

fn setup_logging(level: &str) {
    let fmt_layer = tracing_subscriber::fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter_layer)
        .init();
}
