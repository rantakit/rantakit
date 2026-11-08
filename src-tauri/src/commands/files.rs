use std::path::{Path, PathBuf};

use rantakit_tasks::{FileKind, file_info as read_info};
use serde::Serialize;
use tauri::AppHandle;
use tauri::ipc::Response;
#[cfg(not(target_os = "android"))]
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "android")]
use crate::android;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub size: Option<u64>,
    pub kind: Option<&'static str>,
    pub pages: Option<usize>,
    pub longest_side: Option<u32>,
    /// Photos only: the info Hapus Info Lokasi would remove.
    pub found: Option<FoundRequest>,
}

/// `FoundInfo` for the frontend.
#[derive(Debug, Serialize)]
pub struct FoundRequest {
    pub location: bool,
    pub camera: bool,
    pub other: bool,
}

/// What the Files step shows next to each Original: size, and the page count
/// for PDFs. Reads file headers, so it runs off the main thread.
#[tauri::command]
pub async fn file_info(app: AppHandle, paths: Vec<PathBuf>) -> Vec<FileInfo> {
    paths
        .iter()
        .map(|path| {
            let info = info(&app, path);
            FileInfo {
                size: info.size,
                kind: info.kind.map(FileKind::as_str),
                pages: info.pages,
                longest_side: info.longest_side,
                found: info.found.map(|found| FoundRequest {
                    location: found.location,
                    camera: found.camera,
                    other: found.other,
                }),
            }
        })
        .collect()
}

fn info(app: &AppHandle, path: &Path) -> rantakit_tasks::FileInfo {
    #[cfg(target_os = "android")]
    if android::is_content(path) {
        return android::file_info(app, path);
    }
    let _ = app;
    read_info(path)
}

/// Android's system file picker for "Pilih foto", "Pilih PDF", and "Buka".
/// The dialog plugin would open the Photo Picker for photos, which hides
/// their names. Picks nothing elsewhere; desktop uses the dialog plugin.
#[tauri::command]
pub async fn pick_files(app: AppHandle, mime_types: Vec<String>) -> Vec<PathBuf> {
    #[cfg(target_os = "android")]
    return android::pick(&app, &mime_types).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, mime_types);
        Vec::new()
    }
}

/// Photos and PDFs shared to Rantakit from another app on Android, since the
/// last call; "shared-files" says when more arrive. None elsewhere.
#[tauri::command]
pub async fn take_shared_files(app: AppHandle) -> Vec<PathBuf> {
    #[cfg(target_os = "android")]
    return crate::share_in::take(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Vec::new()
    }
}

/// The names of files chosen on Android, whose `content://` addresses do not
/// contain them. `None` for a plain path, which ends in its name.
#[tauri::command]
pub async fn file_names(app: AppHandle, paths: Vec<PathBuf>) -> Vec<Option<String>> {
    #[cfg(target_os = "android")]
    return paths
        .iter()
        .map(|path| {
            android::is_content(path)
                .then(|| android::name(&app, path))
                .flatten()
        })
        .collect();
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        vec![None; paths.len()]
    }
}

/// A small JPG of a photo, as raw bytes, for the file rows and the Result
/// preview. Fails quietly with an empty body; the row then shows an icon.
#[tauri::command]
pub async fn thumbnail(app: AppHandle, path: PathBuf, size: u32) -> Response {
    let size = size.clamp(32, 1024);
    #[cfg(target_os = "android")]
    if android::is_content(&path) {
        return Response::new(android::thumbnail(&app, &path, size).unwrap_or_default());
    }
    let _ = app;
    Response::new(rantakit_tasks::thumbnail(&path, size).unwrap_or_default())
}

/// Opens a Result in the app the system uses for it ("Buka"). Android asks
/// which app, since other apps can't open a path there.
#[tauri::command]
pub fn open_path(app: AppHandle, path: PathBuf) {
    #[cfg(target_os = "android")]
    if let Err(error) = android::open(&app, &path) {
        eprintln!("open failed: {error}");
    }
    #[cfg(not(target_os = "android"))]
    if let Err(error) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        eprintln!("open failed: {error}");
    }
}

/// Android 9 and earlier ask for the storage permission before the first
/// Task, so that Results can be saved in `Download/`. Always true elsewhere.
#[tauri::command]
pub async fn can_save_results(app: AppHandle) -> bool {
    #[cfg(target_os = "android")]
    return android::can_save(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        true
    }
}

/// Shows the system's storage permission prompt, after the app has said why.
#[tauri::command]
pub async fn ask_to_save_results(app: AppHandle) -> bool {
    #[cfg(target_os = "android")]
    return android::ask_to_save(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        true
    }
}
