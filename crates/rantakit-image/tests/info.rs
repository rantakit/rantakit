//! Hapus Info Lokasi (K2): the info goes, the picture stays byte for byte,
//! and the orientation is kept.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{ImageDecoder, ImageReader, RgbImage};
use rantakit_image::{CancelToken, ErrorKind, FoundInfo, ImageFormat, found_info_of, remove_info};

const PHOTO: &[u8] = include_bytes!("fixtures/photo.jpg");
const ROTATED: &[u8] = include_bytes!("fixtures/rotated_exif6.jpg");

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-image-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn found(bytes: &[u8]) -> FoundInfo {
    found_info_of(Cursor::new(bytes)).unwrap()
}

fn clean(input: &Path, output: &Path) -> Result<rantakit_image::Cleaned, ErrorKind> {
    remove_info(input, output, &mut |_| {}, &CancelToken::new())
}

fn contains(bytes: &[u8], text: &[u8]) -> bool {
    bytes.windows(text.len()).any(|w| w == text)
}

fn orientation(bytes: &[u8]) -> Orientation {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    decoder.orientation().unwrap()
}

/// The pixels as stored, before any orientation.
fn pixels(bytes: &[u8]) -> RgbImage {
    image::load_from_memory(bytes).unwrap().to_rgb8()
}

