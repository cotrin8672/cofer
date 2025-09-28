use anyhow::{bail, Result};
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Platform-specific Podman socket paths
#[cfg(target_os = "windows")]
const DEFAULT_SOCKET: &str = "npipe:////./pipe/podman-machine-default";

#[cfg(unix)]
const DEFAULT_SOCKET: &str = "unix:///run/podman/podman.sock";

#[cfg(unix)]
const USER_SOCKET: &str = "unix:///run/user/1000/podman/podman.sock";

/// Podman diagnostics and pre-check utilities
pub struct PodmanDiagnostics;

impl PodmanDiagnostics {
    /// Check if Podman is available and running
    pub fn check_podman_available() -> Result<PodmanStatus> {
        // First, check if Podman command exists
        let version_result = Command::new("podman")
            .arg("version")
            .arg("--format")
            .arg("json")
            .output();

        match version_result {
            Ok(output) => {
                if output.status.success() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    debug!("Podman version output: {}", version_str);

                    // Try to parse version info
                    if let Ok(version_info) = serde_json::from_str::<serde_json::Value>(&version_str) {
                        let client_version = version_info["Client"]["Version"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();

                        info!("Podman client version: {}", client_version);

                        // Check if Podman service is running
                        let service_status = Self::check_service_status();

                        return Ok(PodmanStatus {
                            available: true,
                            version: Some(client_version),
                            service_running: service_status,
                            socket_path: Self::detect_socket_path(),
                        });
                    }
                }

                // Podman exists but version check failed
                warn!("Podman found but version check failed");
                Ok(PodmanStatus {
                    available: true,
                    version: None,
                    service_running: Self::check_service_status(),
                    socket_path: Self::detect_socket_path(),
                })
            }
            Err(_) => {
                error!("Podman command not found in PATH");
                Ok(PodmanStatus {
                    available: false,
                    version: None,
                    service_running: false,
                    socket_path: None,
                })
            }
        }
    }

    /// Check if Podman service/daemon is running
    fn check_service_status() -> bool {
        // Try a simple info command
        let result = Command::new("podman")
            .arg("info")
            .arg("--format")
            .arg("json")
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Detect the appropriate socket path for the current platform
    fn detect_socket_path() -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, check if Podman machine is running
            if Self::check_service_status() {
                Some(DEFAULT_SOCKET.to_string())
            } else {
                None
            }
        }

        #[cfg(unix)]
        {
            // Check various socket paths on Unix
            use std::path::Path;

            // Try system socket first
            if Path::new("/run/podman/podman.sock").exists() {
                return Some(DEFAULT_SOCKET.to_string());
            }

            // Try user socket with actual UID
            let uid = std::env::var("UID").unwrap_or_else(|_| "1000".to_string());
            let actual_user_socket = USER_SOCKET.replace("1000", &uid);
            if Path::new(&actual_user_socket.replace("unix://", "")).exists() {
                return Some(actual_user_socket);
            }

            // Check if DOCKER_HOST or CONTAINER_HOST is set
            if let Ok(host) = std::env::var("DOCKER_HOST") {
                return Some(host);
            }

            if let Ok(host) = std::env::var("CONTAINER_HOST") {
                return Some(host);
            }

            None
        }
    }

    /// Get startup instructions for the current platform
    pub fn get_startup_instructions() -> String {
        #[cfg(target_os = "windows")]
        {
            r#"Podman is not running. Please start it with one of these methods:

1. Start Podman Desktop application
2. Run in PowerShell (as Administrator):
   podman machine init
   podman machine start
3. For API access, run:
   podman system service --time=0

For more information: https://podman.io/docs/installation#windows"#.to_string()
        }

        #[cfg(target_os = "linux")]
        {
            r#"Podman is not running. Please start it with one of these methods:

1. For rootless mode (recommended):
   systemctl --user start podman.socket

2. For root mode:
   sudo systemctl start podman.socket

3. For API service:
   podman system service --time=0 unix:///tmp/podman.sock

For more information: https://podman.io/docs/installation#linux"#.to_string()
        }

        #[cfg(target_os = "macos")]
        {
            r#"Podman is not running. Please start it with one of these methods:

1. Start Podman Desktop application
2. Run in terminal:
   podman machine init
   podman machine start
3. For API access:
   podman system service --time=0

For more information: https://podman.io/docs/installation#macos"#.to_string()
        }
    }

    /// Diagnose and report Podman issues
    pub fn diagnose() -> Result<()> {
        let status = Self::check_podman_available()?;

        if !status.available {
            bail!(
                "Podman is not installed. Please install Podman first.\n\n{}",
                Self::get_installation_instructions()
            );
        }

        if !status.service_running {
            bail!(
                "Podman service is not running.\n\n{}",
                Self::get_startup_instructions()
            );
        }

        info!("Podman diagnostics passed");
        if let Some(version) = &status.version {
            info!("  Version: {}", version);
        }
        if let Some(socket) = &status.socket_path {
            info!("  Socket: {}", socket);
        }

        Ok(())
    }

    /// Get installation instructions for the current platform
    fn get_installation_instructions() -> String {
        #[cfg(target_os = "windows")]
        {
            "Install Podman Desktop from: https://podman-desktop.io/downloads".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            "Install with your package manager:\n  Ubuntu/Debian: sudo apt-get install podman\n  Fedora/RHEL: sudo dnf install podman\n  Arch: sudo pacman -S podman".to_string()
        }

        #[cfg(target_os = "macos")]
        {
            "Install with Homebrew: brew install podman\nOr download Podman Desktop from: https://podman-desktop.io/downloads".to_string()
        }
    }
}

/// Podman availability status
#[derive(Debug, Clone)]
pub struct PodmanStatus {
    /// Whether Podman command is available
    pub available: bool,
    /// Podman version if available
    pub version: Option<String>,
    /// Whether Podman service is running
    pub service_running: bool,
    /// Detected socket path
    pub socket_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_status_structure() {
        let status = PodmanStatus {
            available: true,
            version: Some("4.0.0".to_string()),
            service_running: true,
            socket_path: Some("unix:///run/podman/podman.sock".to_string()),
        };

        assert!(status.available);
        assert_eq!(status.version, Some("4.0.0".to_string()));
        assert!(status.service_running);
        assert!(status.socket_path.is_some());
    }

    #[test]
    fn test_startup_instructions_exist() {
        let instructions = PodmanDiagnostics::get_startup_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.contains("Podman"));
    }

    #[test]
    fn test_installation_instructions_exist() {
        let instructions = PodmanDiagnostics::get_installation_instructions();
        assert!(!instructions.is_empty());
    }

    // Integration test - will only pass if Podman is actually installed
    #[test]
    #[ignore] // Ignore by default since it requires Podman
    fn test_check_podman_available_integration() {
        let result = PodmanDiagnostics::check_podman_available();
        assert!(result.is_ok());

        let status = result.unwrap();
        if status.available {
            // If Podman is available, version should be detected
            assert!(status.version.is_some());
        }
    }

    #[test]
    fn test_diagnose_error_messages() {
        // This test checks that diagnose returns appropriate error messages
        let result = PodmanDiagnostics::diagnose();

        // The result depends on whether Podman is installed and running
        if result.is_err() {
            let err_msg = result.unwrap_err().to_string();
            // Should contain helpful information
            assert!(
                err_msg.contains("Podman") || err_msg.contains("not installed") || err_msg.contains("not running"),
                "Error message should be informative: {}",
                err_msg
            );
        }
    }
}