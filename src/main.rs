use clap::{Parser, Subcommand};
use pigeon::Config;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Parser, Debug)]
struct Cli {
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let mut config = load_config(cli.config_path).await?;

    match cli.command {
        Command::Poll => {}
        Command::Push => {}
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
