use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use rantakit_core::{ErrorKind, Result};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::picture::PictureFormat;
use crate::sniff::{OfficeKind, Sniffed, office_kind, sniff, sniff_bytes};

/// A document's parts may unpack to at most this much, so a ZIP bomb can't
/// fill the memory.
const MAX_UNPACKED_BYTES: u64 = 1 << 30;
/// The strongest deflate, for the XML parts written again.
const BEST_DEFLATE: i64 = 9;
/// What one entry adds to a ZIP besides its data and twice its name: the
/// local header, the central directory record, and room for a descriptor.
pub(crate) const ENTRY_OVERHEAD: u64 = 30 + 46 + 16;

const CONTENT_TYPES: &str = "[Content_Types].xml";

/// What the Files step shows about a document before a Task runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentInfo {
    pub kind: OfficeKind,
    /// JPG and PNG pictures, the only parts Kompres Dokumen can make smaller.
    pub pictures: usize,
}

pub fn document_info(path: &Path) -> Result<DocumentInfo> {
    let file = File::open(path).map_err(|e| ErrorKind::reading(&e))?;
    document_info_of(file)
}

/// `document_info` of an open document, such as one Android handed over
/// without a path.
pub fn document_info_of(mut file: impl Read + Seek) -> Result<DocumentInfo> {
    let kind = match sniff(&mut file) {
        Some(Sniffed::Office(kind)) => kind,
        Some(Sniffed::Locked) => return Err(ErrorKind::PasswordProtected),
        Some(Sniffed::Old) | None => return Err(ErrorKind::UnsupportedFormat),
    };
    let archive = ZipArchive::new(file).map_err(|_| ErrorKind::Damaged)?;
    let pictures = archive
        .file_names()
        .filter_map(|name| name.ok())
        .filter(|name| is_media(name) && PictureFormat::from_name(name).is_some())
        .count();
    Ok(DocumentInfo { kind, pictures })
}

/// A part of the document, as the original ZIP has it.
pub(crate) struct Entry {
    pub name: String,
    /// Its bytes in the ZIP, compressed.
    pub stored_len: u64,
    pub picture: Option<PictureFormat>,
}

/// An Office document opened for Kompres Dokumen.
pub(crate) struct Package<'a> {
    original: ZipArchive<Cursor<&'a [u8]>>,
    /// Every part but the pictures, deflated again at the strongest level.
    deflated: ZipArchive<Cursor<Vec<u8>>>,
    /// For each entry: whether the deflated copy is the smaller one.
    use_deflated: Vec<bool>,
    pub entries: Vec<Entry>,
}

