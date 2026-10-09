use std::io::Read;
use std::sync::{Arc, Mutex};

const REPO_OWNER: &str = "CoheterosUS";
const REPO_NAME: &str = "coheteros-gsui-rust";
const ASSET_NAME: &str = "coheteros-gsui.exe";

#[derive(Clone)]
pub enum UpdateStatus {
    Checking,
    Available { version: String, url: String },
    UpToDate,
    Downloading,
    ReadyToRestart,
    Error(String),
}

pub fn spawn_check(ctx: egui::Context) -> Arc<Mutex<UpdateStatus>> {
    let status = Arc::new(Mutex::new(UpdateStatus::Checking));
    let s = status.clone();
    std::thread::spawn(move || {
        let result = check_latest();
        if let Ok(mut guard) = s.lock() {
            *guard = match result {
                Ok(Some((version, url))) => UpdateStatus::Available { version, url },
                Ok(None) => UpdateStatus::UpToDate,
                Err(e) => UpdateStatus::Error(e),
            };
        }
        ctx.request_repaint();
    });
    status
}

fn check_latest() -> Result<Option<(String, String)>, String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );
    let resp = ureq::get(&api_url)
        .set("User-Agent", "coheteros-gsui")
        .call()
        .map_err(|e| format!("{}", e))?;

    let body = resp.into_string().map_err(|e| format!("{}", e))?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("{}", e))?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or("no tag_name in release")?;

    let latest = semver::Version::parse(tag).map_err(|e| format!("bad release tag '{}': {}", tag, e))?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| format!("bad pkg version: {}", e))?;

    if latest > current {
        let assets = json["assets"].as_array().ok_or("no assets in release")?;
        let url = assets
            .iter()
            .find(|a| a["name"].as_str() == Some(ASSET_NAME))
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or_else(|| format!("no '{}' asset in release {}", ASSET_NAME, tag))?
            .to_string();
        Ok(Some((tag.to_string(), url)))
    } else {
        Ok(None)
    }
}

pub fn spawn_download(url: String, status: Arc<Mutex<UpdateStatus>>, ctx: egui::Context) {
    if let Ok(mut guard) = status.lock() {
        *guard = UpdateStatus::Downloading;
    }
    std::thread::spawn(move || {
        let result = download_and_replace(&url);
        if let Ok(mut guard) = status.lock() {
            *guard = match result {
                Ok(()) => UpdateStatus::ReadyToRestart,
                Err(e) => UpdateStatus::Error(e),
            };
        }
        ctx.request_repaint();
    });
}

fn download_and_replace(url: &str) -> Result<(), String> {
    let resp = ureq::get(url)
        .set("User-Agent", "coheteros-gsui")
        .call()
        .map_err(|e| format!("download failed: {}", e))?;

    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read failed: {}", e))?;

    let tmp = std::env::temp_dir().join("coheteros-gsui-update.exe");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("write temp: {}", e))?;

    self_replace::self_replace(&tmp).map_err(|e| format!("replace failed: {}", e))?;

    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

pub fn restart() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    std::process::exit(0);
}
