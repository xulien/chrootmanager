mod cli;
mod error;
mod config;
mod chroot;
mod downloader;
mod profile;
mod mirror;
mod elevation;

use clap::Parser;
use cli::command::{Cli, Commands};
use cli::create_interactive::create_chroot_interactive;
use cli::create::create_chroot;
use cli::list_interactive::list_chroots_interactive;
use cli::mirror_interactive::setup_mirrors_interactive;
use cli::mirror::setup_mirrors;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::List { interactive: true }) {
        Commands::Create { name, arch, profile, interactive } => {
            if interactive {
                // Interactive mode explicitly requested with -i
                create_chroot_interactive(name).await?
            } else {
                match (arch, profile) {
                    (Some(arch), Some(profile)) => {
                        // Non-interactive mode with all parameters provided
                        create_chroot(name, arch, profile).await?
                    },
                    _ => {
                        // Default non-interactive mode, but missing parameters
                        eprintln!("❌ Error: Architecture and profile required in non-interactive mode");
                        eprintln!("💡 Use -i for interactive mode or specify -a <arch> -p <profile>");
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::List { interactive } => {
            if interactive {
                list_chroots_interactive().await?
            } else {
                cli::list::list_chroots().await?
            }
        },
        Commands::Mirror { new_mirror, interactive } => {
            if interactive {
                setup_mirrors_interactive().await?
            } else {
                match new_mirror {
                    None => {
                        eprintln!("❌ Error: A mirror URL is required in non-interactive mode");
                        std::process::exit(1);
                    }
                    Some(new_mirror) => setup_mirrors(new_mirror).await?
                }
            }
        },
    };

    Ok(())
}