//! History ("Riwayat"): one entry per finished Task, stored as one JSON file
//! in the app data folder. The shell says where that folder is.
//!
//! All access goes through [`History`], so the storage can later move to
//! SQLite without touching the shells or the frontend.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::catalogue::ToolId;
use crate::task::{FileOutcome, Task};

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("history was saved by a newer version of Rantakit (format {0})")]
    Newer(u32),
}

pub type Result<T> = std::result::Result<T, HistoryError>;

/// Bump when the file layout changes. Version 1 kept one entry per file;
/// v0.2 starts History afresh rather than migrating it.
const VERSION: u32 = 2;
/// Oldest entries are dropped beyond this.
const MAX_ENTRIES: usize = 500;
/// Entries older than this are removed: "Disimpan 30 hari".
pub const KEEP_DAYS: u64 = 30;
const KEEP_MS: u64 = KEEP_DAYS * 24 * 60 * 60 * 1000;

/// One finished Task with at least one Result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// The Tool that ran, by `ToolId::as_str`.
    pub tool: String,
    /// Unix time in milliseconds.
    pub created_at: u64,
    /// Each Result, in the order the files were given.
    pub files: Vec<EntryFile>,
}

impl Entry {
    /// Names the Tool by its id since 0.4.1 when an older one ran: Ubah
    /// Format Foto is Ubah ke PNG when it made PNGs, otherwise Ubah ke JPG.
    fn rename_folded_tool(&mut self) {
        let made_png = || {
            self.files.iter().all(|file| {
                file.result_path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            })
        };
        let tool = match ToolId::parse(&self.tool) {
            Some(ToolId::UbahKeJpg) if self.tool == "ubah-format-foto" && made_png() => {
                ToolId::UbahKePng
            }
            Some(tool) => tool,
            None => return,
        };
        self.tool = tool.as_str().into();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFile {
    pub original_path: PathBuf,
    pub result_path: PathBuf,
    pub original_bytes: u64,
    pub result_bytes: u64,
}

#[derive(Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    /// Newest first.
    entries: Vec<Entry>,
}

/// Read before the rest, to tell an old or newer file from a broken one.
#[derive(Deserialize)]
struct Header {
    version: u32,
}

/// What the history file holds.
enum Stored {
    Current(Vec<Entry>),
    Older,
    Newer(u32),
}

impl Stored {
    fn parse(text: &str) -> serde_json::Result<Self> {
        let header: Header = serde_json::from_str(text)?;
        Ok(match header.version {
            VERSION => Self::Current(serde_json::from_str::<HistoryFile>(text)?.entries),
            newer if newer > VERSION => Self::Newer(newer),
            _ => Self::Older,
        })
    }
}

pub struct History {
    path: PathBuf,
    /// Serializes read-modify-write cycles between commands.
    lock: Mutex<()>,
}

impl History {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Entries, newest first.
    pub fn list(&self) -> Result<Vec<Entry>> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.load()
    }

    pub fn add(&self, entry: Entry) -> Result<()> {
        self.update(|entries| {
            entries.insert(0, entry);
            entries.truncate(MAX_ENTRIES);
        })
    }

    /// Adds a finished Task as one entry with each of its Results. An
    /// Original that made several Results (X3) adds each of them, in order,
    /// with the same Original. A Task without any Result leaves no entry.
    pub fn record(&self, task: &Task, outcomes: &[FileOutcome]) -> Result<()> {
        let files: Vec<EntryFile> = task
            .originals
            .iter()
            .zip(outcomes)
            .filter_map(|(original, outcome)| outcome.as_ref().ok().map(|result| (original, result)))
            .flat_map(|(original, result)| {
                result.files().into_iter().map(|file| EntryFile {
                    original_path: original.clone(),
                    result_path: file.path,
                    original_bytes: result.original_size,
                    result_bytes: file.size,
                })
            })
            .collect();
        if files.is_empty() {
            return Ok(());
        }
        let created_at = now_ms();
        self.add(Entry {
            id: new_id(created_at),
            tool: task.tool.id().as_str().into(),
            created_at,
            files,
        })
    }

    pub fn clear(&self) -> Result<()> {
        self.update(Vec::clear)
    }

