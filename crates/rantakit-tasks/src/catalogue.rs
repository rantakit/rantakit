//! Facts about every Tool planned before 1.0 (RANTAKIT.md §4), and which
//! Tools fit a set of files. Names, descriptions, and icons belong to the
//! frontend, keyed by `ToolId::as_str`.

use std::path::Path;

use crate::kinds::{FileKind, detect_kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    KompresFoto,
    KompresPdf,
    UbahFormatFoto,
    UbahUkuranFoto,
    KompresDokumen,
    KompresAudio,
    UbahFormatAudio,
    KompresVideo,
    WordKePdf,
    ExcelKePdf,
    JpgKePdf,
    ScanDokumen,
    GabungPdf,
    PisahPdf,
    AturHalamanPdf,
    PdfKeJpg,
    BeriWatermark,
    HapusInfoLokasi,
}

impl ToolId {
    /// The stable id the frontend keys names, descriptions, and icons by.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KompresFoto => "kompres-foto",
            Self::KompresPdf => "kompres-pdf",
            Self::UbahFormatFoto => "ubah-format-foto",
            Self::UbahUkuranFoto => "ubah-ukuran-foto",
            Self::KompresDokumen => "kompres-dokumen",
            Self::KompresAudio => "kompres-audio",
            Self::UbahFormatAudio => "ubah-format-audio",
            Self::KompresVideo => "kompres-video",
            Self::WordKePdf => "word-ke-pdf",
            Self::ExcelKePdf => "excel-ke-pdf",
            Self::JpgKePdf => "jpg-ke-pdf",
            Self::ScanDokumen => "scan-dokumen",
            Self::GabungPdf => "gabung-pdf",
            Self::PisahPdf => "pisah-pdf",
            Self::AturHalamanPdf => "atur-halaman-pdf",
            Self::PdfKeJpg => "pdf-ke-jpg",
            Self::BeriWatermark => "beri-watermark",
            Self::HapusInfoLokasi => "hapus-info-lokasi",
        }
    }
}

/// The five jobs-to-be-done; Home groups Tools by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Job {
    /// "Kecilkan & Ubah"
    KecilkanUbah,
    /// "Scan"
    Scan,
    /// "Atur PDF"
    AturPdf,
    /// "Ambil Teks"
    AmbilTeks,
    /// "Bagikan dengan Aman"
    BagikanAman,
}

impl Job {
    /// The stable id the frontend keys Job headings by.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KecilkanUbah => "kecilkan-ubah",
            Self::Scan => "scan",
            Self::AturPdf => "atur-pdf",
            Self::AmbilTeks => "ambil-teks",
            Self::BagikanAman => "bagikan-aman",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    Linux,
    Android,
}

impl Platform {
    /// The platform this build runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Shipped,
    /// Shown greyed out with "Segera hadir"; cannot be used yet.
    ComingSoon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub id: ToolId,
    pub job: Job,
    /// File kinds the Tool takes; empty for Tools that start from the camera.
    pub accepts: &'static [FileKind],
    pub platforms: &'static [Platform],
    pub availability: Availability,
    /// How many files the Tool takes at once.
    pub files: FileCount,
}

/// How many files a Tool takes at once (X5 in docs/stories/v0.4.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCount {
    /// One or more, each made into its own Result.
    Any,
    /// Gabung PDF: there is nothing to merge in one file.
    AtLeastTwo,
    /// Atur Halaman PDF: one PDF per Task.
    One,
}

impl FileCount {
    pub fn allows(self, count: usize) -> bool {
        match self {
            Self::Any => count >= 1,
            Self::AtLeastTwo => count >= 2,
            Self::One => count == 1,
        }
    }
}

impl Tool {
    /// Whether the Tool takes all of `kinds` at once: each kind, as many as
    /// there are, and all from one family, so photos and PDFs never mix.
    pub fn takes(&self, kinds: &[FileKind]) -> bool {
        self.files.allows(kinds.len())
            && kinds.iter().all(|kind| self.accepts.contains(kind))
            && kinds.windows(2).all(|pair| family(pair[0]) == family(pair[1]))
    }

