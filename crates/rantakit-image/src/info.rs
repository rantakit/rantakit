//! Hapus Info Lokasi (K2): what a photo says about where and with what it
//! was taken, and the same photo without it. The picture itself is copied
//! byte for byte, so nothing is lost; only the info around it goes. The
//! orientation stays, as the one tag of a new, minimal EXIF, so the photo
//! still shows upright.

use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::Path;

use rantakit_core::{
    CancelToken, ErrorKind, Progress, Result, Tracker, read_original, write_result,
};

use crate::options::ImageFormat;

/// Progress once the Original is read and its info taken out.
const READ: f32 = 0.3;
const CLEANED: f32 = 0.6;

/// What a photo holds besides its picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FoundInfo {
    /// Where it was taken: GPS latitude or longitude.
    pub location: bool,
    /// What took it: the camera's make, model, lens, or serial number.
    pub camera: bool,
    /// Anything else: the date, the software, an author, a comment, a
    /// thumbnail, an extra picture or video.
    pub other: bool,
}

impl FoundInfo {
    /// Nothing to remove: the photo is shared as it is.
    pub fn is_clean(self) -> bool {
        !(self.location || self.camera || self.other)
    }

    fn add(&mut self, more: FoundInfo) {
        self.location |= more.location;
        self.camera |= more.camera;
        self.other |= more.other;
    }

    const OTHER: FoundInfo = FoundInfo {
        location: false,
        camera: false,
        other: true,
    };
}

/// A Hapus Info Lokasi Result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleaned {
    /// Size of the Result in bytes.
    pub size: u64,
    pub format: ImageFormat,
    /// What was taken out.
    pub removed: FoundInfo,
}

/// What the photo in `file` holds besides its picture, for its row in the
/// Files step. Reads only the info, skipping over the picture.
pub fn found_info_of(mut file: impl BufRead + Seek) -> Result<FoundInfo> {
    let mut start = [0u8; 8];
    file.read_exact(&mut start)
        .map_err(|e| ErrorKind::reading(&e))?;
    let found = if start.starts_with(&[0xFF, 0xD8]) {
        file.seek(SeekFrom::Start(2))
            .and_then(|_| jpeg_head(&mut file))
    } else if start == PNG_SIGNATURE {
        png_head(&mut file)
    } else {
        return Err(ErrorKind::UnsupportedFormat);
    };
    found.map_err(|_| ErrorKind::Damaged)
}

/// Writes the photo at `input` without its info to `output`, which must not
/// exist yet. A photo with nothing to remove gets no Result: `Unchanged`.
pub fn remove_info(
    input: &Path,
    output: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> Result<Cleaned> {
    let mut tracker = Tracker::new(progress, cancel);
    let original = read_original(input)?;
    tracker.step(READ)?;
    let split = without_info(&original)?;
    if split.found.is_clean() {
        return Err(ErrorKind::Unchanged);
    }
    tracker.step(CLEANED)?;
    let size = write_result(output, &mut tracker, |file| file.write_all(&split.bytes))?;
    Ok(Cleaned {
        size,
        format: split.format,
        removed: split.found,
    })
}

/// The photo without its info, and what was in it.
struct Split {
    bytes: Vec<u8>,
    format: ImageFormat,
    found: FoundInfo,
}

/// The photo `bytes` without its info, and what was in it.
fn without_info(bytes: &[u8]) -> Result<Split> {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        jpeg(bytes)
    } else if bytes.starts_with(&PNG_SIGNATURE) {
        png(bytes)
    } else {
        Err(ErrorKind::UnsupportedFormat)
    }
}

/// What one segment or chunk is: kept as it is, or info that is taken out.
struct Verdict {
    keep: bool,
    found: FoundInfo,
    /// The photo's orientation, from its EXIF.
    orientation: Option<u16>,
}

impl Verdict {
    const KEEP: Verdict = Verdict {
        keep: true,
        found: FoundInfo {
            location: false,
            camera: false,
            other: false,
        },
        orientation: None,
    };

    fn remove(found: FoundInfo) -> Self {
        Self {
            keep: false,
            found,
            orientation: None,
        }
    }

    fn exif(tiff: &[u8]) -> Self {
        let facts = exif_facts(tiff);
        Self {
            keep: false,
            found: facts.found,
            orientation: Some(facts.orientation),
        }
    }
}

// ---------------------------------------------------------------- JPG

const EXIF: &[u8] = b"Exif\0\0";
const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const XMP_EXTENSION: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const ICC: &[u8] = b"ICC_PROFILE\0";
const SOS: u8 = 0xDA;
const EOI: u8 = 0xD9;

