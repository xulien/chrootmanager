use crate::config::Config;
use crate::error::ChrootError;
use std::fs;
use std::path::{Path, PathBuf};
use crate::profile::selected::SelectedProfile;

#[derive(Debug, Clone)]
pub struct ChrootUnit {
    pub name: String,
    pub chroot_path: PathBuf,
    pub profile: Option<SelectedProfile>,
}

impl ChrootUnit {
    pub async fn new(
        name: String,
        profile: Option<&SelectedProfile>,
        config: &Config,
    ) -> Result<Self, ChrootError> {
        let chroot_path = Path::new(&config.chroot_base_dir).join(&name);

        Ok(Self {
            name,
            chroot_path,
            profile: profile.cloned(),
        })
    }

    pub fn load(path: &Path) -> Result<ChrootUnit, ChrootError> {
        let name = path.file_name().unwrap().to_str().unwrap();
        log::debug!("load name: {name}");

        let mut unit = Self {
            name: name.to_string(),
            chroot_path: path.to_path_buf(),
            profile: None,
        };
        
        // Try to read the profile info
        match unit.read_arch_profile_info() {
            Ok(profile_info) => {
                // Profile info is in the format "architecture-profile"
                if let Some(separator_pos) = profile_info.find('-') {
                    let architecture = profile_info[..separator_pos].to_string();
                    let profile = profile_info[separator_pos + 1..].to_string();
                    unit.profile = Some(crate::profile::selected::SelectedProfile::new(architecture, profile));
                }
            },
            Err(_) => {
                // Keep profile as None if we can't read it
                log::debug!("Could not read profile info for chroot: {name}");
            }
        }
        
        log::debug!("load unit: {unit:?}");

        Ok(unit)
    }

    /// Prepare the chroot directory
    pub async fn prepare_chroot_directory(&self) -> Result<(), ChrootError> {
        log::info!(
            "Creating the chroot directory: {}",
            self.chroot_path.display()
        );
        if self.chroot_path.exists() {
            log::warn!(
                "The chroot directory already exists: {}",
                self.chroot_path.display()
            );
        } else {
            fs::create_dir_all(&self.chroot_path)?;
        }
        Ok(())
    }

    /// Extract stage3 into the chroot directory
    pub async fn extract_stage3(&self, cached_stage3_path: &Path) -> Result<(), ChrootError> {
        log::info!(
            "Extracting from stage3: {} to {}",
            cached_stage3_path.display(),
            self.chroot_path.display()
        );

        let cached_stage3_path_str = cached_stage3_path.to_str().unwrap();
        let chroot_path_str = self.chroot_path.to_str().unwrap();

        let tar_args = vec![
            "xpvf",
            cached_stage3_path_str,
            "--xattrs-include=*.*",
            "--numeric-owner",
            "-C",
            chroot_path_str,
        ];

        self.execute_command_with_logging("tar", &tar_args, "Stage3 extraction")?;
        log::info!("Stage3 successfully extracted");
        Ok(())
    }

    /// Write Profile info
    pub fn write_arch_profile_info(&self) -> Result<(), ChrootError> {
        if let Some(profile) = &self.profile {
            let profile_info = format!("{}-{}", profile.architecture, profile.profile);
            let profile_path = self.chroot_path.join("etc/arch-chroot-profile");

            let temp_file = format!("/tmp/arch-chroot-profile-{}", std::process::id());

            fs::write(&temp_file, &profile_info).map_err(ChrootError::Io)?;

            self.execute_elevated("mv", &[&temp_file, &profile_path.to_string_lossy()])?;

            log::debug!("Profile info written to {}", profile_path.display());
            Ok(())
        } else {
            Err(ChrootError::NoProfile)
        }
    }

    pub fn read_arch_profile_info(&self) -> Result<String, ChrootError> {
        let profile_path = self.chroot_path.join("etc/arch-chroot-profile");

        if !profile_path.exists() {
            log::debug!("Profile file doesn't exist: {}", profile_path.display());
            return Err(ChrootError::NoProfile);
        }

        match fs::read_to_string(&profile_path) {
            Ok(content) => Ok(content.trim().to_string()),
            Err(e) => {
                log::debug!("Failed to read a profile file: {e}");
                Err(ChrootError::Io(e))
            }
        }
    }

    /// Find all chroot units in the configured directory
    /// This function is intended for bulk operations and GUI integration
    #[allow(dead_code)]
    pub fn find_units(config: &Config) -> Result<Vec<ChrootUnit>, ChrootError> {
        let rd = fs::read_dir(&config.chroot_base_dir);

        if let Err(e) = rd {
            return Err(ChrootError::from(e));
        }

        let dirs = rd?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|e| e.is_dir())
            .collect::<Vec<_>>();

        let units = dirs
            .iter()
            .map(|p| ChrootUnit::load(p))
            .collect::<Result<Vec<ChrootUnit>, ChrootError>>()?;

        Ok(units)
    }
}