use std::{env, fs, path::PathBuf, process::Command};

use crate::SourceKind;
use crate::config;
use crate::ui;

const REPO_API: &str = "https://api.github.com/repos/noctalia-dev/noctalia-shell";
const REPO_CODELOAD_MAIN: &str = "https://codeload.github.com/noctalia-dev/noctalia-shell/tar.gz/refs/heads/main";
const TARGET_ROOT: &str = "/etc/xdg/quickshell/noctalia-shell";

pub fn run(source: SourceKind) {
    ui::section("Noctalia Shell");
    ui::info(&format!("Source: {}", source));
    ui::info("Installing into /etc/xdg/quickshell/noctalia-shell requires sudo permissions. You will be prompted via sudo.");

    // Install dependencies first
    ui::section("Installing Dependencies");
    let required_packages = vec!["quickshell", "gpu-screen-recorder", "brightnessctl"];
    match install_dependencies(&required_packages) {
        Ok(()) => {
            ui::success("All dependencies installed successfully");
        }
        Err(e) => {
            ui::error(&format!("Failed to install dependencies: {}", e));
            ui::section("Installation Aborted");
            ui::error("Cannot proceed with shell installation until all dependencies are available.");
            ui::info("Please install the missing packages manually and run the installation again.");
            std::process::exit(1);
        }
    }

    let version = match source {
        SourceKind::Git => {
            ui::step("Fetching latest commit from git main");
            let commit_sha = match get_latest_commit_sha() {
                Ok(sha) => sha,
                Err(e) => {
                    ui::error(&format!("Failed to fetch latest commit: {}", e));
                    std::process::exit(1);
                }
            };
            let display = if commit_sha.len() >= 8 { &commit_sha[..8] } else { commit_sha.as_str() };
            ui::info(&format!("Latest commit: {}", display));
            ui::step("Downloading (git main)");
            if let Err(e) = download_and_extract_git_main() {
                ui::error(&format!("Failed to install noctalia-shell (git): {}", e));
                std::process::exit(1);
            } else {
                ui::info("Completed (git main)");
            }
            commit_sha
        }
        SourceKind::Release => {
            ui::step("Fetching latest release");
            let release_info = match get_latest_release_info() {
                Ok(info) => info,
                Err(e) => {
                    ui::error(&format!("Failed to fetch latest release: {}", e));
                    std::process::exit(1);
                }
            };
            ui::info(&format!("Latest release: {}", release_info.tag_name));
            ui::step("Downloading (latest release)");
            if let Err(e) = download_and_extract_latest_release() {
                ui::error(&format!("Failed to install noctalia-shell (release): {}", e));
                std::process::exit(1);
            } else {
                ui::info("Completed (latest release)");
            }
            release_info.tag_name
        }
    };

    let (mut cfg, path) = config::CliConfig::load().expect("load config");
    cfg.set_component_source("shell", source);
    cfg.set_installed("shell", true);
    cfg.set_component_version("shell", version);
    let _ = cfg.save(&path);
    ui::success(&format!("Installed to {}", TARGET_ROOT));
}

fn downloads_dir() -> PathBuf {
    // Prefer $HOME/Downloads on Linux; create if missing
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(home).join("Downloads");
    if let Err(e) = fs::create_dir_all(&path) {
        eprintln!("Warning: could not create Downloads dir ({}), falling back to /tmp", e);
        return PathBuf::from("/tmp");
    }
    path
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("noctalia-cli (+https://github.com/noctalia-dev/noctalia)")
        .build()
        .expect("failed to build http client")
}

fn download_git_main() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let client = http_client();
    let resp = client.get(REPO_CODELOAD_MAIN).send()?;
    if !resp.status().is_success() { return Err(format!("http {}", resp.status()).into()); }
    let bytes = resp.bytes()?;
    let out = downloads_dir().join("noctalia-shell-main.tar.gz");
    fs::write(&out, &bytes)?;
    Ok(out)
}

#[derive(serde::Deserialize)]
struct ReleaseInfo { 
    tag_name: String, 
    tarball_url: String 
}

#[derive(serde::Deserialize)]
struct CommitInfo {
    sha: String,
}

fn get_latest_commit_sha() -> Result<String, Box<dyn std::error::Error>> {
    let client = http_client();
    let url = format!("{}/commits/main", REPO_API);
    let commit: CommitInfo = client.get(url).send()?.json()?;
    Ok(commit.sha)
}