    const fn taking(self, files: FileCount) -> Self {
        Self { files, ..self }
    }
}

/// Files of one family can share a Task; a Tool page shows one kind of file.
fn family(kind: FileKind) -> u8 {
    match kind {
        Jpg | Png | Heic | Webp => 0,
        Pdf => 1,
        Word | Excel | PowerPoint => 2,
        Audio => 3,
        Video => 4,
    }
}

use Availability::*;
use FileKind::*;
use Job::*;

const ALL: &[Platform] = &[Platform::Windows, Platform::Linux, Platform::Android];
const PHOTOS: &[FileKind] = &[Jpg, Png];
const OFFICE: &[FileKind] = &[Word, Excel, PowerPoint];

/// Every Tool planned before 1.0, in the order Home shows them within a Job.
/// Tools planned for after 1.0 are not listed, so they never appear.
pub const CATALOGUE: &[Tool] = &[
    tool(ToolId::KompresFoto, KecilkanUbah, PHOTOS, ALL, Shipped),
    tool(ToolId::KompresPdf, KecilkanUbah, &[Pdf], ALL, Shipped),
    tool(ToolId::UbahFormatFoto, KecilkanUbah, PHOTOS, ALL, Shipped),
    tool(ToolId::UbahUkuranFoto, KecilkanUbah, PHOTOS, ALL, Shipped),
    tool(
        ToolId::KompresDokumen,
        KecilkanUbah,
        OFFICE,
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::KompresAudio,
        KecilkanUbah,
        &[Audio],
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::UbahFormatAudio,
        KecilkanUbah,
        &[Audio],
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::KompresVideo,
        KecilkanUbah,
        &[Video],
        ALL,
        ComingSoon,
    ),
    tool(ToolId::WordKePdf, KecilkanUbah, &[Word], ALL, ComingSoon),
    tool(ToolId::ExcelKePdf, KecilkanUbah, &[Excel], ALL, ComingSoon),
    tool(ToolId::JpgKePdf, Scan, PHOTOS, ALL, ComingSoon),
    tool(
        ToolId::ScanDokumen,
        Scan,
        &[],
        &[Platform::Android],
        ComingSoon,
    ),
    tool(ToolId::GabungPdf, AturPdf, &[Pdf], ALL, Shipped).taking(FileCount::AtLeastTwo),
    tool(ToolId::PisahPdf, AturPdf, &[Pdf], ALL, Shipped),
    tool(ToolId::AturHalamanPdf, AturPdf, &[Pdf], ALL, Shipped).taking(FileCount::One),
    tool(ToolId::PdfKeJpg, AturPdf, &[Pdf], ALL, ComingSoon),
    tool(
        ToolId::BeriWatermark,
        BagikanAman,
        &[Jpg, Png, Pdf],
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::HapusInfoLokasi,
        BagikanAman,
        PHOTOS,
        ALL,
        ComingSoon,
    ),
];

const fn tool(
    id: ToolId,
    job: Job,
    accepts: &'static [FileKind],
    platforms: &'static [Platform],
    availability: Availability,
) -> Tool {
    Tool {
        id,
        job,
        accepts,
        platforms,
        availability,
        files: FileCount::Any,
    }
}

/// The Tools to list on `platform`, shipped or coming soon.
pub fn tools_for(platform: Platform) -> impl Iterator<Item = &'static Tool> {
    CATALOGUE
        .iter()
        .filter(move |tool| tool.platforms.contains(&platform))
}

/// The shipped Tools on `platform` that take all of `kinds` at once (see
/// [`Tool::takes`]), in catalogue order. Empty when `kinds` is empty.
pub fn tools_for_kinds(kinds: &[FileKind], platform: Platform) -> Vec<ToolId> {
    tools_for(platform)
        .filter(|tool| tool.availability == Shipped)
        .filter(|tool| tool.takes(kinds))
        .map(|tool| tool.id)
        .collect()
}

