//! Scan Dokumen's camera (D3). Android only; elsewhere there is no camera,
//! and Scan Dokumen is not listed.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

/// "granted", "ask", or "refused"; "refused" where there is no camera.
#[tauri::command]
pub async fn camera_permission(app: AppHandle) -> String {
    #[cfg(target_os = "android")]
    return crate::camera::permission(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        "refused".into()
    }
}

/// Shows Android's camera prompt, after the app has said why.
#[tauri::command]
pub async fn ask_camera_permission(app: AppHandle) -> bool {
    #[cfg(target_os = "android")]
    return crate::camera::ask_permission(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        false
    }
}

/// Opens Rantakit's page in the phone's Settings, to allow the camera there.
#[tauri::command]
pub async fn open_app_settings(app: AppHandle) {
    #[cfg(target_os = "android")]
    crate::camera::open_settings(&app).await;
    #[cfg(not(target_os = "android"))]
    let _ = app;
}

/// What the camera hands back.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Taken {
    /// The photos, in the order they were taken.
    pub paths: Vec<PathBuf>,
    /// "Dari Galeri" closed the camera: the frontend opens the picker next.
    pub gallery: bool,
}

/// Opens the camera; `words` is its text. The photos taken, in order, and
/// whether "Dari Galeri" closed it.
#[tauri::command]
pub async fn take_photos(app: AppHandle, words: Value) -> Taken {
    #[cfg(target_os = "android")]
    return crate::camera::take(&app, words).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, words);
        Taken::default()
    }
}

/// Removes photos the camera took, once their Task has ended or been left.
/// Anything not taken by the camera is left alone.
#[tauri::command]
pub async fn discard_photos(app: AppHandle, paths: Vec<PathBuf>) {
    #[cfg(target_os = "android")]
    for path in paths
        .iter()
        .filter(|path| crate::camera::is_scan(&app, path))
    {
        let _ = std::fs::remove_file(path);
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, paths);
}
