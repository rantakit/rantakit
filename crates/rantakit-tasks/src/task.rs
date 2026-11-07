use std::fs;
use std::path::{Path, PathBuf};

use rantakit_core::{CancelToken, ErrorKind, Progress};
use rantakit_image::{ImageFormat, ImageOptions, OutputFormat, Target, WHITE};
use rantakit_pdf::PdfOptions;

use crate::catalogue::ToolId;
use crate::names::result_path;

/// One run of a Tool on one or more Originals.
#[derive(Debug, Clone)]
pub struct Task {
    pub tool: ToolOptions,
    pub originals: Vec<PathBuf>,
    /// The Results folder chosen in Settings; `None` puts each Result next to
    /// its Original.
    pub results_folder: Option<PathBuf>,
}

/// A Tool and its options for one Task.
#[derive(Debug, Clone)]
pub enum ToolOptions {
    KompresFoto(ImageOptions),
    KompresPdf(PdfOptions),
    /// Always `OutputFormat::To`; build it with `ToolOptions::convert`.
    UbahFormatFoto(ImageOptions),
    /// Always has `max_dimension`; build it with `ToolOptions::resize`.
    UbahUkuranFoto(ImageOptions),
}

/// Ubah Ukuran Foto's plain choices for the longest side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PhotoSize {
    /// "Besar": still clear on a large screen.
    Large,
    /// "Sedang": right for sending on WhatsApp.
    #[default]
    Medium,
    /// "Kecil": right for online forms.
    Small,
}

impl PhotoSize {
    /// The longest side in pixels.
    pub fn side(self) -> u32 {
        match self {
            Self::Large => 1920,
            Self::Medium => 1280,
            Self::Small => 800,
        }
    }
}

impl ToolOptions {
    /// Ubah Format Foto: every photo to `to`. `quality` applies to JPG, and
    /// `background` shows through a transparent PNG that becomes a JPG.
    pub fn convert(to: ImageFormat, quality: u8, background: [u8; 3]) -> Self {
        Self::UbahFormatFoto(ImageOptions {
            format: OutputFormat::To(to),
            target: Target::Quality(quality),
            max_dimension: None,
            background,
        })
    }

    /// Ubah Ukuran Foto: every photo at most `side` pixels on its longest
    /// side, in its own format. Photos are never enlarged.
    pub fn resize(side: u32) -> Self {
        Self::UbahUkuranFoto(ImageOptions {
            format: OutputFormat::Same,
            target: Target::Quality(90),
            max_dimension: Some(side),
            background: WHITE,
        })
    }

    pub fn id(&self) -> ToolId {
        match self {
            Self::KompresFoto(_) => ToolId::KompresFoto,
            Self::KompresPdf(_) => ToolId::KompresPdf,
            Self::UbahFormatFoto(_) => ToolId::UbahFormatFoto,
            Self::UbahUkuranFoto(_) => ToolId::UbahUkuranFoto,
        }
    }

    /// The English word in the Result name: `ijazah (compressed).pdf`.
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::KompresFoto(_) | Self::KompresPdf(_) => "compressed",
            Self::UbahFormatFoto(_) => "converted",
            Self::UbahUkuranFoto(_) => "resized",
        }
    }
}

/// How one file of a Task ended: its Result, or why it has none.
pub type FileOutcome = Result<ResultFile, ErrorKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct ResultFile {
    pub path: PathBuf,
    /// Size of the Original in bytes.
    pub original_size: u64,
    /// Size of the Result in bytes.
    pub size: u64,
    pub details: Details,
}

/// What the engine reports about a Result, by kind of file.
#[derive(Debug, Clone, PartialEq)]
pub enum Details {
    Photo {
        format: ImageFormat,
        width: u32,
        height: u32,
        quality: Option<u8>,
    },
    Pdf {
        images: usize,
        images_recompressed: usize,
    },
    /// One Original made several Results (X3), such as Pisah PDF's pages:
    /// `ResultFile::path` is the new folder they are in, such as
    /// `ijazah (pages)/`, and `ResultFile::size` is their total.
    Folder { files: Vec<FolderFile> },
}

