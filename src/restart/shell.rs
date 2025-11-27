use std::process::Command;

use crate::config;
use crate::ui;

fn is_systemd_running() -> bool {
    // Check if systemd is running by checking for /run/systemd/system
    // or by checking if systemctl exists and can be run
    if std::path::PathBuf::from("/run/systemd/system").exists() {
        return true;
    }
    
    // Fallback: try to run systemctl
    Command::new("systemctl")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn is_service_enabled() -> bool {
    // Check if the systemd service is enabled
    Command::new("systemctl")
        .args(["--user", "is-enabled", "noctalia.service"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn restart_systemd_service() -> Result<(), Box<dyn std::error::Error>> {
    ui::step("Restarting noctalia.service via systemd");
    
    let status = Command::new("systemctl")
        .args(["--user", "restart", "noctalia.service"])
        .status()?;
    
    if status.success() {
        ui::success("Service restarted successfully");
        Ok(())
    } else {
        Err("Failed to restart systemd service".into())
    }
}

fn restart_manual() -> Result<(), Box<dyn std::error::Error>> {
    ui::step("Stopping existing noctalia-shell processes");
    
    // Kill existing qs processes running noctalia-shell
    let status = Command::new("pkill")
        .args(["-f", "qs.*noctalia-shell"])
        .status();
    
    match status {
        Ok(exit_status) => {
            if exit_status.success() || exit_status.code() == Some(1) {
                // Exit code 1 means no processes found, which is fine
                ui::info("Stopped existing processes (or none were running)");
            } else {
                ui::error("Failed to stop existing processes");
                return Err("pkill failed".into());
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to run pkill: {}", e));
            return Err("pkill command failed".into());
        }
    }
    
    // Wait a moment for processes to fully stop
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    ui::step("Starting noctalia-shell");
    
    // Start noctalia-shell using the run command
    let status = Command::new("qs")
        .arg("-c")
        .arg("noctalia-shell")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    
    match status {
        Ok(exit_status) => {
            if !exit_status.success() {
                std::process::exit(exit_status.code().unwrap_or(1));
            }
            Ok(())
        }
        Err(e) => {
            ui::error(&format!("Failed to start noctalia-shell: {}", e));
            ui::info("Make sure 'qs' (quickshell) is installed and available in your PATH.");
            Err("Failed to start noctalia-shell".into())
        }
    }
}

pub fn run() {
    ui::section("Restart Noctalia Shell");
    
    // Check if shell is installed
    let (cfg, _path) = config::CliConfig::load().expect("load config");
    if !cfg.is_component_installed("shell") {
        ui::error("Noctalia shell is not installed. Run 'noctalia install shell' first.");
        std::process::exit(1);
    }
    
    // Check if systemd is available and service is enabled
    if is_systemd_running() && is_service_enabled() {
        ui::info("Detected systemd service is enabled");
        match restart_systemd_service() {
            Ok(()) => {}
            Err(e) => {
                ui::error(&format!("Failed to restart via systemd: {}", e));
                ui::info("Falling back to manual restart");
                if let Err(e) = restart_manual() {
                    ui::error(&format!("Failed to restart manually: {}", e));
                    std::process::exit(1);
                }
            }
        }
    } else {
        ui::info("Using manual restart method");
        if let Err(e) = restart_manual() {
            ui::error(&format!("Failed to restart: {}", e));
            std::process::exit(1);
        }
    }
}

