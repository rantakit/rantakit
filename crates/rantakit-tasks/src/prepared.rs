//! Scan Dokumen's pages made ahead of time: each photo is straightened and
//! cleaned while the user is still looking the pages over, so "Simpan PDF"
//! only packs pages that are already done.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rantakit_core::{ErrorKind, Result};
use rantakit_image::PhotoEdit;
use rantakit_pdf::PagePhoto;

use crate::task::{PAGE_PHOTO_SIDE, Task};

/// How many versions of one photo are kept, so switching "Bersihkan" and
/// "Warna asli" back and forth does not make the pages again.
const KEPT_PER_PHOTO: usize = 2;

/// One photo made into its page, saved as a JPG in the cache folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// One gray channel rather than three colors.
    pub gray: bool,
}

impl PreparedPage {
    /// The page as a PDF wants it.
    pub fn read(&self) -> Result<PagePhoto> {
        Ok(PagePhoto {
            jpeg: fs::read(&self.path).map_err(|err| ErrorKind::reading(&err))?,
            width: self.width,
            height: self.height,
            gray: self.gray,
        })
    }
}

struct Entry {
    original: PathBuf,
    page: PreparedPage,
    /// When it was last made or asked for; the oldest goes first.
    used: u64,
}

#[derive(Default)]
struct Pages {
    by_key: HashMap<String, Entry>,
    clock: u64,
}

/// The pages made so far, by photo and edit. Only a cache: a page that is
/// missing is made again when the PDF is saved.
pub struct PreparedPages {
    folder: PathBuf,
    pages: Mutex<Pages>,
}

impl PreparedPages {
    /// Pages are kept in `folder`; whatever an earlier run left there is removed.
    pub fn new(folder: PathBuf) -> Self {
        let _ = fs::remove_dir_all(&folder);
        Self {
            folder,
            pages: Mutex::new(Pages::default()),
        }
    }

    /// The photo at `original`, whose bytes are `bytes`, made into its page
    /// as `edit` says, or the page made earlier for the same photo and edit.
    pub fn prepare(&self, original: &Path, bytes: &[u8], edit: &PhotoEdit) -> Result<PreparedPage> {
        if let Some(page) = self.get(original, edit) {
            return Ok(page);
        }
        let photo = rantakit_image::page_photo(bytes, edit, PAGE_PHOTO_SIDE)?;
        fs::create_dir_all(&self.folder).map_err(|err| ErrorKind::writing(&err))?;

        let mut pages = self.lock();
        pages.clock += 1;
        let used = pages.clock;
        let page = PreparedPage {
            path: self.folder.join(format!("{used}.jpg")),
            width: photo.width,
            height: photo.height,
            gray: photo.gray,
        };
        fs::write(&page.path, &photo.jpeg).map_err(|err| ErrorKind::writing(&err))?;
        pages.by_key.insert(
            key(original, edit),
            Entry {
                original: original.to_path_buf(),
                page: page.clone(),
                used,
            },
        );
        pages.trim(original);
        Ok(page)
    }

    /// The page made earlier for this photo and edit, if it is still kept.
    pub fn get(&self, original: &Path, edit: &PhotoEdit) -> Option<PreparedPage> {
        let mut pages = self.lock();
        pages.clock += 1;
        let used = pages.clock;
        let entry = pages.by_key.get_mut(&key(original, edit))?;
        entry.used = used;
        Some(entry.page.clone())
    }

    /// Hands a photos-to-PDF Task the pages made for it, so it does not
    /// make them again.
    pub fn attach(&self, task: &mut Task) {
        let Some(opts) = task.tool.photos_mut() else {
            return;
        };
        opts.prepared = task
            .originals
            .iter()
            .enumerate()
            .map(|(file, original)| self.get(original, &opts.edit(file)))
            .collect();
    }

