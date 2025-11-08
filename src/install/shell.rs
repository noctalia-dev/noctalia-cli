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

