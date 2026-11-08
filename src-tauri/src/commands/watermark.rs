//! Beri Watermark's live preview (K1): the first file, a photo or a PDF's
//! first page, with the watermark as it is being typed.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rantakit_tasks::{ErrorKind, PhotoEdit, WatermarkOptions};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

use super::pages::OpenPdfs;
use super::photos::read;
use super::task::WatermarkRequest;

/// The last small picture the preview drew on, so typing does not read and
/// shrink the file again for every letter.
#[derive(Default)]
pub struct PreviewBase(Mutex<Option<Kept>>);

/// The file, the size asked for, and the picture.
type Kept = (PathBuf, u32, Arc<Vec<u8>>);

impl PreviewBase {
    fn get(
        &self,
        app: &AppHandle,
        path: PathBuf,
        pdf: bool,
        size: u32,
    ) -> Result<Arc<Vec<u8>>, ErrorKind> {
        let mut kept = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, kept_size, picture)) = kept.as_ref()
            && *at == path
            && *kept_size == size
        {
            return Ok(picture.clone());
        }
        let picture = Arc::new(if pdf {
            app.state::<OpenPdfs>()
                .get(app, &path)?
                .thumbnail(0, size)?
        } else {
            rantakit_tasks::edited_preview(&read(app, &path)?, &PhotoEdit::default(), size)?
        });
        *kept = Some((path, size, picture.clone()));
        Ok(picture)
    }
}

/// A JPG of the file at `path` with `watermark` on it, at most `size` px
/// on its longest side; `pdf` says it is a PDF, shown by its first page.
/// An empty body when it can't be made, so the preview shows nothing new.
#[tauri::command]
pub async fn watermark_preview(
    app: AppHandle,
    path: PathBuf,
    pdf: bool,
    watermark: WatermarkRequest,
    size: u32,
) -> Response {
    let size = size.clamp(64, 1600);
    let made = tauri::async_runtime::spawn_blocking(move || {
        let picture = app.state::<PreviewBase>().get(&app, path, pdf, size)?;
        rantakit_tasks::watermark_preview(&picture, &WatermarkOptions::from(watermark))
    })
    .await;
    match made {
        Ok(Ok(jpg)) => Response::new(jpg),
        Ok(Err(err)) => {
            eprintln!("watermark preview failed: {err}");
            Response::new(Vec::new())
        }
        Err(err) => {
            eprintln!("watermark preview stopped: {err}");
            Response::new(Vec::new())
        }
    }
}