/// What the picture needs to be shown (tables, frame, scans, color profile,
/// the JFIF and Adobe markers) is kept. EXIF, XMP, IPTC, comments, MPF's
/// extra pictures, and maker data are not.
fn jpeg_segment(marker: u8, data: &[u8]) -> Verdict {
    match marker {
        0xE0 if data.starts_with(b"JFIF\0") => Verdict::KEEP,
        0xE1 if data.starts_with(EXIF) => Verdict::exif(&data[EXIF.len()..]),
        0xE1 if data.starts_with(XMP) || data.starts_with(XMP_EXTENSION) => {
            Verdict::remove(xmp_facts(data))
        }
        0xE2 if data.starts_with(ICC) => Verdict::KEEP,
        0xEE if data.starts_with(b"Adobe") => Verdict::KEEP,
        0xE0..=0xEF | 0xFE => Verdict::remove(FoundInfo::OTHER),
        _ => Verdict::KEEP,
    }
}

/// A marker that stands alone, without a length: restarts, and TEM.
fn stands_alone(marker: u8) -> bool {
    (0xD0..=0xD7).contains(&marker) || marker == 0x01
}

/// A JPG is a list of segments, then the picture's compressed data.
/// Whatever follows the picture's end, such as a motion photo's video, is
/// left out too.
fn jpeg(bytes: &[u8]) -> Result<Split> {
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..2]);
    let mut found = FoundInfo::default();
    let mut orientation = 1;
    // Where the minimal EXIF goes: after SOI, or after a JFIF APP0.
    let mut exif_at = out.len();
    let mut at = 2;

    loop {
        // Fill bytes (0xFF repeated) may come before a marker.
        while bytes.get(at) == Some(&0xFF) && bytes.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        if bytes.get(at) != Some(&0xFF) {
            return Err(ErrorKind::Damaged);
        }
        let marker = *bytes.get(at + 1).ok_or(ErrorKind::Damaged)?;
        if marker == EOI || stands_alone(marker) {
            out.extend_from_slice(&bytes[at..at + 2]);
            at += 2;
            if marker == EOI {
                break;
            }
            continue;
        }
        let length = match bytes.get(at + 2..at + 4) {
            Some(b) => u16::from_be_bytes([b[0], b[1]]) as usize,
            None => return Err(ErrorKind::Damaged),
        };
        let end = at + 2 + length;
        if length < 2 || end > bytes.len() {
            return Err(ErrorKind::Damaged);
        }
        let segment = &bytes[at..end];
        let verdict = jpeg_segment(marker, &segment[4..]);
        found.add(verdict.found);
        if let Some(value) = verdict.orientation {
            orientation = value;
        }
        if verdict.keep {
            out.extend_from_slice(segment);
            if marker == 0xE0 && exif_at == 2 {
                exif_at = out.len();
            }
        }
        at = end;

        if marker == SOS {
            // The scan's compressed data runs to the next marker that is not
            // a stuffed 0xFF00, a restart, or a fill byte.
            let start = at;
            while at + 1 < bytes.len() {
                let next = bytes[at + 1];
                if bytes[at] == 0xFF
                    && next != 0x00
                    && next != 0xFF
                    && !(0xD0..=0xD7).contains(&next)
                {
                    break;
                }
                at += 1;
            }
            if at + 1 >= bytes.len() {
                return Err(ErrorKind::Damaged);
            }
            out.extend_from_slice(&bytes[start..at]);
        }
    }

    if bytes[at..].iter().any(|&b| b != 0) {
        found.add(FoundInfo::OTHER);
    }
    if orientation != 1 {
        let tiff = orientation_tiff(orientation);
        let mut app1 = vec![0xFF, 0xE1];
        app1.extend_from_slice(&((2 + EXIF.len() + tiff.len()) as u16).to_be_bytes());
        app1.extend_from_slice(EXIF);
        app1.extend_from_slice(&tiff);
        out.splice(exif_at..exif_at, app1);
    }
    Ok(Split {
        bytes: out,
        format: ImageFormat::Jpeg,
        found,
    })
}

/// `jpeg`'s findings from the segments before the first scan, reading only
/// those that may hold info. `file` is just past SOI.
fn jpeg_head(file: &mut (impl BufRead + Seek)) -> io::Result<FoundInfo> {
    let mut found = FoundInfo::default();
    loop {
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte)?;
        if byte[0] != 0xFF {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut marker = 0xFF;
        while marker == 0xFF {
            file.read_exact(&mut byte)?;
            marker = byte[0];
        }
        if marker == SOS || marker == EOI {
            return Ok(found);
        }
        if stands_alone(marker) {
            continue;
        }
        let mut length = [0u8; 2];
        file.read_exact(&mut length)?;
        let length = (u16::from_be_bytes(length) as usize)
            .checked_sub(2)
            .ok_or(io::ErrorKind::InvalidData)?;
        if (0xE0..=0xEF).contains(&marker) || marker == 0xFE {
            let mut data = vec![0u8; length];
            file.read_exact(&mut data)?;
            found.add(jpeg_segment(marker, &data).found);
        } else {
            file.seek(SeekFrom::Current(length as i64))?;
        }
    }
}

