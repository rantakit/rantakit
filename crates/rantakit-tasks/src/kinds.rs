use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use rantakit_office::{OfficeKind, Sniffed};

/// What a file is, judged by its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Jpg,
    Png,
    Heic,
    Webp,
    Pdf,
    Word,
    Excel,
    PowerPoint,
    /// A Word, Excel, or PowerPoint document saved with a password. Which of
    /// the three it is stays hidden until it is opened.
    LockedOffice,
    /// The older `.doc`, `.xls`, or `.ppt`, which no Tool opens.
    OldOffice,
    Audio,
    Video,
}

impl FileKind {
    /// The stable id the frontend uses for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Heic => "heic",
            Self::Webp => "webp",
            Self::Pdf => "pdf",
            Self::Word => "word",
            Self::Excel => "excel",
            Self::PowerPoint => "powerpoint",
            Self::LockedOffice => "locked-office",
            Self::OldOffice => "old-office",
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

/// PDF readers accept the `%PDF-` header anywhere in the first 1024 bytes.
const HEADER_BYTES: usize = 1024;

/// The kind of the file at `path` from its first bytes, not its extension.
/// `None` when it cannot be read or is not a kind Rantakit knows. Audio
/// and video files are recognized once their Tools ship.
pub fn detect_kind(path: &Path) -> Option<FileKind> {
    read_kind(&mut File::open(path).ok()?)
}

/// `detect_kind` of an open file, from its start. An Office document is a ZIP
/// whose list of parts, at its end, tells Word, Excel, and PowerPoint apart.
pub fn read_kind(file: &mut (impl Read + Seek)) -> Option<FileKind> {
    let mut header = Vec::with_capacity(HEADER_BYTES);
    file.rewind().ok()?;
    file.by_ref()
        .take(HEADER_BYTES as u64)
        .read_to_end(&mut header)
        .ok()?;
    if header.starts_with(ZIP_MAGIC) || header.starts_with(COMPOUND_MAGIC) {
        return match rantakit_office::sniff(file)? {
            Sniffed::Office(OfficeKind::Word) => Some(FileKind::Word),
            Sniffed::Office(OfficeKind::Excel) => Some(FileKind::Excel),
            Sniffed::Office(OfficeKind::PowerPoint) => Some(FileKind::PowerPoint),
            Sniffed::Locked => Some(FileKind::LockedOffice),
            Sniffed::Old => Some(FileKind::OldOffice),
        };
    }
    kind_of(&header)
}

const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const COMPOUND_MAGIC: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn kind_of(header: &[u8]) -> Option<FileKind> {
    let at = |offset: usize, bytes: &[u8]| header.get(offset..offset + bytes.len()) == Some(bytes);

    if at(0, &[0xFF, 0xD8, 0xFF]) {
        Some(FileKind::Jpg)
    } else if at(0, b"\x89PNG\r\n\x1a\n") {
        Some(FileKind::Png)
    } else if at(0, b"RIFF") && at(8, b"WEBP") {
        Some(FileKind::Webp)
    } else if at(4, b"ftyp") && HEIC_BRANDS.iter().any(|brand| at(8, brand)) {
        Some(FileKind::Heic)
    } else if header.windows(5).any(|window| window == b"%PDF-") {
        Some(FileKind::Pdf)
    } else {
        None
    }
}

const HEIC_BRANDS: &[&[u8]] = &[
    b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_kinds_by_their_first_bytes() {
        assert_eq!(kind_of(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(FileKind::Jpg));
        assert_eq!(kind_of(b"\x89PNG\r\n\x1a\n...."), Some(FileKind::Png));
        assert_eq!(kind_of(b"RIFF\0\0\0\0WEBPVP8 "), Some(FileKind::Webp));
        assert_eq!(kind_of(b"\0\0\0\x18ftypheic"), Some(FileKind::Heic));
        assert_eq!(kind_of(b"%PDF-1.7\n"), Some(FileKind::Pdf));
        assert_eq!(kind_of(b"\r\n%PDF-1.4"), Some(FileKind::Pdf));
    }

    #[test]
    fn unknown_or_empty_is_none() {
        assert_eq!(kind_of(b""), None);
        assert_eq!(kind_of(b"just some text"), None);
        assert_eq!(
            kind_of(b"\0\0\0\x18ftypisom"),
            None,
            "MP4 video is not recognized yet"
        );
    }
}
