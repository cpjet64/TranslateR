use std::{
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use flate2::read::GzDecoder;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::app::TranslateRApp;
use crate::i18n::{tr, tr_format};
use crate::util::hashing::lower_hex;

pub const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/cpjet64/TranslateR/releases/latest";
const USER_AGENT: &str = "TranslateR update checker";
const HOURLY_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

fn update_http_client() -> Result<reqwest::blocking::Client> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("failed to create update HTTP client")
}

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

    pub fn apply_downloaded_update(&mut self) {
        if let Err(err) = self.updates.apply_downloaded_update() {
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
    ReadyToApply,
    Applying,
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

    pub fn apply_downloaded_update(&mut self) -> Result<()> {
        let Some(downloaded) = self.downloaded.as_ref() else {
            self.status = UpdateStatus::Error;
            self.message = tr("No downloaded update is ready to apply.").into_owned();
            return Err(anyhow!("no downloaded update is ready to apply"));
        };
        self.status = UpdateStatus::Applying;
        self.message = tr("Applying update and restarting TranslateR...").into_owned();
        let prepared = prepare_portable_update(downloaded)?;
        launch_update_handoff(&prepared)?;
        std::process::exit(0);
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
                self.status = UpdateStatus::ReadyToApply;
                self.message = tr_format(
                    "{version} was downloaded. Apply the update to restart TranslateR from this folder.",
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
    let body = update_http_client()?
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
    let digest = Sha256::digest(bytes);
    let actual = lower_hex(digest.as_ref());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(anyhow!("download checksum did not match release digest"))
    }
}

pub fn download_and_stage_update(release: ReleaseInfo) -> Result<DownloadedUpdate> {
    let bytes = update_http_client()?
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedUpdate {
    pub current_exe: PathBuf,
    pub staged_exe: PathBuf,
    pub app_dir: PathBuf,
    pub script_path: PathBuf,
}

pub fn prepare_portable_update(downloaded: &DownloadedUpdate) -> Result<PreparedUpdate> {
    let current_exe =
        std::env::current_exe().context("failed to locate running TranslateR binary")?;
    let install = install_paths_from_exe(&current_exe)?;
    let extract_dir = downloaded.staging_dir.path().join("extracted");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).context("failed to reset update extraction directory")?;
    }
    fs::create_dir_all(&extract_dir).context("failed to create update extraction directory")?;
    unpack_update_archive(&downloaded.archive_path, &extract_dir)?;

    let package_root = package_root(&extract_dir)?;
    let packaged_exe = find_packaged_binary(&package_root)?;
    let staged_exe = temporary_binary_path(&install.current_exe)?;
    stage_package_contents(
        &package_root,
        &install.install_root,
        &install.current_exe,
        &packaged_exe,
        &staged_exe,
    )?;
    let script_path = write_update_handoff_script(
        downloaded.staging_dir.path(),
        &install.current_exe,
        &staged_exe,
    )?;
    Ok(PreparedUpdate {
        current_exe: install.current_exe,
        staged_exe,
        app_dir: install.app_dir,
        script_path,
    })
}

pub fn launch_update_handoff(prepared: &PreparedUpdate) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(&prepared.script_path)
            .spawn()
            .context("failed to launch update handoff script")?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .arg(&prepared.script_path)
            .spawn()
            .context("failed to launch update handoff script")?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPaths {
    pub current_exe: PathBuf,
    pub install_root: PathBuf,
    pub app_dir: PathBuf,
}

pub fn install_paths_from_exe(current_exe: &Path) -> Result<InstallPaths> {
    let current_exe = current_exe.to_path_buf();
    let app_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow!("running binary has no parent directory"))?
        .to_path_buf();

    #[cfg(target_os = "macos")]
    {
        if let Some(bundle_root) = macos_bundle_root(&current_exe) {
            let install_root = bundle_root
                .parent()
                .ok_or_else(|| anyhow!("macOS app bundle has no parent directory"))?
                .to_path_buf();
            return Ok(InstallPaths {
                current_exe,
                install_root,
                app_dir,
            });
        }
    }

    Ok(InstallPaths {
        current_exe,
        install_root: app_dir.clone(),
        app_dir,
    })
}

