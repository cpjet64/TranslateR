use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::app::TranslateRApp;
use crate::i18n::{tr, tr_format};

pub const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/cpjet64/TranslateR/releases/latest";
const USER_AGENT: &str = "TranslateR update checker";
const HOURLY_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: Version,
    pub tag_name: String,
    pub html_url: String,
    pub body: String,
    pub published_at: String,
    pub asset: ReleaseAsset,
}

impl TranslateRApp {
    pub fn check_for_updates(&mut self, ctx: &egui::Context) {
        self.updates
            .start_check(env!("CARGO_PKG_VERSION").to_string(), false, ctx.clone());
        self.status = tr("Checking for updates...").into_owned();
    }

    pub fn download_update(&mut self, ctx: &egui::Context) {
        self.updates.start_download(ctx.clone());
        self.status = tr("Downloading update...").into_owned();
    }

    pub fn open_downloaded_update(&mut self) {
        if let Err(err) = self.updates.open_downloaded_package() {
            self.last_error = Some(err.to_string());
        }
    }

    pub fn update_tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if self.updates.should_start_automatic_check(
            now,
            self.config.update.check_on_startup,
            self.config.update.check_hourly,
        ) {
            self.updates
                .start_check(env!("CARGO_PKG_VERSION").to_string(), true, ctx.clone());
        }

        for event in self.updates.poll() {
            self.updates.apply_event(event);
        }
        if !self.updates.message.is_empty() {
            self.status = self.updates.message.clone();
        }
        if self.config.update.check_hourly {
            ctx.request_repaint_after(Duration::from_secs(30));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub digest: Option<String>,
}

#[derive(Debug)]
pub struct DownloadedUpdate {
    pub release: ReleaseInfo,
    pub archive_path: PathBuf,
    pub staging_dir: TempDir,
}

#[derive(Debug)]
pub enum UpdateEvent {
    CheckFinished(Result<Option<ReleaseInfo>, String>),
    DownloadFinished(Result<DownloadedUpdate, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Current,
    UpdateAvailable,
    Downloading,
    ReadyToOpen,
    Error,
}

#[derive(Debug)]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub show_dialog: bool,
    pub latest: Option<ReleaseInfo>,
    pub downloaded: Option<DownloadedUpdate>,
    pub message: String,
    receiver: Option<Receiver<UpdateEvent>>,
    startup_check_started: bool,
    last_hourly_check: Option<Instant>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            show_dialog: false,
            latest: None,
            downloaded: None,
            message: String::new(),
            receiver: None,
            startup_check_started: false,
            last_hourly_check: None,
        }
    }
}

impl UpdateState {
    pub fn should_start_automatic_check(
        &self,
        now: Instant,
        check_on_startup: bool,
        check_hourly: bool,
    ) -> bool {
        if matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading
        ) {
            return false;
        }
        if !self.startup_check_started {
            return check_on_startup || check_hourly;
        }
        if !check_hourly {
            return false;
        }
        self.last_hourly_check
            .is_none_or(|last| now.duration_since(last) >= HOURLY_CHECK_INTERVAL)
    }

    pub fn start_check(&mut self, current_version: String, automatic: bool, ctx: egui::Context) {
        if matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading
        ) {
            return;
        }
        if automatic {
            self.startup_check_started = true;
            self.last_hourly_check = Some(Instant::now());
        } else {
            self.show_dialog = true;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.status = UpdateStatus::Checking;
        self.message = tr("Checking for updates...").into_owned();
        thread::spawn(move || {
            let result = check_latest_release(&current_version).map_err(|err| err.to_string());
            let _ = tx.send(UpdateEvent::CheckFinished(result));
            ctx.request_repaint();
        });
    }

    pub fn start_download(&mut self, ctx: egui::Context) {
        let Some(release) = self.latest.clone() else {
            self.status = UpdateStatus::Error;
            self.message = tr("No update is available to download.").into_owned();
            return;
        };
        if matches!(self.status, UpdateStatus::Downloading) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.status = UpdateStatus::Downloading;
        self.message = tr_format(
            "Downloading {version}...",
            &[("version", release.tag_name.clone())],
        );
        thread::spawn(move || {
            let result = download_and_stage_update(release).map_err(|err| err.to_string());
            let _ = tx.send(UpdateEvent::DownloadFinished(result));
            ctx.request_repaint();
        });
    }

    pub fn open_downloaded_package(&mut self) -> Result<()> {
        let Some(downloaded) = self.downloaded.as_ref() else {
            self.status = UpdateStatus::Error;
            self.message = tr("No downloaded update is ready to open.").into_owned();
            return Err(anyhow!("no downloaded update is ready to open"));
        };
        open_path(&downloaded.archive_path)?;
        self.message = tr("Opened the downloaded update package.").into_owned();
        Ok(())
    }

    pub fn poll(&mut self) -> Vec<UpdateEvent> {
        let mut events = Vec::new();
        let Some(rx) = self.receiver.take() else {
            return events;
        };
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        if events.is_empty() {
            self.receiver = Some(rx);
        }
        events
    }

    pub fn apply_event(&mut self, event: UpdateEvent) {
        match event {
            UpdateEvent::CheckFinished(Ok(Some(release))) => {
                self.status = UpdateStatus::UpdateAvailable;
                self.message = tr_format(
                    "Update available: {version}",
                    &[("version", release.tag_name.clone())],
                );
                self.latest = Some(release);
                self.show_dialog = true;
            }
            UpdateEvent::CheckFinished(Ok(None)) => {
                self.status = UpdateStatus::Current;
                self.message = tr("TranslateR is up to date.").into_owned();
            }
            UpdateEvent::CheckFinished(Err(err)) => {
                self.status = UpdateStatus::Error;
                self.message = err;
            }
            UpdateEvent::DownloadFinished(Ok(downloaded)) => {
                self.status = UpdateStatus::ReadyToOpen;
                self.message = tr_format(
                    "{version} was downloaded. Open the package to update TranslateR manually.",
                    &[("version", downloaded.release.tag_name.clone())],
                );
                self.downloaded = Some(downloaded);
                self.show_dialog = true;
            }
            UpdateEvent::DownloadFinished(Err(err)) => {
                self.status = UpdateStatus::Error;
                self.message = err;
                self.show_dialog = true;
            }
        }
    }
}