/// `tools` with the recently used ones first, newest first, then the rest
/// in their own order. `recent` holds `ToolId::as_str` ids.
pub fn recent_first(mut tools: Vec<ToolId>, recent: &[impl AsRef<str>]) -> Vec<ToolId> {
    let rank = |tool: &ToolId| {
        recent
            .iter()
            .position(|id| id.as_ref() == tool.as_str())
            .unwrap_or(recent.len())
    };
    tools.sort_by_key(rank);
    tools
}

/// The shipped Tools on `platform` that accept every one of `files`, judged by
/// content. A file Rantakit cannot recognize fits no Tool.
pub fn tools_for_files(files: &[impl AsRef<Path>], platform: Platform) -> Vec<ToolId> {
    let kinds: Option<Vec<FileKind>> = files.iter().map(|f| detect_kind(f.as_ref())).collect();
    match kinds {
        Some(kinds) => tools_for_kinds(&kinds, platform),
        None => Vec::new(),
    }
}

/// What Rantakit can do with files dragged onto the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMatch {
    /// The shipped Tools that take every file, in catalogue order or, after
    /// [`FileMatch::recent_first`], recently used first; never
    /// empty. `kinds` has each file's kind, in order.
    Fits {
        kinds: Vec<FileKind>,
        tools: Vec<ToolId>,
    },
    /// Some file is not one any shipped Tool opens.
    Unsupported,
    /// Each file fits some Tool, but no Tool takes them all, like photos and
    /// PDFs together.
    Mixed,
}

impl FileMatch {
    /// The same match with the recently used Tools first ([`recent_first`]).
    pub fn recent_first(self, recent: &[impl AsRef<str>]) -> Self {
        match self {
            Self::Fits { kinds, tools } => Self::Fits {
                kinds,
                tools: recent_first(tools, recent),
            },
            other => other,
        }
    }
}

/// Which shipped Tools on `platform` can take all of `kinds`.
pub fn match_kinds(kinds: &[FileKind], platform: Platform) -> FileMatch {
    if kinds.is_empty() {
        return FileMatch::Unsupported;
    }
    let tools = tools_for_kinds(kinds, platform);
    if !tools.is_empty() {
        return FileMatch::Fits {
            kinds: kinds.to_vec(),
            tools,
        };
    }
    let each_fits = kinds
        .iter()
        .all(|kind| !tools_for_kinds(&[*kind], platform).is_empty());
    if each_fits {
        FileMatch::Mixed
    } else {
        FileMatch::Unsupported
    }
}

