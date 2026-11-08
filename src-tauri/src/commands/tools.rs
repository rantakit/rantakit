use std::path::{Path, PathBuf};

use rantakit_tasks::{
    Availability, FileKind, FileMatch, Platform, RecentTools, ToolId, detect_kind, match_kinds,
    tools_for,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "android")]
use crate::android;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub id: &'static str,
    pub job: &'static str,
    pub coming_soon: bool,
}

/// The Tools to list on this platform, in catalogue order. Names, descriptions,
/// and icons come from the frontend, keyed by `id`.
#[tauri::command]
pub fn list_tools() -> Vec<ToolInfo> {
    tools_for(Platform::current())
        .map(|tool| ToolInfo {
            id: tool.id.as_str(),
            job: tool.job.as_str(),
            coming_soon: tool.availability == Availability::ComingSoon,
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum DropMatch {
    Fits {
        kinds: Vec<&'static str>,
        tools: Vec<&'static str>,
    },
    Unsupported,
    Mixed,
    /// An older `.doc`, `.xls`, or `.ppt`.
    OldOffice,
}

/// Which shipped Tools can take every one of `paths`, judged by content, the
/// recently used ones first. Asked while files are dragged over the window,
/// so it runs off the main thread.
#[tauri::command]
pub async fn match_files(app: AppHandle, paths: Vec<PathBuf>) -> DropMatch {
    let kinds: Option<Vec<FileKind>> = paths.iter().map(|path| kind(&app, path)).collect();
    // Only an order; unreadable recent Tools leave the catalogue order.
    let recent = app.state::<RecentTools>().list().unwrap_or_default();
    let matched = match kinds {
        Some(kinds) => match_kinds(&kinds, Platform::current()).recent_first(&recent),
        None => FileMatch::Unsupported,
    };
    match matched {
        FileMatch::Fits { kinds, tools } => DropMatch::Fits {
            kinds: kinds.into_iter().map(FileKind::as_str).collect(),
            tools: tools.into_iter().map(ToolId::as_str).collect(),
        },
        FileMatch::Unsupported => DropMatch::Unsupported,
        FileMatch::Mixed => DropMatch::Mixed,
        FileMatch::OldOffice => DropMatch::OldOffice,
    }
}

/// Android's chosen and shared files are `content://` addresses, which can't be opened as paths.
fn kind(app: &AppHandle, path: &Path) -> Option<FileKind> {
    #[cfg(target_os = "android")]
    if android::is_content(path) {
        return android::kind(app, path);
    }
    let _ = app;
    detect_kind(path)
}