fn get_latest_release_info() -> Result<ReleaseInfo, Box<dyn std::error::Error>> {
    let client = http_client();
    let url = format!("{}/releases/latest", REPO_API);
    let info: ReleaseInfo = client.get(url).send()?.json()?;
    Ok(info)
}

fn download_latest_release() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let client = http_client();
    let info = get_latest_release_info()?;
    let resp = client.get(info.tarball_url).send()?;
    if !resp.status().is_success() { return Err(format!("http {}", resp.status()).into()); }
    let bytes = resp.bytes()?;
    let filename = format!("noctalia-shell-{}.tar.gz", info.tag_name);
    let out = downloads_dir().join(filename);
    fs::write(&out, &bytes)?;
    Ok(out)
}

fn download_and_extract_git_main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = download_git_main()?;
    extract_with_sudo(&archive)?;
    // Remove the archive to leave only the folder
    let _ = fs::remove_file(&archive);
    Ok(())
}

fn download_and_extract_latest_release() -> Result<(), Box<dyn std::error::Error>> {
    let archive = download_latest_release()?;
    extract_with_sudo(&archive)?;
    // Remove the archive to leave only the folder
    let _ = fs::remove_file(&archive);
    Ok(())
}

fn shell_quote(path: &PathBuf) -> String {
    let s = path.display().to_string();
    s.replace("'", "'\\''")
}

