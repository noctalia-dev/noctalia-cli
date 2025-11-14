use std::{env, fs, path::PathBuf, process::Command};

use crate::SourceKind;
use crate::config;
use crate::ui;

const REPO_API: &str = "https://api.github.com/repos/noctalia-dev/noctalia-shell";
const REPO_CODELOAD_MAIN: &str = "https://codeload.github.com/noctalia-dev/noctalia-shell/tar.gz/refs/heads/main";

fn find_installation_path() -> Option<PathBuf> {
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

#[derive(serde::Deserialize)]
struct ReleaseInfo { 
    tag_name: String, 
    tarball_url: String 
}

#[derive(serde::Deserialize)]
struct CommitInfo {
    sha: String,
}

pub fn run(source: SourceKind) {
    ui::section("Update Noctalia Shell");
    
    // Check if shell is installed
    let (cfg, _path) = config::CliConfig::load().expect("load config");
    if !cfg.is_component_installed("shell") {
        ui::error("Noctalia shell is not installed. Run 'noctalia install shell' first.");
        std::process::exit(1);
    }

    let installed_version = cfg.get_component_version("shell");
    let installed_source = cfg.get_component_source("shell").unwrap_or(source);

    ui::info(&format!("Current source: {}", installed_source));
    if let Some(ref ver) = installed_version {
        match installed_source {
            SourceKind::Git => {
                let display = if ver.len() >= 8 { &ver[..8] } else { ver.as_str() };
                ui::info(&format!("Installed commit: {}", display));
            }
            SourceKind::Release => ui::info(&format!("Installed version: {}", ver)),
        }
    } else {
        ui::info("Installed version: unknown (installed before version tracking)");
    }

    ui::step("Checking for updates");

    let (latest_version, needs_update) = match source {
        SourceKind::Git => {
            ui::info("Fetching latest commit from git main");
            let latest_sha = match get_latest_commit_sha() {
                Ok(sha) => sha,
                Err(e) => {
                    ui::error(&format!("Failed to fetch latest commit: {}", e));
                    std::process::exit(1);
                }
            };
            let display = if latest_sha.len() >= 8 { &latest_sha[..8] } else { latest_sha.as_str() };
            ui::info(&format!("Latest commit: {}", display));
            
            let needs_update = installed_version.as_ref().map(|v| v != &latest_sha).unwrap_or(true);
            (latest_sha, needs_update)
        }
        SourceKind::Release => {
            ui::info("Fetching latest release");
            let release_info = match get_latest_release_info() {
                Ok(info) => info,
                Err(e) => {
                    ui::error(&format!("Failed to fetch latest release: {}", e));
                    std::process::exit(1);
                }
            };
            ui::info(&format!("Latest release: {}", release_info.tag_name));
            
            let needs_update = installed_version.as_ref().map(|v| v != &release_info.tag_name).unwrap_or(true);
            (release_info.tag_name, needs_update)
        }
    };

    if !needs_update {
        ui::success("Noctalia shell is already up to date!");
        return;
    }

    ui::step("Update available, downloading...");

    match source {
        SourceKind::Git => {
            if let Err(e) = download_and_extract_git_main() {
                ui::error(&format!("Failed to update noctalia-shell (git): {}", e));
                std::process::exit(1);
            }
        }
        SourceKind::Release => {
            if let Err(e) = download_and_extract_latest_release() {
                ui::error(&format!("Failed to update noctalia-shell (release): {}", e));
                std::process::exit(1);
            }
        }
    }

    let (mut cfg, path) = config::CliConfig::load().expect("load config");
    cfg.set_component_source("shell", source);
    cfg.set_component_version("shell", latest_version.clone());
    let _ = cfg.save(&path);

    let version_display = match source {
        SourceKind::Git => {
            let display = if latest_version.len() >= 8 { &latest_version[..8] } else { latest_version.as_str() };
            format!("commit {}", display)
        }
        SourceKind::Release => latest_version,
    };
    ui::success(&format!("Successfully updated noctalia-shell to {}", version_display));
}

fn downloads_dir() -> PathBuf {
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

fn download_git_main() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let client = http_client();
    let resp = client.get(REPO_CODELOAD_MAIN).send()?;
    if !resp.status().is_success() { return Err(format!("http {}", resp.status()).into()); }
    let bytes = resp.bytes()?;
    let out = downloads_dir().join("noctalia-shell-main.tar.gz");
    fs::write(&out, &bytes)?;
    Ok(out)
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
    extract(&archive)?;
    let _ = fs::remove_file(&archive);
    Ok(())
}

fn download_and_extract_latest_release() -> Result<(), Box<dyn std::error::Error>> {
    let archive = download_latest_release()?;
    extract(&archive)?;
    let _ = fs::remove_file(&archive);
    Ok(())
}

fn extract(archive_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Find where the shell is actually installed
    let target = match find_installation_path() {
        Some(path) => {
            ui::info(&format!("Found installation at: {}", path.display()));
            path
        }
        None => {
            // If not found, use the new default location
            let home = env::var("HOME").expect("HOME environment variable not set");
            let new_path = PathBuf::from(home).join(".config/quickshell/noctalia-shell");
            ui::info(&format!("No existing installation found, will install to: {}", new_path.display()));
            new_path
        }
    };
    
    // Check if we need sudo for the old location
    let needs_sudo = target.starts_with("/etc");
    
    // Remove existing directory if it exists
    if target.exists() {
        if needs_sudo {
            // Use sudo to remove the directory
            let target_str = target.to_str().unwrap();
            let status = Command::new("sudo")
                .args(["rm", "-rf", target_str])
                .status()?;
            if !status.success() {
                return Err("Failed to remove existing installation".into());
            }
        } else {
            fs::remove_dir_all(&target)?;
        }
    }
    
    // Create parent directories
    if let Some(parent) = target.parent() {
        if needs_sudo {
            let parent_str = parent.to_str().unwrap();
            let status = Command::new("sudo")
                .args(["mkdir", "-p", parent_str])
                .status()?;
            if !status.success() {
                return Err("Failed to create parent directory".into());
            }
        } else {
            fs::create_dir_all(parent)?;
        }
    }
    
    // Extract archive
    if needs_sudo {
        // For old location, extract to temp directory first, then move with sudo
        let temp_dir = std::env::temp_dir().join(format!("noctalia-shell-update-{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        
        let file = fs::File::open(archive_path)?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive.unpack(&temp_dir)?;
        
        // Move contents up one level (strip-components=1 equivalent)
        let extracted_dir = temp_dir.join("noctalia-shell-main");
        let temp_target = if extracted_dir.exists() {
            // Move all contents from noctalia-shell-main to temp_target
            let temp_target = temp_dir.join("noctalia-shell");
            fs::create_dir_all(&temp_target)?;
            for entry in fs::read_dir(&extracted_dir)? {
                let entry = entry?;
                let dest = temp_target.join(entry.file_name());
                fs::rename(entry.path(), dest)?;
            }
            fs::remove_dir(&extracted_dir)?;
            temp_target
        } else {
            // Try with release tag name pattern
            let entries: Vec<_> = fs::read_dir(&temp_dir)?.collect();
            if entries.len() == 1 {
                if let Some(Ok(entry)) = entries.into_iter().next() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let temp_target = temp_dir.join("noctalia-shell");
                        fs::create_dir_all(&temp_target)?;
                        // Move all contents from the single subdirectory to temp_target
                        for sub_entry in fs::read_dir(&entry_path)? {
                            let sub_entry = sub_entry?;
                            let dest = temp_target.join(sub_entry.file_name());
                            fs::rename(sub_entry.path(), dest)?;
                        }
                        fs::remove_dir(&entry_path)?;
                        temp_target
                    } else {
                        temp_dir.clone()
                    }
                } else {
                    temp_dir.clone()
                }
            } else {
                temp_dir.clone()
            }
        };
        
        // Use sudo to move the extracted directory to the target
        let temp_target_str = temp_target.to_str().unwrap();
        let target_str = target.to_str().unwrap();
        let cmd = format!("cp -r '{}'/* '{}'/ && rm -rf '{}'", temp_target_str, target_str, temp_target_str);
        
        ui::info("Elevating with sudo. You may be prompted for your password.");
        let status = std::process::Command::new("sudo")
            .args(["sh", "-c", &cmd])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;
        
        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir);
        
        if !status.success() {
            return Err("Failed to install updated files".into());
        }
    } else {
        // For new location, extract directly
        let file = fs::File::open(archive_path)?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive.unpack(&target)?;
        
        // Move contents up one level (strip-components=1 equivalent)
        let extracted_dir = target.join("noctalia-shell-main");
        if extracted_dir.exists() {
            // Move all contents from noctalia-shell-main to target
            for entry in fs::read_dir(&extracted_dir)? {
                let entry = entry?;
                let dest = target.join(entry.file_name());
                fs::rename(entry.path(), dest)?;
            }
            fs::remove_dir(&extracted_dir)?;
        } else {
            // Try with release tag name pattern
            let entries: Vec<_> = fs::read_dir(&target)?.collect();
            if entries.len() == 1 {
                if let Some(Ok(entry)) = entries.into_iter().next() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        // Move all contents from the single subdirectory to target
                        for sub_entry in fs::read_dir(&entry_path)? {
                            let sub_entry = sub_entry?;
                            let dest = target.join(sub_entry.file_name());
                            fs::rename(sub_entry.path(), dest)?;
                        }
                        fs::remove_dir(&entry_path)?;
                    }
                }
            }
        }
    }
    
    Ok(())
}