/// One of several Results in a [`Details::Folder`].
#[derive(Debug, Clone, PartialEq)]
pub struct FolderFile {
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
}

impl ResultFile {
    /// Each file made, in order: the Result itself, or every file in its folder.
    pub fn files(&self) -> Vec<FolderFile> {
        match &self.details {
            Details::Folder { files } => files.clone(),
            _ => vec![FolderFile {
                path: self.path.clone(),
                size: self.size,
            }],
        }
    }
}

/// What happens while a Task runs. `file` is an index into `Task::originals`.
#[derive(Debug)]
pub enum TaskEvent<'a> {
    Progress {
        file: usize,
        progress: Progress,
    },
    FileDone {
        file: usize,
        outcome: &'a FileOutcome,
    },
}

/// Runs `task` one file at a time. A failed file does not stop the others.
/// Once `cancel` is set, the file being worked on leaves nothing behind and
/// every file after it ends with `Cancelled`; finished Results stay.
pub fn run_task(
    task: &Task,
    on_event: &mut dyn FnMut(TaskEvent),
    cancel: &CancelToken,
) -> Vec<FileOutcome> {
    let mut outcomes = Vec::with_capacity(task.originals.len());

    for (file, original) in task.originals.iter().enumerate() {
        let outcome = if cancel.is_cancelled() {
            Err(ErrorKind::Cancelled)
        } else {
            let mut progress = |progress| on_event(TaskEvent::Progress { file, progress });
            run_file(task, original, &mut progress, cancel)
        };
        on_event(TaskEvent::FileDone {
            file,
            outcome: &outcome,
        });
        outcomes.push(outcome);
    }

    outcomes
}

fn run_file(
    task: &Task,
    original: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> FileOutcome {
    let original_size = fs::metadata(original)
        .map_err(|e| ErrorKind::reading(&e))?
        .len();
    let folder = match &task.results_folder {
        Some(folder) => folder.as_path(),
        None => original.parent().unwrap_or(Path::new("")),
    };
    let suffix = task.tool.suffix();

    match &task.tool {
        ToolOptions::KompresFoto(opts) => photo_result(
            original,
            original_size,
            folder,
            suffix,
            opts,
            progress,
            cancel,
        ),
        ToolOptions::UbahFormatFoto(opts) => {
            if OutputFormat::To(rantakit_image::detect_format(original)?) == opts.format {
                return Err(ErrorKind::Unchanged);
            }
            photo_result(
                original,
                original_size,
                folder,
                suffix,
                opts,
                progress,
                cancel,
            )
        }
        ToolOptions::UbahUkuranFoto(opts) => {
            let info = rantakit_image::photo_info(original)?;
            if info.width.max(info.height) <= opts.max_dimension.unwrap_or(u32::MAX) {
                return Err(ErrorKind::Unchanged);
            }
            photo_result(
                original,
                original_size,
                folder,
                suffix,
                opts,
                progress,
                cancel,
            )
        }
        ToolOptions::KompresPdf(opts) => {
            let path = result_path(original, folder, suffix, "pdf");
            let out = rantakit_pdf::compress_pdf(original, &path, opts, progress, cancel)?;
            Ok(ResultFile {
                path,
                original_size,
                size: out.size,
                details: Details::Pdf {
                    images: out.images,
                    images_recompressed: out.images_recompressed,
                },
            })
        }
    }
}

/// Runs the photo engine on one Original and describes its Result.
fn photo_result(
    original: &Path,
    original_size: u64,
    folder: &Path,
    suffix: &str,
    opts: &ImageOptions,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> FileOutcome {
    let format = match opts.format {
        OutputFormat::To(format) => format,
        OutputFormat::Same => rantakit_image::detect_format(original)?,
    };
    let path = result_path(original, folder, suffix, photo_extension(format));
    let out = rantakit_image::compress_image(original, &path, opts, progress, cancel)?;
    Ok(ResultFile {
        path,
        original_size,
        size: out.size,
        details: Details::Photo {
            format: out.format,
            width: out.width,
            height: out.height,
            quality: out.quality,
        },
    })
}

/// JPG and JPEG are the same format; Results always use `.jpg`.
fn photo_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
    }
}
