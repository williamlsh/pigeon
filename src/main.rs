use clap::{Parser, Subcommand};
use env_logger::Env;
use pigeon::{App, Config};
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};

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
        true => setup_log("debug"),
        false => setup_log("info"),
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

fn setup_log(level: &str) {
    env_logger::Builder::from_env(Env::default().default_filter_or(level))
        .format_timestamp_secs()
        .init();
}
