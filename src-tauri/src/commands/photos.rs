//! "Potong & luruskan" (D1): finding the paper in a photo, and showing the
//! photo as it will go on its page.

use std::path::{Path, PathBuf};

use rantakit_tasks::{Corners, ErrorKind, PhotoEdit, PreparedPages};
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tauri::ipc::Response;

#[cfg(target_os = "android")]
use crate::android;

/// How one photo becomes a page, as the frontend keeps it.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequest {
    /// Fractions of the upright photo, clockwise from the top left; the
    /// whole photo when left out.
    #[serde(default)]
    pub corners: Option<Corners>,
    /// Quarter turns clockwise; negative turns left.
    #[serde(default)]
    pub turns: i32,
    /// "Bersihkan", for a Scan Dokumen row's preview.
    #[serde(default)]
    pub clean: bool,
}

impl From<EditRequest> for PhotoEdit {
    fn from(edit: EditRequest) -> Self {
        Self {
            corners: edit.corners,
            quarter_turns: edit.turns.rem_euclid(4) as u8,
            clean: edit.clean,
        }
    }
}

pub(crate) fn read(app: &AppHandle, path: &Path) -> Result<Vec<u8>, ErrorKind> {
    #[cfg(target_os = "android")]
    if android::is_content(path) {
        return android::read(app, path);
    }
    let _ = app;
    std::fs::read(path).map_err(|err| ErrorKind::reading(&err))
}

/// Where the paper is in the photo at `path`; None when no clear edge is
/// found or the photo can't be read, and the photo is then used whole.
#[tauri::command]
pub async fn find_paper(app: AppHandle, path: PathBuf) -> Option<Corners> {
    let found = tauri::async_runtime::spawn_blocking(move || {
        read(&app, &path).and_then(|bytes| rantakit_tasks::find_paper(&bytes))
    })
    .await;
    match found {
        Ok(Ok(corners)) => corners,
        Ok(Err(err)) => {
            eprintln!("finding the paper failed: {err}");
            None
        }
        Err(err) => {
            eprintln!("finding the paper stopped: {err}");
            None
        }
    }
}

/// A small JPG of the photo at `path` cropped, straightened and turned as
/// `edit` says, at most `size` px on its longest side. An empty body when
/// it can't be made, so the row shows the plain thumbnail.
#[tauri::command]
pub async fn edited_preview(app: AppHandle, path: PathBuf, edit: EditRequest, size: u32) -> Response {
    let size = size.clamp(32, 2048);
    let made = tauri::async_runtime::spawn_blocking(move || {
        read(&app, &path).and_then(|bytes| rantakit_tasks::edited_preview(&bytes, &edit.into(), size))
    })
    .await;
    match made {
        Ok(Ok(jpg)) => Response::new(jpg),
        Ok(Err(err)) => {
            eprintln!("photo preview failed: {err}");
            Response::new(Vec::new())
        }
        Err(err) => {
            eprintln!("photo preview stopped: {err}");
            Response::new(Vec::new())
        }
    }
}

/// Makes the photo at `path` into its Scan Dokumen page as `edit` says,
/// keeping it for "Simpan PDF", and answers with a JPG of that page at most
/// `size` px on its longest side. An empty body when it can't be made; the
/// page is then made again when the PDF is saved.
#[tauri::command]
pub async fn prepare_page(app: AppHandle, path: PathBuf, edit: EditRequest, size: u32) -> Response {
    let size = size.clamp(32, 2048);
    let made = tauri::async_runtime::spawn_blocking(move || {
        let edit = PhotoEdit::from(edit);
        let pages = app.state::<PreparedPages>();
        let page = match pages.get(&path, &edit) {
            Some(page) => page,
            None => pages.prepare(&path, &read(&app, &path)?, &edit)?,
        };
        let jpeg = std::fs::read(&page.path).map_err(|err| ErrorKind::reading(&err))?;
        rantakit_tasks::edited_preview(&jpeg, &PhotoEdit::default(), size)
    })
    .await;
    match made {
        Ok(Ok(jpg)) => Response::new(jpg),
        Ok(Err(err)) => {
            eprintln!("preparing the page failed: {err}");
            Response::new(Vec::new())
        }
        Err(err) => {
            eprintln!("preparing the page stopped: {err}");
            Response::new(Vec::new())
        }
    }
}