#[cfg(target_os = "macos")]
fn macos_bundle_root(current_exe: &Path) -> Option<PathBuf> {
    let mut candidate = current_exe.parent();
    while let Some(path) = candidate {
        if path.extension().is_some_and(|extension| extension == "app") {
            return Some(path.to_path_buf());
        }
        candidate = path.parent();
    }
    None
}

fn unpack_update_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".zip") {
        unpack_zip_archive(archive_path, destination)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        unpack_tar_gz_archive(archive_path, destination)
    } else {
        Err(anyhow!("unsupported update archive format: {name}"))
    }
}

fn unpack_zip_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive_path).context("failed to open update zip archive")?;
    let mut archive = zip::ZipArchive::new(file).context("failed to read update zip archive")?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("failed to read update zip entry")?;
        let output_path = checked_archive_path(destination, entry.name())?;
        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .context("failed to create extracted update directory")?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .context("failed to create extracted update parent directory")?;
        }
        let mut output =
            File::create(&output_path).context("failed to create extracted update file")?;
        io::copy(&mut entry, &mut output).context("failed to extract update zip entry")?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output_path, fs::Permissions::from_mode(mode))
                .context("failed to set extracted update permissions")?;
        }
    }
    Ok(())
}

fn unpack_tar_gz_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive_path).context("failed to open update tar archive")?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    for entry in archive
        .entries()
        .context("failed to read update tar archive")?
    {
        let mut entry = entry.context("failed to read update tar entry")?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            continue;
        }
        let rel_path = entry
            .path()
            .context("failed to read update tar entry path")?;
        let output_path = checked_archive_path(destination, rel_path.as_ref())?;
        if entry_type.is_dir() {
            fs::create_dir_all(&output_path)
                .context("failed to create extracted update directory")?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)
                    .context("failed to create extracted update parent directory")?;
            }
            entry
                .unpack(&output_path)
                .context("failed to extract update tar entry")?;
        }
    }
    Ok(())
}

fn checked_archive_path<P: AsRef<Path>>(base: &Path, relative: P) -> Result<PathBuf> {
    let mut output = base.to_path_buf();
    for component in relative.as_ref().components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            _ => return Err(anyhow!("update archive contains an unsafe path")),
        }
    }
    Ok(output)
}

fn package_root(extract_dir: &Path) -> Result<PathBuf> {
    let entries = fs::read_dir(extract_dir)
        .context("failed to read extracted update directory")?
        .collect::<Result<Vec<_>, _>>()
        .context("failed to inspect extracted update directory")?;
    if entries.len() == 1 {
        let path = entries[0].path();
        if path.is_dir() {
            return Ok(path);
        }
    }
    Ok(extract_dir.to_path_buf())
}

fn packaged_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "translater.exe"
    } else {
        "translater"
    }
}

fn find_packaged_binary(package_root: &Path) -> Result<PathBuf> {
    WalkDir::new(package_root)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == packaged_binary_name()
        })
        .map(|entry| entry.into_path())
        .ok_or_else(|| {
            anyhow!(
                "downloaded update package did not contain {}",
                packaged_binary_name()
            )
        })
}

fn temporary_binary_path(current_exe: &Path) -> Result<PathBuf> {
    let file_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("running binary has no file name"))?;
    Ok(current_exe.with_file_name(format!(".translater-update-{file_name}")))
}

