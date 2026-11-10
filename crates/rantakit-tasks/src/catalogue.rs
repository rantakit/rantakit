//! Facts about every Tool planned before 1.0 (RANTAKIT.md §4), and which
//! Tools fit a set of files. Names, descriptions, and icons belong to the
//! frontend, keyed by `ToolId::as_str`.

use std::path::Path;

use crate::kinds::{FileKind, detect_kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    CompressPhoto,
    CompressPdf,
    ConvertToJpg,
    ConvertToPng,
    ResizePhoto,
    CompressDocument,
    CompressAudio,
    ConvertAudioFormat,
    CompressVideo,
    ConvertToPdf,
    ScanDocument,
    MergePdf,
    SplitPdf,
    OrganizePdf,
    AddWatermark,
    RemoveLocationInfo,
}

/// Ids of the Tools that 0.4.1 folded into the goal-based convert Tools,
/// still found in History, recent Tools, and Presets saved before then.
const LEGACY_IDS: &[(&str, ToolId)] = &[
    ("ubah-format-foto", ToolId::ConvertToJpg),
    ("jpg-ke-pdf", ToolId::ConvertToPdf),
    ("pdf-ke-jpg", ToolId::ConvertToJpg),
];

impl ToolId {
    /// The stable id the frontend keys names, descriptions, and icons by.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompressPhoto => "kompres-foto",
            Self::CompressPdf => "kompres-pdf",
            Self::ConvertToJpg => "ubah-ke-jpg",
            Self::ConvertToPng => "ubah-ke-png",
            Self::ResizePhoto => "ubah-ukuran-foto",
            Self::CompressDocument => "kompres-dokumen",
            Self::CompressAudio => "kompres-audio",
            Self::ConvertAudioFormat => "ubah-format-audio",
            Self::CompressVideo => "kompres-video",
            Self::ConvertToPdf => "ubah-ke-pdf",
            Self::ScanDocument => "scan-dokumen",
            Self::MergePdf => "gabung-pdf",
            Self::SplitPdf => "pisah-pdf",
            Self::OrganizePdf => "atur-halaman-pdf",
            Self::AddWatermark => "beri-watermark",
            Self::RemoveLocationInfo => "hapus-info-lokasi",
        }
    }

    /// The Tool an id names, including the id of a Tool that was folded into
    /// another one: "pdf-ke-jpg" is Ubah ke JPG now.
    pub fn parse(id: &str) -> Option<Self> {
        CATALOGUE
            .iter()
            .map(|tool| tool.id)
            .find(|tool| tool.as_str() == id)
            .or_else(|| {
                LEGACY_IDS
                    .iter()
                    .find(|(old, _)| *old == id)
                    .map(|(_, tool)| *tool)
            })
    }

    /// Whether `id` names this Tool, now or before 0.4.1.
    pub fn is_named(self, id: &str) -> bool {
        Self::parse(id) == Some(self)
    }
}

/// The five jobs-to-be-done; the All tools page groups Tools by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Job {
    /// "Kecilkan & Ubah"
    ShrinkAndConvert,
    /// "Scan"
    Scan,
    /// "Atur PDF"
    RearrangePdf,
    /// "Ambil Teks"
    GetText,
    /// "Bagikan dengan Aman"
    ShareSafely,
}

impl Job {
    /// The stable id the frontend keys Job headings by.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShrinkAndConvert => "kecilkan-ubah",
            Self::Scan => "scan",
            Self::RearrangePdf => "atur-pdf",
            Self::GetText => "ambil-teks",
            Self::ShareSafely => "bagikan-aman",
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
    /// The file kinds the Tool takes, in groups: one Task takes files from
    /// one group only, so a Tool page shows one kind of file. Ubah ke JPG
    /// takes photos or PDFs, never both at once. Empty for Tools that start
    /// from the camera.
    pub accepts: &'static [&'static [FileKind]],
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
    /// Whether the Tool takes all of `kinds` in one Task: as many files as
    /// there are, all from one of its groups.
    pub fn takes(&self, kinds: &[FileKind]) -> bool {
        self.files.allows(kinds.len())
            && self
                .accepts
                .iter()
                .any(|group| kinds.iter().all(|kind| group.contains(kind)))
    }

    /// Whether the Tool takes files of `kind` at all.
    pub fn accepts_kind(&self, kind: FileKind) -> bool {
        self.accepts.iter().any(|group| group.contains(&kind))
    }

    const fn taking(self, files: FileCount) -> Self {
        Self { files, ..self }
    }
}

