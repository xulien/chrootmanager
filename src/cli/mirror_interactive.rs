use inquire::{InquireError, Select};
use crate::cli::error::ChrootManagerError;
use crate::cli::{configure_mirrors, load_config};
use colored::Colorize;

/// Sets up mirrors interactively by allowing the user to choose from options
pub async fn setup_mirrors_interactive() -> Result<(), ChrootManagerError> {
    let mut config = load_config().await?;

    let options = vec![
        "Select a mirror from the official list (recommended)",
        "Use Gentoo's default mirror",
    ];

    let mirror_configuration_select: Result<&str, InquireError> =
        Select::new("🔧 Mirror Configuration", options)
            .without_help_message()
            .prompt();

    match mirror_configuration_select {
        Ok(choice) => match choice {
            "Select a mirror from the official list (recommended)" => {
                configure_mirrors(&mut config).await?;
                // Save the configuration after configuring mirrors
                config.save()?;
            }
            "Use Gentoo's default mirror" => {
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
                println!("{}", "✅ Using Gentoo's default mirror".green().bold());
                // Save the configuration after setting the default mirror
                config.save()?;
            }
            _ => {
                println!("{}", "❌ Error during choice".red().bold());
                println!("Using the default mirror...");
                config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
                config.save()?;
            }
        },
        Err(e) => {
            println!("{}", format!("❌ Error during configuration: {e}").red().bold());
            println!("Using the default mirror...");
            config.mirrors_url = vec!["https://distfiles.gentoo.org/".to_string()];
            config.save()?;
        }
    }

    Ok(())
}