impl<'a> Package<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self> {
        match sniff_bytes(bytes) {
            Some(Sniffed::Office(_)) => {}
            Some(Sniffed::Locked) => return Err(ErrorKind::PasswordProtected),
            Some(Sniffed::Old) | None => return Err(ErrorKind::UnsupportedFormat),
        }
        let mut original = ZipArchive::new(Cursor::new(bytes)).map_err(|_| ErrorKind::Damaged)?;
        let mut entries = Vec::with_capacity(original.len());
        let mut unpacked = 0u64;
        for index in 0..original.len() {
            let file = original.by_index_raw(index).map_err(|_| ErrorKind::Damaged)?;
            if file.encrypted() {
                return Err(ErrorKind::PasswordProtected);
            }
            unpacked = unpacked.saturating_add(file.size());
            let name = file.name().map_err(|_| ErrorKind::Damaged)?.into_owned();
            let picture = PictureFormat::from_name(&name).filter(|_| is_media(&name));
            entries.push(Entry {
                stored_len: file.compressed_size(),
                picture,
                name,
            });
        }
        if unpacked > MAX_UNPACKED_BYTES || office_kind(entries.iter().map(|e| e.name.as_str())).is_none() {
            return Err(ErrorKind::Damaged);
        }

        let mut package = Self {
            original,
            deflated: ZipArchive::new(Cursor::new(Vec::new())).unwrap_or_else(|_| empty_archive()),
            use_deflated: vec![false; entries.len()],
            entries,
        };
        package.deflate_parts()?;
        Ok(package)
    }

    /// Deflates every part but the media again, once, and keeps whichever of
    /// the two is smaller. Pictures change on every attempt, and other media,
    /// like a slide's video, are already compressed.
    fn deflate_parts(&mut self) -> Result<()> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let mut written = Vec::new();
        for index in 0..self.entries.len() {
            let name = self.entries[index].name.clone();
            if is_media(&name) || name.ends_with('/') {
                continue;
            }
            let bytes = self.read(index)?;
            writer
                .start_file(name, deflate_options())
                .and_then(|()| writer.write_all(&bytes).map_err(Into::into))
                .map_err(|_| ErrorKind::Damaged)?;
            written.push(index);
        }
        let cursor = writer.finish().map_err(|_| ErrorKind::Damaged)?;
        self.deflated = ZipArchive::new(cursor).map_err(|_| ErrorKind::Damaged)?;
        for (at, index) in written.into_iter().enumerate() {
            let smaller = self
                .deflated
                .by_index_raw(at)
                .map(|file| file.compressed_size() < self.entries[index].stored_len)
                .unwrap_or(false);
            self.use_deflated[index] = smaller;
        }
        Ok(())
    }

    /// The unpacked bytes of entry `index`.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>> {
        let mut file = self.original.by_index(index).map_err(|_| ErrorKind::Damaged)?;
        let mut bytes = Vec::with_capacity(file.size().min(MAX_UNPACKED_BYTES) as usize);
        file.by_ref()
            .take(MAX_UNPACKED_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|_| ErrorKind::Damaged)?;
        Ok(bytes)
    }

    /// The bytes every entry but the pictures takes in the Result, headers
    /// included, and the end of the ZIP.
    pub fn base_len(&mut self) -> u64 {
        let mut total = 22;
        let mut deflated_at = 0;
        for (index, entry) in self.entries.iter().enumerate() {
            let in_deflated = !is_media(&entry.name) && !entry.name.ends_with('/');
            let len = if self.use_deflated[index] {
                self.deflated
                    .by_index_raw(deflated_at)
                    .map(|file| file.compressed_size())
                    .unwrap_or(entry.stored_len)
            } else {
                entry.stored_len
            };
            if in_deflated {
                deflated_at += 1;
            }
            if entry.picture.is_none() {
                total += entry_len(&entry.name, len);
            }
        }
        total
    }

    /// The document with each picture in `pictures` (by entry index) put in
    /// place of the original; every other part is copied as it is.
    pub fn pack(&mut self, pictures: &HashMap<usize, NewPicture>) -> Result<Vec<u8>> {
        let renames = self.renames(pictures);
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let fail = |_| ErrorKind::Damaged;
        let mut deflated_at = 0;
        for index in 0..self.entries.len() {
            let name = self.entries[index].name.clone();
            let in_deflated = !is_media(&name) && !name.ends_with('/');
            let deflated_index = deflated_at;
            if in_deflated {
                deflated_at += 1;
            }

            if let Some(picture) = pictures.get(&index) {
                let name = renames.get(&index).cloned().unwrap_or(name);
                writer.start_file(name, stored_options()).map_err(fail)?;
                writer.write_all(&picture.bytes).map_err(|_| ErrorKind::Damaged)?;
            } else if !renames.is_empty() && (name == CONTENT_TYPES || name.ends_with(".rels")) {
                let text = String::from_utf8(self.read(index)?).map_err(|_| ErrorKind::Damaged)?;
                let text = if name == CONTENT_TYPES {
                    retype(&text, &renames, &self.entries)
                } else {
                    retarget(&text, &name, &renames, &self.entries)
                };
                writer.start_file(name, deflate_options()).map_err(fail)?;
                writer.write_all(text.as_bytes()).map_err(|_| ErrorKind::Damaged)?;
            } else if self.use_deflated[index] {
                let file = self.deflated.by_index_raw(deflated_index).map_err(fail)?;
                writer.raw_copy_file(file).map_err(fail)?;
            } else {
                let file = self.original.by_index_raw(index).map_err(fail)?;
                writer.raw_copy_file(file).map_err(fail)?;
            }
        }
        Ok(writer.finish().map_err(fail)?.into_inner())
    }

    /// The new name of each picture that became a JPG: `image3.png` turns
    /// into `image3.jpeg`, or `image3-1.jpeg` when that is taken.
    fn renames(&self, pictures: &HashMap<usize, NewPicture>) -> HashMap<usize, String> {
        let mut taken: HashSet<String> = self.entries.iter().map(|e| e.name.to_ascii_lowercase()).collect();
        let mut renames = HashMap::new();
        let mut converted: Vec<usize> = pictures
            .iter()
            .filter(|(_, picture)| picture.became_jpeg)
            .map(|(&index, _)| index)
            .collect();
        converted.sort_unstable();
        for index in converted {
            let name = &self.entries[index].name;
            let stem = name.rsplit_once('.').map_or(name.as_str(), |(stem, _)| stem);
            let new = (0..)
                .map(|n| match n {
                    0 => format!("{stem}.jpeg"),
                    n => format!("{stem}-{n}.jpeg"),
                })
                .find(|candidate| !taken.contains(&candidate.to_ascii_lowercase()))
                .expect("some name is free");
            taken.insert(new.to_ascii_lowercase());
            renames.insert(index, new);
        }
        renames
    }
}