pub fn check_latest_release(current_version: &str) -> Result<Option<ReleaseInfo>> {
    let body = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(GITHUB_LATEST_RELEASE_URL)
        .send()
        .context("failed to contact GitHub releases")?
        .error_for_status()
        .context("GitHub release request failed")?
        .text()
        .context("failed to read GitHub release response")?;
    parse_latest_release(&body, current_version)
}

pub fn parse_latest_release(json: &str, current_version: &str) -> Result<Option<ReleaseInfo>> {
    let release: GithubRelease =
        serde_json::from_str(json).context("invalid GitHub release JSON")?;
    let latest_version = parse_release_version(&release.tag_name)?;
    let current = parse_release_version(current_version)?;
    if latest_version <= current {
        return Ok(None);
    }
    let asset = select_platform_asset(&release.assets)
        .ok_or_else(|| anyhow!("no release asset matches this platform"))?;
    Ok(Some(ReleaseInfo {
        version: latest_version,
        tag_name: release.tag_name,
        html_url: release.html_url,
        body: release.body.unwrap_or_default(),
        published_at: release.published_at.unwrap_or_default(),
        asset,
    }))
}

pub fn parse_release_version(value: &str) -> Result<Version> {
    Version::parse(value.trim_start_matches('v'))
        .with_context(|| format!("invalid version {value}"))
}

pub fn platform_asset_name() -> &'static str {
    platform_asset_name_for(std::env::consts::OS)
}

pub fn platform_asset_name_for(os: &str) -> &'static str {
    match os {
        "windows" => "translater-windows-x86_64.zip",
        "macos" => "translater-macos-x86_64.tar.gz",
        "linux" => "translater-ubuntu-x86_64.tar.gz",
        _ => "translater-ubuntu-x86_64.tar.gz",
    }
}

pub fn select_platform_asset(assets: &[GithubAsset]) -> Option<ReleaseAsset> {
    let wanted = platform_asset_name();
    assets
        .iter()
        .find(|asset| asset.name == wanted)
        .map(|asset| ReleaseAsset {
            name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
            size: asset.size,
            digest: asset.digest.clone(),
        })
}

pub fn verify_digest(bytes: &[u8], digest: Option<&str>) -> Result<()> {
    let Some(digest) = digest else {
        return Ok(());
    };
    let expected = digest.strip_prefix("sha256:").unwrap_or(digest);
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(anyhow!("download checksum did not match release digest"))
    }
}

