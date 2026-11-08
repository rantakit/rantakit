use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rantakit_core::{CancelToken, ErrorKind};
use rantakit_office::{
    CompressedDocument, DocumentInfo, OfficeKind, OfficeOptions, Sniffed, compress_document, document_info, sniff,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/><Default Extension="mp4" ContentType="video/mp4"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOCUMENT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.jpeg"/><Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image3.png"/><Relationship Id="rId7" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/video" Target="media/media1.mp4"/></Relationships>"#;

fn document_xml() -> String {
    let paragraphs: String = (1..=200)
        .map(|n| format!("<w:p><w:r><w:t>Paragraf {n} dari laporan kegiatan bulanan.</w:t></w:r></w:p>"))
        .collect();
    format!(r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>{paragraphs}</w:body></w:document>"#)
}

/// A noisy picture, like a phone photo, that compresses badly as PNG.
fn noisy(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        let n = (x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)).wrapping_mul(2_654_435_761);
        let base = ((x + y) % 256) as u8;
        Rgb([base.wrapping_add((n >> 27) as u8), base / 2 + (n >> 28) as u8, 200 - (n >> 28) as u8])
    })
}

fn encoded(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    match format {
        ImageFormat::Jpeg => {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 95)
                .encode_image(&image)
                .unwrap();
        }
        _ => image.write_to(&mut Cursor::new(&mut bytes), format).unwrap(),
    }
    bytes
}

/// A pasted photo stored as PNG.
fn photo_png() -> Vec<u8> {
    encoded(DynamicImage::ImageRgb8(noisy(600, 450)), ImageFormat::Png)
}

/// A big phone photo.
fn photo_jpeg() -> Vec<u8> {
    encoded(DynamicImage::ImageRgb8(noisy(1600, 1200)), ImageFormat::Jpeg)
}

/// A logo with a transparent background.
fn logo_png() -> Vec<u8> {
    let logo = RgbaImage::from_fn(300, 120, |x, y| {
        if (x / 20 + y / 20) % 2 == 0 { Rgba([200, 30, 30, 255]) } else { Rgba([0, 0, 0, 0]) }
    });
    encoded(DynamicImage::ImageRgba8(logo), ImageFormat::Png)
}

