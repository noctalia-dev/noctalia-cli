use std::process::Command;

use crate::config;
use crate::ui;

pub fn run(debug: bool) {
    ui::section("Run Noctalia Shell");
    
    // Check if shell is installed
    let (cfg, _path) = config::CliConfig::load().expect("load config");
    if !cfg.is_component_installed("shell") {
        ui::error("Noctalia shell is not installed. Run 'noctalia install shell' first.");
        std::process::exit(1);
    }

    if debug {
        ui::info("Debug mode enabled (NOCTALIA_DEBUG=1)");
    }
    
    ui::step("Starting noctalia-shell");
    
    // Execute qs -c noctalia-shell
    let mut cmd = Command::new("qs");
    cmd.arg("-c")
        .arg("noctalia-shell")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    
    // Set NOCTALIA_DEBUG=1 if debug flag is enabled
    if debug {
        cmd.env("NOCTALIA_DEBUG", "1");
    }
    
    let status = cmd.status();

    match status {
        Ok(exit_status) => {
            if !exit_status.success() {
                std::process::exit(exit_status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to start noctalia-shell: {}", e));
            ui::info("Make sure 'qs' (quickshell) is installed and available in your PATH.");
            std::process::exit(1);
        }
    }
}