pub fn download_and_stage_update(release: ReleaseInfo) -> Result<DownloadedUpdate> {
    let bytes = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(&release.asset.download_url)
        .send()
        .context("failed to download update archive")?
        .error_for_status()
        .context("update archive download failed")?
        .bytes()
        .context("failed to read update archive")?
        .to_vec();
    verify_digest(&bytes, release.asset.digest.as_deref())?;

    let staging_dir = tempfile::tempdir().context("failed to create update staging directory")?;
    let archive_path = staging_dir.path().join(&release.asset.name);
    fs::write(&archive_path, &bytes).context("failed to write update archive")?;
    Ok(DownloadedUpdate {
        release,
        archive_path,
        staging_dir,
    })
}

pub fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .context("failed to open downloaded update package")?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .context("failed to open downloaded update package")?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("failed to open downloaded update package")?;
    }
    Ok(())
}

pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(url)
            .spawn()
            .context("failed to open release page")?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .context("failed to open release page")?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("failed to open release page")?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions_and_selects_newer_github_release() {
        let json = r#"{
            "tag_name": "v1.2.3",
            "html_url": "https://github.example/release",
            "body": "changes",
            "published_at": "2026-06-19T00:00:00Z",
            "assets": [
                {
                    "name": "translater-windows-x86_64.zip",
                    "browser_download_url": "https://github.example/win.zip",
                    "size": 10,
                    "digest": "sha256:abc"
                },
                {
                    "name": "translater-ubuntu-x86_64.tar.gz",
                    "browser_download_url": "https://github.example/linux.tar.gz",
                    "size": 20,
                    "digest": null
                },
                {
                    "name": "translater-macos-x86_64.tar.gz",
                    "browser_download_url": "https://github.example/mac.tar.gz",
                    "size": 30,
                    "digest": null
                }
            ]
        }"#;

        let release = parse_latest_release(json, "1.2.2").unwrap().unwrap();
        assert_eq!(release.version, Version::parse("1.2.3").unwrap());
        assert_eq!(release.tag_name, "v1.2.3");
        assert_eq!(release.html_url, "https://github.example/release");
        assert_eq!(release.body, "changes");
        assert_eq!(release.asset.name, platform_asset_name());
        assert!(parse_latest_release(json, "1.2.3").unwrap().is_none());
    }

    #[test]
    fn update_parse_reports_invalid_release_shapes() {
        assert!(parse_release_version("v0.1.2").is_ok());
        assert!(parse_release_version("not-a-version").is_err());
        assert!(parse_latest_release("{not-json", "0.1.0").is_err());

        let no_matching_asset = r#"{
            "tag_name": "v9.0.0",
            "html_url": "https://github.example/release",
            "assets": []
        }"#;
        assert!(parse_latest_release(no_matching_asset, "0.1.0").is_err());
    }

    #[test]
    fn platform_asset_names_are_stable() {
        assert_eq!(
            platform_asset_name_for("windows"),
            "translater-windows-x86_64.zip"
        );
        assert_eq!(
            platform_asset_name_for("macos"),
            "translater-macos-x86_64.tar.gz"
        );
        assert_eq!(
            platform_asset_name_for("linux"),
            "translater-ubuntu-x86_64.tar.gz"
        );
        assert_eq!(
            platform_asset_name_for("freebsd"),
            "translater-ubuntu-x86_64.tar.gz"
        );
    }

    #[test]
    fn digest_verification_accepts_missing_or_matching_digest() {
        let bytes = b"update";
        let digest = format!("sha256:{:x}", Sha256::digest(bytes));
        verify_digest(bytes, None).unwrap();
        verify_digest(bytes, Some(&digest)).unwrap();
        verify_digest(bytes, Some(digest.trim_start_matches("sha256:"))).unwrap();
        assert!(verify_digest(bytes, Some("sha256:bad")).is_err());
    }

    #[test]
    fn scheduler_checks_once_at_startup_then_hourly() {
        let state = UpdateState::default();
        let now = Instant::now();
        assert!(state.should_start_automatic_check(now, true, true));
        assert!(!state.should_start_automatic_check(now, false, false));

        let mut state = UpdateState::default();
        state.startup_check_started = true;
        state.last_hourly_check = Some(now);
        assert!(!state.should_start_automatic_check(now + Duration::from_secs(30), true, true));
        assert!(!state.should_start_automatic_check(
            now + Duration::from_secs(60 * 60),
            true,
            false
        ));
        assert!(state.should_start_automatic_check(now + Duration::from_secs(60 * 60), true, true));

        state.status = UpdateStatus::Checking;
        assert!(!state.should_start_automatic_check(
            now + Duration::from_secs(60 * 61),
            true,
            true
        ));
    }
}
