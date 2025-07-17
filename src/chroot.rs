use crate::config::Config;
use crate::error::ChrootManagerError;
use crate::profile::amd64::Amd64Profile;
use crate::profile::arch::Arch;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ChrootUnit {
    pub name: String,
    pub chroot_path: PathBuf,
    pub stage3_profile: Arch,
}

impl ChrootUnit {
    pub async fn new(name: String, profile: Option<&Arch>) -> Result<Self, ChrootManagerError> {
        let config = Config::load().await?;
        let stage3_profile = profile.unwrap_or(&Arch::Amd64(Amd64Profile::Openrc));
        let chroot_path = Path::new(&config.chroot_base_dir).join(&name);
        Ok(Self {
            name,
            chroot_path,
            stage3_profile: stage3_profile.clone(),
        })
    }
    pub fn load(path: &Path) -> Result<ChrootUnit, ChrootManagerError> {
        let name = path.file_name().unwrap().to_str().unwrap();
        log::debug!("load name: {name}");

        let profile = Arch::read_fs(path)?;
        log::debug!("load profile: {profile:?}");

        let unit = Self {
            name: name.to_string(),
            chroot_path: path.to_path_buf(),
            stage3_profile: profile,
        };
        log::debug!("load unit: {unit:?}");

        Ok(unit)
    }

    /// Prepare the chroot directory
    pub async fn prepare_chroot_directory(&self) -> Result<(), ChrootManagerError> {
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
    pub async fn extract_stage3(
        &self,
        cached_stage3_path: &Path,
    ) -> Result<(), ChrootManagerError> {
        log::info!(
            "Extracting from stage3: {} to {}",
            cached_stage3_path.display(),
            self.chroot_path.display()
        );

        let id_output = Command::new("id").output()?;

        if id_output.status.success() {
            let msg = String::from_utf8_lossy(&id_output.stdout);
            println!("{msg}");
        }

        let output = Command::new("tar")
            .arg("xpvf")
            .arg(cached_stage3_path)
            .arg("--xattrs-include='*.*'")
            .arg("--numeric-owner")
            .arg("-C")
            .arg(&self.chroot_path)
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ChrootManagerError::Command(format!(
                "Stage3 extraction failed: {error_msg}"
            )));
        }