use Availability::*;
use FileKind::*;
use Job::*;

const ALL: &[Platform] = &[Platform::Windows, Platform::Linux, Platform::Android];
const PHOTOS: &[FileKind] = &[Jpg, Png, Heic, Webp];
const PDF: &[FileKind] = &[Pdf];
/// A locked document goes to Kompres Dokumen, which says it needs its password.
const OFFICE: &[FileKind] = &[Word, Excel, PowerPoint, LockedOffice];

/// Every Tool planned before 1.0, in the order the All tools page shows them
/// within a Job. Tools planned for after 1.0 are not listed, so they never
/// appear.
pub const CATALOGUE: &[Tool] = &[
    tool(ToolId::CompressPhoto, ShrinkAndConvert, &[PHOTOS], ALL, Shipped),
    tool(ToolId::CompressPdf, ShrinkAndConvert, &[PDF], ALL, Shipped),
    tool(
        ToolId::ConvertToJpg,
        ShrinkAndConvert,
        &[PHOTOS, PDF],
        ALL,
        Shipped,
    ),
    tool(ToolId::ConvertToPng, ShrinkAndConvert, &[PHOTOS], ALL, Shipped),
    tool(
        ToolId::ResizePhoto,
        ShrinkAndConvert,
        &[PHOTOS],
        ALL,
        Shipped,
    ),
    tool(
        ToolId::CompressDocument,
        ShrinkAndConvert,
        &[OFFICE],
        ALL,
        Shipped,
    ),
    tool(
        ToolId::CompressAudio,
        ShrinkAndConvert,
        &[&[Audio]],
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::ConvertAudioFormat,
        ShrinkAndConvert,
        &[&[Audio]],
        ALL,
        ComingSoon,
    ),
    tool(
        ToolId::CompressVideo,
        ShrinkAndConvert,
        &[&[Video]],
        ALL,
        ComingSoon,
    ),
    tool(ToolId::ConvertToPdf, Scan, &[PHOTOS], ALL, Shipped),
    tool(
        ToolId::ScanDocument,
        Scan,
        &[],
        &[Platform::Android],
        Shipped,
    ),
    tool(ToolId::MergePdf, RearrangePdf, &[PDF], ALL, Shipped).taking(FileCount::AtLeastTwo),
    tool(ToolId::SplitPdf, RearrangePdf, &[PDF], ALL, Shipped),
    tool(ToolId::OrganizePdf, RearrangePdf, &[PDF], ALL, Shipped).taking(FileCount::One),
    tool(
        ToolId::AddWatermark,
        ShareSafely,
        &[PHOTOS, PDF],
        ALL,
        Shipped,
    ),
    tool(
        ToolId::RemoveLocationInfo,
        ShareSafely,
        &[PHOTOS],
        ALL,
        Shipped,
    ),
];

const fn tool(
    id: ToolId,
    job: Job,
    accepts: &'static [&'static [FileKind]],
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

fn shipped_on(id: ToolId, platform: Platform) -> bool {
    tools_for(platform).any(|tool| tool.id == id && tool.availability == Shipped)
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
/// in their own order. `recent` holds Tool ids, old ones included.
pub fn recent_first(mut tools: Vec<ToolId>, recent: &[impl AsRef<str>]) -> Vec<ToolId> {
    let recent: Vec<Option<ToolId>> = recent.iter().map(|id| ToolId::parse(id.as_ref())).collect();
    let rank = |tool: &ToolId| {
        recent
            .iter()
            .position(|id| *id == Some(*tool))
            .unwrap_or(recent.len())
    };
    tools.sort_by_key(rank);
    tools
}

/// How many Tools Home shows in its grid, next to the "Semua" tile.
pub const HOME_TOOLS: usize = 7;

/// The Tools Home shows to someone who has used none yet, in order.
fn home_defaults(platform: Platform) -> &'static [ToolId] {
    use ToolId::*;
    match platform {
        Platform::Android => &[
            ScanDocument,
            CompressPhoto,
            CompressPdf,
            ConvertToPdf,
            MergePdf,
            ConvertToJpg,
            AddWatermark,
        ],
        Platform::Windows | Platform::Linux => &[
            CompressPdf,
            CompressPhoto,
            ConvertToPdf,
            MergePdf,
            ConvertToJpg,
            SplitPdf,
            AddWatermark,
        ],
    }
}