// ---------------------------------------------------------------- PNG

const PNG_SIGNATURE: [u8; 8] = *b"\x89PNG\r\n\x1a\n";

/// `eXIf`, the text chunks (which hold XMP too), and `tIME` are taken out;
/// color and picture chunks are kept.
fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Verdict {
    match kind {
        b"eXIf" => Verdict::exif(data),
        b"iTXt" if data.starts_with(b"XML:com.adobe.xmp\0") => Verdict::remove(xmp_facts(data)),
        b"tEXt" | b"zTXt" | b"iTXt" | b"tIME" => Verdict::remove(FoundInfo::OTHER),
        _ => Verdict::KEEP,
    }
}

/// Whether `png_chunk` reads this chunk's data.
fn may_hold_info(kind: &[u8; 4]) -> bool {
    matches!(kind, b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"tIME")
}

/// A PNG is a list of chunks, up to `IEND`.
fn png(bytes: &[u8]) -> Result<Split> {
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&PNG_SIGNATURE);
    let mut found = FoundInfo::default();
    let mut orientation = 1;
    let mut exif_at = None;
    let mut at = PNG_SIGNATURE.len();

    loop {
        let header = bytes.get(at..at + 8).ok_or(ErrorKind::Damaged)?;
        let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = [header[4], header[5], header[6], header[7]];
        let end = at
            .checked_add(12)
            .and_then(|n| n.checked_add(length))
            .filter(|&end| end <= bytes.len())
            .ok_or(ErrorKind::Damaged)?;
        let verdict = png_chunk(&kind, &bytes[at + 8..at + 8 + length]);
        found.add(verdict.found);
        if let Some(value) = verdict.orientation {
            orientation = value;
        }
        if &kind == b"IDAT" && exif_at.is_none() {
            exif_at = Some(out.len());
        }
        if verdict.keep {
            out.extend_from_slice(&bytes[at..end]);
        }
        at = end;
        if &kind == b"IEND" {
            break;
        }
    }

    if bytes[at..].iter().any(|&b| b != 0) {
        found.add(FoundInfo::OTHER);
    }
    if orientation != 1 {
        let tiff = orientation_tiff(orientation);
        let mut chunk = Vec::with_capacity(12 + tiff.len());
        chunk.extend_from_slice(&(tiff.len() as u32).to_be_bytes());
        chunk.extend_from_slice(b"eXIf");
        chunk.extend_from_slice(&tiff);
        chunk.extend_from_slice(&crc32(&chunk[4..]).to_be_bytes());
        let at = exif_at.ok_or(ErrorKind::Damaged)?;
        out.splice(at..at, chunk);
    }
    Ok(Split {
        bytes: out,
        format: ImageFormat::Png,
        found,
    })
}

