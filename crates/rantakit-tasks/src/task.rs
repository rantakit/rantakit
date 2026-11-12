use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rantakit_core::{CancelToken, ErrorKind, Progress, Tracker, read_original, write_result};
use rantakit_image::{ImageFormat, ImageOptions, OutputFormat, PhotoEdit, PhotoKind, Placement, Target, WHITE, Watermark};
use rantakit_office::OfficeOptions;
use rantakit_pdf::{
    PageEdit, PagePhoto, PaperSize, PdfDocument, PdfOptions, PdfPages, StampFont, StampLine, StampStyle, page_jpeg,
};

use crate::catalogue::ToolId;
use crate::kinds::{FileKind, detect_kind};
use crate::names::{named_path, pages_suffix, result_folder, result_path};
use crate::prepared::PreparedPage;

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
    CompressPhoto(ImageOptions),
    CompressPdf(PdfOptions),
    /// Word, Excel, and PowerPoint files with their pictures made smaller.
    CompressDocument(OfficeOptions),
    /// Always `OutputFormat::To`; build it with `ToolOptions::convert`.
    UbahFormatFoto(ImageOptions),
    /// Always has `max_dimension`; build it with `ToolOptions::resize`.
    ResizePhoto(ImageOptions),
    /// Every Original, in order, into one PDF named after `date`, the user's
    /// today such as "2026-11-07": `merged 2026-11-07.pdf`.
    MergePdf { date: String },
    SplitPdf(Split),
    /// The one Original's pages as they will be, in order.
    OrganizePdf(Vec<PageEdit>),
    /// Every Original, a photo, in order, as one page each of one PDF.
    JpgKePdf(PhotosToPdf),
    /// The photos the camera took, in order, as one page each of one PDF.
    ScanDocument(PhotosToPdf),
    /// Every photo without its info, picture untouched; a photo with
    /// nothing to remove gets no Result.
    RemoveLocationInfo,
    /// Text over every photo, and every page of every PDF.
    AddWatermark(WatermarkOptions),
    /// Pages of every PDF as JPG photos.
    PdfKeJpg(PdfToJpg),
}

/// Beri Watermark's choices, in the user's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkOptions {
    /// "Hanya untuk verifikasi Bank X".
    pub text: String,
    /// "Tambahkan tanggal hari ini": the user's today as they write it,
    /// such as "08/11/2026", on a line under the text.
    pub date: Option<String>,
    /// "Posisi".
    pub placement: Placement,
    /// "Ukuran".
    pub size: MarkSize,
    /// "Transparansi".
    pub opacity: MarkOpacity,
    /// "Warna".
    pub color: MarkColor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarkSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarkOpacity {
    /// "Samar".
    Faint,
    /// "Sedang".
    #[default]
    Medium,
    /// "Jelas".
    Strong,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarkColor {
    #[default]
    Gray,
    Black,
    White,
    Red,
}

impl WatermarkOptions {
    /// How the watermark looks, for the engines.
    pub fn watermark(&self) -> Watermark {
        let mut lines = vec![rantakit_image::shown(&self.text)];
        if let Some(date) = &self.date {
            lines.push(rantakit_image::shown(date));
        }
        Watermark {
            lines,
            placement: self.placement,
            // Shares of the page's shorter side.
            size: match self.size {
                MarkSize::Small => 0.035,
                MarkSize::Medium => 0.05,
                MarkSize::Large => 0.07,
            },
            opacity: match self.opacity {
                MarkOpacity::Faint => 0.15,
                MarkOpacity::Medium => 0.3,
                MarkOpacity::Strong => 0.5,
            },
            color: match self.color {
                MarkColor::Gray => [128, 128, 128],
                MarkColor::Black => [0, 0, 0],
                MarkColor::White => [255, 255, 255],
                MarkColor::Red => [211, 47, 47],
            },
        }
    }
}

/// The live preview of Beri Watermark: `picture`, a small JPG of a photo
/// or of a PDF's first page, with the watermark on it.
pub fn watermark_preview(picture: &[u8], opts: &WatermarkOptions) -> Result<Vec<u8>, ErrorKind> {
    rantakit_image::watermark_preview(picture, &opts.watermark())
}

/// PDF ke JPG's choices (O2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfToJpg {
    /// "Pilih halaman": the pages to take, counting from 1. `None` is
    /// "Semua halaman".
    pub pages: Option<Vec<usize>>,
    /// "Ketajaman", in dots per inch of the paper: 150 for "Standar", 300
    /// for "Tajam", or the number from Advanced options.
    pub dpi: u32,
}

