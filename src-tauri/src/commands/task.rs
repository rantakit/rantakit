use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use rantakit_tasks::{
    CancelToken, Compression, Details, ErrorKind, FileOutcome, History, ImageFormat, Level, OutputFormat,
    PageEdit, PhotoSize, Progress, RecentLimits, RecentTools, SettingsStore, Split, Task, TaskEvent,
    ToolOptions, WHITE, dpi_to_side, run_task,
};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

/// Cancel tokens of the Tasks that are still running, by Task id.
#[derive(Default)]
pub struct RunningTasks {
    next_id: AtomicU64,
    tokens: Mutex<HashMap<u64, CancelToken>>,
}

impl RunningTasks {
    fn add(&self, cancel: CancelToken) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.tokens().insert(id, cancel);
        id
    }

    fn remove(&self, id: u64) -> Option<CancelToken> {
        self.tokens().remove(&id)
    }

    fn tokens(&self) -> std::sync::MutexGuard<'_, HashMap<u64, CancelToken>> {
        self.tokens.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRequest {
    pub options: OptionsRequest,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "tool",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum OptionsRequest {
    KompresFoto {
        /// Output format; `None` keeps the Original's format.
        #[serde(default)]
        format: Option<Format>,
        #[serde(flatten)]
        compression: CompressionRequest,
    },
    KompresPdf {
        #[serde(flatten)]
        compression: CompressionRequest,
    },
    UbahFormatFoto {
        to: Format,
        /// JPG quality; ignored for PNG.
        #[serde(default = "default_convert_quality")]
        quality: u8,
        /// "#rrggbb" behind transparent parts of a PNG that becomes a JPG;
        /// white when omitted or unreadable.
        #[serde(default)]
        background: Option<String>,
    },
    UbahUkuranFoto {
        #[serde(default)]
        size: SizeRequest,
        /// "Sisi terpanjang (px)" from Advanced options; wins over `size`.
        #[serde(default)]
        max_side: Option<u32>,
    },
    GabungPdf {
        /// The user's today, such as "2026-11-07", for `merged 2026-11-07.pdf`.
        date: String,
    },
    PisahPdf {
        /// "Ambil halaman": pages counted from 1, in order.
        #[serde(default)]
        pages: Option<Vec<usize>>,
        /// "Pisah semua halaman" (1) or "Setiap N halaman"; used when `pages` is left out.
        #[serde(default)]
        every: Option<usize>,
    },
    AturHalamanPdf {
        /// The Result's pages in order.
        pages: Vec<PageRequest>,
    },
}

/// One page of Atur Halaman PDF's Result.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    /// The page of the Original, counted from 0.
    pub page: usize,
    /// Quarter turns clockwise; negative turns left.
    #[serde(default)]
    pub turns: i32,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeRequest {
    Large,
    #[default]
    Medium,
    Small,
}

impl From<SizeRequest> for PhotoSize {
    fn from(size: SizeRequest) -> Self {
        match size {
            SizeRequest::Large => Self::Large,
            SizeRequest::Medium => Self::Medium,
            SizeRequest::Small => Self::Small,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Jpeg,
    Png,
}

fn default_convert_quality() -> u8 {
    90
}

/// "#1a2b3c" as RGB; white for anything else.
fn parse_color(hex: Option<&str>) -> [u8; 3] {
    let channel = |hex: &str, at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    hex.and_then(|hex| hex.strip_prefix('#'))
        .filter(|hex| hex.len() == 6)
        .and_then(|hex| Some([channel(hex, 0)?, channel(hex, 2)?, channel(hex, 4)?]))
        .unwrap_or(WHITE)
}

impl OptionsRequest {
    /// The Size limit typed for a compress Tool, if any.
    fn limit(&self) -> Option<u64> {
        match self {
            Self::KompresFoto { compression, .. } | Self::KompresPdf { compression } => {
                compression.limit
            }
            _ => None,
        }
    }
}

/// The compress Tools' Options; `rantakit_tasks::Compression` turns them into engine options.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionRequest {
    /// The Size limit in bytes; omit to compress by `level`.
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub level: LevelRequest,
    /// The exact quality from Advanced options.
    #[serde(default)]
    pub quality: Option<u8>,
    #[serde(default)]
    pub max_dimension: Option<u32>,
    /// Kompres PDF's "Resolusi gambar maksimum", in dpi on an A4 page.
    #[serde(default)]
    pub max_dpi: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LevelRequest {
    Smallest,
    #[default]
    Balanced,
    Clearest,
}

impl From<CompressionRequest> for Compression {
    fn from(request: CompressionRequest) -> Self {
        Self {
            limit: request.limit,
            level: match request.level {
                LevelRequest::Smallest => Level::Smallest,
                LevelRequest::Balanced => Level::Balanced,
                LevelRequest::Clearest => Level::Clearest,
            },
            quality: request.quality,
            max_dimension: request.max_dimension.or(request.max_dpi.map(dpi_to_side)),
        }
    }
}

impl From<OptionsRequest> for ToolOptions {
    fn from(request: OptionsRequest) -> Self {
        match request {
            OptionsRequest::KompresFoto {
                format,
                compression,
            } => Self::KompresFoto(Compression::from(compression).photo(match format {
                None => OutputFormat::Same,
                Some(Format::Jpeg) => OutputFormat::To(ImageFormat::Jpeg),
                Some(Format::Png) => OutputFormat::To(ImageFormat::Png),
            })),
            OptionsRequest::KompresPdf { compression } => {
                Self::KompresPdf(Compression::from(compression).pdf())
            }
            OptionsRequest::UbahFormatFoto {
                to,
                quality,
                background,
            } => Self::convert(
                match to {
                    Format::Jpeg => ImageFormat::Jpeg,
                    Format::Png => ImageFormat::Png,
                },
                quality,
                parse_color(background.as_deref()),
            ),
            OptionsRequest::UbahUkuranFoto { size, max_side } => {
                Self::resize(max_side.unwrap_or(PhotoSize::from(size).side()))
            }
            OptionsRequest::GabungPdf { date } => Self::GabungPdf { date },
            OptionsRequest::PisahPdf { pages, every } => Self::PisahPdf(match pages {
                Some(pages) => Split::Take(pages),
                None => Split::Every(every.unwrap_or(1).max(1)),
            }),
            OptionsRequest::AturHalamanPdf { pages } => Self::AturHalamanPdf(
                pages
                    .into_iter()
                    .map(|page| PageEdit {
                        page: page.page,
                        quarter_turns: page.turns.rem_euclid(4) as u8,
                    })
                    .collect(),
            ),
        }
    }
}

/// What the frontend hears while a Task runs. `file` is an index into the
/// request's `paths`.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TaskMessage {
    Progress {
        file: usize,
        fraction: f32,
    },
    FileDone {
        file: usize,
        outcome: Outcome,
    },
    /// Every file has ended; nothing more is sent.
    Finished,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Outcome {
    Done { result: ResultMessage },
    Failed { error: ErrorMessage },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMessage {
    pub input_path: PathBuf,
    /// The Result, or the new folder holding several (X3).
    pub output_path: PathBuf,
    pub original_bytes: u64,
    pub compressed_bytes: u64,
    /// When one Original made several Results: each, in order, inside
    /// `output_path`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<PartMessage>>,
    /// How many pages a PDF made of pages has: merged, taken out, or arranged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<usize>,
    /// Pages asked for that the Original does not have, counted from 1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_pages: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartMessage {
    pub path: PathBuf,
    pub bytes: u64,
}

/// An error kind as a code; `best` is set for "size-limit-unreachable".
#[derive(Debug, Clone, Serialize)]
pub struct ErrorMessage {
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<u64>,
}

impl From<&ErrorKind> for ErrorMessage {
    fn from(kind: &ErrorKind) -> Self {
        let best = match kind {
            ErrorKind::SizeLimitUnreachable { best } => Some(*best),
            _ => None,
        };
        Self {
            code: kind.code(),
            best,
        }
    }
}

fn outcome(original: &Path, outcome: &FileOutcome) -> Outcome {
    match outcome {
        Ok(result) => Outcome::Done {
            result: ResultMessage {
                input_path: original.to_path_buf(),
                output_path: result.path.clone(),
                original_bytes: result.original_size,
                compressed_bytes: result.size,
                files: match &result.details {
                    Details::Folder { files } => Some(
                        files
                            .iter()
                            .map(|file| PartMessage {
                                path: file.path.clone(),
                                bytes: file.size,
                            })
                            .collect(),
                    ),
                    _ => None,
                },
                pages: match &result.details {
                    Details::Pages { pages, .. } => Some(*pages),
                    _ => None,
                },
                skipped_pages: match &result.details {
                    Details::Pages { skipped, .. } => skipped.clone(),
                    _ => Vec::new(),
                },
            },
        },
        Err(kind) => Outcome::Failed { error: kind.into() },
    }
}

fn fraction(progress: Progress) -> f32 {
    match progress {
        Progress::Fraction(fraction) => fraction,
        Progress::Page { page, pages } => page as f32 / pages.max(1) as f32,
    }
}

/// Starts a Task in the background and returns its id for `cancel_task`.
/// Progress and each file's outcome arrive on `on_event`, then `Finished`.
#[tauri::command]
pub fn start_task(
    app: AppHandle,
    running: State<'_, RunningTasks>,
    request: TaskRequest,
    on_event: Channel<TaskMessage>,
) -> u64 {
    if let Some(limit) = request.options.limit() {
        // Only a convenience for next time, so a failure is only logged.
        if let Err(err) = app.state::<RecentLimits>().record(limit) {
            eprintln!("could not remember the Size limit: {err}");
        }
    }
    // "Tempat menyimpan hasil" from Settings; unreadable settings save next to each Original.
    let settings = app.state::<SettingsStore>().get().unwrap_or_default();
    let task = Task {
        tool: request.options.into(),
        originals: request.paths,
        results_folder: settings.results_folder.clone(),
    };
    // Recent tools are kept even while History is off.
    if let Err(err) = app
        .state::<RecentTools>()
        .record(task.tool.id().as_str().into())
    {
        eprintln!("could not remember the Tool: {err}");
    }
    let cancel = CancelToken::new();
    let id = running.add(cancel.clone());

    tauri::async_runtime::spawn_blocking(move || {
        // The Kotlin side runs on the main thread, so it is called from
        // here; the app is still on screen, as Android requires.
        #[cfg(target_os = "android")]
        let mut notice =
            crate::task_notice::Notice::start(&app, task.tool.id(), task.originals.len());
        let send = |message| {
            if let Err(err) = on_event.send(message) {
                eprintln!("task {id}: could not reach the frontend: {err}");
            }
        };
        let outcomes = run(
            &app,
            id,
            &task,
            &mut |event| match event {
                TaskEvent::Progress { file, progress } => {
                    let fraction = fraction(progress);
                    #[cfg(target_os = "android")]
                    notice.progress(file, fraction);
                    send(TaskMessage::Progress { file, fraction })
                }
                TaskEvent::FileDone {
                    file,
                    outcome: done,
                } => send(TaskMessage::FileDone {
                    file,
                    outcome: outcome(&task.originals[file], done),
                }),
            },
            &cancel,
        );

        // The Results are already saved, so a History failure is only logged.
        if settings.keep_history
            && let Err(err) = app.state::<History>().record(&task, &outcomes)
        {
            eprintln!("task {id}: could not record history: {err}");
        }
        let running = app.state::<RunningTasks>();
        running.remove(id);
        // The service stays while another Task still runs.
        #[cfg(target_os = "android")]
        if running.tokens().is_empty() {
            notice.finish(&outcomes);
        }
        send(TaskMessage::Finished);
    });

    id
}

/// Whether Task notifications may be shown: "granted", "ask" before the
/// first Task on Android 13 and later, or "refused". Always "granted" elsewhere.
#[tauri::command]
pub async fn notification_permission(app: AppHandle) -> String {
    #[cfg(target_os = "android")]
    return crate::task_notice::permission(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        "granted".into()
    }
}

/// Shows Android's notification prompt, after the app has said why. A Task
/// runs whatever the answer; only its notification is hidden.
#[tauri::command]
pub async fn ask_notification_permission(app: AppHandle) -> bool {
    #[cfg(target_os = "android")]
    return crate::task_notice::ask_permission(&app).await;
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        true
    }
}

/// Runs `task` where the platform keeps its files: as it is on desktop.
#[cfg(not(target_os = "android"))]
fn run(
    _app: &AppHandle,
    _id: u64,
    task: &Task,
    on_event: &mut dyn FnMut(TaskEvent),
    cancel: &CancelToken,
) -> Vec<FileOutcome> {
    run_task(task, on_event, cancel)
}

/// Runs `task` where the platform keeps its files: on Android each Original
/// is copied into the cache when its turn comes, and each Result is saved to
/// `Download/Rantakit/`.
#[cfg(target_os = "android")]
fn run(
    app: &AppHandle,
    id: u64,
    task: &Task,
    on_event: &mut dyn FnMut(TaskEvent),
    cancel: &CancelToken,
) -> Vec<FileOutcome> {
    use crate::android;

    let workspace = android::Workspace::new(app, id);
    if task.tool.merges() {
        return match &workspace {
            Ok(workspace) => merge_on_android(app, workspace, task, on_event, cancel),
            Err(err) => {
                let outcomes: Vec<FileOutcome> = task
                    .originals
                    .iter()
                    .map(|_| Err(ErrorKind::writing(err)))
                    .collect();
                for (file, outcome) in outcomes.iter().enumerate() {
                    on_event(TaskEvent::FileDone { file, outcome });
                }
                outcomes
            }
        };
    }
    let mut outcomes = Vec::with_capacity(task.originals.len());
    for (file, original) in task.originals.iter().enumerate() {
        let outcome = match &workspace {
            _ if cancel.is_cancelled() => Err(ErrorKind::Cancelled),
            Err(err) => Err(ErrorKind::writing(err)),
            Ok(workspace) => workspace.copy_in(app, file, original).and_then(|copy| {
                let one = Task {
                    tool: task.tool.clone(),
                    originals: vec![copy.clone()],
                    results_folder: Some(workspace.results()),
                };
                let mut progress = |event: TaskEvent| {
                    if let TaskEvent::Progress { progress, .. } = event {
                        on_event(TaskEvent::Progress { file, progress });
                    }
                };
                let result = run_task(&one, &mut progress, cancel).pop();
                let saved = result
                    .expect("one Original")
                    .and_then(|result| android::save(app, result));
                workspace.remove(file);
                saved
            }),
        };
        on_event(TaskEvent::FileDone {
            file,
            outcome: &outcome,
        });
        outcomes.push(outcome);
    }
    outcomes
}

/// Gabung PDF on Android: every Original is copied in first, then merged and
/// saved once. A file that cannot be copied ends the Task as a file that
/// cannot be read does: that file says why and the rest are `Cancelled`.
#[cfg(target_os = "android")]
fn merge_on_android(
    app: &AppHandle,
    workspace: &crate::android::Workspace,
    task: &Task,
    on_event: &mut dyn FnMut(TaskEvent),
    cancel: &CancelToken,
) -> Vec<FileOutcome> {
    let count = task.originals.len();
    fn finish(on_event: &mut dyn FnMut(TaskEvent), outcomes: Vec<FileOutcome>) -> Vec<FileOutcome> {
        for (file, outcome) in outcomes.iter().enumerate() {
            on_event(TaskEvent::FileDone { file, outcome });
        }
        outcomes
    }
    let fail = |file: usize, kind: ErrorKind| -> Vec<FileOutcome> {
        (0..count)
            .map(|at| Err(if at == file { kind.clone() } else { ErrorKind::Cancelled }))
            .collect()
    };

    let mut copies = Vec::with_capacity(count);
    for (file, original) in task.originals.iter().enumerate() {
        if cancel.is_cancelled() {
            return finish(on_event, fail(count, ErrorKind::Cancelled));
        }
        match workspace.copy_in(app, file, original) {
            Ok(copy) => copies.push(copy),
            Err(kind) => return finish(on_event, fail(file, kind)),
        }
    }
    let merged = Task {
        tool: task.tool.clone(),
        originals: copies,
        results_folder: Some(workspace.results()),
    };
    // Progress goes through as it is; the outcomes wait until the Result is saved.
    let mut progress = |event: TaskEvent| {
        if let TaskEvent::Progress { file, progress } = event {
            on_event(TaskEvent::Progress { file, progress });
        }
    };
    let outcomes = run_task(&merged, &mut progress, cancel);
    let saved = match outcomes.iter().find_map(|outcome| outcome.as_ref().ok()) {
        Some(result) => crate::android::save(app, result.clone()).map(|saved| saved.path),
        None => Err(ErrorKind::Cancelled),
    };
    for file in 0..count {
        workspace.remove(file);
    }
    let outcomes = match saved {
        Ok(path) => outcomes
            .into_iter()
            .map(|outcome| {
                outcome.map(|result| rantakit_tasks::ResultFile {
                    path: path.clone(),
                    ..result
                })
            })
            .collect(),
        Err(_) if outcomes.iter().all(Result::is_err) => outcomes,
        Err(kind) => fail(0, kind),
    };
    finish(on_event, outcomes)
}

/// Stops a running Task. Finished Results stay; the file being worked on
/// leaves nothing behind. Does nothing once the Task has finished.
#[tauri::command]
pub fn cancel_task(running: State<'_, RunningTasks>, id: u64) {
    if let Some(cancel) = running.remove(id) {
        cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use rantakit_tasks::{ImageOptions, Target};
    use serde_json::json;

    use super::*;

    #[test]
    fn frontend_request_becomes_tool_options() {
        let request: TaskRequest = serde_json::from_value(json!({
            "options": { "tool": "kompres-foto", "limit": 200000 },
            "paths": ["C:/Users/ani/ktp.jpg"],
        }))
        .unwrap();

        assert!(matches!(
            request.options.into(),
            ToolOptions::KompresFoto(ImageOptions {
                format: OutputFormat::Same,
                target: Target::MaxBytes(200_000),
                max_dimension: None,
                ..
            })
        ));
    }

    #[test]
    fn background_colors_are_read_from_hex() {
        assert_eq!(parse_color(Some("#000000")), [0, 0, 0]);
        assert_eq!(parse_color(Some("#1a2B3c")), [0x1a, 0x2b, 0x3c]);
        assert_eq!(parse_color(Some("merah")), WHITE);
        assert_eq!(parse_color(None), WHITE);
    }

    #[test]
    fn errors_reach_the_frontend_as_codes() {
        let message = TaskMessage::FileDone {
            file: 1,
            outcome: outcome(
                Path::new("ijazah.pdf"),
                &Err(ErrorKind::SizeLimitUnreachable { best: 1234 }),
            ),
        };

        assert_eq!(
            serde_json::to_value(message).unwrap(),
            json!({
                "event": "fileDone",
                "file": 1,
                "outcome": {
                    "status": "failed",
                    "error": { "code": "size-limit-unreachable", "best": 1234 },
                },
            })
        );
    }
}
