use std::process::Command;

use crate::config;
use crate::ui;

fn is_noctalia_running() -> bool {
    // Check if quickshell is running with noctalia-shell
    // We check for processes that match "qs" and contain "noctalia-shell"
    let output = Command::new("pgrep")
        .args(["-f", "qs.*noctalia-shell"])
        .output();
    
    match output {
        Ok(output) => output.status.success(),
        Err(_) => {
            // If pgrep fails, try using ps as fallback
            let ps_output = Command::new("ps")
                .args(["-eo", "cmd"])
                .output();
            
            match ps_output {
                Ok(ps_output) => {
                    let stdout = String::from_utf8_lossy(&ps_output.stdout);
                    stdout.lines().any(|line| {
                        line.contains("qs") && line.contains("noctalia-shell")
                    })
                }
                Err(_) => false,
            }
        }
    }
}

fn check_prerequisites() {
    // Check if shell is installed
    let (cfg, _path) = config::CliConfig::load().expect("load config");
    if !cfg.is_component_installed("shell") {
        ui::error("Noctalia shell is not installed. Run 'noctalia install shell' first.");
        std::process::exit(1);
    }

    // Check if noctalia-shell is running (only show message if not running)
    if !is_noctalia_running() {
        ui::error("Noctalia shell is not running. Run 'noctalia run' first.");
        std::process::exit(1);
    }
}

pub fn run_call(target: String, function: String) {
    ui::section("Noctalia IPC Call");
    check_prerequisites();
    
    ui::step(&format!("Sending IPC call: {} {}", target, function));
    
    // Execute qs -c noctalia-shell ipc call <target> <function>
    let status = Command::new("qs")
        .arg("-c")
        .arg("noctalia-shell")
        .arg("ipc")
        .arg("call")
        .arg(&target)
        .arg(&function)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(exit_status) => {
            if !exit_status.success() {
                std::process::exit(exit_status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to send IPC call: {}", e));
            ui::info("Make sure 'qs' (quickshell) is installed and available in your PATH.");
            std::process::exit(1);
        }
    }
}

fn format_function_signature(func_sig: &str) -> String {
    // Parse function signature like "set(path: string, screen: string): void"
    // and format it as "set(path, screen)"
    
    if let Some(paren_start) = func_sig.find('(') {
        let func_name = &func_sig[..paren_start];
        let rest = &func_sig[paren_start + 1..];
        
        if let Some(paren_end) = rest.find(')') {
            let params = &rest[..paren_end];
            
            // Extract parameter names (remove types)
            let param_names: Vec<String> = params
                .split(',')
                .map(|p| {
                    let p = p.trim();
                    // Remove type annotation (e.g., "path: string" -> "path")
                    if let Some(colon_pos) = p.find(':') {
                        p[..colon_pos].trim().to_string()
                    } else {
                        p.to_string()
                    }
                })
                .filter(|p| !p.is_empty())
                .collect();
            
            if param_names.is_empty() {
                func_name.to_string()
            } else {
                format!("{}({})", func_name, param_names.join(", "))
            }
        } else {
            func_name.to_string()
        }
    } else {
        func_sig.to_string()
    }
}

fn format_ipc_show_output(output: &str) {
    let mut current_target: Option<String> = None;
    let mut functions: Vec<String> = Vec::new();
    
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        if line.starts_with("target ") {
            // If we have a previous target, format it
            if let Some(target) = current_target.take() {
                ui::info(&format!("{}", target));
                for func in &functions {
                    println!("  • {}", func);
                }
                println!();
                functions.clear();
            }
            // Set new target
            current_target = Some(line.trim_start_matches("target ").to_string());
        } else if line.starts_with("function ") {
            // Extract function signature and format it
            let func_sig = line.trim_start_matches("function ");
            let formatted = format_function_signature(func_sig);
            functions.push(formatted);
        }
    }
    
    // Handle the last target
    if let Some(target) = current_target {
        ui::info(&format!("{}", target));
        for func in &functions {
            println!("  • {}", func);
        }
    }
}

pub fn run_show() {
    ui::section("Noctalia IPC Show");
    check_prerequisites();
    
    ui::step("Fetching available IPC targets and functions");
    
    // Execute qs -c noctalia-shell ipc show
    let output = Command::new("qs")
        .arg("-c")
        .arg("noctalia-shell")
        .arg("ipc")
        .arg("show")
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                ui::error("Failed to get IPC information");
                std::process::exit(output.status.code().unwrap_or(1));
            }
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stdout.trim().is_empty() {
                ui::info("No IPC targets found");
            } else {
                ui::info("Available IPC Targets and Functions:");
                println!();
                format_ipc_show_output(&stdout);
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to get IPC information: {}", e));
            ui::info("Make sure 'qs' (quickshell) is installed and available in your PATH.");
            std::process::exit(1);
        }
    }
}