/// A picture made smaller, to put in place of the original.
pub(crate) struct NewPicture {
    pub bytes: Vec<u8>,
    /// A PNG that became a JPG, so its part gets a new name.
    pub became_jpeg: bool,
}

/// The bytes an entry with `name` and `len` bytes of data takes in a ZIP.
pub(crate) fn entry_len(name: &str, len: u64) -> u64 {
    ENTRY_OVERHEAD + 2 * name.len() as u64 + len
}

/// Pictures live in the program's `media` folder, such as `word/media/`.
fn is_media(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["word/media/", "xl/media/", "ppt/media/"]
        .iter()
        .any(|folder| lower.starts_with(folder))
}

fn deflate_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(BEST_DEFLATE))
}

/// Pictures are compressed already; deflating them again saves nothing.
fn stored_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

fn empty_archive() -> ZipArchive<Cursor<Vec<u8>>> {
    let empty = ZipWriter::new(Cursor::new(Vec::new()))
        .finish()
        .expect("an empty ZIP");
    ZipArchive::new(empty).expect("an empty ZIP reads back")
}

/// `[Content_Types].xml` with the renamed parts typed as JPG: a `jpeg`
/// default, and any override of an old name moved to the new one.
fn retype(text: &str, renames: &HashMap<usize, String>, entries: &[Entry]) -> String {
    let moved: HashMap<String, &str> = renames
        .iter()
        .map(|(&index, new)| (format!("/{}", entries[index].name).to_ascii_lowercase(), new.as_str()))
        .collect();
    let mut out = String::with_capacity(text.len() + 80);
    let mut rest = text;
    while let Some(start) = rest.find("<Override") {
        let Some(len) = rest[start..].find('>') else {
            break;
        };
        let element = &rest[start..=start + len];
        out.push_str(&rest[..start]);
        match attribute(element, "PartName").and_then(|part| moved.get(&part.to_ascii_lowercase())) {
            Some(new) => out.push_str(&format!(r#"<Override PartName="/{new}" ContentType="image/jpeg"/>"#)),
            None => out.push_str(element),
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);

    let has_jpeg = out
        .match_indices("<Default")
        .filter_map(|(at, _)| out[at..].find('>').map(|len| &out[at..=at + len]))
        .any(|element| attribute(element, "Extension").is_some_and(|ext| ext.eq_ignore_ascii_case("jpeg")));
    if !has_jpeg && let Some(types) = out.find("<Types") && let Some(len) = out[types..].find('>') {
        out.insert_str(types + len + 1, r#"<Default Extension="jpeg" ContentType="image/jpeg"/>"#);
    }
    out
}

/// A relationships part with every target that names a renamed picture
/// pointed at its new name.
fn retarget(text: &str, rels_name: &str, renames: &HashMap<usize, String>, entries: &[Entry]) -> String {
    // `word/_rels/document.xml.rels` describes `word/document.xml`, whose
    // folder relative targets start from.
    let folder = rels_name
        .rsplit_once("_rels/")
        .map_or("", |(folder, _)| folder);
    let moved: HashMap<String, &str> = renames
        .iter()
        .map(|(&index, new)| (entries[index].name.to_ascii_lowercase(), new.as_str()))
        .collect();

    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<Relationship ") {
        let Some(len) = rest[start..].find('>') else {
            break;
        };
        let element = &rest[start..=start + len];
        out.push_str(&rest[..start]);
        let external = attribute(element, "TargetMode").is_some_and(|mode| mode == "External");
        let new = attribute(element, "Target")
            .filter(|_| !external)
            .and_then(|target| moved.get(&resolve(folder, &target).to_ascii_lowercase()).map(|new| (target, *new)));
        match new {
            Some((target, new)) => {
                let file = new.rsplit('/').next().unwrap_or(new).replace(' ', "%20");
                let kept = target.rsplit_once('/').map_or("", |(dir, _)| dir);
                let new_target = if kept.is_empty() { file.to_owned() } else { format!("{kept}/{file}") };
                out.push_str(&element.replacen(&format!("\"{target}\""), &format!("\"{new_target}\""), 1)
                    .replacen(&format!("'{target}'"), &format!("'{new_target}'"), 1));
            }
            None => out.push_str(element),
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

/// The value of `name="…"` (or `name='…'`) in one XML element.
fn attribute(element: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = element[from..].find(name) {
        let start = from + at;
        from = start + name.len();
        let before = element[..start].chars().last();
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let after = element[from..].trim_start();
        let Some(after) = after.strip_prefix('=') else {
            continue;
        };
        let after = after.trim_start();
        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let value = &after[1..];
        return value.find(quote).map(|end| value[..end].to_owned());
    }
    None
}

/// A relationship target as a part name: `../media/image1.png` from
/// `word/charts/` is `word/media/image1.png`.
fn resolve(folder: &str, target: &str) -> String {
    let target = percent_decoded(target);
    let mut parts: Vec<&str> = if let Some(absolute) = target.strip_prefix('/') {
        return normalized(absolute.split('/').collect());
    } else {
        folder.split('/').filter(|part| !part.is_empty()).collect()
    };
    parts.extend(target.split('/'));
    normalized(parts)
}

fn normalized(parts: Vec<&str>) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in parts {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            part => out.push(part),
        }
    }
    out.join("/")
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && let Some(hex) = text.get(at + 1..at + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> Entry {
        Entry {
            name: name.into(),
            stored_len: 0,
            picture: None,
        }
    }

    #[test]
    fn targets_resolve_from_the_part_folder() {
        assert_eq!(resolve("word/", "media/image1.png"), "word/media/image1.png");
        assert_eq!(resolve("word/charts/", "../media/image1.png"), "word/media/image1.png");
        assert_eq!(resolve("ppt/slides/", "/ppt/media/a%20b.png"), "ppt/media/a b.png");
        assert_eq!(resolve("", "word/document.xml"), "word/document.xml");
    }

    #[test]
    fn renamed_pictures_are_pointed_at_and_typed() {
        let entries = vec![entry("ppt/media/image1.png"), entry("ppt/media/image2.png")];
        let renames = HashMap::from([(0, "ppt/media/image1.jpeg".to_owned())]);

        let rels = r#"<Relationships><Relationship Id="rId2" Type="…/image" Target="../media/image1.png"/><Relationship Id="rId3" Type="…/image" Target="../media/image2.png"/><Relationship Id="rId4" TargetMode="External" Target="../media/image1.png"/></Relationships>"#;
        let out = retarget(rels, "ppt/slides/_rels/slide1.xml.rels", &renames, &entries);
        assert!(out.contains(r#"Id="rId2" Type="…/image" Target="../media/image1.jpeg""#), "{out}");
        assert!(out.contains(r#"Target="../media/image2.png""#));
        assert!(out.contains(r#"TargetMode="External" Target="../media/image1.png""#));

        let types = r#"<?xml version="1.0"?><Types xmlns="x"><Default Extension="png" ContentType="image/png"/><Override PartName="/ppt/media/image1.png" ContentType="image/png"/></Types>"#;
        let out = retype(types, &renames, &entries);
        assert!(out.contains(r#"<Types xmlns="x"><Default Extension="jpeg" ContentType="image/jpeg"/>"#), "{out}");
        assert!(out.contains(r#"<Override PartName="/ppt/media/image1.jpeg" ContentType="image/jpeg"/>"#));
        assert!(!out.contains("/ppt/media/image1.png"));

        let typed = retype(&out, &renames, &entries);
        assert_eq!(typed.matches(r#"Extension="jpeg""#).count(), 1, "one jpeg default");
    }

    #[test]
    fn attributes_need_their_whole_name() {
        let element = r#"<Relationship Id="1" TargetMode='External' Target="a.png"/>"#;
        assert_eq!(attribute(element, "Target").as_deref(), Some("a.png"));
        assert_eq!(attribute(element, "TargetMode").as_deref(), Some("External"));
        assert_eq!(attribute(element, "Type"), None);
    }
}
