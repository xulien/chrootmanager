use crate::error::{ChrootError, ElevationError};
use std::fs;
use std::path::{Path, PathBuf};

use super::auth::SHARED_ELEVATION;

/// Terminal and interactive operations for ChrootUnit
impl crate::chroot::core::ChrootUnit {
    /// Enter chroot environment interactively
    pub fn enter_chroot_interactive(&self) -> Result<(), ChrootError> {
        if !self.is_authenticated() {
            return Err(ChrootError::Elevation(
                ElevationError::AuthenticationRequired,
            ));
        }

        log::info!("Entering chroot environment: {}", self.name);

        let chroot_path_str = self.chroot_path.to_str().unwrap();

        println!("🚀 Entering chroot environment '{}'...", self.name);
        println!("💡 Type 'exit' to quit the chroot environment");

        // Use shared business logic
        let bashrc_path = self.prepare_chroot_bashrc()?;

        // Use the cached elevation system instead of direct pkexec
        let elevation = SHARED_ELEVATION.lock().unwrap();
        let output = elevation
            .execute_command_interactive(
                "chroot",
                &[
                    chroot_path_str,
                    "/bin/bash",
                    "--rcfile",
                    "/tmp/chroot_bashrc",
                    "-i",
                ],
            )
            .map_err(|e| ChrootError::ElevationError(format!("Failed to enter chroot: {e}")))?;

        // Cleanup using shared logic
        self.cleanup_chroot_bashrc(&bashrc_path);

        if !output.status.success() {
            return Err(ChrootError::ElevationError(
                "Chroot execution failed".to_string(),
            ));
        }

        println!("✅ Exited chroot '{}'", self.name);
        log::info!("Successfully exited chroot environment: {}", self.name);
        Ok(())
    }

    /// Generate chroot command for external terminal (for GUI)
    /// This method is intended for future GUI integration
    #[allow(dead_code)]
    pub fn get_chroot_command_for_terminal(&self) -> Result<(String, PathBuf), ChrootError> {
        // Use shared business logic to prepare bashrc
        let bashrc_path = self.prepare_chroot_bashrc()?;
        let chroot_args = self.get_chroot_command_args(&bashrc_path);
        
        // Build the complete command string for the external terminal
        let chroot_command = format!("sudo chroot {}", chroot_args.join(" "));
        
        Ok((chroot_command, bashrc_path))
    }

    /// SHARED BUSINESS LOGIC: Generate chroot command arguments
    /// This method provides reusable logic for both CLI and future GUI interfaces
    #[allow(dead_code)]
    pub fn get_chroot_command_args(&self, bashrc_path: &Path) -> Vec<String> {
        let chroot_path_str = self.chroot_path.to_str().unwrap();
        let bashrc_relative = bashrc_path
            .strip_prefix(&self.chroot_path)
            .map(|p| format!("/{}", p.to_string_lossy()))
            .unwrap_or_else(|_| "/tmp/chroot_bashrc".to_string());

        vec![
            chroot_path_str.to_string(),
            "/bin/bash".to_string(),
            "--rcfile".to_string(),
            bashrc_relative,
            "-i".to_string(),
        ]
    }

    /// SHARED BUSINESS LOGIC: Cleanup temporary bashrc
    pub fn cleanup_chroot_bashrc(&self, bashrc_path: &Path) {
        if let Err(e) = fs::remove_file(bashrc_path) {
            let path_display = bashrc_path.display();
            log::debug!("Failed to clean up a bashrc file {path_display}: {e}");
        } else {
            let path_display = bashrc_path.display();
            log::debug!("Cleaned up a bashrc file: {path_display}");
        }
    }

    /// Prepare bashrc and return the path to it
    pub fn prepare_chroot_bashrc(&self) -> Result<PathBuf, ChrootError> {
        // Create a temporary bashrc file inside the chroot
        let bashrc_path = self.chroot_path.join("tmp/chroot_bashrc");

        // Ensure the tmp directory exists
        let tmp_dir = self.chroot_path.join("tmp");
        if !tmp_dir.exists() {
            fs::create_dir_all(&tmp_dir)?;
        }

        // Bashrc content (no shebang, no exec, just configuration)
        let bashrc_content = format!(
            r#"#!/bin/bash
export ENV="/tmp/chroot_env.sh"
cat > /tmp/chroot_env.sh << 'EOF'
source /etc/profile 2>/dev/null || true
export TERM=xterm-256color
eval "$(dircolors -b 2>/dev/null || true)"
alias ls='ls --color=auto'
alias ll='ls -l --color=auto'
alias la='ls -la --color=auto'
alias grep='grep --color=auto'
export PS1='\[\e[1;32m\](chroot) \[\e[01;31m\]{}\[\e[01;34m\] \w \$\[\e[00m\] '
EOF
exec bash --posix -i
"#,
            self.name
        );

        // Write the bashrc file
        fs::write(&bashrc_path, bashrc_content).map_err(ChrootError::Io)?;
        Ok(bashrc_path)
    }
}