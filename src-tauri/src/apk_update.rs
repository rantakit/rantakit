//! "New version" for the APK from GitHub. The Kotlin side, `UpdatePlugin.kt`,
//! reads the version in the latest release's `latest.json`; the user then
//! downloads and installs the new APK from the release page, so Rantakit needs
//! no permission to install apps.

use serde_json::json;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Wry};

/// The same file the desktop updater reads (`plugins.updater` in tauri.conf.json).
const LATEST: &str = "https://github.com/moez-rd/rantakit/releases/latest/download/latest.json";

/// Builds for Google Play or F-Droid, whose store updates the app, are made
/// with `RANTAKIT_STORE_BUILD` set and never check.
pub const AVAILABLE: bool = option_env!("RANTAKIT_STORE_BUILD").is_none();

struct Plugin(PluginHandle<Wry>);

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("apk-update")
        .setup(|app, api| {
            app.manage(Plugin(
                api.register_android_plugin("com.rantakit", "UpdatePlugin")?,
            ));
            Ok(())
        })
        .build()
}

#[derive(serde::Deserialize)]
struct Latest {
    version: String,
}

/// The latest release's version when it is newer than this one; `None` when
/// it isn't, or when GitHub can't be reached.
pub async fn newer_version(app: &AppHandle) -> Option<String> {
    let plugin = &app.state::<Plugin>().0;
    let latest = match plugin
        .run_mobile_plugin_async::<Latest>("latestVersion", json!({ "url": LATEST }))
        .await
    {
        Ok(latest) => latest.version,
        Err(err) => {
            eprintln!("update check failed: {err}");
            return None;
        }
    };
    let version = semver::Version::parse(latest.trim_start_matches('v')).ok()?;
    (version > app.package_info().version).then(|| version.to_string())
}