/// A little-endian EXIF block: Make "Qz7", Orientation 6, a date in the EXIF
/// IFD, and a latitude in the GPS IFD.
fn exif_tiff() -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(b"II*\0");
    t.extend_from_slice(&8u32.to_le_bytes());
    let entry = |t: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: [u8; 4]| {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&kind.to_le_bytes());
        t.extend_from_slice(&count.to_le_bytes());
        t.extend_from_slice(&value);
    };
    // IFD0 at 8: 4 entries, then the next-IFD offset; ends at 8 + 2 + 48 + 4 = 62.
    let exif_ifd = 62u32;
    let gps_ifd = exif_ifd + 2 + 12 + 4;
    t.extend_from_slice(&4u16.to_le_bytes());
    entry(&mut t, 0x010F, 2, 4, *b"Qz7\0");
    entry(&mut t, 0x0112, 3, 1, [6, 0, 0, 0]);
    entry(&mut t, 0x8769, 4, 1, exif_ifd.to_le_bytes());
    entry(&mut t, 0x8825, 4, 1, gps_ifd.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    // EXIF IFD: DateTimeOriginal, cut short for the test.
    t.extend_from_slice(&1u16.to_le_bytes());
    entry(&mut t, 0x9003, 2, 4, *b"2026");
    t.extend_from_slice(&0u32.to_le_bytes());
    // GPS IFD: GPSLatitude, pointing at its own entry as the value.
    t.extend_from_slice(&1u16.to_le_bytes());
    entry(&mut t, 0x0002, 5, 3, gps_ifd.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    t
}

fn segment(marker: u8, data: &[u8]) -> Vec<u8> {
    let mut s = vec![0xFF, marker];
    s.extend_from_slice(&((data.len() + 2) as u16).to_be_bytes());
    s.extend_from_slice(data);
    s
}

/// `PHOTO` with EXIF (location, camera, date, orientation 6), XMP, a
/// comment, and a trailer after the picture's end.
fn jpg_with_everything() -> Vec<u8> {
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend_from_slice(&exif_tiff());
    let mut xmp = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    xmp.extend_from_slice(b"<x:xmpmeta><rdf:Description exif:GPSLatitude=\"6,10.5S\" xmp:CreatorTool=\"Kamera\"/></x:xmpmeta>");
    let mut out = PHOTO[..2].to_vec();
    out.extend(segment(0xE1, &exif));
    out.extend(segment(0xE1, &xmp));
    out.extend(segment(0xFE, b"Dipotret di rumah"));
    out.extend_from_slice(&PHOTO[2..]);
    out.extend_from_slice(b"TRAILER video");
    out
}

#[test]
fn a_jpg_loses_its_info_and_keeps_its_picture_and_orientation() {
    let dir = scratch("info_jpg");
    let original = jpg_with_everything();
    assert_eq!(
        found(&original),
        FoundInfo {
            location: true,
            camera: true,
            other: true
        }
    );
    let input = dir.join("foto.jpg");
    fs::write(&input, &original).unwrap();
    let output = dir.join("foto (cleaned).jpg");

    let cleaned = clean(&input, &output).unwrap();
    assert_eq!(cleaned.format, ImageFormat::Jpeg);
    assert!(cleaned.removed.location && cleaned.removed.camera);

    let result = fs::read(&output).unwrap();
    assert_eq!(cleaned.size, result.len() as u64);
    for gone in [
        &b"Qz7"[..],
        b"2026",
        b"GPSLatitude",
        b"Dipotret",
        b"TRAILER",
    ] {
        assert!(contains(&original, gone) && !contains(PHOTO, gone));
        assert!(
            !contains(&result, gone),
            "{} is still there",
            String::from_utf8_lossy(gone)
        );
    }
    assert!(found(&result).is_clean());
    assert_eq!(orientation(&result), Orientation::Rotate90);
    assert_eq!(pixels(&result), pixels(PHOTO));
    // The compressed picture is copied as it is, not encoded again.
    assert!(contains(&result, &PHOTO[PHOTO.len() / 2..]));
    // The Original is untouched.
    assert_eq!(fs::read(&input).unwrap(), original);
}

#[test]
fn a_photo_with_nothing_to_remove_gets_no_result() {
    let dir = scratch("info_clean");
    for (name, bytes) in [("plain.jpg", PHOTO), ("rotated.jpg", ROTATED)] {
        assert!(found(bytes).is_clean(), "{name}");
        let input = dir.join(name);
        fs::write(&input, bytes).unwrap();
        let output = dir.join(format!("{name} (cleaned).jpg"));
        assert_eq!(clean(&input, &output), Err(ErrorKind::Unchanged), "{name}");
        assert!(!output.exists());
    }
}

/// A PNG chunk with its CRC.
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut c = (data.len() as u32).to_be_bytes().to_vec();
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in &c[4..] {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    c.extend_from_slice(&(!crc).to_be_bytes());
    c
}

#[test]
fn a_png_loses_its_info_and_keeps_its_picture_and_orientation() {
    let dir = scratch("info_png");
    let mut plain = Vec::new();
    RgbImage::from_fn(30, 20, |x, y| image::Rgb([x as u8 * 8, y as u8 * 12, 90]))
        .write_to(&mut Cursor::new(&mut plain), image::ImageFormat::Png)
        .unwrap();
    assert!(found(&plain).is_clean());
    // IHDR is always first, 8 + 25 bytes; the info goes right after it.
    let ihdr_end = 8 + 25;
    let mut original = plain[..ihdr_end].to_vec();
    original.extend(chunk(b"eXIf", &exif_tiff()));
    original.extend(chunk(b"tEXt", b"Software\0Kamera Qz7"));
    original.extend_from_slice(&plain[ihdr_end..]);
    assert_eq!(
        found(&original),
        FoundInfo {
            location: true,
            camera: true,
            other: true
        }
    );

    let input = dir.join("layar.png");
    fs::write(&input, &original).unwrap();
    let output = dir.join("layar (cleaned).png");
    let cleaned = clean(&input, &output).unwrap();
    assert_eq!(cleaned.format, ImageFormat::Png);

    let result = fs::read(&output).unwrap();
    assert!(!contains(&result, b"Qz7") && !contains(&result, b"Software"));
    assert!(found(&result).is_clean());
    assert_eq!(orientation(&result), Orientation::Rotate90);
    assert_eq!(pixels(&result), pixels(&plain));
}

#[test]
fn a_cut_off_photo_is_damaged() {
    let dir = scratch("info_damaged");
    let original = jpg_with_everything();
    let input = dir.join("potong.jpg");
    fs::write(&input, &original[..original.len() / 2]).unwrap();
    assert_eq!(clean(&input, &dir.join("out.jpg")), Err(ErrorKind::Damaged));
    assert_eq!(
        found_info_of(Cursor::new(&original[..10])),
        Err(ErrorKind::Damaged)
    );
}
