use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "chrootmanager",
    about = "Gentoo chroot manager with CLI interface",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new chroot
    Create {
        /// Chroot name
        name: String,
        /// Architecture
        #[arg(short, long)]
        arch: Option<String>,
        /// Profile
        #[arg(short, long)]
        profile: Option<String>,
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// List all chroots
    List {
        /// Interactive mode
        #[arg(short, long, default_value_t = false)]
        interactive: bool,
    },
    /// Configure mirrors
    Mirror {
        /// Mirror URL (optional in interactive mode)
        #[arg(index = 1)]
        new_mirror: Option<String>,
        /// Interactive mode
        #[arg(short, long, default_value_t = false)]
        interactive: bool,
    },
}