    /// Removes every page made from these photos, once they are no longer used.
    pub fn forget(&self, originals: &[PathBuf]) {
        self.lock().by_key.retain(|_, entry| {
            let gone = originals.contains(&entry.original);
            if gone {
                let _ = fs::remove_file(&entry.page.path);
            }
            !gone
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Pages> {
        // A panic while holding the lock leaves nothing half done worth refusing.
        self.pages.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Pages {
    /// Keeps only the newest versions of the photo at `original`.
    fn trim(&mut self, original: &Path) {
        let mut versions: Vec<(u64, String)> = self
            .by_key
            .iter()
            .filter(|(_, entry)| entry.original == original)
            .map(|(key, entry)| (entry.used, key.clone()))
            .collect();
        if versions.len() <= KEPT_PER_PHOTO {
            return;
        }
        versions.sort_unstable();
        for (_, key) in &versions[..versions.len() - KEPT_PER_PHOTO] {
            if let Some(entry) = self.by_key.remove(key) {
                let _ = fs::remove_file(&entry.page.path);
            }
        }
    }
}

fn key(original: &Path, edit: &PhotoEdit) -> String {
    format!("{}\n{edit:?}", original.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{PhotosToPdf, ToolOptions};
    use rantakit_pdf::PaperSize;

    fn scratch(test: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rantakit-prepared-{test}-{}", std::process::id()))
    }

    fn photo() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(40, 30, image::Rgb([200, 180, 160]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    fn edit(turns: u8, clean: bool) -> PhotoEdit {
        PhotoEdit {
            corners: None,
            quarter_turns: turns,
            clean,
        }
    }

    #[test]
    fn a_page_is_made_once_and_found_again() {
        let pages = PreparedPages::new(scratch("once"));
        let original = Path::new("photo-1.jpg");
        let made = pages.prepare(original, &photo(), &edit(1, false)).unwrap();
        assert_eq!((made.width, made.height), (30, 40));
        assert!(made.path.exists());
        assert_eq!(pages.prepare(original, b"not read again", &edit(1, false)).unwrap(), made);
        assert_eq!(pages.get(original, &edit(1, false)), Some(made));
        assert_eq!(pages.get(original, &edit(0, false)), None);
    }

    #[test]
    fn only_the_newest_versions_of_a_photo_are_kept() {
        let pages = PreparedPages::new(scratch("newest"));
        let original = Path::new("photo-1.jpg");
        let oldest = pages.prepare(original, &photo(), &edit(0, false)).unwrap();
        pages.prepare(original, &photo(), &edit(1, false)).unwrap();
        pages.prepare(original, &photo(), &edit(2, false)).unwrap();
        assert_eq!(pages.get(original, &edit(0, false)), None);
        assert!(!oldest.path.exists());
        assert!(pages.get(original, &edit(1, false)).is_some());
        assert!(pages.get(original, &edit(2, false)).is_some());
    }

    #[test]
    fn forgotten_photos_lose_their_pages() {
        let pages = PreparedPages::new(scratch("forget"));
        let kept = PathBuf::from("photo-1.jpg");
        let gone = PathBuf::from("photo-2.jpg");
        pages.prepare(&kept, &photo(), &edit(0, false)).unwrap();
        let removed = pages.prepare(&gone, &photo(), &edit(0, false)).unwrap();
        pages.forget(std::slice::from_ref(&gone));
        assert!(pages.get(&kept, &edit(0, false)).is_some());
        assert_eq!(pages.get(&gone, &edit(0, false)), None);
        assert!(!removed.path.exists());
    }

    #[test]
    fn a_scan_task_gets_the_pages_made_for_its_look() {
        let pages = PreparedPages::new(scratch("attach"));
        let first = PathBuf::from("photo-1.jpg");
        let second = PathBuf::from("photo-2.jpg");
        let made = pages.prepare(&first, &photo(), &edit(1, true)).unwrap();
        pages.prepare(&second, &photo(), &edit(0, false)).unwrap();
        let mut task = Task {
            tool: ToolOptions::ScanDocument(PhotosToPdf {
                date: "2026-11-11".into(),
                paper: PaperSize::A4,
                margin: true,
                edits: vec![edit(1, false)],
                clean: true,
                prepared: Vec::new(),
            }),
            originals: vec![first, second],
            results_folder: None,
        };
        pages.attach(&mut task);
        let ToolOptions::ScanDocument(opts) = &task.tool else {
            unreachable!()
        };
        // The second photo was made in "Warna asli", not the Task's "Bersihkan".
        assert_eq!(opts.prepared, vec![Some(made), None]);
    }
}