        log::info!("Stage3 successfully extracted");
        Ok(())
    }

    fn proc_mount(&self) -> Result<&Self, ChrootManagerError> {
        let proc_output = Command::new("mount")
            .arg("-t")
            .arg("proc")
            .arg("/proc")
            .arg(self.chroot_path.join("proc"))
            .output()
            .map_err(ChrootManagerError::Io)?;

        if proc_output.status.success() {
            log::info!("/proc mount successful");
            if !proc_output.stdout.is_empty() {
                log::info!("Sortie : {}", String::from_utf8_lossy(&proc_output.stdout));
            }
            Ok(self)
        } else {
            let error_msg = String::from_utf8_lossy(&proc_output.stderr);
            log::error!("Error mounting /proc : {error_msg}");
            Err(ChrootManagerError::Command(format!(
                "Failed to mount /proc: {error_msg}"
            )))
        }
    }
    fn rbind_mount(&self, mount_point: &str) -> Result<&Self, ChrootManagerError> {
        let output = Command::new("mount")
            .arg("--rbind")
            .arg(format!("/{mount_point}"))
            .arg(self.chroot_path.join(mount_point))
            .output()
            .map_err(ChrootManagerError::Io)?;

        if output.status.success() {
            log::info!("rbind mount of /{mount_point} successful");
            if !output.stdout.is_empty() {
                log::info!("Out : {}", String::from_utf8_lossy(&output.stdout));
            }
            Ok(self)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log::error!("Error mounting rbind of /{mount_point} : {error_msg}");
            Err(ChrootManagerError::Command(format!(
                "rbind mount failed /{mount_point}: {error_msg}"
            )))
        }
    }

    fn make_rslave_mount(&self, mount_point: &str) -> Result<&Self, ChrootManagerError> {
        let output = Command::new("mount")
            .arg("--make-rslave")
            .arg(format!("{}/{}", self.chroot_path.display(), mount_point))
            .output()
            .map_err(|e| {
                log::error!("Error mounting make_rslave of /{mount_point} : {e}");
                ChrootManagerError::Io(e)
            })?;

        if output.status.success() {
            log::info!("make_rslave mount of /{mount_point} successful");
            if !output.stdout.is_empty() {
                log::info!("Out : {}", String::from_utf8_lossy(&output.stdout));
            }
            Ok(self)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(ChrootManagerError::Command(format!(
                "Error mounting make_rslave of /{mount_point}: {error_msg}"
            )))
        }
    }

    fn make_slave_mount(&self, mount_point: &str) -> Result<&Self, ChrootManagerError> {
        let output = Command::new("mount")
            .arg("--make-slave")
            .arg(format!("{}/{}", self.chroot_path.display(), mount_point))
            .output()
            .map_err(ChrootManagerError::Io)?;

        if output.status.success() {
            log::info!("make_slave mount of /{mount_point} successful");
            if !output.stdout.is_empty() {
                log::info!("Out : {}", String::from_utf8_lossy(&output.stdout));
            }
            Ok(self)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(ChrootManagerError::Command(format!(
                "Error while mounting make_slave of /{mount_point}: {error_msg}"
            )))
        }
    }

    /// Mounts the file systems needed for chroot
    pub fn mount_filesystems(&self) -> Result<&Self, ChrootManagerError> {
        log::info!("Mounting file systems for chroot");
        self.proc_mount()?;
        self.rbind_mount("sys")?;
        self.make_rslave_mount("sys")?;
        self.rbind_mount("dev")?;
        self.make_rslave_mount("dev")?;
        self.rbind_mount("run")?;
        self.make_slave_mount("run")?;
        Ok(self)
    }

    /// Copies DNS resolution files
    pub fn copy_dns_info(&self) -> Result<(), ChrootManagerError> {
        log::info!("Copy DNS information");

        let resolv_conf_src = Path::new("/etc/resolv.conf");
        let resolv_conf_dst = self.chroot_path.join("etc/resolv.conf");

        if resolv_conf_src.exists() {
            fs::copy(resolv_conf_src, resolv_conf_dst)?;
            log::info!("resolv.conf copied");
        }

        Ok(())
    }

    pub fn enter_chroot_interactive(&self) -> Result<(), ChrootManagerError> {
        println!("entry into the chroot:: {}", self.chroot_path.display());
        println!("Type 'exit' to exit the chroot");

        let status = Command::new("chroot").arg(&self.chroot_path).status()?;

        if status.success() {
            println!("Sortie du chroot");
            log::debug!("le chroot_path: {}", self.chroot_path.display());
            self.cleanup(false)?;
        } else {
            return Err(ChrootManagerError::Command(
                "Chroot execution failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Unmount all filesystems from the chroot
    pub fn unmount_filesystems(&self) -> Result<(), ChrootManagerError> {
        let dev_mount_points = vec!["/dev/shm", "/dev/pts", "/dev"];

        for mount_point in dev_mount_points {
            let full_path = format!("{}{}", self.chroot_path.display(), mount_point);
            log::debug!("umount full_path: {full_path}");

            let output = Command::new("umount")
                .arg("-l") // Lazy unmount
                .arg(&full_path)
                .output()?;

            if output.status.success() {
                log::info!("unmounted: {full_path}");
            } else {
                log::warn!(
                    "failed to unmount {}: {}",
                    full_path,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        let others_mount_points = vec!["/run", "/sys", "/proc"];

        for mount_point in others_mount_points {
            log::debug!(
                "le chroot_path dans la boucle dans unmount_filesystems {}",
                self.chroot_path.display()
            );
            let full_path = format!("{}{}", self.chroot_path.display(), mount_point);
            log::debug!("umount full_path: {full_path}");

            let output = Command::new("umount").arg("-R").arg(&full_path).output()?;

            if output.status.success() {
                log::info!("unmounted: {full_path}");
            } else {
                log::warn!(
                    "failed to unmount {}: {}",
                    full_path,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(())
    }
    /// Cleans the chroot (unmounts and optionally deletes)
    pub fn cleanup(&self, remove_directory: bool) -> Result<(), ChrootManagerError> {
        log::info!("Cleaning the chroot");

        self.unmount_filesystems()?;

        if remove_directory && self.chroot_path.exists() {
            fs::remove_dir_all(&self.chroot_path)?;
            log::info!("Deleted chroot directory: {:?}", self.chroot_path);
        }

        Ok(())
    }
}
