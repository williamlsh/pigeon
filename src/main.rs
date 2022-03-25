use clap::StructOpt;
use pigeon::{
    archive::archive,
    cli::{Cli, Commands},
    export::export,
    poll::poll,
    sync::sync,
};

fn main() {
    env_logger::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Archive(args) => archive(args),
        Commands::Sync(args) => sync(args),
        Commands::Export(args) => export(args),
        Commands::Poll(args) => poll(args),
    }
}
