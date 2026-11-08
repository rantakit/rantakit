use std::io::{Cursor, Read, Seek, SeekFrom};

use zip::ZipArchive;

/// Which Office program a document belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OfficeKind {
    Word,
    Excel,
    PowerPoint,
}

/// What an Office-looking file turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sniffed {
    /// A `.docx`, `.xlsx`, or `.pptx`, or their macro and template variants.
    Office(OfficeKind),
    /// A document saved with a password: Office wraps the encrypted package
    /// in the old compound file format.
    Locked,
    /// The older `.doc`, `.xls`, or `.ppt`, which Rantakit doesn't open.
    Old,
}

const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
/// The compound file format of the older Office files and of locked ones.
const COMPOUND_MAGIC: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
/// The stream a locked document keeps its encrypted package in, as the
/// compound file's directory spells it: UTF-16, little-endian.
const ENCRYPTED_PACKAGE: &str = "EncryptedPackage";
/// Read at a time while looking for that name.
const CHUNK: usize = 1 << 20;

/// Whether `file` is an Office document, judged by its content; `None` for
/// anything else. It reads from the start and may leave `file` anywhere.
pub fn sniff(file: &mut (impl Read + Seek)) -> Option<Sniffed> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut magic = [0; 8];
    file.read_exact(&mut magic).ok()?;
    if magic.starts_with(ZIP_MAGIC) {
        let archive = ZipArchive::new(file).ok()?;
        let names: Vec<String> = archive.file_names().filter_map(|name| name.ok().map(|name| name.into_owned())).collect();
        office_kind(names.iter().map(String::as_str)).map(Sniffed::Office)
    } else if magic == COMPOUND_MAGIC {
        Some(if mentions_encrypted_package(file)? {
            Sniffed::Locked
        } else {
            Sniffed::Old
        })
    } else {
        None
    }
}

/// [`sniff`] of a document already in memory.
pub(crate) fn sniff_bytes(bytes: &[u8]) -> Option<Sniffed> {
    sniff(&mut Cursor::new(bytes))
}

/// The program whose folder holds the parts, from the names in the ZIP.
/// Every Office document has a `[Content_Types].xml`.
pub(crate) fn office_kind<'a>(names: impl Iterator<Item = &'a str>) -> Option<OfficeKind> {
    let mut types = false;
    let mut kind = None;
    for name in names {
        if name.eq_ignore_ascii_case("[Content_Types].xml") {
            types = true;
        }
        let folder = name.split('/').next().unwrap_or_default().to_ascii_lowercase();
        kind = kind.or(match folder.as_str() {
            "word" => Some(OfficeKind::Word),
            "xl" => Some(OfficeKind::Excel),
            "ppt" => Some(OfficeKind::PowerPoint),
            _ => None,
        });
    }
    kind.filter(|_| types)
}

/// Whether a compound file names an `EncryptedPackage` stream. The name sits
/// in a directory sector that can be anywhere, so the whole file is read.
fn mentions_encrypted_package(file: &mut impl Read) -> Option<bool> {
    let needle: Vec<u8> = ENCRYPTED_PACKAGE
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut window = Vec::with_capacity(CHUNK + needle.len());
    let mut chunk = vec![0; CHUNK];
    loop {
        let read = file.read(&mut chunk).ok()?;
        if read == 0 {
            return Some(false);
        }
        window.extend_from_slice(&chunk[..read]);
        if window.windows(needle.len()).any(|w| w == needle) {
            return Some(true);
        }
        // Keep the tail, in case the name spans two reads.
        let keep = window.len().min(needle.len() - 1);
        window.drain(..window.len() - keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_names_the_program() {
        let kind = |names: &[&str]| office_kind(names.iter().copied());
        assert_eq!(
            kind(&["[Content_Types].xml", "_rels/.rels", "word/document.xml"]),
            Some(OfficeKind::Word)
        );
        assert_eq!(kind(&["[Content_Types].xml", "xl/workbook.xml"]), Some(OfficeKind::Excel));
        assert_eq!(
            kind(&["[Content_Types].xml", "ppt/presentation.xml"]),
            Some(OfficeKind::PowerPoint)
        );
        assert_eq!(kind(&["word/document.xml"]), None, "no content types");
        assert_eq!(kind(&["[Content_Types].xml", "content.xml"]), None);
    }

    #[test]
    fn compound_files_are_old_or_locked() {
        let mut old = COMPOUND_MAGIC.to_vec();
        old.extend(vec![0; 3000]);
        old.extend("WordDocument".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(sniff_bytes(&old), Some(Sniffed::Old));

        let mut locked = COMPOUND_MAGIC.to_vec();
        locked.extend(vec![0; CHUNK - 10]);
        locked.extend(ENCRYPTED_PACKAGE.encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(sniff_bytes(&locked), Some(Sniffed::Locked), "across two reads");

        assert_eq!(sniff_bytes(b"%PDF-1.7 and more"), None);
        assert_eq!(sniff_bytes(b"PK"), None);
    }
}