/// The bytes of a ZIP with `parts`, each deflated at the strongest level
/// unless it is stored.
fn zip_of(parts: &[(&str, Vec<u8>, bool)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes, stored) in parts {
        let options = if *stored {
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
        } else {
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .compression_level(Some(9))
        };
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn report_docx() -> Vec<u8> {
    zip_of(&[
        ("[Content_Types].xml", CONTENT_TYPES.into(), false),
        ("_rels/.rels", ROOT_RELS.into(), false),
        ("word/document.xml", document_xml().into_bytes(), false),
        ("word/_rels/document.xml.rels", DOCUMENT_RELS.into(), false),
        ("word/media/image1.png", photo_png(), false),
        ("word/media/image2.jpeg", photo_jpeg(), false),
        ("word/media/image3.png", logo_png(), false),
        ("word/media/media1.mp4", b"\0\0\0\x18ftypmp42 not really a video".to_vec(), true),
    ])
}

fn text_only_docx() -> Vec<u8> {
    zip_of(&[
        ("[Content_Types].xml", CONTENT_TYPES.into(), false),
        ("_rels/.rels", ROOT_RELS.into(), false),
        ("word/document.xml", document_xml().into_bytes(), false),
    ])
}

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-office-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn compress(original: &Path, output: &Path, opts: OfficeOptions) -> Result<CompressedDocument, ErrorKind> {
    compress_document(original, output, &opts, &mut |_| {}, &CancelToken::new())
}

fn at_quality(quality: u8, max_dimension: Option<u32>) -> OfficeOptions {
    OfficeOptions {
        quality,
        max_dimension,
        max_bytes: None,
    }
}

fn read_part(zip: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    Some(bytes)
}

fn names(zip: &[u8]) -> Vec<String> {
    let archive = ZipArchive::new(Cursor::new(zip)).unwrap();
    archive.file_names().map(|name| name.unwrap().into_owned()).collect()
}

#[test]
fn pictures_shrink_and_everything_else_stays() {
    let dir = scratch("shrink");
    let docx = report_docx();
    let original = dir.join("laporan.docx");
    fs::write(&original, &docx).unwrap();
    let output = dir.join("laporan (compressed).docx");

    let result = compress(&original, &output, at_quality(70, Some(800))).unwrap();
    assert_eq!(fs::read(&original).unwrap(), docx, "the Original is untouched");
    let out = fs::read(&output).unwrap();
    assert_eq!(result.size, out.len() as u64);
    assert!(out.len() * 2 < docx.len(), "{} → {}", docx.len(), out.len());
    assert_eq!(result.pictures, 3);
    assert_eq!(result.pictures_recompressed, 2, "the photos, not the logo");

    // The pasted photo became a JPG under a new name, and every reference followed.
    assert_eq!(
        names(&out),
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "word/media/image1.jpeg",
            "word/media/image2.jpeg",
            "word/media/image3.png",
            "word/media/media1.mp4",
        ]
    );
    let rels = String::from_utf8(read_part(&out, "word/_rels/document.xml.rels").unwrap()).unwrap();
    assert!(rels.contains(r#"Target="media/image1.jpeg""#), "{rels}");
    assert!(!rels.contains("image1.png"));
    assert_eq!(
        read_part(&out, "[Content_Types].xml").unwrap(),
        CONTENT_TYPES.as_bytes(),
        "jpeg is already typed"
    );

    let photo = image::load_from_memory(&read_part(&out, "word/media/image2.jpeg").unwrap()).unwrap();
    assert_eq!((photo.width(), photo.height()), (800, 600));
    let pasted = image::load_from_memory(&read_part(&out, "word/media/image1.jpeg").unwrap()).unwrap();
    assert_eq!((pasted.width(), pasted.height()), (600, 450), "smaller than the side already");

    for part in ["word/document.xml", "word/media/image3.png", "word/media/media1.mp4", "_rels/.rels"] {
        assert_eq!(read_part(&out, part), read_part(&docx, part), "{part} is kept");
    }
}

#[test]
fn a_size_limit_finds_a_quality_that_fits() {
    let dir = scratch("limit");
    let docx = report_docx();
    let original = dir.join("laporan.docx");
    fs::write(&original, &docx).unwrap();

    let limit = docx.len() as u64 / 2;
    let output = dir.join("fits.docx");
    let result = compress(
        &original,
        &output,
        OfficeOptions {
            quality: 90,
            max_dimension: None,
            max_bytes: Some(limit),
        },
    )
    .unwrap();
    assert!(result.size <= limit, "{}", result.size);
    assert_eq!(fs::metadata(&output).unwrap().len(), result.size);

    let too_small = compress(
        &original,
        &dir.join("never.docx"),
        OfficeOptions {
            quality: 90,
            max_dimension: None,
            max_bytes: Some(5_000),
        },
    );
    match too_small {
        Err(ErrorKind::SizeLimitUnreachable { best }) => assert!(best > 5_000 && best < limit, "{best}"),
        other => panic!("expected the smallest size reached, got {other:?}"),
    }
    assert!(!dir.join("never.docx").exists());

    let already = compress(
        &original,
        &dir.join("already.docx"),
        OfficeOptions {
            quality: 90,
            max_dimension: None,
            max_bytes: Some(docx.len() as u64),
        },
    );
    assert_eq!(already, Err(ErrorKind::Unchanged));
}

#[test]
fn a_document_without_pictures_is_left_unchanged() {
    let dir = scratch("text_only");
    let original = dir.join("surat.docx");
    fs::write(&original, text_only_docx()).unwrap();
    let output = dir.join("surat (compressed).docx");
    assert_eq!(compress(&original, &output, at_quality(70, None)), Err(ErrorKind::Unchanged));
    assert!(!output.exists());
}

#[test]
fn documents_are_told_apart_by_content() {
    let dir = scratch("info");
    let docx = dir.join("laporan.docx");
    fs::write(&docx, report_docx()).unwrap();
    assert_eq!(
        document_info(&docx),
        Ok(DocumentInfo {
            kind: OfficeKind::Word,
            pictures: 3
        })
    );
    assert_eq!(sniff(&mut fs::File::open(&docx).unwrap()), Some(Sniffed::Office(OfficeKind::Word)));

    let slides = dir.join("presentasi.pptx");
    fs::write(
        &slides,
        zip_of(&[
            ("[Content_Types].xml", CONTENT_TYPES.into(), false),
            ("ppt/presentation.xml", b"<p:presentation/>".to_vec(), false),
        ]),
    )
    .unwrap();
    assert_eq!(
        document_info(&slides),
        Ok(DocumentInfo {
            kind: OfficeKind::PowerPoint,
            pictures: 0
        })
    );

    let other_zip = dir.join("arsip.zip");
    fs::write(&other_zip, zip_of(&[("catatan.txt", b"halo".to_vec(), false)])).unwrap();
    assert_eq!(document_info(&other_zip), Err(ErrorKind::UnsupportedFormat));
}

#[test]
fn locked_and_old_documents_get_their_own_errors() {
    let dir = scratch("locked");
    let compound = |name: &str| {
        let mut bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        bytes.extend(vec![0; 2000]);
        bytes.extend(name.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    };
    let locked = dir.join("rahasia.docx");
    fs::write(&locked, compound("EncryptedPackage")).unwrap();
    let old = dir.join("lama.doc");
    fs::write(&old, compound("WordDocument")).unwrap();
    let broken = dir.join("rusak.docx");
    fs::write(&broken, b"PK\x03\x04 cut short").unwrap();

    let out = dir.join("out.docx");
    assert_eq!(compress(&locked, &out, at_quality(70, None)), Err(ErrorKind::PasswordProtected));
    assert_eq!(compress(&old, &out, at_quality(70, None)), Err(ErrorKind::UnsupportedFormat));
    assert!(compress(&broken, &out, at_quality(70, None)).is_err());
    assert!(!out.exists());
}
