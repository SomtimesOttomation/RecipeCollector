//! Update check against the GitHub releases of the repo, the same way the dashes do it:
//! compare the latest release tag with the running version, download the installer for
//! this platform, verify it against its published .sha256, then hand it to the OS.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const REPO: &str = "SomtimesOttomation/RecipeCollector";

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize, Clone)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateStatus {
    /// Running the newest release.
    UpToDate { current: String, latest: String },
    /// A newer release exists.
    Available {
        current: String,
        latest: String,
        notes: String,
        page_url: String,
        installer_url: Option<String>,
        checksum_url: Option<String>,
    },
    /// No public release could be read (repo private, no releases yet, offline...).
    Unavailable { current: String, reason: String },
}

/// Which installer file belongs to this platform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    Windows,
    MacOs,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Other
        }
    }

    fn matches(self, file: &str) -> bool {
        let f = file.to_ascii_lowercase();
        match self {
            Platform::Windows => f.ends_with("-setup.exe"),
            Platform::MacOs => f.ends_with(".dmg"),
            Platform::Other => false,
        }
    }
}

/// "v1.10.2" / "1.10.2" into comparable numbers; anything after '-' or '+' is ignored.
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    // Tolerates "v1.2.3", "V1.2.3", "1.2.3" and slips like "v.1.2.3".
    let core = v.trim().trim_start_matches(['v', 'V', '.', ' ']).split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>());
    let major = parts.next()?.ok()?;
    let minor = parts.next().unwrap_or(Ok(0)).ok()?;
    let patch = parts.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("RecipeCollector/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Network(e.to_string()))
}

fn status_from(release: GhRelease, current: &str, platform: Platform) -> UpdateStatus {
    let latest = release.tag_name.trim().trim_start_matches(['v', 'V', '.', ' ']).to_string();
    if !is_newer(&latest, current) {
        return UpdateStatus::UpToDate { current: current.to_string(), latest };
    }
    let installer = release.assets.iter().find(|a| platform.matches(&a.name)).cloned();
    let checksum = installer.as_ref().and_then(|i| {
        let want = format!("{}.sha256", i.name);
        release.assets.iter().find(|a| a.name == want).map(|a| a.browser_download_url.clone())
    });
    UpdateStatus::Available {
        current: current.to_string(),
        latest,
        notes: release.body.or(release.name).unwrap_or_default(),
        page_url: release.html_url,
        installer_url: installer.map(|a| a.browser_download_url),
        checksum_url: checksum,
    }
}

/// Asks GitHub for the latest published release.
pub fn check(current: &str) -> UpdateStatus {
    let unavailable = |reason: String| UpdateStatus::Unavailable { current: current.to_string(), reason };
    let client = match client() {
        Ok(c) => c,
        Err(e) => return unavailable(e.to_string()),
    };
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = match client.get(url).header("Accept", "application/vnd.github+json").send() {
        Ok(r) => r,
        Err(_) => return unavailable("Couldn't reach GitHub. Check your internet connection.".into()),
    };
    match resp.status().as_u16() {
        200 => {}
        404 => return unavailable("No public releases to check yet.".into()),
        403 | 429 => return unavailable("GitHub is limiting requests right now. Try again later.".into()),
        s => return unavailable(format!("GitHub answered {s}.")),
    }
    match resp.json::<GhRelease>() {
        Ok(r) if !r.draft && !r.prerelease => status_from(r, current, Platform::current()),
        Ok(_) => UpdateStatus::UpToDate { current: current.to_string(), latest: current.to_string() },
        Err(e) => unavailable(format!("Couldn't read the release info: {e}")),
    }
}

/// Downloads the installer into `dir`, checks it against the published SHA-256 and
/// returns its path. Refuses to continue if the checksum is missing or doesn't match.
pub fn download_installer(installer_url: &str, checksum_url: Option<&str>, dir: &Path) -> Result<PathBuf> {
    let checksum_url = checksum_url.ok_or_else(|| Error::Network("the release has no checksum file, so it can't be verified".into()))?;
    let client = client()?;
    let expected = client
        .get(checksum_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| Error::Network(e.to_string()))?;
    let expected = expected.split_whitespace().next().unwrap_or("").to_ascii_lowercase();

    let bytes = client
        .get(installer_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|e| Error::Network(e.to_string()))?;
    let actual = hex(&Sha256::digest(&bytes));
    if expected.len() != 64 || actual != expected {
        return Err(Error::Network("the download didn't match its checksum, so it wasn't installed".into()));
    }

    let name = installer_url.rsplit('/').next().filter(|n| !n.is_empty()).unwrap_or("RecipeCollector-update");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &[&str]) -> GhRelease {
        GhRelease {
            tag_name: tag.into(),
            name: Some(format!("Recipe Collector {tag}")),
            body: Some("**Feature**\n- Folders".into()),
            html_url: format!("https://github.com/{REPO}/releases/tag/{tag}"),
            draft: false,
            prerelease: false,
            assets: assets
                .iter()
                .map(|n| GhAsset { name: n.to_string(), browser_download_url: format!("https://dl/{n}") })
                .collect(),
        }
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10"), Some((0, 10, 0)));
        assert_eq!(parse_version("2.0.0-beta.1"), Some((2, 0, 0)));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("0.2.0", "0.1.12"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.2.0"));
        assert!(!is_newer("nonsense", "0.1.0"));
        assert_eq!(parse_version("v.0.2.0"), Some((0, 2, 0)));
        assert!(is_newer("v.0.3.0", "0.2.0"));
    }

    #[test]
    fn picks_platform_installer_and_checksum() {
        let assets = [
            "RecipeCollector_0.2.0_x64-setup.exe",
            "RecipeCollector_0.2.0_x64-setup.exe.sha256",
            "RecipeCollector_0.2.0_universal.dmg",
            "RecipeCollector_0.2.0_universal.dmg.sha256",
        ];
        match status_from(release("v0.2.0", &assets), "0.1.0", Platform::Windows) {
            UpdateStatus::Available { latest, installer_url, checksum_url, .. } => {
                assert_eq!(latest, "0.2.0");
                assert_eq!(installer_url.as_deref(), Some("https://dl/RecipeCollector_0.2.0_x64-setup.exe"));
                assert_eq!(checksum_url.as_deref(), Some("https://dl/RecipeCollector_0.2.0_x64-setup.exe.sha256"));
            }
            other => panic!("expected an update, got {other:?}"),
        }
        match status_from(release("v0.2.0", &assets), "0.1.0", Platform::MacOs) {
            UpdateStatus::Available { installer_url, .. } => {
                assert_eq!(installer_url.as_deref(), Some("https://dl/RecipeCollector_0.2.0_universal.dmg"))
            }
            other => panic!("expected an update, got {other:?}"),
        }
    }

    #[test]
    fn up_to_date_and_missing_asset() {
        assert!(matches!(status_from(release("v0.1.0", &[]), "0.1.0", Platform::Windows), UpdateStatus::UpToDate { .. }));
        match status_from(release("v0.3.0", &["notes.txt"]), "0.1.0", Platform::Windows) {
            UpdateStatus::Available { installer_url, .. } => assert!(installer_url.is_none()),
            other => panic!("expected an update, got {other:?}"),
        }
    }
}