    fn update(&self, change: impl FnOnce(&mut Vec<Entry>)) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries = self.load()?;
        change(&mut entries);
        self.save(entries)
    }

    fn load(&self) -> Result<Vec<Entry>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };

        match Stored::parse(&text) {
            Ok(Stored::Current(mut entries)) => {
                let oldest = now_ms().saturating_sub(KEEP_MS);
                entries.retain(|entry| entry.created_at >= oldest);
                entries.iter_mut().for_each(Entry::rename_folded_tool);
                Ok(entries)
            }
            // Written by a newer app version: leave it untouched rather than lose data.
            Ok(Stored::Newer(version)) => Err(HistoryError::Newer(version)),
            // An older layout: History starts afresh.
            Ok(Stored::Older) => {
                fs::remove_file(&self.path)?;
                Ok(Vec::new())
            }
            Err(err) => {
                // Keep the broken file for inspection and start over.
                eprintln!("history file is unreadable, moving it aside: {err}");
                fs::rename(&self.path, self.path.with_extension("json.corrupt"))?;
                Ok(Vec::new())
            }
        }
    }

    /// Writes to a temporary file first so a crash can't leave a half-written history.
    fn save(&self, entries: Vec<Entry>) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let file = HistoryFile {
            version: VERSION,
            entries,
        };
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&file)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Unique within this machine: the timestamp plus a counter for same-millisecond entries.
pub(crate) fn new_id(created_at: u64) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{created_at:x}-{:x}",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::task::{Details, ResultFile, ToolOptions};

    fn used(id: &str, tool: &str) -> Entry {
        Entry {
            id: id.into(),
            tool: tool.into(),
            created_at: now_ms(),
            files: vec![EntryFile {
                original_path: "in.jpg".into(),
                result_path: "in (compressed).jpg".into(),
                original_bytes: 100,
                result_bytes: 50,
            }],
        }
    }

    fn temp_history(name: &str) -> (History, PathBuf) {
        let dir = std::env::temp_dir().join(format!("rantakit-history-{name}"));
        let _ = fs::remove_dir_all(&dir);
        (History::new(dir.join("history.json")), dir)
    }

    #[test]
    fn add_and_clear_round_trip() {
        let (history, dir) = temp_history("round-trip");
        assert!(history.list().unwrap().is_empty());

        history.add(used("a", "kompres-foto")).unwrap();
        history.add(used("b", "kompres-foto")).unwrap();
        let ids: Vec<_> = history.list().unwrap().into_iter().map(|e| e.id).collect();
        assert_eq!(ids, ["b", "a"], "newest entry should come first");

        history.clear().unwrap();
        assert!(history.list().unwrap().is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn entries_of_folded_tools_name_the_tool_they_became() {
        let (history, dir) = temp_history("folded");
        let mut png = used("png", "ubah-format-foto");
        png.files[0].result_path = "in (converted).PNG".into();
        history.add(used("jpg", "ubah-format-foto")).unwrap();
        history.add(png).unwrap();
        history.add(used("pdf", "jpg-ke-pdf")).unwrap();
        history.add(used("pages", "pdf-ke-jpg")).unwrap();
        history.add(used("gone", "word-ke-pdf")).unwrap();

        let tools: Vec<_> = history.list().unwrap().into_iter().map(|e| e.tool).collect();
        assert_eq!(
            tools,
            ["word-ke-pdf", "ubah-ke-jpg", "ubah-ke-pdf", "ubah-ke-png", "ubah-ke-jpg"]
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_task_is_one_entry_with_each_of_its_results() {
        let (history, dir) = temp_history("record");
        let task = Task {
            tool: ToolOptions::KompresPdf(crate::PdfOptions {
                quality: 70,
                max_dimension: None,
                max_bytes: None,
            }),
            originals: vec!["a.pdf".into(), "b.pdf".into(), "c.pdf".into()],
            results_folder: None,
        };
        let done = |path: &str| {
            Ok(ResultFile {
                path: path.into(),
                original_size: 100,
                size: 40,
                details: Details::Pdf {
                    images: 1,
                    images_recompressed: 1,
                },
            })
        };
        let outcomes = [
            done("a (compressed).pdf"),
            Err(crate::ErrorKind::Damaged),
            done("c (compressed).pdf"),
        ];
        history.record(&task, &outcomes).unwrap();

        let entries = history.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool, "kompres-pdf");
        let files: Vec<(&Path, &Path)> = entries[0]
            .files
            .iter()
            .map(|f| (f.original_path.as_path(), f.result_path.as_path()))
            .collect();
        assert_eq!(
            files,
            [
                (Path::new("a.pdf"), Path::new("a (compressed).pdf")),
                (Path::new("c.pdf"), Path::new("c (compressed).pdf"))
            ]
        );

        let failed = [
            Err(crate::ErrorKind::Damaged),
            Err(crate::ErrorKind::Unchanged),
        ];
        history.record(&task, &failed).unwrap();
        assert_eq!(history.list().unwrap().len(), 1, "no Result, no entry");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn several_results_from_one_original_stay_one_entry() {
        let (history, dir) = temp_history("folder");
        let task = Task {
            tool: ToolOptions::KompresPdf(crate::PdfOptions {
                quality: 70,
                max_dimension: None,
                max_bytes: None,
            }),
            originals: vec!["ijazah.pdf".into()],
            results_folder: None,
        };
        let page = |n: u32| crate::task::FolderFile {
            path: format!("ijazah (pages)/ijazah (page {n}).pdf").into(),
            size: 10,
        };
        let outcomes = [Ok(ResultFile {
            path: "ijazah (pages)".into(),
            original_size: 100,
            size: 30,
            details: Details::Folder {
                files: vec![page(1), page(2), page(3)],
            },
        })];
        history.record(&task, &outcomes).unwrap();

        let entries = history.list().unwrap();
        assert_eq!(entries.len(), 1, "one Task, one entry");
        let results: Vec<&Path> = entries[0].files.iter().map(|f| f.result_path.as_path()).collect();
        assert_eq!(
            results,
            [
                Path::new("ijazah (pages)/ijazah (page 1).pdf"),
                Path::new("ijazah (pages)/ijazah (page 2).pdf"),
                Path::new("ijazah (pages)/ijazah (page 3).pdf"),
            ]
        );
        assert!(entries[0].files.iter().all(|f| f.original_path == Path::new("ijazah.pdf")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn history_from_version_1_is_removed() {
        let (history, dir) = temp_history("v1");
        fs::create_dir_all(&dir).unwrap();
        let old = r#"{"version":1,"entries":[{"id":"a","createdAt":0,"inputPath":"a","outputPath":"b","originalBytes":2,"compressedBytes":1,"format":"pdf","width":null,"height":null,"quality":null}]}"#;
        fs::write(&history.path, old).unwrap();

        assert!(history.list().unwrap().is_empty());
        assert!(!history.path.exists(), "the old file is removed");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn keeps_at_most_max_entries() {
        let (history, dir) = temp_history("cap");
        let full = (0..MAX_ENTRIES)
            .rev()
            .map(|i| used(&i.to_string(), "kompres-foto"))
            .collect();
        history.save(full).unwrap();

        history.add(used("new", "kompres-foto")).unwrap();
        let entries = history.list().unwrap();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].id, "new");
        assert_eq!(
            entries[MAX_ENTRIES - 1].id,
            "1",
            "oldest entry should be dropped"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_history_from_newer_version() {
        let (history, dir) = temp_history("newer");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&history.path, r#"{"version":99,"entries":[]}"#).unwrap();

        assert!(matches!(history.list(), Err(HistoryError::Newer(99))));
        assert!(
            history.add(used("a", "kompres-foto")).is_err(),
            "must not overwrite a newer file"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn entries_older_than_30_days_are_removed() {
        let (history, dir) = temp_history("old");
        let day = 24 * 60 * 60 * 1000;
        let at = |id: &str, days: u64| Entry {
            created_at: now_ms() - days * day,
            ..used(id, "kompres-foto")
        };
        history
            .save(vec![at("new", 1), at("month", 29), at("old", 31)])
            .unwrap();

        let ids: Vec<_> = history.list().unwrap().into_iter().map(|e| e.id).collect();
        assert_eq!(ids, ["new", "month"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn moves_corrupt_file_aside() {
        let (history, dir) = temp_history("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&history.path, "not json").unwrap();

        assert!(history.list().unwrap().is_empty());
        assert!(dir.join("history.json.corrupt").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
