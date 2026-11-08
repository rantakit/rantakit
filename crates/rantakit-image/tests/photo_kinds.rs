//! HEIC and WebP (O1): read like any photo, saved as JPG unless PNG is
//! chosen, and every Result keeps the Original's color profile.
//!
//! WebP files are made here. HEIC can't be made without an HEVC encoder,
//! so HEIC is checked on real photos in `tests/local/`, which git ignores,
//! when there are any.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::webp::WebPEncoder;
use image::{
    DynamicImage, ImageDecoder, ImageEncoder, ImageReader, Rgb, RgbImage, Rgba, RgbaImage,
};
use rantakit_image::{
    CancelToken, ErrorKind, ImageFormat, ImageOptions, OutputFormat, PhotoKind, Target, WHITE,
    compress_image, detect_kind, found_info_of, photo_info, remove_info, thumbnail,
};

/// Stands in for a real profile such as Display P3: only its bytes matter here.
const PROFILE: &[u8] = b"rantakit test color profile, not a real ICC";

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-image-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn opts(format: OutputFormat) -> ImageOptions {
    ImageOptions {
        format,
        target: Target::Quality(85),
        max_dimension: None,
        background: WHITE,
    }
}

/// A 40×20 picture, red on the left half and blue on the right.
fn halves() -> RgbImage {
    RgbImage::from_fn(40, 20, |x, _| {
        if x < 20 {
            Rgb([200, 30, 30])
        } else {
            Rgb([30, 30, 200])
        }
    })
}

fn webp(img: &DynamicImage, exif: Option<Vec<u8>>, icc: Option<&[u8]>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = WebPEncoder::new_lossless(&mut bytes);
    if let Some(exif) = exif {
        encoder.set_exif_metadata(exif).unwrap();
    }
    if let Some(icc) = icc {
        encoder.set_icc_profile(icc.to_vec()).unwrap();
    }
    encoder
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            img.color().into(),
        )
        .unwrap();
    bytes
}