fn stage_package_contents(
    package_root: &Path,
    install_root: &Path,
    current_exe: &Path,
    packaged_exe: &Path,
    staged_exe: &Path,
) -> Result<()> {
    let mut staged_binary = false;
    for entry in WalkDir::new(package_root).min_depth(1) {
        let entry = entry.context("failed to inspect extracted update content")?;
        let relative = entry
            .path()
            .strip_prefix(package_root)
            .context("failed to map extracted update content")?;
        let destination = install_root.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)
                .context("failed to create update destination directory")?;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .context("failed to create update destination parent directory")?;
        }
        if destination == current_exe {
            fs::copy(entry.path(), staged_exe)
                .context("failed to stage updated TranslateR binary")?;
            make_executable(staged_exe)?;
            staged_binary = true;
        } else {
            fs::copy(entry.path(), &destination).with_context(|| {
                format!("failed to copy update file to {}", destination.display())
            })?;
            if entry.path() == packaged_exe {
                fs::copy(entry.path(), staged_exe)
                    .context("failed to stage updated TranslateR binary")?;
                make_executable(staged_exe)?;
                staged_binary = true;
            }
        }
    }

    if !staged_binary {
        fs::copy(packaged_exe, staged_exe).context("failed to stage updated TranslateR binary")?;
        make_executable(staged_exe)?;
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .context("failed to read staged binary permissions")?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).context("failed to set staged binary executable")?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn write_update_handoff_script(
    staging_dir: &Path,
    current_exe: &Path,
    staged_exe: &Path,
) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let script_path = staging_dir.join("translater-update.cmd");
        let app_dir = current_exe
            .parent()
            .ok_or_else(|| anyhow!("running binary has no parent directory"))?;
        let current_name = current_exe
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("running binary has no file name"))?;
        let script = format!(
            "@echo off\r\n\
             setlocal\r\n\
             set \"OLD={old}\"\r\n\
             set \"NEW={new}\"\r\n\
             set \"APPDIR={appdir}\"\r\n\
             set \"FINAL={final_name}\"\r\n\
             :wait\r\n\
             del /f /q \"%OLD%\" >nul 2>nul\r\n\
             if exist \"%OLD%\" (\r\n\
             \ttimeout /t 1 /nobreak >nul\r\n\
             \tgoto wait\r\n\
             )\r\n\
             ren \"%NEW%\" \"%FINAL%\"\r\n\
             start \"\" /D \"%APPDIR%\" \"%OLD%\"\r\n\
             del /f /q \"%~f0\" >nul 2>nul\r\n",
            old = batch_escape(&current_exe.display().to_string()),
            new = batch_escape(&staged_exe.display().to_string()),
            appdir = batch_escape(&app_dir.display().to_string()),
            final_name = batch_escape(current_name),
        );
        fs::write(&script_path, script).context("failed to write update handoff script")?;
        Ok(script_path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let script_path = staging_dir.join("translater-update.sh");
        let app_dir = current_exe
            .parent()
            .ok_or_else(|| anyhow!("running binary has no parent directory"))?;
        let script = format!(
            "#!/bin/sh\n\
             OLD={old}\n\
             NEW={new}\n\
             APPDIR={appdir}\n\
             while [ -e \"$OLD\" ]; do\n\
             \trm -f \"$OLD\" 2>/dev/null && break\n\
             \tsleep 1\n\
             done\n\
             mv \"$NEW\" \"$OLD\"\n\
             chmod 755 \"$OLD\" 2>/dev/null || true\n\
             cd \"$APPDIR\"\n\
             \"$OLD\" >/dev/null 2>&1 &\n\
             rm -f \"$0\"\n",
            old = shell_quote(&current_exe.display().to_string()),
            new = shell_quote(&staged_exe.display().to_string()),
            appdir = shell_quote(&app_dir.display().to_string()),
        );
        fs::write(&script_path, script).context("failed to write update handoff script")?;
        make_executable(&script_path)?;
        Ok(script_path)
    }
}

#[cfg(target_os = "windows")]
fn batch_escape(value: &str) -> String {
    value.replace('%', "%%")
}