/// `png`'s findings, reading only the chunks that may hold info and
/// skipping over the picture. `file` is just past the signature.
fn png_head(file: &mut (impl BufRead + Seek)) -> io::Result<FoundInfo> {
    let mut found = FoundInfo::default();
    loop {
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let kind = [header[4], header[5], header[6], header[7]];
        if &kind == b"IEND" {
            return Ok(found);
        }
        if may_hold_info(&kind) {
            let mut data = Vec::new();
            file.by_ref().take(length as u64).read_to_end(&mut data)?;
            if data.len() != length as usize {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            found.add(png_chunk(&kind, &data).found);
            file.seek(SeekFrom::Current(4))?;
        } else {
            file.seek(SeekFrom::Current(length as i64 + 4))?;
        }
    }
}

/// The CRC-32 a PNG chunk ends with, over its type and data.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

// ---------------------------------------------------------------- EXIF

/// What an EXIF block says: its orientation (1 when it has none) and what else it holds.
struct ExifFacts {
    orientation: u16,
    found: FoundInfo,
}

const ORIENTATION: u16 = 0x0112;
const EXIF_IFD: u16 = 0x8769;
const GPS_IFD: u16 = 0x8825;
const INTEROP_IFD: u16 = 0xA005;
/// Make, Model; MakerNote; owner, body serial, lens make, model, serial.
const CAMERA_TAGS: &[u16] = &[
    0x010F, 0x0110, 0x927C, 0xA430, 0xA431, 0xA432, 0xA433, 0xA434, 0xA435,
];
/// GPS latitude, longitude, and the destination's.
const LOCATION_TAGS: &[u16] = &[0x0002, 0x0004, 0x0014, 0x0016];
/// Tags that only say how to show the picture: size, resolution, color
/// space, versions. On their own they are not info about the photo.
const PLAIN_TAGS: &[u16] = &[
    0x0100, 0x0101, 0x011A, 0x011B, 0x0128, 0x0213, 0x9000, 0x9101, 0xA000, 0xA001, 0xA002, 0xA003,
];

/// Reads the TIFF structure of an EXIF block. One that cannot be read is
/// all "other": it is removed, and its orientation is not known.
fn exif_facts(tiff: &[u8]) -> ExifFacts {
    let unreadable = ExifFacts {
        orientation: 1,
        found: FoundInfo::OTHER,
    };
    let little = match tiff.get(..4) {
        Some(b"II*\0") => true,
        Some(b"MM\0*") => false,
        _ => return unreadable,
    };
    let reader = Tiff {
        bytes: tiff,
        little,
    };
    let Some((entries, next)) = reader.u32(4).and_then(|first| reader.ifd(first as usize)) else {
        return unreadable;
    };
    let mut facts = ExifFacts {
        orientation: 1,
        found: FoundInfo::default(),
    };
    // IFD1: the thumbnail, a small copy of the photo.
    facts.found.other = next != 0;
    let sub_ifd = |entry: &Entry| {
        reader
            .u32(entry.value_at)
            .and_then(|at| reader.ifd(at as usize))
    };
    for entry in &entries {
        match entry.tag {
            ORIENTATION => {
                if let Some(value) = reader.u16(entry.value_at).filter(|v| (1..=8).contains(v)) {
                    facts.orientation = value;
                }
            }
            EXIF_IFD => match sub_ifd(entry) {
                Some((exif, _)) => {
                    for entry in exif {
                        if CAMERA_TAGS.contains(&entry.tag) {
                            facts.found.camera = true;
                        } else if entry.tag != INTEROP_IFD && !PLAIN_TAGS.contains(&entry.tag) {
                            facts.found.other = true;
                        }
                    }
                }
                None => facts.found.other = true,
            },
            GPS_IFD => match sub_ifd(entry) {
                Some((gps, _)) => {
                    for entry in gps {
                        if LOCATION_TAGS.contains(&entry.tag) {
                            facts.found.location = true;
                        } else {
                            facts.found.other = true;
                        }
                    }
                }
                None => facts.found.other = true,
            },
            tag if CAMERA_TAGS.contains(&tag) => facts.found.camera = true,
            tag if PLAIN_TAGS.contains(&tag) => {}
            _ => facts.found.other = true,
        }
    }
    facts
}

struct Tiff<'a> {
    bytes: &'a [u8],
    little: bool,
}

struct Entry {
    tag: u16,
    /// Where the entry's value, or the offset to it, starts.
    value_at: usize,
}

impl Tiff<'_> {
    fn u16(&self, at: usize) -> Option<u16> {
        let b = self.bytes.get(at..at + 2)?;
        Some(match self.little {
            true => u16::from_le_bytes([b[0], b[1]]),
            false => u16::from_be_bytes([b[0], b[1]]),
        })
    }

    fn u32(&self, at: usize) -> Option<u32> {
        let b = self.bytes.get(at..at + 4)?;
        Some(match self.little {
            true => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            false => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        })
    }

    /// The entries of the IFD at `at`, and where the next IFD is (0 for none).
    fn ifd(&self, at: usize) -> Option<(Vec<Entry>, u32)> {
        let count = self.u16(at)? as usize;
        let entries = (0..count)
            .map(|i| {
                let entry = at + 2 + i * 12;
                Some(Entry {
                    tag: self.u16(entry)?,
                    value_at: entry + 8,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let next = self.u32(at + 2 + count * 12).unwrap_or(0);
        Some((entries, next))
    }
}

/// A big-endian TIFF holding only the Orientation tag: the minimal EXIF.
fn orientation_tiff(orientation: u16) -> Vec<u8> {
    let mut tiff = Vec::with_capacity(26);
    tiff.extend_from_slice(b"MM\0*");
    tiff.extend_from_slice(&8u32.to_be_bytes());
    tiff.extend_from_slice(&1u16.to_be_bytes());
    tiff.extend_from_slice(&ORIENTATION.to_be_bytes());
    tiff.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    tiff.extend_from_slice(&1u32.to_be_bytes());
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0]);
    tiff.extend_from_slice(&0u32.to_be_bytes());
    tiff
}

// ---------------------------------------------------------------- XMP

/// XMP is text; its location and camera fields have well-known names.
fn xmp_facts(xmp: &[u8]) -> FoundInfo {
    let has = |name: &[u8]| xmp.windows(name.len()).any(|w| w == name);
    FoundInfo {
        location: has(b"GPSLatitude") || has(b"GPSLongitude"),
        camera: has(b"tiff:Make")
            || has(b"tiff:Model")
            || has(b"LensModel")
            || has(b"SerialNumber")
            || has(b"aux:Lens"),
        other: true,
    }
}
