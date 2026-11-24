//! Self-update from the latest GitHub release. Its `latest.json` lists a signed bundle per
//! platform, checked against the public key compiled into release builds; local builds have
//! no key and only point at the release page.

use cargo_packager_updater::semver::Version;
use cargo_packager_updater::{Config, Update, UpdaterBuilder};
use serde::Deserialize;

pub(crate) const REPO: &str = "eve-marketeer/marketeer";
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
const TAG_PREFIX: &str = "marketeer-v";
/// Set by CI from the repository's `UPDATER_PUBKEY` variable.
const PUBLIC_KEY: Option<&str> = option_env!("MARKETEER_UPDATER_PUBKEY");

/// A newer release than the one running.
#[derive(Clone)]
pub(crate) struct Available {
    pub version: String,
    pub page: String,
    /// Set when this copy can replace itself: a signed bundle exists, and the app runs
    /// from something the updater knows how to swap (the installed exe, the `.app`, the
    /// AppImage).
    pub installer: Option<Update>,
}

/// The part of `latest.json` read here; the updater reads the rest.
#[derive(Deserialize)]
struct Manifest {
    version: String,
}

fn manifest_url() -> String {
    format!("https://github.com/{REPO}/releases/latest/download/latest.json")
}

/// Ask GitHub. Blocking, so off the UI thread.
pub(crate) fn check() -> Result<Option<Available>, String> {
    let current: Version = VERSION.parse().map_err(|e| format!("{e}"))?;
    let response = ureq::get(manifest_url())
        .header(
            "User-Agent",
            concat!("marketeer/", env!("CARGO_PKG_VERSION")),
        )
        .call();
    let mut response = match response {
        Ok(response) => response,
        // The latest release predates self-updates and has no manifest.
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(err) => {
            return Err(format!(
                "Could not ask GitHub for the latest release: {err}"
            ));
        }
    };
    let manifest: Manifest = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Could not read the latest release: {e}"))?;
    let Some(version) = newer(&manifest.version, &current) else {
        return Ok(None);
    };
    Ok(Some(Available {
        page: format!("https://github.com/{REPO}/releases/tag/{TAG_PREFIX}{version}"),
        installer: installer_for(&version),
        version: version.to_string(),
    }))
}

/// `latest` when it is a version above `current`.
fn newer(latest: &str, current: &Version) -> Option<Version> {
    let latest: Version = latest.trim_start_matches('v').parse().ok()?;
    (latest > *current).then_some(latest)
}

/// The signed bundle for this platform, when this copy can use it.
fn installer_for(version: &Version) -> Option<Update> {
    let pubkey = PUBLIC_KEY.filter(|key| !key.is_empty())?;
    if !replaceable() {
        return None;
    }
    let config = Config {
        endpoints: vec![manifest_url().parse().ok()?],
        pubkey: pubkey.to_string(),
        ..Default::default()
    };
    let update = UpdaterBuilder::new(VERSION.parse().ok()?, config)
        .build()
        .ok()?
        .check()
        .ok()??;
    (update.version.trim_start_matches('v') == version.to_string()).then_some(update)
}

/// Whether the running copy is one the updater can swap in place. A `.deb` install or a
/// binary built by hand is not; those get the release page instead.
fn replaceable() -> bool {
    if cfg!(target_os = "linux") {
        return std::env::var_os("APPIMAGE").is_some();
    }
    if cfg!(target_os = "macos") {
        return std::env::current_exe()
            .is_ok_and(|exe| exe.to_string_lossy().contains(".app/Contents/MacOS/"));
    }
    // The installer puts the app under the user's local app data.
    std::env::current_exe()
        .is_ok_and(|exe| dirs::data_local_dir().is_some_and(|data| exe.starts_with(data)))
}

/// Download, check the signature and install. On Windows the installer takes over and the
/// process ends here; elsewhere the caller restarts.
pub(crate) fn install(update: &Update) -> Result<(), String> {
    update.download_and_install().map_err(|e| format!("{e}"))
}

/// Start the new version. An AppImage runs from a mount that goes away with this process,
/// so start the file itself again, a moment later so the shortcut is free by then.
pub(crate) fn restart(cx: &mut gpui_kit::App) {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let started = std::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 1; exec \"$0\"")
            .arg(appimage)
            .spawn();
        if started.is_ok() {
            cx.quit();
            return;
        }
    }
    cx.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_higher_version_is_an_update() {
        let current: Version = "0.3.0".parse().unwrap();
        assert_eq!(newer("0.3.1", &current).unwrap().to_string(), "0.3.1");
        assert_eq!(newer("v1.0.0", &current).unwrap().to_string(), "1.0.0");
        assert!(newer("0.3.0", &current).is_none());
        assert!(newer("0.2.9", &current).is_none());
        assert!(newer("nonsense", &current).is_none());
    }
}