fn extract_with_sudo(archive_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let arch_q = shell_quote(archive_path);
    let target_q = shell_quote(&PathBuf::from(TARGET_ROOT));
    let cmd = format!(
        "rm -rf '{target}' && mkdir -p '{target}' && tar -xzf '{arch}' -C '{target}' --strip-components=1",
        target = target_q,
        arch = arch_q
    );

    ui::info("Elevating with sudo. You may be prompted for your password.");

    let status = Command::new("sudo")
        .args(["sh", "-c", &cmd])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err("sudo command failed".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Distribution {
    Arch,
    Fedora,
    Debian,
    Gentoo,
    Void,
    Unknown,
}

fn detect_distribution() -> Distribution {
    // Check /etc/os-release first (most reliable for modern distros)
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        let mut id_value: Option<String> = None;
        let mut id_like_value: Option<String> = None;
        
        // Parse ID and ID_LIKE fields from os-release
        for line in content.lines() {
            if line.starts_with("ID=") {
                let id = line.trim_start_matches("ID=").trim_matches('"').trim_matches('\'').to_string();
                id_value = Some(id);
            } else if line.starts_with("ID_LIKE=") {
                let id_like = line.trim_start_matches("ID_LIKE=").trim_matches('"').trim_matches('\'').to_string();
                id_like_value = Some(id_like);
            }
        }
        
        // Check ID first
        if let Some(id) = &id_value {
            match id.as_str() {
                // Arch and Arch-based distributions
                "arch" | "archlinux" | "archarm" | "archcraft" | "cachyos" | "endeavouros" | "manjaro" | "manjaro-arm" | "arcolinux" | "artix" | "garuda" | "parabola" => return Distribution::Arch,
                "void" => return Distribution::Void,
                "fedora" => return Distribution::Fedora,
                "debian" => return Distribution::Debian,
                "ubuntu" => return Distribution::Debian,
                "gentoo" => return Distribution::Gentoo,
                _ => {}
            }
        }
        
        // Check ID_LIKE for forks that don't have explicit ID matches
        if let Some(id_like) = &id_like_value {
            if id_like.contains("arch") {
                return Distribution::Arch;
            }
            if id_like.contains("debian") || id_like.contains("ubuntu") {
                return Distribution::Debian;
            }
            if id_like.contains("fedora") {
                return Distribution::Fedora;
            }
        }
    }

    // Fallback to traditional detection methods
    if PathBuf::from("/etc/arch-release").exists() {
        return Distribution::Arch;
    }
    if PathBuf::from("/etc/fedora-release").exists() || PathBuf::from("/etc/redhat-release").exists() {
        return Distribution::Fedora;
    }
    if PathBuf::from("/etc/debian_version").exists() {
        return Distribution::Debian;
    }
    if PathBuf::from("/etc/gentoo-release").exists() {
        return Distribution::Gentoo;
    }
    
    Distribution::Unknown
}

fn get_package_mapping(dist: Distribution) -> Vec<(&'static str, Option<&'static str>)> {
    // Returns (generic_name, distro_specific_name)
    // None means package doesn't exist in this distro
    match dist {
        Distribution::Arch => vec![
            ("quickshell", Some("quickshell")),
            ("gpu-screen-recorder", Some("gpu-screen-recorder")),
            ("brightnessctl", Some("brightnessctl")),
        ],
        Distribution::Fedora => vec![
            ("quickshell", None), // May need COPR or manual build
            ("gpu-screen-recorder", Some("gpu-screen-recorder")),
            ("brightnessctl", Some("brightnessctl")),
        ],
        Distribution::Debian => vec![
            ("quickshell", None), // May need PPA or manual build
            ("gpu-screen-recorder", Some("gpu-screen-recorder")),
            ("brightnessctl", Some("brightnessctl")),
        ],
        Distribution::Gentoo => vec![
            ("quickshell", None), // May need overlay
            ("gpu-screen-recorder", Some("gpu-screen-recorder")),
            ("brightnessctl", Some("brightnessctl")),
        ],
        Distribution::Void => vec![
            ("quickshell", Some("quickshell")),
            ("gpu-screen-recorder", Some("gpu-screen-recorder")),
            ("brightnessctl", Some("brightnessctl")),
        ],
        Distribution::Unknown => vec![
            ("quickshell", None),
            ("gpu-screen-recorder", None),
            ("brightnessctl", None),
        ],
    }
}

fn install_dependencies(packages: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let dist = detect_distribution();
    let package_map = get_package_mapping(dist);

    match dist {
        Distribution::Arch => install_arch_packages(&package_map),
        Distribution::Fedora => install_fedora_packages(&package_map),
        Distribution::Debian => install_debian_packages(&package_map),
        Distribution::Gentoo => install_gentoo_packages(&package_map),
        Distribution::Void => install_void_packages(&package_map),
        Distribution::Unknown => {
            ui::error("Unknown Linux distribution detected.");
            list_required_packages(packages);
            Err("Cannot determine package manager for unknown distribution".into())
        }
    }
}

fn install_arch_packages(package_map: &[(&str, Option<&str>)]) -> Result<(), Box<dyn std::error::Error>> {
    // Check for AUR helpers
    let aur_helper = if Command::new("yay").arg("--version").output().is_ok() {
        Some("yay")
    } else if Command::new("paru").arg("--version").output().is_ok() {
        Some("paru")
    } else {
        None
    };

    let mut to_install = Vec::new();
    let mut missing = Vec::new();

    for (generic_name, arch_name) in package_map {
        if let Some(pkg) = arch_name {
            // Check if already installed
            let output = Command::new("pacman")
                .args(["-Q", pkg])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    ui::info(&format!("{} is already installed", generic_name));
                    continue;
                }
            }
            to_install.push(*pkg);
        } else {
            missing.push(*generic_name);
        }
    }

    if !missing.is_empty() {
        ui::error("The following packages are not available in Arch repositories:");
        for pkg in &missing {
            ui::error(&format!("  - {}", pkg));
        }
        return Err("Some required packages are not available in repositories".into());
    }

    if to_install.is_empty() {
        ui::success("All packages are already installed");
        return Ok(());
    }

    match aur_helper {
        Some(helper) => {
            ui::info(&format!("Using {} to install packages", helper));
            ui::step(&format!("Installing {} package(s)", to_install.len()));
            let mut args = vec!["-S", "--noconfirm"];
            args.extend(to_install.iter().map(|s| *s));
            
            let status = Command::new(helper)
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;
            
            if !status.success() {
                return Err("Failed to install packages".into());
            }
            ui::success("Packages installed successfully");
        }
        None => {
            ui::error("No AUR helper found (yay/paru). Please install one of the following:");
            ui::info("  yay: https://github.com/Jguer/yay");
            ui::info("  paru: https://github.com/Morganamilo/paru");
            ui::info("");
            ui::info("Then install the required packages manually:");
            let pkg_list = to_install.join(" ");
            ui::info(&format!("  yay -S {}", pkg_list));
            return Err("No AUR helper available to install packages".into());
        }
    }

    Ok(())
}

fn install_fedora_packages(package_map: &[(&str, Option<&str>)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut to_install = Vec::new();
    let mut missing = Vec::new();

    for (generic_name, fedora_name) in package_map {
        if let Some(pkg) = fedora_name {
            // Check if already installed
            let output = Command::new("rpm")
                .args(["-q", pkg])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    ui::info(&format!("{} is already installed", generic_name));
                    continue;
                }
            }
            to_install.push(*pkg);
        } else {
            missing.push(*generic_name);
        }
    }

    if !missing.is_empty() {
        ui::error("The following packages are not available in Fedora repositories:");
        for pkg in &missing {
            ui::error(&format!("  - {}", pkg));
        }
        ui::info("You may need to install them from COPR or build from source.");
        return Err("Some required packages are not available in repositories".into());
    }

    if to_install.is_empty() {
        ui::success("All packages are already installed");
        return Ok(());
    }

    ui::step(&format!("Installing {} package(s) with dnf", to_install.len()));
    let mut args = vec!["install", "-y"];
    args.extend(to_install.iter().map(|s| *s));

    let status = Command::new("sudo")
        .arg("dnf")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("Failed to install packages with dnf".into());
    }

    ui::success("Packages installed successfully");
    Ok(())
}

