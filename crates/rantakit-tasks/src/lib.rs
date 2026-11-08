//! The product rules that must behave the same on every platform. The shells
//! only turn requests into calls here; engines only turn one file into one
//! Result.

mod catalogue;
mod compression;
mod history;
mod info;
mod kinds;
mod names;
mod presets;
mod recent;
mod settings;
mod store;
mod task;

pub use catalogue::{
    Availability, CATALOGUE, FileCount, FileMatch, Job, Platform, Tool, ToolId, match_files,
    match_kinds, recent_first, tools_for, tools_for_files, tools_for_kinds,
};
pub use compression::{Compression, Level, dpi_to_side};
pub use history::{Entry, EntryFile, History, HistoryError};
pub use info::{FileInfo, file_info, file_info_of};
pub use kinds::{FileKind, detect_kind, read_kind};
pub use names::{
    named_path, numbered, pages_suffix, result_folder, result_folder_name, result_name, result_path,
};
pub use presets::{Preset, Presets};
pub use recent::{RECENT_LIMITS, RECENT_TOOLS, Recent, RecentLimits, RecentTools};
pub use settings::{Settings, SettingsStore};
pub use store::{JsonFile, StoreError};
pub use task::{
    Details, FileOutcome, FolderFile, MarkColor, MarkOpacity, MarkSize, PhotoSize, PhotosToPdf, ResultFile, Split, Task,
    TaskEvent, ToolOptions, WatermarkOptions, run_task, watermark_preview,
};

pub use rantakit_core::{CancelToken, ErrorKind, Progress};
pub use rantakit_image::{
    Corners, FoundInfo, ImageFormat, Placement, ImageOptions, OutputFormat, PhotoEdit, Target, WHITE, edited_preview, find_paper, thumbnail,
};
pub use rantakit_pdf::{PageEdit, PaperSize, PdfDocument, PdfOptions, PdfPages};