/// JPG ke PDF's and Scan Dokumen's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotosToPdf {
    /// The user's today, such as "2026-11-07": `scan 2026-11-07.pdf`.
    pub date: String,
    /// "Ukuran kertas".
    pub paper: PaperSize,
    /// The thin margin around each photo; "Tanpa margin" turns it off.
    pub margin: bool,
    /// How each Original, by index, is cropped and turned; one left out is
    /// used whole.
    pub edits: Vec<PhotoEdit>,
    /// "Bersihkan": every page made to look scanned. Scan Dokumen's
    /// "Tampilan"; JPG ke PDF leaves it off.
    pub clean: bool,
    /// Pages already made for each Original, by index, by
    /// [`PreparedPages::attach`](crate::PreparedPages::attach); one left out
    /// is made while the PDF is saved.
    pub prepared: Vec<Option<PreparedPage>>,
}

impl PhotosToPdf {
    /// How the Original at `file` becomes its page.
    pub fn edit(&self, file: usize) -> PhotoEdit {
        PhotoEdit {
            clean: self.clean,
            ..self.edits.get(file).copied().unwrap_or_default()
        }
    }
}

/// A page photo is at most this long, about 200 dpi on A4, so a few phone
/// photos do not make a PDF too big to send.
pub(crate) const PAGE_PHOTO_SIDE: u32 = 2400;