fn install_debian_packages(package_map: &[(&str, Option<&str>)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut to_install = Vec::new();
    let mut missing = Vec::new();

    for (generic_name, debian_name) in package_map {
        if let Some(pkg) = debian_name {
            // Check if already installed
            let output = Command::new("dpkg")
                .args(["-l", pkg])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("ii") {
                        ui::info(&format!("{} is already installed", generic_name));
                        continue;
                    }
                }
            }
            to_install.push(*pkg);
        } else {
            missing.push(*generic_name);
        }
    }

    if !missing.is_empty() {
        ui::error("The following packages are not available in Debian/Ubuntu repositories:");
        for pkg in &missing {
            ui::error(&format!("  - {}", pkg));
        }
        ui::info("You may need to add a PPA or build from source.");
        return Err("Some required packages are not available in repositories".into());
    }

    if to_install.is_empty() {
        ui::success("All packages are already installed");
        return Ok(());
    }

    ui::step(&format!("Installing {} package(s) with apt", to_install.len()));
    let mut args = vec!["install", "-y"];
    args.extend(to_install.iter().map(|s| *s));

    let status = Command::new("sudo")
        .arg("apt")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("Failed to install packages with apt".into());
    }

    ui::success("Packages installed successfully");
    Ok(())
}

fn install_gentoo_packages(package_map: &[(&str, Option<&str>)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut to_install = Vec::new();
    let mut missing = Vec::new();

    for (generic_name, gentoo_name) in package_map {
        if let Some(pkg) = gentoo_name {
            // Check if already installed
            let output = Command::new("equery")
                .args(["list", pkg])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    ui::info(&format!("{} is already installed", generic_name));
                    continue;
                }
            }
            to_install.push(*pkg);
        } else {
            missing.push(*generic_name);
        }
    }

    if !missing.is_empty() {
        ui::error("The following packages are not available in Gentoo portage:");
        for pkg in &missing {
            ui::error(&format!("  - {}", pkg));
        }
        ui::info("You may need to add an overlay or build from source.");
        return Err("Some required packages are not available in repositories".into());
    }

    if to_install.is_empty() {
        ui::success("All packages are already installed");
        return Ok(());
    }

    ui::step(&format!("Installing {} package(s) with emerge", to_install.len()));
    let mut args = vec!["-av"];
    args.extend(to_install.iter().map(|s| *s));

    let status = Command::new("sudo")
        .arg("emerge")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("Failed to install packages with emerge".into());
    }

    ui::success("Packages installed successfully");
    Ok(())
}

fn install_void_packages(package_map: &[(&str, Option<&str>)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut to_install = Vec::new();
    let mut missing = Vec::new();

    for (generic_name, void_name) in package_map {
        if let Some(pkg) = void_name {
            // Check if already installed
            let output = Command::new("xbps-query")
                .arg(pkg)
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    ui::info(&format!("{} is already installed", generic_name));
                    continue;
                }
            }
            to_install.push(*pkg);
        } else {
            missing.push(*generic_name);
        }
    }

    if !missing.is_empty() {
        ui::error("The following packages are not available in Void repositories:");
        for pkg in &missing {
            ui::error(&format!("  - {}", pkg));
        }
        ui::info("You may need to build from source or use xbps-src.");
        return Err("Some required packages are not available in repositories".into());
    }

    if to_install.is_empty() {
        ui::success("All packages are already installed");
        return Ok(());
    }

    ui::step(&format!("Installing {} package(s) with xbps-install", to_install.len()));
    let mut args = vec!["-S", "-y"];
    args.extend(to_install.iter().map(|s| *s));

    let status = Command::new("sudo")
        .arg("xbps-install")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("Failed to install packages with xbps-install".into());
    }

    ui::success("Packages installed successfully");
    Ok(())
}

fn list_required_packages(packages: &[&str]) {
    ui::info("Required packages for your distribution:");
    for pkg in packages {
        ui::info(&format!("  - {}", pkg));
    }
    ui::info("");
    ui::info("Please install these packages manually using your distribution's package manager.");
}

