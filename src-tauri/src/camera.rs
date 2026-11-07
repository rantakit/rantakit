//! Scan Dokumen's camera on Android (D3). The Kotlin side, `CameraPlugin.kt`,
//! asks for the camera permission and opens `CameraActivity.kt`, which saves
//! each photo into the app's cache (`scans/`), never to Gallery. The photos
//! are removed when the Task ends or is left, and at the next launch.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Wry};

use crate::commands::camera::Taken;

struct Plugin(PluginHandle<Wry>);

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("camera")
        .setup(|app, api| {
            app.manage(Plugin(
                api.register_android_plugin("com.rantakit", "CameraPlugin")?,
            ));
            Ok(())
        })
        .build()
}

/// Where the camera puts its photos.
pub fn folder(app: &AppHandle) -> io::Result<PathBuf> {
    let cache = app.path().app_cache_dir().map_err(io::Error::other)?;
    Ok(cache.join("scans"))
}

/// Whether `path` is a photo the camera took, which Rantakit may remove.
pub fn is_scan(app: &AppHandle, path: &Path) -> bool {
    folder(app).is_ok_and(|folder| path.parent() == Some(folder.as_path()))
}

/// Removes the photos of Scan Dokumen Tasks Android cut short; called at launch.
pub fn clear(app: &AppHandle) {
    if let Ok(folder) = folder(app) {
        let _ = fs::remove_dir_all(folder);
    }
}

async fn call(app: &AppHandle, command: &str, args: Value) -> Option<Value> {
    let plugin = &app.state::<Plugin>().0;
    match plugin.run_mobile_plugin_async(command, args).await {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("camera: {command} failed: {err}");
            None
        }
    }
}

/// "granted", "ask", or "refused" (only the phone's Settings can allow it now).
pub async fn permission(app: &AppHandle) -> String {
    call(app, "cameraPermission", json!({}))
        .await
        .and_then(|value| value["state"].as_str().map(String::from))
        .unwrap_or_else(|| "refused".into())
}

/// Shows Android's prompt; whether the camera may be used.
pub async fn ask_permission(app: &AppHandle) -> bool {
    call(app, "askCameraPermission", json!({}))
        .await
        .and_then(|value| value["granted"].as_bool())
        .unwrap_or(false)
}

/// Opens Rantakit's page in the phone's Settings.
pub async fn open_settings(app: &AppHandle) {
    call(app, "openSettings", json!({})).await;
}

/// Opens the camera with `words`, its text in the app's language.
pub async fn take(app: &AppHandle, words: Value) -> Taken {
    let Ok(folder) = folder(app) else {
        return Taken::default();
    };
    let args = json!({ "folder": folder, "words": words });
    call(app, "takePhotos", args)
        .await
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}
