use crate::error::ElevationError;
use log::{debug, info, warn};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::thread;

/// Global shared elevation instance to maintain authentication across operations
static GLOBAL_ELEVATION: OnceLock<Arc<Mutex<SecureElevation>>> = OnceLock::new();

/// Get the global elevation instance
pub(crate) fn get_global_elevation() -> Arc<Mutex<SecureElevation>> {
    GLOBAL_ELEVATION
        .get_or_init(|| Arc::new(Mutex::new(SecureElevation::new())))
        .clone()
}

/// Authentication cache to avoid repeated elevation requests
#[derive(Debug)]
pub(crate) struct ElevationCache {
    authenticated: Arc<Mutex<bool>>,
    cache_duration: Duration,
    last_auth: Arc<Mutex<Option<Instant>>>,
    session_keeper_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl ElevationCache {
    /// Creates a new cache with a validity duration in minutes
    pub fn new(cache_duration_minutes: u64) -> Self {
        Self {
            authenticated: Arc::new(Mutex::new(false)),
            cache_duration: Duration::from_secs(cache_duration_minutes * 60),
            last_auth: Arc::new(Mutex::new(None)),
            session_keeper_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Checks if authentication is still valid in the cache
    pub fn is_authenticated(&self) -> bool {
        let authenticated = self.authenticated.lock().unwrap();
        let last_auth = self.last_auth.lock().unwrap();

        if let Some(last) = *last_auth {
            if last.elapsed() < self.cache_duration {
                return *authenticated;
            }
        }
        false
    }

    /// Authenticates the user and starts a session keeper
    pub fn authenticate(&self) -> Result<(), ElevationError> {
        // Check if already authenticated in a cache
        if self.is_authenticated() {
            debug!("Using cached sudo authentication");
            return Ok(());
        }

        info!("Requesting sudo authentication for privileged operations...");
        
        // Test sudo access and establish a session
        let output = Command::new("sudo")
            .arg("-v") // Validate and extend sudo timeout
            .output()?;

        if output.status.success() {
            *self.authenticated.lock().unwrap() = true;
            *self.last_auth.lock().unwrap() = Some(Instant::now());
            
            // Start background session keeper
            self.start_session_keeper();
            
            info!(
                "Sudo authentication successful - session maintained for {} minutes",
                self.cache_duration.as_secs() / 60
            );
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Sudo authentication failed: {stderr}");
        Err(ElevationError::AccessDenied)
    }

    /// Starts a background thread to keep the sudo session alive
    fn start_session_keeper(&self) {
        let authenticated = Arc::clone(&self.authenticated);
        let cache_duration = self.cache_duration;
        
        // Stop any existing session keeper
        self.stop_session_keeper();
        
        let handle = thread::spawn(move || {
            let mut iterations = 0;
            let max_iterations = (cache_duration.as_secs() / 60) as usize; // One iteration per minute
            
            while iterations < max_iterations {
                thread::sleep(Duration::from_secs(60)); // Wait 1 minute
                
                // Check if we should continue keeping the session alive
                if let Ok(auth_status) = authenticated.lock() {
                    if !*auth_status {
                        debug!("Session keeper stopping - authentication invalidated");
                        break;
                    }
                } else {
                    break;
                }
                
                // Refresh sudo timestamp
                let result = Command::new("sudo")
                    .arg("-n") // Non-interactive
                    .arg("-v") // Validate/extend timeout
                    .output();
                
                match result {
                    Ok(output) => {
                        if !output.status.success() {
                            debug!("Sudo session expired, stopping session keeper");
                            if let Ok(mut auth_status) = authenticated.lock() {
                                *auth_status = false;
                            }
                            break;
                        }
                        debug!("Sudo session refreshed by keeper");
                    }
                    Err(_) => {
                        debug!("Failed to refresh sudo session, stopping keeper");
                        break;
                    }
                }
                
                iterations += 1;
            }
            
            debug!("Sudo session keeper thread terminated");
        });
        
        *self.session_keeper_handle.lock().unwrap() = Some(handle);
    }

    /// Stops the session keeper thread
    fn stop_session_keeper(&self) {
        if let Ok(mut handle_option) = self.session_keeper_handle.lock() {
            if let Some(handle) = handle_option.take() {
                // Signal the thread to stop by invalidating authentication
                if let Ok(mut auth_status) = self.authenticated.lock() {
                    *auth_status = false;
                }
                
                // Optionally wait for the thread to finish
                if let Err(e) = handle.join() {
                    debug!("Session keeper thread panicked: {e:?}");
                } else {
                    debug!("Session keeper thread stopped cleanly");
                }
            }
        } else {
            debug!("Failed to acquire session keeper handle lock");
        }
    }

    /// Invalidates the authentication cache and stops session keeper
    pub fn invalidate(&self) {
        *self.authenticated.lock().unwrap() = false;
        *self.last_auth.lock().unwrap() = None;
        self.stop_session_keeper();
        debug!("Sudo elevation cache invalidated");
    }
}

/// Secure elevation manager with cache
#[derive(Debug)]
pub(crate) struct SecureElevation {
    cache: ElevationCache,
}

impl SecureElevation {
    /// Creates a new instance with a 45-minute cache
    pub fn new() -> Self {
        if !is_sudo_available() {
            warn!("sudo is not available on this system");
        } else {
            info!("Using sudo for privilege elevation with session management");
        }

        Self {
            cache: ElevationCache::new(45), // 45-minute cache for GUI sessions
        }
    }

    /// Pre-authenticate to establish a sudo session and avoid multiple password prompts
    pub fn pre_authenticate(&self) -> Result<(), ElevationError> {
        if !is_sudo_available() {
            return Err(ElevationError::SudoNotAvailable);
        }

        if !self.cache.is_authenticated() {
            info!("Establishing sudo session for upcoming privileged operations...");
            self.cache.authenticate()?;
        } else {
            debug!("Using an existing sudo session");
        }
        Ok(())
    }

    /// Executes a command with privilege elevation using sudo
    pub fn execute_command(&self, command: &str, args: &[&str]) -> Result<Output, ElevationError> {
        if !is_sudo_available() {
            return Err(ElevationError::SudoNotAvailable);
        }

        // Ensure we're authenticated
        if !self.cache.is_authenticated() {
            warn!("No active sudo session, attempting to authenticate...");
            self.cache.authenticate()?;
        }

        let mut cmd = Command::new("sudo");
        cmd.arg("-n"); // Non-interactive mode (will fail if the session expired)
        cmd.arg(command);
        for arg in args {
            cmd.arg(arg);
        }

        debug!("Executing with sudo: {} {}", command, args.join(" "));
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if stderr.contains("a password is required") || stderr.contains("sorry, try again") {
                warn!("Sudo session expired, invalidating cache");
                self.cache.invalidate();
                return Err(ElevationError::AccessDenied);
            }
            
            if stderr.contains("Permission denied") || stderr.contains("permission denied") {
                warn!("Permission denied: {stderr}");
                return Err(ElevationError::PermissionDenied);
            }
        }
        
        Ok(output)
    }

    /// Executes a command interactively (for chroot operations)
    pub fn execute_command_interactive(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<Output, ElevationError> {
        if !is_sudo_available() {
            return Err(ElevationError::SudoNotAvailable);
        }

        if !self.is_authenticated() {
            return Err(ElevationError::AuthenticationRequired);
        }

        let mut cmd = Command::new("sudo");
        cmd.arg("-n"); // Non-interactive for privilege escalation
        cmd.arg(command);
        cmd.args(args);
        cmd.stdin(std::process::Stdio::inherit());
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());

        let output = cmd.output()?;
        Ok(output)
    }

    /// Batch executes multiple commands to optimize sudo session usage
    pub fn execute_batch_commands(&self, commands: Vec<(&str, Vec<&str>)>) -> Result<Vec<Output>, ElevationError> {
        if !is_sudo_available() {
            return Err(ElevationError::SudoNotAvailable);
        }

        // Ensure we're authenticated before batch execution
        if !self.cache.is_authenticated() {
            self.cache.authenticate()?;
        }

        let mut results = Vec::new();
        for (command, args) in commands {
            let result = self.execute_command(command, &args)?;
            results.push(result);
        }
        
        Ok(results)
    }

    /// Checks if authentication is cached
    pub fn is_authenticated(&self) -> bool {
        self.cache.is_authenticated()
    }

    /// Invalidates the authentication cache and stops session keeper
    /// This method is important for security cleanup and error recovery
    #[allow(dead_code)]
    pub fn invalidate_cache(&self) {
        self.cache.invalidate();
    }
}

impl Default for SecureElevation {
    fn default() -> Self {
        Self::new()
    }
}

/// Checks if sudo is available on the system
pub(crate) fn is_sudo_available() -> bool {
    Command::new("which")
        .arg("sudo")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}