/// How Pisah PDF splits each PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Split {
    /// "Ambil halaman": these pages, counted from 1, in this order, as one
    /// PDF. A page a PDF does not have is skipped.
    Take(Vec<usize>),
    /// "Pisah semua halaman" (1), or "Setiap N halaman": a PDF for every `n`
    /// pages, in a new folder.
    Every(usize),
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
    /// Ubah ke JPG or Ubah ke PNG: every photo to `to`. `quality` applies to
    /// JPG, and `background` shows through a transparent PNG that becomes a
    /// JPG.
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
        Self::ResizePhoto(ImageOptions {
            format: OutputFormat::Same,
            target: Target::Quality(90),
            max_dimension: Some(side),
            background: WHITE,
        })
    }

    /// The listed Tool this Task belongs to. Ubah Format Foto, JPG ke PDF, and
    /// PDF ke JPG are listed as the goal-based Tools since 0.4.1.
    pub fn id(&self) -> ToolId {
        match self {
            Self::CompressPhoto(_) => ToolId::CompressPhoto,
            Self::CompressPdf(_) => ToolId::CompressPdf,
            Self::CompressDocument(_) => ToolId::CompressDocument,
            Self::UbahFormatFoto(ImageOptions {
                format: OutputFormat::To(ImageFormat::Png),
                ..
            }) => ToolId::ConvertToPng,
            Self::UbahFormatFoto(_) => ToolId::ConvertToJpg,
            Self::ResizePhoto(_) => ToolId::ResizePhoto,
            Self::MergePdf { .. } => ToolId::MergePdf,
            Self::SplitPdf(_) => ToolId::SplitPdf,
            Self::OrganizePdf(_) => ToolId::OrganizePdf,
            Self::JpgKePdf(_) => ToolId::ConvertToPdf,
            Self::ScanDocument(_) => ToolId::ScanDocument,
            Self::RemoveLocationInfo => ToolId::RemoveLocationInfo,
            Self::AddWatermark(_) => ToolId::AddWatermark,
            Self::PdfKeJpg(_) => ToolId::ConvertToJpg,
        }
    }

    /// Whether every Original goes into one Result, as in Gabung PDF.
    pub fn merges(&self) -> bool {
        matches!(self, Self::MergePdf { .. } | Self::JpgKePdf(_) | Self::ScanDocument(_))
    }

    /// The choices of a Tool that makes photos into a PDF.
    fn photos(&self) -> Option<&PhotosToPdf> {
        match self {
            Self::JpgKePdf(opts) | Self::ScanDocument(opts) => Some(opts),
            _ => None,
        }
    }

    pub(crate) fn photos_mut(&mut self) -> Option<&mut PhotosToPdf> {
        match self {
            Self::JpgKePdf(opts) | Self::ScanDocument(opts) => Some(opts),
            _ => None,
        }
    }

    /// The English word in the Result name: `ijazah (compressed).pdf`.
    /// Pisah PDF names each Result after its pages instead.
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::CompressPhoto(_) | Self::CompressPdf(_) | Self::CompressDocument(_) => "compressed",
            Self::UbahFormatFoto(_) => "converted",
            Self::ResizePhoto(_) => "resized",
            Self::MergePdf { .. } => "merged",
            Self::SplitPdf(_) => "pages",
            Self::OrganizePdf(_) => "edited",
            Self::JpgKePdf(_) | Self::ScanDocument(_) => "scan",
            Self::RemoveLocationInfo => "cleaned",
            Self::AddWatermark(_) => "watermarked",
            Self::PdfKeJpg(_) => "pages",
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
    /// A Word, Excel, or PowerPoint file: its JPG and PNG pictures, and how
    /// many of them were made smaller.
    Document {
        pictures: usize,
        pictures_recompressed: usize,
    },
    /// A PDF made of pages of other PDFs: merged, taken out, or arranged.
    /// `skipped` are the pages asked for that the Original does not have,
    /// counted from 1.
    Pages { pages: usize, skipped: Vec<usize> },
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
    if task.tool.merges() {
        let outcomes = merge(task, on_event, cancel);
        for (file, outcome) in outcomes.iter().enumerate() {
            on_event(TaskEvent::FileDone { file, outcome });
        }
        return outcomes;
    }

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
        ToolOptions::CompressPhoto(opts) => photo_result(
            original,
            original_size,
            folder,
            suffix,
            opts,
            progress,
            cancel,
        ),
        ToolOptions::UbahFormatFoto(opts) => {
            if is_already(rantakit_image::detect_kind(original)?, opts.format) {
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
        ToolOptions::ResizePhoto(opts) => {
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
        ToolOptions::MergePdf { .. } | ToolOptions::JpgKePdf(_) | ToolOptions::ScanDocument(_) => {
            unreachable!("run_task merges every Original at once")
        }
        ToolOptions::SplitPdf(split) => {
            let pdf = PdfDocument::open(original)?;
            let count = pdf.page_count();
            let mut tracker = Tracker::new(progress, cancel);
            match split {
                Split::Take(pages) => {
                    let (kept, skipped): (Vec<usize>, Vec<usize>) =
                        pages.iter().partition(|&&page| (1..=count).contains(&page));
                    if kept.is_empty() {
                        return Err(ErrorKind::NoSuchPages);
                    }
                    if kept.iter().copied().eq(1..=count) {
                        return Err(ErrorKind::Unchanged);
                    }
                    let edits: Vec<PageEdit> = kept.iter().map(|&page| PageEdit::keep(page - 1)).collect();
                    let path = result_path(original, folder, &pages_suffix(&kept), "pdf");
                    let size = pdf.arrange(&edits)?.save(&path, &mut tracker)?;
                    Ok(ResultFile {
                        path,
                        original_size,
                        size,
                        details: Details::Pages {
                            pages: kept.len(),
                            skipped,
                        },
                    })
                }
                Split::Every(every) => split_every(original, original_size, folder, &pdf, *every, &mut tracker),
            }
        }
        ToolOptions::OrganizePdf(edits) => {
            let pdf = PdfDocument::open(original)?;
            let unchanged = edits.len() == pdf.page_count()
                && edits
                    .iter()
                    .enumerate()
                    .all(|(at, edit)| edit.page == at && edit.quarter_turns % 4 == 0);
            if unchanged {
                return Err(ErrorKind::Unchanged);
            }
            let path = result_path(original, folder, suffix, "pdf");
            let mut tracker = Tracker::new(progress, cancel);
            let size = pdf.arrange(edits)?.save(&path, &mut tracker)?;
            Ok(ResultFile {
                path,
                original_size,
                size,
                details: Details::Pages {
                    pages: edits.len(),
                    skipped: Vec::new(),
                },
            })
        }
        ToolOptions::RemoveLocationInfo => {
            let format = rantakit_image::detect_kind(original)?.saved_as();
            let info = rantakit_image::photo_info(original)?;
            let path = result_path(original, folder, suffix, photo_extension(format));
            let out = rantakit_image::remove_info(original, &path, progress, cancel)?;
            Ok(ResultFile {
                path,
                original_size,
                size: out.size,
                details: Details::Photo {
                    format: out.format,
                    width: info.width,
                    height: info.height,
                    quality: None,
                },
            })
        }
        ToolOptions::AddWatermark(opts) => {
            let watermark = opts.watermark();
            if watermark.lines.iter().all(String::is_empty) {
                return Err(ErrorKind::Unchanged);
            }
            if detect_kind(original) == Some(FileKind::Pdf) {
                return watermark_pdf(original, original_size, folder, suffix, &watermark, progress, cancel);
            }
            let format = rantakit_image::detect_kind(original)?.saved_as();
            let path = result_path(original, folder, suffix, photo_extension(format));
            let out = rantakit_image::watermark_image(original, &path, &watermark, progress, cancel)?;
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
        ToolOptions::PdfKeJpg(opts) => pdf_to_jpg(original, original_size, folder, opts, progress, cancel),
        ToolOptions::CompressDocument(opts) => {
            // The Result keeps the Original's extension, so a `.docm` keeps its macros working.
            let extension = original
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| document_extension(original).into());
            let path = result_path(original, folder, suffix, &extension);
            let out = rantakit_office::compress_document(original, &path, opts, progress, cancel)?;
            Ok(ResultFile {
                path,
                original_size,
                size: out.size,
                details: Details::Document {
                    pictures: out.pictures,
                    pictures_recompressed: out.pictures_recompressed,
                },
            })
        }
        ToolOptions::CompressPdf(opts) => {
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

/// The usual extension of an Office document that has none.
fn document_extension(original: &Path) -> &'static str {
    match detect_kind(original) {
        Some(FileKind::Excel) => "xlsx",
        Some(FileKind::PowerPoint) => "pptx",
        _ => "docx",
    }
}

/// The pieces a merging Tool reads from each Original before it writes.
enum Parts {
    Pdfs(Vec<PdfDocument>),
    Photos(Vec<PagePhoto>),
}

/// Gabung PDF, JPG ke PDF: every Original, in order, into one PDF. A file
/// that cannot be read ends the Task before anything is written; it gets
/// its own error and every other file `Cancelled`. Each file's outcome is
/// the one Result, with that file's own size as its Original's, so History
/// and the totals add up.
fn merge(task: &Task, on_event: &mut dyn FnMut(TaskEvent), cancel: &CancelToken) -> Vec<FileOutcome> {
    let count = task.originals.len();
    let fail = |file: usize, kind: ErrorKind| -> Vec<FileOutcome> {
        (0..count)
            .map(|at| Err(if at == file { kind.clone() } else { ErrorKind::Cancelled }))
            .collect()
    };
    let (date, mut parts) = match (&task.tool, task.tool.photos()) {
        (ToolOptions::MergePdf { date }, _) => (date, Parts::Pdfs(Vec::with_capacity(count))),
        (_, Some(opts)) => (&opts.date, Parts::Photos(Vec::with_capacity(count))),
        _ => return fail(0, ErrorKind::UnsupportedFormat),
    };

    let mut sizes = Vec::with_capacity(count);
    for (file, original) in task.originals.iter().enumerate() {
        if cancel.is_cancelled() {
            return fail(count, ErrorKind::Cancelled);
        }
        on_event(TaskEvent::Progress {
            file,
            progress: Progress::Fraction(0.0),
        });
        let read = fs::metadata(original)
            .map_err(|e| ErrorKind::reading(&e))
            .and_then(|meta| {
                match (&mut parts, task.tool.photos()) {
                    (Parts::Pdfs(pdfs), _) => pdfs.push(PdfDocument::open(original)?),
                    (Parts::Photos(photos), Some(opts)) => {
                        // A page made ahead is used when it can still be read.
                        let ready = opts.prepared.get(file).cloned().flatten().and_then(|page| page.read().ok());
                        let page = match ready {
                            Some(page) => page,
                            None => {
                                let made = rantakit_image::page_photo(&read_original(original)?, &opts.edit(file), PAGE_PHOTO_SIDE)?;
                                PagePhoto {
                                    jpeg: made.jpeg,
                                    width: made.width,
                                    height: made.height,
                                    gray: made.gray,
                                }
                            }
                        };
                        photos.push(page);
                    }
                    (Parts::Photos(_), _) => return Err(ErrorKind::UnsupportedFormat),
                }
                Ok(meta.len())
            });
        match read {
            Ok(size) => sizes.push(size),
            Err(kind) => return fail(file, kind),
        }
        on_event(TaskEvent::Progress {
            file,
            progress: Progress::Fraction(1.0),
        });
    }

    let first = &task.originals[0];
    let folder = match &task.results_folder {
        Some(folder) => folder.as_path(),
        None => first.parent().unwrap_or(Path::new("")),
    };
    let path = named_path(folder, &dated_stem(task.tool.suffix(), date), "pdf");
    let mut quiet = |_| {};
    let mut tracker = Tracker::new(&mut quiet, cancel);
    let made = match (parts, task.tool.photos()) {
        (Parts::Pdfs(pdfs), _) => PdfDocument::merge(pdfs),
        (Parts::Photos(photos), Some(opts)) => PdfDocument::from_photos(photos, opts.paper, opts.margin),
        (Parts::Photos(_), _) => Err(ErrorKind::UnsupportedFormat),
    };
    let saved = made.and_then(|merged| {
        let pages = merged.page_count();
        Ok((pages, merged.save(&path, &mut tracker)?))
    });
    match saved {
        Ok((pages, size)) => sizes
            .into_iter()
            .map(|original_size| {
                Ok(ResultFile {
                    path: path.clone(),
                    original_size,
                    size,
                    details: Details::Pages {
                        pages,
                        skipped: Vec::new(),
                    },
                })
            })
            .collect(),
        // Writing failed for the Task as a whole; the first file carries why.
        Err(ErrorKind::Cancelled) => fail(count, ErrorKind::Cancelled),
        Err(kind) => fail(0, kind),
    }
}

/// `merged 2026-11-07` or `scan 2026-11-07`; just the word when the date
/// is not one.
fn dated_stem(word: &str, date: &str) -> String {
    let is_date = date.len() == 10
        && date
            .chars()
            .enumerate()
            .all(|(at, c)| if at == 4 || at == 7 { c == '-' } else { c.is_ascii_digit() });
    match is_date {
        true => format!("{word} {date}"),
        false => word.to_string(),
    }
}

/// Pisah PDF's "Pisah semua halaman" and "Setiap N halaman": a PDF for every
/// `every` pages, in a new folder named after the Original. A stop or a
/// failure halfway removes the folder, so it leaves nothing behind.
fn split_every(
    original: &Path,
    original_size: u64,
    folder: &Path,
    pdf: &PdfDocument,
    every: usize,
    tracker: &mut Tracker<'_>,
) -> FileOutcome {
    let count = pdf.page_count();
    let every = every.max(1);
    if every >= count {
        return Err(ErrorKind::Unchanged);
    }
    let parts: Vec<Vec<usize>> = (1..=count)
        .collect::<Vec<_>>()
        .chunks(every)
        .map(<[usize]>::to_vec)
        .collect();
    pages_folder(original, original_size, folder, &parts, tracker, |pages, into, quiet| {
        let edits: Vec<PageEdit> = pages.iter().map(|&page| PageEdit::keep(page - 1)).collect();
        let path = result_path(original, into, &pages_suffix(pages), "pdf");
        let size = pdf.arrange(&edits)?.save(&path, quiet)?;
        Ok(FolderFile { path, size })
    })
}

/// Writes one file for each of `items` into a new `<Original> (pages)`
/// folder next to the Original, the folder's progress moving one item at a
/// time. `write` gets the item, the folder, and a tracker that reports
/// nothing. A stop or a failure halfway removes the folder, so it leaves
/// nothing behind.
fn pages_folder<T>(
    original: &Path,
    original_size: u64,
    folder: &Path,
    items: &[T],
    tracker: &mut Tracker<'_>,
    mut write: impl FnMut(&T, &Path, &mut Tracker<'_>) -> Result<FolderFile, ErrorKind>,
) -> FileOutcome {
    let into = result_folder(original, folder, "pages").map_err(|e| ErrorKind::writing(&e))?;
    let mut files = Vec::with_capacity(items.len());
    for (at, item) in items.iter().enumerate() {
        let written = tracker.check().and_then(|()| {
            let mut quiet = |_| {};
            let cancel = CancelToken::new();
            write(item, &into, &mut Tracker::new(&mut quiet, &cancel))
        });
        match written.and_then(|file| tracker.step((at + 1) as f32 / items.len() as f32).map(|()| file)) {
            Ok(file) => files.push(file),
            Err(kind) => {
                let _ = fs::remove_dir_all(&into);
                return Err(kind);
            }
        }
    }
    Ok(ResultFile {
        path: into,
        original_size,
        size: files.iter().map(|file| file.size).sum(),
        details: Details::Folder { files },
    })
}

/// JPG quality for PDF ke JPG's photos: sharp text, still small enough to send.
const PAGE_PHOTO_QUALITY: u8 = 90;

/// PDF ke JPG: each page taken as a JPG. One page is one file next to the
/// Original, `ijazah (page 1).jpg`; several go into a new folder, `ijazah
/// (pages)/`, which a stop or a failure halfway removes.
fn pdf_to_jpg(
    original: &Path,
    original_size: u64,
    folder: &Path,
    opts: &PdfToJpg,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> FileOutcome {
    let mut tracker = Tracker::new(progress, cancel);
    let pdf = PdfPages::open(original)?;
    let count = pdf.count();
    let kept: Vec<usize> = match &opts.pages {
        None => (1..=count).collect(),
        Some(pages) => pages.iter().copied().filter(|page| (1..=count).contains(page)).collect(),
    };
    if kept.is_empty() {
        return Err(ErrorKind::NoSuchPages);
    }
    let photo = |page: usize| -> Result<(Vec<u8>, u32, u32), ErrorKind> {
        let img = pdf.render_at(page - 1, opts.dpi)?;
        Ok((page_jpeg(&img, PAGE_PHOTO_QUALITY)?, img.width(), img.height()))
    };

    if let [page] = kept[..] {
        let (jpg, width, height) = photo(page)?;
        tracker.step(0.9)?;
        let path = result_path(original, folder, &pages_suffix(&[page]), "jpg");
        let size = write_result(&path, &mut tracker, |file| file.write_all(&jpg))?;
        return Ok(ResultFile {
            path,
            original_size,
            size,
            details: Details::Photo {
                format: ImageFormat::Jpeg,
                width,
                height,
                quality: Some(PAGE_PHOTO_QUALITY),
            },
        });
    }

    pages_folder(original, original_size, folder, &kept, &mut tracker, |&page, into, quiet| {
        let (jpg, _, _) = photo(page)?;
        let path = result_path(original, into, &pages_suffix(&[page]), "jpg");
        let size = write_result(&path, quiet, |file| file.write_all(&jpg))?;
        Ok(FolderFile { path, size })
    })
}

/// Beri Watermark on a PDF: the watermark over every page, laid out for
/// each page as a reader shows it, in the bundled font.
fn watermark_pdf(
    original: &Path,
    original_size: u64,
    folder: &Path,
    suffix: &str,
    watermark: &Watermark,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> FileOutcome {
    let mut tracker = Tracker::new(progress, cancel);
    let mut pdf = PdfDocument::open(original)?;
    tracker.step(0.3)?;
    let metrics = rantakit_image::font_metrics();
    let font = StampFont {
        name: rantakit_image::watermark::FONT_NAME,
        file: rantakit_image::watermark::FONT,
        widths: &metrics.widths,
        ascent: metrics.ascent,
        descent: metrics.descent,
        cap_height: metrics.cap_height,
        bbox: metrics.bbox,
    };
    let style = StampStyle {
        color: watermark.color,
        opacity: watermark.opacity,
    };
    pdf.stamp(&font, style, &mut |width, height| {
        watermark
            .layout(width, height)
            .into_iter()
            .map(|line| StampLine {
                text: line.text,
                size: line.size,
                x: line.x,
                y: line.y,
                angle: line.angle,
            })
            .collect()
    })?;
    tracker.step(0.6)?;
    let pages = pdf.page_count();
    let path = result_path(original, folder, suffix, "pdf");
    let size = pdf.save(&path, &mut tracker)?;
    Ok(ResultFile {
        path,
        original_size,
        size,
        details: Details::Pages {
            pages,
            skipped: Vec::new(),
        },
    })
}

/// Whether the Original is already in the `format` Ubah Format Foto turns it
/// into: a JPG to JPG. A HEIC or WebP never is, since it is read only.
fn is_already(kind: PhotoKind, format: OutputFormat) -> bool {
    matches!(
        (kind, format),
        (PhotoKind::Jpeg, OutputFormat::To(ImageFormat::Jpeg)) | (PhotoKind::Png, OutputFormat::To(ImageFormat::Png))
    )
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
        OutputFormat::Same => rantakit_image::detect_kind(original)?.saved_as(),
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
