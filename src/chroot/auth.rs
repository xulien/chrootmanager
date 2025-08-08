use crate::elevation::{SecureElevation, get_global_elevation};
use crate::error::{ChrootError, ElevationError};
use std::sync::{Arc, Mutex};

// Shared global instance of the elevation system with cache
pub static SHARED_ELEVATION: std::sync::LazyLock<Arc<Mutex<SecureElevation>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(SecureElevation::new())));

/// Authentication and elevation methods for ChrootUnit
impl crate::chroot::core::ChrootUnit {
    /// Execute a command with shared cached elevation
    pub fn execute_elevated(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<std::process::Output, ChrootError> {
        let elevation = SHARED_ELEVATION.lock().unwrap();
        elevation
            .execute_command(command, args)
            .map_err(ChrootError::from)
    }

    pub fn execute_command_with_logging(
        &self,
        command: &str,
        args: &[&str],
        operation_desc: &str,
    ) -> Result<std::process::Output, ChrootError> {
        log::debug!("Executing {command} with cached elevation: {args:?}");

        let output = self.execute_elevated(command, args)?;

        if output.status.success() {
            log::info!("{operation_desc} successful");
            if !output.stdout.is_empty() {
                log::info!("Output: {}", String::from_utf8_lossy(&output.stdout));
            }
            Ok(output)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log::error!("Error during {operation_desc}: {error_msg}");
            Err(ChrootError::Command(format!(
                "{operation_desc} failed: {error_msg}"
            )))
        }
    }

    /// Invalidate the shared elevation cache (useful for security)
    /// This method is important for security cleanup when privileges are no longer needed
    #[allow(dead_code)]
    pub fn invalidate_elevation_cache(&self) {
        if let Ok(elevation) = SHARED_ELEVATION.lock() {
            elevation.invalidate_cache();
            log::info!(
                "Shared elevation cache invalidated by chroot: {}",
                self.name
            );
        }
    }

    /// Check if elevation is currently cached
    /// This method is useful for determining if authentication is needed before operations
    #[allow(dead_code)]
    pub fn is_elevation_cached(&self) -> bool {
        if let Ok(elevation) = SHARED_ELEVATION.lock() {
            elevation.is_authenticated()
        } else {
            false
        }
    }

    /// Pre-authenticate for upcoming privileged operations to avoid multiple password prompts
    /// This should be called before performing mount and chroot operations
    pub fn pre_authenticate_operations(&self) -> Result<(), ChrootError> {
        let elevation = get_global_elevation();
        let elevation_guard = elevation
            .lock()
            .map_err(|_| ChrootError::Elevation(ElevationError::FailedToAcquireElevationLock))?;

        elevation_guard
            .pre_authenticate()
            .map_err(ChrootError::Elevation)?;

        log::info!("Successfully pre-authenticated for chroot operations");
        Ok(())
    }

    /// Check if we have cached authentication for privileged operations
    pub fn is_authenticated(&self) -> bool {
        if let Ok(elevation) = get_global_elevation().lock() {
            elevation.is_authenticated()
        } else {
            false
        }
    }

    /// Invalidate the authentication cache (useful for cleanup or error recovery)
    /// This method is important for security cleanup after operations are complete
    #[allow(dead_code)]
    pub fn invalidate_authentication(&self) {
        if let Ok(elevation) = get_global_elevation().lock() {
            elevation.invalidate_cache();
            log::debug!("Authentication cache invalidated for chroot operations");
        }
    }
}