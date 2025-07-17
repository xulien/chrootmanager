mod chroot;
mod cli;
mod config;
mod downloader;
mod error;
mod mirror;
mod profile;
mod tui;

use clap::Parser;
use cli::*;
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();
    let mut config = Config::load().await?;

    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Create {
            name,
            force_download,
        } => create_chroot(name, force_download, &config).await?,
        Commands::List => list_chroots(&config).await?,
        Commands::Mirror => setup_mirrors(&mut config).await?,
        Commands::Tui => tui::run(&config)?,
    };

    Ok(())
}
