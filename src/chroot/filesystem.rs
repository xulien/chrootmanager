use crate::error::{ChrootError, ElevationError};
use std::fs;
use std::path::Path;

use super::auth::SHARED_ELEVATION;

/// Filesystem operations for ChrootUnit
impl crate::chroot::core::ChrootUnit {
    /// Mount the necessary filesystems for chroot operation
    pub fn mount_filesystems(&self) -> Result<&Self, ChrootError> {
        if !self.is_authenticated() {
            return Err(ChrootError::Elevation(
                ElevationError::AuthenticationRequired,
            ));
        }

        log::info!("Mounting filesystems for chroot: {}", self.name);

        // Prepare all mount commands to execute in batch
        let proc_path = self.chroot_path.join("proc");
        let sys_path = self.chroot_path.join("sys");
        let dev_path = self.chroot_path.join("dev");
        let dev_pts_path = self.chroot_path.join("dev/pts");
        let dev_shm_path = self.chroot_path.join("dev/shm");

        let mount_commands = vec![
            // Mount proc
            (
                "mount",
                vec!["-t", "proc", "/proc", proc_path.to_str().unwrap()],
            ),
            // Mount sys with rbind
            ("mount", vec!["--rbind", "/sys", sys_path.to_str().unwrap()]),
            // Mount dev with rbind
            ("mount", vec!["--rbind", "/dev", dev_path.to_str().unwrap()]),
            // Mount dev/pts with rbind
            (
                "mount",
                vec!["--rbind", "/dev/pts", dev_pts_path.to_str().unwrap()],
            ),
            // Mount dev/shm with rbind
            (
                "mount",
                vec!["--rbind", "/dev/shm", dev_shm_path.to_str().unwrap()],
            ),
            // Make sys slave
            ("mount", vec!["--make-slave", sys_path.to_str().unwrap()]),
            // Make dev slave
            ("mount", vec!["--make-slave", dev_path.to_str().unwrap()]),
        ];

        let elevation = SHARED_ELEVATION.lock().unwrap();
        let results = elevation
            .execute_batch_commands(mount_commands)
            .map_err(ChrootError::from)?;

        // Check if all commands succeeded
        for (i, result) in results.iter().enumerate() {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                log::error!("Mount command {i} failed: {stderr}");
                return Err(ChrootError::Command(format!(
                    "Mount operation failed: {stderr}"
                )));
            }
        }

        log::info!(
            "Successfully mounted all filesystems for chroot: {}",
            self.name
        );
        Ok(self)
    }

    /// Copies DNS resolution files
    pub fn copy_dns_info(&self) -> Result<(), ChrootError> {
        log::info!("Copy DNS information with cached elevation");

        let resolv_conf_src = Path::new("/etc/resolv.conf");
        let resolv_conf_dst = self.chroot_path.join("etc/resolv.conf");

        if resolv_conf_src.exists() {
            let resolv_conf_src_str = resolv_conf_src.to_str().unwrap();
            let resolv_conf_dst_str = resolv_conf_dst.to_str().unwrap();

            let cp_args = vec![resolv_conf_src_str, resolv_conf_dst_str];

            self.execute_command_with_logging("cp", &cp_args, "DNS info copy")?;
            log::info!("resolv.conf copied with cached elevation");
        }

        Ok(())
    }

    /// Unmount without blocking
    pub fn unmount_filesystems(&self) -> Result<&Self, ChrootError> {
        if !self.is_authenticated() {
            return Err(ChrootError::Elevation(
                ElevationError::AuthenticationRequired,
            ));
        }

        log::info!("Cleaning up mount points for chroot: {}", self.name);

        let elevation = SHARED_ELEVATION.lock().unwrap();
        let chroot_path_str = self.chroot_path.to_string_lossy();

        // Use a single command to unmount everything recursively with a lazy option
        let unmount_result = elevation.execute_command(
            "umount",
            &[
                "-l", // lazy unmount
                "-R", // recursive
                &chroot_path_str,
            ],
        );

        match unmount_result {
            Ok(output) => {
                if output.status.success() {
                    log::info!("✓ Successfully unmounted all filesystems");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("not mounted") {
                        log::debug!("No filesystems were mounted");
                    } else {
                        log::warn!("Unmount warning: {stderr}");
                    }
                }
            }
            Err(e) => {
                log::warn!("Unmount command failed: {e}");
            }
        }

        // Additional cleanup with individual mount points if needed
        let mount_points = ["dev/shm", "dev/pts", "dev", "sys", "proc"];

        for mount_point in mount_points {
            let full_path = self.chroot_path.join(mount_point);
            if full_path.exists() {
                let path_str = full_path.to_string_lossy();
                let _ = elevation.execute_command("umount", &["-l", &path_str]);
            }
        }

        log::info!("Mount point cleanup completed");
        Ok(self)
    }

    /// Cleans the chroot (unmounts and optionally deletes)
    pub fn cleanup(&self, remove_directory: bool) -> Result<(), ChrootError> {
        log::info!("Cleaning the chroot");

        self.unmount_filesystems()?;

        if remove_directory && self.chroot_path.exists() {
            fs::remove_dir_all(&self.chroot_path)?;
            log::info!("Deleted chroot directory: {:?}", self.chroot_path);
        }

        Ok(())
    }
}