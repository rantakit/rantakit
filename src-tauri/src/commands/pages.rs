use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rantakit_tasks::{ErrorKind, PdfPages};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "android")]
use crate::android;

/// PDFs kept open for their thumbnails, newest last. A page grid asks for one
/// page at a time, and opening a PDF reads the whole file, so the few in use
/// stay open.
#[derive(Default)]
pub struct OpenPdfs(Mutex<Vec<(PathBuf, Arc<PdfPages>)>>);

/// Enough for Gabung PDF's rows and one page grid at once.
const KEEP_OPEN: usize = 6;

impl OpenPdfs {
    pub(crate) fn get(&self, app: &AppHandle, path: &Path) -> Result<Arc<PdfPages>, ErrorKind> {
        if let Some(pages) = self.find(path) {
            return Ok(pages);
        }
        let pages = Arc::new(open(app, path)?);
        let mut open = self.0.lock().unwrap_or_else(|e| e.into_inner());
        open.retain(|(kept, _)| kept != path);
        open.push((path.to_path_buf(), pages.clone()));
        if open.len() > KEEP_OPEN {
            open.remove(0);
        }
        Ok(pages)
    }

    fn find(&self, path: &Path) -> Option<Arc<PdfPages>> {
        let mut open = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let at = open.iter().position(|(kept, _)| kept == path)?;
        let entry = open.remove(at);
        let pages = entry.1.clone();
        open.push(entry);
        Some(pages)
    }
}

fn open(app: &AppHandle, path: &Path) -> Result<PdfPages, ErrorKind> {
    #[cfg(target_os = "android")]
    if android::is_content(path) {
        return PdfPages::from_bytes(android::read(app, path)?);
    }
    let _ = app;
    PdfPages::open(path)
}

/// Page `page` of the PDF at `path`, counting from 0, as a small JPG at most
/// `size` px on its longest side. Drawn off the main thread; an empty body
/// when it can't be drawn, so the page stays a numbered box.
#[tauri::command]
pub async fn page_thumbnail(app: AppHandle, path: PathBuf, page: usize, size: u32) -> Response {
    let size = size.clamp(32, 1024);
    let drawn = tauri::async_runtime::spawn_blocking(move || {
        app.state::<OpenPdfs>()
            .get(&app, &path)
            .and_then(|pages| pages.thumbnail(page, size))
    })
    .await;
    match drawn {
        Ok(Ok(jpg)) => Response::new(jpg),
        Ok(Err(err)) => {
            eprintln!("page thumbnail failed: {err}");
            Response::new(Vec::new())
        }
        Err(err) => {
            eprintln!("page thumbnail stopped: {err}");
            Response::new(Vec::new())
        }
    }
}