/// [`match_kinds`] for files on disk, judged by content. A file that can't be
/// read or recognized makes the whole set unsupported.
pub fn match_files(files: &[impl AsRef<Path>], platform: Platform) -> FileMatch {
    let kinds: Option<Vec<FileKind>> = files.iter().map(|f| detect_kind(f.as_ref())).collect();
    match kinds {
        Some(kinds) => match_kinds(&kinds, platform),
        None => FileMatch::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<_> = CATALOGUE.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(ids.len(), CATALOGUE.len());
    }

    #[test]
    fn photos_fit_the_photo_tools_and_pdfs_fit_the_shipped_pdf_tools() {
        let on = Platform::Windows;
        assert_eq!(
            tools_for_kinds(&[Jpg, Png], on),
            [
                ToolId::KompresFoto,
                ToolId::UbahFormatFoto,
                ToolId::UbahUkuranFoto
            ]
        );
        assert_eq!(
            tools_for_kinds(&[Pdf, Pdf], on),
            [ToolId::KompresPdf, ToolId::GabungPdf, ToolId::PisahPdf]
        );
    }

    #[test]
    fn photos_and_pdfs_together_fit_nothing() {
        assert!(tools_for_kinds(&[Jpg, Pdf], Platform::Linux).is_empty());
    }

    #[test]
    fn coming_soon_tools_never_match() {
        assert!(tools_for_kinds(&[Webp], Platform::Linux).is_empty());
        assert!(tools_for_kinds(&[Word], Platform::Linux).is_empty());
    }

    #[test]
    fn matching_explains_why_nothing_fits() {
        let on = Platform::Windows;
        assert_eq!(
            match_kinds(&[Jpg, Png], on),
            FileMatch::Fits {
                kinds: vec![Jpg, Png],
                tools: vec![
                    ToolId::KompresFoto,
                    ToolId::UbahFormatFoto,
                    ToolId::UbahUkuranFoto
                ]
            }
        );
        assert_eq!(match_kinds(&[Jpg, Pdf], on), FileMatch::Mixed);
        assert_eq!(match_kinds(&[Jpg, Webp], on), FileMatch::Unsupported);
        assert_eq!(match_kinds(&[Word], on), FileMatch::Unsupported);
        assert_eq!(match_kinds(&[], on), FileMatch::Unsupported);
    }

    #[test]
    fn unreadable_files_are_unsupported() {
        let missing = ["definitely-not-here.jpg"];
        assert_eq!(
            match_files(&missing, Platform::Linux),
            FileMatch::Unsupported
        );
    }

    #[test]
    fn scan_dokumen_is_android_only() {
        let listed = |platform| tools_for(platform).any(|t| t.id == ToolId::ScanDokumen);
        assert!(listed(Platform::Android));
        assert!(!listed(Platform::Windows));
        assert!(!listed(Platform::Linux));
    }

    /// Every Tool that would take `kinds` once it ships, in catalogue order.
    fn once_shipped(kinds: &[FileKind]) -> Vec<ToolId> {
        CATALOGUE
            .iter()
            .filter(|tool| tool.takes(kinds))
            .map(|tool| tool.id)
            .collect()
    }

    #[test]
    fn pdfs_lead_to_the_pdf_tools_by_how_many_there_are() {
        use ToolId::*;
        assert_eq!(
            once_shipped(&[Pdf]),
            [KompresPdf, PisahPdf, AturHalamanPdf, PdfKeJpg, BeriWatermark],
            "one PDF: no Gabung PDF"
        );
        assert_eq!(
            once_shipped(&[Pdf, Pdf]),
            [KompresPdf, GabungPdf, PisahPdf, PdfKeJpg, BeriWatermark],
            "two PDFs: Gabung PDF, and no Atur Halaman PDF"
        );
    }

    #[test]
    fn photos_lead_to_the_photo_tools() {
        use ToolId::*;
        assert_eq!(
            once_shipped(&[Jpg, Png]),
            [
                KompresFoto,
                UbahFormatFoto,
                UbahUkuranFoto,
                JpgKePdf,
                BeriWatermark,
                HapusInfoLokasi
            ]
        );
    }

    #[test]
    fn office_documents_lead_to_kompres_dokumen() {
        assert!(once_shipped(&[Word, Excel, PowerPoint]).contains(&ToolId::KompresDokumen));
    }

    #[test]
    fn photos_and_pdfs_never_share_a_tool() {
        assert!(
            once_shipped(&[Jpg, Pdf]).is_empty(),
            "Beri Watermark takes both, but not together"
        );
    }

    #[test]
    fn recently_used_tools_come_first() {
        use ToolId::*;
        let tools = vec![KompresFoto, UbahFormatFoto, UbahUkuranFoto, JpgKePdf];
        assert_eq!(
            recent_first(tools.clone(), &["jpg-ke-pdf", "kompres-pdf", "ubah-format-foto"]),
            [JpgKePdf, UbahFormatFoto, KompresFoto, UbahUkuranFoto]
        );
        assert_eq!(recent_first(tools.clone(), &[] as &[&str]), tools);
    }
}
