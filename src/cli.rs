use crate::{
    chroot::ChrootUnit,
    config::Config,
    downloader::download_stage3_with_cache,
    error::ChrootManagerError,
    profile::{amd64::Amd64Profile, arch::Arch, arm64::Arm64Profile},
};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::{InquireError, Select};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use strum::IntoEnumIterator;

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
        /// Force download even if the file exists in the cache
        #[arg(long)]
        force_download: bool,
    },
    /// List all chroots
    List,
    Mirror,
}

pub fn amd64_profile_selection() -> Result<Arch, ChrootManagerError> {
    let profile_list = Amd64Profile::iter().collect::<Vec<_>>();

    let profile_selection: Result<Amd64Profile, InquireError> =
        Select::new("What's your Amd64 profile ?", profile_list).prompt();
    let selected_profile = profile_selection?;

    Ok(Arch::Amd64(selected_profile))
}

pub fn arm64_profile_selection() -> Result<Arch, ChrootManagerError> {
    let profile_list = Arm64Profile::iter().collect::<Vec<_>>();

    let profile_selection: Result<Arm64Profile, InquireError> =
        Select::new("What's your Arm64 profile ?", profile_list).prompt();
    let selected_profile = profile_selection?;

    Ok(Arch::Arm64(selected_profile))
}

pub async fn create_chroot(
    name: String,
    force_download: bool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "📦 Création du chroot...".green().bold());
    println!(
        "   📂 Répertoire de base : {}",
        config.chroot_base_dir.display()
    );

    config.ensure_chroot_base_dir()?;

    let arch_list = Arch::iter()
        .map(|p| p.arch().to_string())
        .collect::<Vec<_>>();
    let arch_selection: Result<String, InquireError> =
        Select::new("What's your Arch ?", arch_list).prompt();
    let arch_selection = arch_selection?;

    let selected_profile = match arch_selection.as_str() {
        "amd64" => amd64_profile_selection()?,
        "arm64" => arm64_profile_selection()?,
        _ => panic!("Invalid architecture"),
    };

    let chroot_unit = ChrootUnit::new(name.clone(), Some(&selected_profile)).await?;

    log::debug!("chroot path: {:?}", chroot_unit.chroot_path);

    // Check if chroot already exists
    if chroot_unit.chroot_path.exists() {
        println!(
            "{}",
            format!("⚠️ The chroot '{}' already exists.", chroot_unit.name)
                .yellow()
                .bold()
        );
        print!("Do you want to delete and recreate it? ? (o/N) : ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase().starts_with('o') {
            println!("{}", "🗑️ Removing the old chroot...".red().bold());
            chroot_unit.cleanup(true)?;
            println!("✅ Old chroot deleted");
        } else {
            return Err(format!(
                "The chroot '{}' already exists. Use another name or delete it first.",
                chroot_unit.name
            )
            .into());
        }
    }

    let cached_path =
        download_stage3_with_cache(&selected_profile.clone(), config, force_download).await?;
    let cached_path = PathBuf::from(cached_path);

    chroot_unit.prepare_chroot_directory().await?;
    chroot_unit.extract_stage3(&cached_path).await?;
    chroot_unit.copy_dns_info()?;

    println!(
        "{}",
        format!("✅ Chroot '{name}' created successfully!")
            .green()
            .bold()
    );
    println!("📍 Path : {}", chroot_unit.chroot_path.display());

    list_chroots(config).await?;

    Ok(())
}

pub async fn list_chroots(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "   📂 Chroot Directory : {}",
        config.chroot_base_dir.display()
    );

    if !config.chroot_base_dir.exists() {
        println!("   ❌ Chroot directory not found");
        println!("   The directory will be created when the chroot is first created");
        println!("   Make sure you have permissions to create chroots");
        return Ok(());
    }

    let rd = std::fs::read_dir(&config.chroot_base_dir);

    if let Err(e) = rd {
        println!("   ❌ Directory access error : {e}");
        println!(
            "   💡 Check permissions for : {}",
            config.chroot_base_dir.display()
        );
        return Ok(());
    }

    let dirs = rd
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_dir())
        .collect::<Vec<_>>();

    let units = dirs
        .iter()
        .map(|p| ChrootUnit::load(p))
        .collect::<Result<Vec<ChrootUnit>, ChrootManagerError>>()?;

    if units.is_empty() {
        println!(
            "   📭 No chroot found in : {}",
            config.chroot_base_dir.display()
        );
        println!("   💡 Use 'create' to create your first chroot");
    }

    let units_choices = units.iter().map(|u| u.name.as_str()).collect::<Vec<_>>();

    let units_selected: Result<&str, InquireError> =
        Select::new("📋 List of chroots", units_choices).prompt();
    let units_selected = units_selected?;
    let unit: Vec<&ChrootUnit> = units.iter().filter(|u| u.name.eq(units_selected)).collect();
    let unit = unit[0];

    unit.mount_filesystems()?.enter_chroot_interactive()?;

    Ok(())
}

pub async fn setup_mirrors(config: &mut Config) -> Result<(), ChrootManagerError> {
    let options = vec![
        "Select mirror from the official list (recommended)",
        "Use Gentoo's default mirror",
    ];

    let mirror_configuration_select: Result<&str, InquireError> =
        Select::new("🔧 Mirror Configuration", options).prompt();

    match mirror_configuration_select {
        Ok(choice) => match choice {
            "Select mirror from the official list (recommended)" => {
                config.configure_mirrors().await?
            }
            "Use Gentoo's default mirror" => {
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
                println!("✅ Using Gentoo's Default Mirror");
            }
            _ => {
                println!("❌ Error during choice");
                println!("Using the default mirror...");
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
            }
        },
        Err(e) => {
            println!("❌ Error during configuration : {e}");
            println!("Using the default mirror...");
            config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
        }
    }

    Ok(())
}
