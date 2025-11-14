use std::{env, path::PathBuf, process::Command};

use crate::config;
use crate::ui;

fn find_shell_installation_path() -> Option<PathBuf> {
    // Check both possible installation locations
    let old_path = PathBuf::from("/etc/xdg/quickshell/noctalia-shell");
    let home = env::var("HOME").ok()?;
    let new_path = PathBuf::from(&home).join(".config/quickshell/noctalia-shell");
    
    if old_path.exists() {
        Some(old_path)
    } else if new_path.exists() {
        Some(new_path)
    } else {
        None
    }
}

fn is_systemd_running() -> bool {
    // Check if systemd is running by checking for /run/systemd/system
    // or by checking if systemctl exists and can be run
    if PathBuf::from("/run/systemd/system").exists() {
        return true;
    }
    
    // Fallback: try to run systemctl
    Command::new("systemctl")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn run() {
    ui::section("Install Systemd Service");
    
    // Check if shell is installed
    let (cfg, _path) = config::CliConfig::load().expect("load config");
    if !cfg.is_component_installed("shell") {
        ui::error("Noctalia shell is not installed. Run 'noctalia install shell' first.");
        std::process::exit(1);
    }
    
    // Check if systemd is running
    ui::step("Checking if systemd is available");
    if !is_systemd_running() {
        ui::error("Systemd is not running on this system.");
        ui::info("This command is only available on systems using systemd.");
        std::process::exit(1);
    }
    
    ui::info("Systemd is available");
    
    // Find the shell installation path
    let shell_path = match find_shell_installation_path() {
        Some(path) => path,
        None => {
            ui::error("Could not find noctalia-shell installation directory.");
            std::process::exit(1);
        }
    };
    
    // Locate the service file
    let service_file = shell_path.join("Assets/Services/systemd/noctalia.service");
    if !service_file.exists() {
        ui::error(&format!("Service file not found at: {}", service_file.display()));
        ui::info("The service file should be located at: Assets/Services/systemd/noctalia.service");
        std::process::exit(1);
    }
    
    ui::step("Installing systemd user service");
    ui::info("This operation requires sudo permissions. You will be prompted for your password.");
    
    // Create target directory and copy service file using sudo
    let target_dir = "/usr/lib/systemd/user";
    let target_file = format!("{}/noctalia.service", target_dir);
    
    // Use sudo to create directory, copy file, and set permissions
    let service_file_str = service_file.to_str().unwrap();
    let cmd = format!(
        "mkdir -p '{}' && cp '{}' '{}' && chmod 644 '{}'",
        target_dir, service_file_str, target_file, target_file
    );
    
    let status = Command::new("sudo")
        .args(["sh", "-c", &cmd])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    
    match status {
        Ok(exit_status) => {
            if !exit_status.success() {
                ui::error("Failed to install service file");
                std::process::exit(1);
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to install service file: {}", e));
            std::process::exit(1);
        }
    }
    
    ui::success("Service file installed successfully");
    
    // Reload systemd daemon
    ui::step("Reloading systemd daemon");
    let status = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    
    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                ui::success("Systemd daemon reloaded");
            } else {
                ui::error("Failed to reload systemd daemon");
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to reload systemd daemon: {}", e));
        }
    }
    
    // Ask if user wants to enable the service
    use dialoguer::{theme::ColorfulTheme, Confirm};
    let theme = ColorfulTheme::default();
    let should_enable = Confirm::with_theme(&theme)
        .with_prompt("Would you like to enable the noctalia.service?")
        .interact()
        .unwrap_or(false);
    
    if should_enable {
        ui::step("Enabling noctalia.service");
        let status = Command::new("systemctl")
            .args(["--user", "enable", "noctalia.service"])
            .status();
        
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    ui::success("Service enabled successfully");
                    
                    // Ask if user wants to start it now
                    let should_start = Confirm::with_theme(&theme)
                        .with_prompt("Would you like to start the service now?")
                        .interact()
                        .unwrap_or(false);
                    
                    if should_start {
                        ui::step("Starting noctalia.service");
                        let start_status = Command::new("systemctl")
                            .args(["--user", "start", "noctalia.service"])
                            .status();
                        
                        match start_status {
                            Ok(exit_status) => {
                                if exit_status.success() {
                                    ui::success("Service started successfully");
                                } else {
                                    ui::error("Failed to start service");
                                }
                            }
                            Err(e) => {
                                ui::error(&format!("Failed to start service: {}", e));
                            }
                        }
                    } else {
                        ui::info("Service enabled. You can start it later with:");
                        ui::info("  systemctl --user start noctalia.service");
                    }
                } else {
                    ui::error("Failed to enable service");
                }
            }
            Err(e) => {
                ui::error(&format!("Failed to enable service: {}", e));
            }
        }
    } else {
        ui::info("Service installed. You can enable it later with:");
        ui::info("  systemctl --user enable noctalia.service");
        ui::info("  systemctl --user start noctalia.service");
    }
}