/// The [`HOME_TOOLS`] Tools Home shows on `platform`: the recently used ones,
/// newest first, then the defaults for that platform, each once. `recent`
/// holds Tool ids, old ones included; an id Rantakit no longer knows, or a
/// Tool that is not shipped there, is skipped.
pub fn home_tools(platform: Platform, recent: &[impl AsRef<str>]) -> Vec<ToolId> {
    let recent = recent.iter().filter_map(|id| ToolId::parse(id.as_ref()));
    let mut tools: Vec<ToolId> = Vec::with_capacity(HOME_TOOLS);
    for tool in recent.chain(home_defaults(platform).iter().copied()) {
        if tools.len() == HOME_TOOLS {
            break;
        }
        if shipped_on(tool, platform) && !tools.contains(&tool) {
            tools.push(tool);
        }
    }
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
    /// An older `.doc`, `.xls`, or `.ppt`, which the user can save again as a
    /// `.docx`, `.xlsx`, or `.pptx` first.
    OldOffice,
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
    if kinds.contains(&OldOffice) {
        return FileMatch::OldOffice;
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
    fn ids_are_unique_and_parse_back() {
        let ids: HashSet<_> = CATALOGUE.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(ids.len(), CATALOGUE.len());
        for tool in CATALOGUE {
            assert_eq!(ToolId::parse(tool.id.as_str()), Some(tool.id));
        }
    }

    #[test]
    fn old_ids_name_the_tools_they_were_folded_into() {
        assert_eq!(ToolId::parse("ubah-format-foto"), Some(ToolId::ConvertToJpg));
        assert_eq!(ToolId::parse("jpg-ke-pdf"), Some(ToolId::ConvertToPdf));
        assert_eq!(ToolId::parse("pdf-ke-jpg"), Some(ToolId::ConvertToJpg));
        assert_eq!(ToolId::parse("word-ke-pdf"), None);
        assert!(ToolId::ConvertToJpg.is_named("pdf-ke-jpg"));
        assert!(!ToolId::ConvertToPng.is_named("ubah-format-foto"));
    }

    #[test]
    fn photos_fit_the_photo_tools_and_pdfs_fit_the_shipped_pdf_tools() {
        use ToolId::*;
        let on = Platform::Windows;
        assert_eq!(
            tools_for_kinds(&[Jpg, Png], on),
            [
                CompressPhoto,
                ConvertToJpg,
                ConvertToPng,
                ResizePhoto,
                ConvertToPdf,
                AddWatermark,
                RemoveLocationInfo
            ]
        );
        assert_eq!(
            tools_for_kinds(&[Pdf, Pdf], on),
            [CompressPdf, ConvertToJpg, MergePdf, SplitPdf, AddWatermark]
        );
    }

    #[test]
    fn heic_and_webp_go_where_jpg_goes() {
        let on = Platform::Android;
        assert_eq!(
            tools_for_kinds(&[Heic, Webp, Jpg], on),
            tools_for_kinds(&[Jpg], on)
        );
    }

    #[test]
    fn photos_and_pdfs_together_fit_nothing_even_where_both_fit() {
        assert!(tools_for_kinds(&[Jpg, Pdf], Platform::Linux).is_empty());
    }

    #[test]
    fn coming_soon_tools_never_match() {
        assert!(tools_for_kinds(&[Audio], Platform::Linux).is_empty());
    }

    #[test]
    fn matching_explains_why_nothing_fits() {
        let on = Platform::Windows;
        assert_eq!(
            match_kinds(&[Jpg, Png], on),
            FileMatch::Fits {
                kinds: vec![Jpg, Png],
                tools: tools_for_kinds(&[Jpg], on),
            }
        );
        assert_eq!(match_kinds(&[Jpg, Pdf], on), FileMatch::Mixed);
        assert_eq!(match_kinds(&[Jpg, Audio], on), FileMatch::Unsupported);
        assert_eq!(match_kinds(&[Video], on), FileMatch::Unsupported);
        assert_eq!(match_kinds(&[OldOffice, Word], on), FileMatch::OldOffice);
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
        let listed = |platform| tools_for(platform).any(|t| t.id == ToolId::ScanDocument);
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
            [
                CompressPdf,
                ConvertToJpg,
                SplitPdf,
                OrganizePdf,
                AddWatermark
            ],
            "one PDF: no Gabung PDF"
        );
        assert_eq!(
            once_shipped(&[Pdf, Pdf]),
            [CompressPdf, ConvertToJpg, MergePdf, SplitPdf, AddWatermark],
            "two PDFs: Gabung PDF, and no Atur Halaman PDF"
        );
    }

    #[test]
    fn office_documents_lead_to_kompres_dokumen() {
        assert!(once_shipped(&[Word, Excel, PowerPoint]).contains(&ToolId::CompressDocument));
        assert_eq!(once_shipped(&[LockedOffice]), [ToolId::CompressDocument]);
        assert!(once_shipped(&[OldOffice]).is_empty());
        assert!(once_shipped(&[Word, Pdf]).is_empty());
    }

    #[test]
    fn photos_and_pdfs_never_share_a_task() {
        assert!(
            once_shipped(&[Jpg, Pdf]).is_empty(),
            "Ubah ke JPG and Beri Watermark take both, but not together"
        );
    }

    #[test]
    fn recently_used_tools_come_first() {
        use ToolId::*;
        let tools = vec![CompressPhoto, ConvertToJpg, ResizePhoto, ConvertToPdf];
        assert_eq!(
            recent_first(
                tools.clone(),
                &["ubah-ke-pdf", "kompres-pdf", "ubah-ke-jpg"]
            ),
            [ConvertToPdf, ConvertToJpg, CompressPhoto, ResizePhoto]
        );
        assert_eq!(
            recent_first(tools.clone(), &["jpg-ke-pdf"]),
            [ConvertToPdf, CompressPhoto, ConvertToJpg, ResizePhoto],
            "an old id still counts"
        );
        assert_eq!(recent_first(tools.clone(), &[] as &[&str]), tools);
    }

    #[test]
    fn home_starts_with_the_defaults_of_each_platform() {
        use ToolId::*;
        let none: &[&str] = &[];
        assert_eq!(
            home_tools(Platform::Android, none),
            [
                ScanDocument,
                CompressPhoto,
                CompressPdf,
                ConvertToPdf,
                MergePdf,
                ConvertToJpg,
                AddWatermark
            ]
        );
        assert_eq!(
            home_tools(Platform::Windows, none),
            [
                CompressPdf,
                CompressPhoto,
                ConvertToPdf,
                MergePdf,
                ConvertToJpg,
                SplitPdf,
                AddWatermark
            ]
        );
    }

    #[test]
    fn home_puts_recent_tools_first_and_tops_up_with_the_defaults() {
        use ToolId::*;
        let recent = [
            "hapus-info-lokasi",
            "pdf-ke-jpg",
            "ubah-ke-jpg",
            "word-ke-pdf",
            "scan-dokumen",
            "kompres-audio",
        ];
        assert_eq!(
            home_tools(Platform::Linux, &recent),
            [
                RemoveLocationInfo,
                ConvertToJpg,
                CompressPdf,
                CompressPhoto,
                ConvertToPdf,
                MergePdf,
                SplitPdf
            ],
            "old ids map to their new Tool once; unknown, Android-only, and coming-soon Tools are skipped"
        );
    }
}