#[cfg(not(target_os = "windows"))]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[allow(dead_code)]
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
        let hash = Sha256::digest(bytes);
        let digest = format!("sha256:{}", lower_hex(hash.as_ref()));
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

        let mut state = UpdateState {
            startup_check_started: true,
            last_hourly_check: Some(now),
            ..UpdateState::default()
        };
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

    #[test]
    fn archive_paths_cannot_escape_the_extraction_directory() {
        let base = Path::new("stage");
        assert_eq!(
            checked_archive_path(base, "folder/file.txt").unwrap(),
            Path::new("stage").join("folder").join("file.txt")
        );
        assert!(checked_archive_path(base, "../outside.txt").is_err());
        assert!(checked_archive_path(base, "/absolute/outside.txt").is_err());
        assert!(checked_archive_path(base, "folder/../../outside.txt").is_err());
    }

    #[test]
    fn temporary_binary_name_is_prepended_with_update_tag() {
        let path = if cfg!(target_os = "windows") {
            Path::new(r"C:\apps\TranslateR\translater.exe")
        } else {
            Path::new("/apps/TranslateR/translater")
        };
        let temp = temporary_binary_path(path).unwrap();
        assert!(
            temp.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".translater-update-")
        );
        assert_eq!(temp.parent(), path.parent());
    }

    #[test]
    fn staging_copies_package_files_but_keeps_running_binary_until_handoff() {
        let temp = tempfile::tempdir().unwrap();
        let package_root = temp.path().join("package");
        let install_root = temp.path().join("install");
        fs::create_dir_all(package_root.join("i18n")).unwrap();
        fs::create_dir_all(&install_root).unwrap();

        let binary_name = packaged_binary_name();
        let packaged_exe = package_root.join(binary_name);
        let current_exe = install_root.join(binary_name);
        let staged_exe = temporary_binary_path(&current_exe).unwrap();
        fs::write(&packaged_exe, b"new binary").unwrap();
        fs::write(package_root.join("README.md"), b"readme").unwrap();
        fs::write(package_root.join("i18n").join("en.po"), b"catalog").unwrap();
        fs::write(&current_exe, b"old binary").unwrap();

        stage_package_contents(
            &package_root,
            &install_root,
            &current_exe,
            &packaged_exe,
            &staged_exe,
        )
        .unwrap();

        assert_eq!(fs::read(&current_exe).unwrap(), b"old binary");
        assert_eq!(fs::read(&staged_exe).unwrap(), b"new binary");
        assert_eq!(fs::read(install_root.join("README.md")).unwrap(), b"readme");
        assert_eq!(
            fs::read(install_root.join("i18n").join("en.po")).unwrap(),
            b"catalog"
        );
    }

    #[test]
    fn staging_keeps_packaged_bundle_binary_when_running_binary_is_elsewhere() {
        let temp = tempfile::tempdir().unwrap();
        let package_root = temp.path().join("package");
        let install_root = temp.path().join("install");
        let package_binary_dir = package_root
            .join("TranslateR.app")
            .join("Contents")
            .join("MacOS");
        fs::create_dir_all(&package_binary_dir).unwrap();
        fs::create_dir_all(&install_root).unwrap();

        let binary_name = packaged_binary_name();
        let packaged_exe = package_binary_dir.join(binary_name);
        let current_exe = install_root.join(binary_name);
        let staged_exe = temporary_binary_path(&current_exe).unwrap();
        let installed_bundle_exe = install_root
            .join("TranslateR.app")
            .join("Contents")
            .join("MacOS")
            .join(binary_name);
        fs::write(&packaged_exe, b"new bundled binary").unwrap();
        fs::write(&current_exe, b"old raw binary").unwrap();

        stage_package_contents(
            &package_root,
            &install_root,
            &current_exe,
            &packaged_exe,
            &staged_exe,
        )
        .unwrap();

        assert_eq!(fs::read(&current_exe).unwrap(), b"old raw binary");
        assert_eq!(fs::read(&staged_exe).unwrap(), b"new bundled binary");
        assert_eq!(
            fs::read(installed_bundle_exe).unwrap(),
            b"new bundled binary"
        );
    }

    #[test]
    fn handoff_script_deletes_then_renames_and_relaunches() {
        let temp = tempfile::tempdir().unwrap();
        let current_exe = temp.path().join(packaged_binary_name());
        let staged_exe = temporary_binary_path(&current_exe).unwrap();
        let script_path =
            write_update_handoff_script(temp.path(), &current_exe, &staged_exe).unwrap();
        let script = fs::read_to_string(script_path).unwrap();

        assert!(script.contains(&current_exe.display().to_string()));
        assert!(script.contains(&staged_exe.display().to_string()));
        if cfg!(target_os = "windows") {
            assert!(script.contains("del /f /q"));
            assert!(script.contains("ren \"%NEW%\" \"%FINAL%\""));
            assert!(script.contains("start \"\""));
        } else {
            assert!(script.contains("rm -f \"$OLD\""));
            assert!(script.contains("mv \"$NEW\" \"$OLD\""));
            assert!(script.contains("\"$OLD\" >/dev/null 2>&1 &"));
        }
    }
}