/// A little-endian EXIF block: Make "Qz7", Orientation 6 (turn right to
/// show it upright), and a latitude in the GPS IFD.
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
    // IFD0 at 8: 3 entries, then the next-IFD offset; ends at 8 + 2 + 36 + 4 = 50.
    let gps_ifd = 50u32;
    t.extend_from_slice(&3u16.to_le_bytes());
    entry(&mut t, 0x010F, 2, 4, *b"Qz7\0");
    entry(&mut t, 0x0112, 3, 1, [6, 0, 0, 0]);
    entry(&mut t, 0x8825, 4, 1, gps_ifd.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(&1u16.to_le_bytes());
    entry(&mut t, 0x0002, 5, 3, gps_ifd.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    t
}

fn profile_of(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    decoder.icc_profile().unwrap()
}

fn contains(bytes: &[u8], text: &[u8]) -> bool {
    bytes.windows(text.len()).any(|w| w == text)
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn webp_is_read_by_its_content() {
    let dir = scratch("webp-kind");
    // Named like a JPG: the content decides.
    let path = write(&dir, "sticker.jpg", &webp(&halves().into(), None, None));
    assert_eq!(detect_kind(&path), Ok(PhotoKind::Webp));
    let info = photo_info(&path).unwrap();
    assert_eq!(
        (info.kind, info.width, info.height),
        (PhotoKind::Webp, 40, 20)
    );
    assert!(image::load_from_memory(&thumbnail(&path, 16).unwrap()).is_ok());
    assert_eq!(PhotoKind::Webp.saved_as(), ImageFormat::Jpeg);
    assert_eq!(PhotoKind::Heic.saved_as(), ImageFormat::Jpeg);
    assert_eq!(PhotoKind::Png.saved_as(), ImageFormat::Png);
}

#[test]
fn webp_becomes_an_upright_jpg_with_its_profile() {
    let dir = scratch("webp-compress");
    let original = write(
        &dir,
        "photo.webp",
        &webp(&halves().into(), Some(exif_tiff()), Some(PROFILE)),
    );
    let output = dir.join("photo (compressed).jpg");
    let out = compress_image(
        &original,
        &output,
        &opts(OutputFormat::Same),
        &mut |_| {},
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(out.format, ImageFormat::Jpeg);
    // Orientation 6 turns the 40×20 picture into a 20×40 one, red on top.
    assert_eq!((out.width, out.height), (20, 40));
    let bytes = fs::read(&output).unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert!(img.get_pixel(10, 5)[0] > 150, "red half on top");
    assert_eq!(profile_of(&bytes).as_deref(), Some(PROFILE));

    let png = dir.join("photo (converted).png");
    let out = compress_image(
        &original,
        &png,
        &opts(OutputFormat::To(ImageFormat::Png)),
        &mut |_| {},
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(out.format, ImageFormat::Png);
    assert_eq!(
        profile_of(&fs::read(&png).unwrap()).as_deref(),
        Some(PROFILE)
    );
}

#[test]
fn transparent_webp_goes_on_white_as_a_jpg() {
    let dir = scratch("webp-transparent");
    let sticker = RgbaImage::from_fn(20, 20, |x, _| {
        if x < 10 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([0, 120, 0, 255])
        }
    });
    let original = write(&dir, "sticker.webp", &webp(&sticker.into(), None, None));
    let output = dir.join("sticker (compressed).jpg");
    compress_image(
        &original,
        &output,
        &opts(OutputFormat::Same),
        &mut |_| {},
        &CancelToken::new(),
    )
    .unwrap();
    let img = image::load_from_memory(&fs::read(&output).unwrap())
        .unwrap()
        .to_rgb8();
    assert!(
        img.get_pixel(2, 10).0.iter().all(|&c| c > 240),
        "transparent part is white"
    );
}

#[test]
fn a_jpg_keeps_its_profile() {
    let dir = scratch("jpg-profile");
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
    encoder.set_icc_profile(PROFILE.to_vec()).unwrap();
    let img = halves();
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    let original = write(&dir, "photo.jpg", &bytes);

    let output = dir.join("photo (compressed).jpg");
    let limit = ImageOptions {
        target: Target::MaxBytes(100_000),
        ..opts(OutputFormat::Same)
    };
    compress_image(&original, &output, &limit, &mut |_| {}, &CancelToken::new()).unwrap();
    assert_eq!(
        profile_of(&fs::read(&output).unwrap()).as_deref(),
        Some(PROFILE)
    );
}

#[test]
fn hapus_info_lokasi_remakes_webp_as_a_jpg_without_info() {
    let dir = scratch("webp-info");
    let located = webp(&halves().into(), Some(exif_tiff()), Some(PROFILE));
    let found = found_info_of(Cursor::new(&located)).unwrap();
    assert!(found.location && found.camera);

    let original = write(&dir, "photo.webp", &located);
    let output = dir.join("photo (cleaned).jpg");
    let cleaned = remove_info(&original, &output, &mut |_| {}, &CancelToken::new()).unwrap();
    assert_eq!(cleaned.format, ImageFormat::Jpeg);
    assert!(cleaned.removed.location);
    let bytes = fs::read(&output).unwrap();
    assert!(!contains(&bytes, b"Qz7"));
    assert!(!contains(&bytes, b"Exif\0\0"));
    let img = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        (img.width(), img.height()),
        (20, 40),
        "turned upright first"
    );
    assert_eq!(profile_of(&bytes).as_deref(), Some(PROFILE));

    // Nothing to remove: no Result.
    let plain = write(
        &dir,
        "plain.webp",
        &webp(&halves().into(), None, Some(PROFILE)),
    );
    assert!(
        found_info_of(Cursor::new(fs::read(&plain).unwrap()))
            .unwrap()
            .is_clean()
    );
    let none = dir.join("plain (cleaned).jpg");
    assert_eq!(
        remove_info(&plain, &none, &mut |_| {}, &CancelToken::new()),
        Err(ErrorKind::Unchanged)
    );
    assert!(!none.exists());
}

#[test]
fn damaged_webp_and_heic_are_damaged() {
    let dir = scratch("damaged-kinds");
    let mut cut = webp(&halves().into(), None, None);
    cut.truncate(30);
    let webp = write(&dir, "cut.webp", &cut);
    let heic = write(&dir, "cut.heic", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic");
    for path in [webp, heic] {
        let output = dir.join("out.jpg");
        let err = compress_image(
            &path,
            &output,
            &opts(OutputFormat::Same),
            &mut |_| {},
            &CancelToken::new(),
        )
        .unwrap_err();
        assert!(
            matches!(err, ErrorKind::Damaged | ErrorKind::UnsupportedFormat),
            "{err:?}"
        );
        assert!(!output.exists());
    }
}

/// Real iPhone photos, when someone has put some in `tests/local/`.
#[test]
fn local_heic_photos_open_upright_and_keep_their_profile() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("local");
    let Ok(entries) = fs::read_dir(&local) else {
        return;
    };
    let dir = scratch("local-heic");
    for entry in entries.flatten() {
        let path = entry.path();
        let is_heic = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("heic") || e.eq_ignore_ascii_case("heif"));
        if !is_heic {
            continue;
        }
        assert_eq!(
            detect_kind(&path),
            Ok(PhotoKind::Heic),
            "{}",
            path.display()
        );
        let info = photo_info(&path).unwrap();
        let output = dir.join("out.jpg");
        let _ = fs::remove_file(&output);
        let out = compress_image(
            &path,
            &output,
            &opts(OutputFormat::Same),
            &mut |_| {},
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(
            (out.width, out.height),
            (info.width, info.height),
            "{}",
            path.display()
        );
        assert!(thumbnail(&path, 96).is_ok());
        let found = found_info_of(std::io::BufReader::new(fs::File::open(&path).unwrap())).unwrap();
        eprintln!(
            "{}: {}x{}, found {found:?}, profile {:?} bytes",
            path.display(),
            info.width,
            info.height,
            profile_of(&fs::read(&output).unwrap()).map(|p| p.len())
        );
    }
}
