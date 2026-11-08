use std::fs;
use std::path::{Path, PathBuf};

use rantakit_image::{
    CancelToken, Compressed, ErrorKind, ImageFormat, ImageOptions, OutputFormat, Progress, Target,
    PhotoKind, WHITE, compress_image, detect_kind,
};

const PHOTO: &[u8] = include_bytes!("fixtures/photo.jpg");
const TRANSPARENT: &[u8] = include_bytes!("fixtures/transparent.png");
const ROTATED: &[u8] = include_bytes!("fixtures/rotated_exif6.jpg");

fn opts(format: OutputFormat, quality: u8, max_dimension: Option<u32>) -> ImageOptions {
    ImageOptions {
        format,
        target: Target::Quality(quality),
        max_dimension,
        background: WHITE,
    }
}

fn max_bytes(format: OutputFormat, max: u64) -> ImageOptions {
    ImageOptions {
        format,
        target: Target::MaxBytes(max),
        max_dimension: None,
        background: WHITE,
    }
}

/// A fresh, empty folder for one test.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-image-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn file_count(dir: &Path) -> usize {
    fs::read_dir(dir).unwrap().count()
}

/// Saves `input` as `name`, runs the engine on it, and checks that the
/// Original is untouched and that nothing but a finished Result was added.
fn run(
    test: &str,
    name: &str,
    input: &[u8],
    opts: &ImageOptions,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> (rantakit_image::Result<Compressed>, Option<Vec<u8>>) {
    let dir = scratch(test);
    let original = dir.join(name);
    fs::write(&original, input).unwrap();
    let output = dir.join("result");

    let outcome = compress_image(&original, &output, opts, progress, cancel);

    assert_eq!(
        fs::read(&original).unwrap(),
        input,
        "the Original must stay untouched"
    );
    let result = fs::read(&output).ok();
    assert_eq!(
        file_count(&dir),
        1 + result.is_some() as usize,
        "nothing but the Result may be left behind"
    );
    (outcome, result)
}

/// A run that must succeed; returns what the engine reported and the Result.
fn compress(test: &str, name: &str, input: &[u8], opts: &ImageOptions) -> (Compressed, Vec<u8>) {
    let (outcome, result) = run(test, name, input, opts, &mut |_| {}, &CancelToken::new());
    let out = outcome.unwrap();
    let bytes = result.expect("a Result should be written");
    assert_eq!(out.size, bytes.len() as u64);
    (out, bytes)
}

/// A run that must fail; returns its error kind.
fn fail(test: &str, input: &[u8], opts: &ImageOptions) -> ErrorKind {
    let (outcome, result) = run(
        test,
        "photo.jpg",
        input,
        opts,
        &mut |_| {},
        &CancelToken::new(),
    );
    assert!(result.is_none(), "a failed file must not leave a Result");
    outcome.unwrap_err()
}

#[test]
fn jpeg_quality_produces_smaller_decodable_jpeg() {
    let (out, bytes) = compress(
        "jpeg_quality",
        "photo.jpg",
        PHOTO,
        &opts(OutputFormat::Same, 70, None),
    );

    assert_eq!(out.format, ImageFormat::Jpeg);
    assert!(!bytes.is_empty());
    assert!(
        bytes.len() < PHOTO.len(),
        "output must be smaller than input"
    );

    let decoded = ::image::load_from_memory(&bytes).unwrap();
    assert_eq!(decoded.width(), out.width);
    assert_eq!(decoded.height(), out.height);
}

#[test]
fn max_dimension_limits_longest_side_and_keeps_ratio() {
    let (out, _) = compress(
        "max_dimension",
        "photo.jpg",
        PHOTO,
        &opts(OutputFormat::Same, 80, Some(800)),
    );

    // original 2000x1500 (4:3) -> 800x600
    assert_eq!((out.width, out.height), (800, 600));
}

#[test]
fn small_image_is_not_upscaled() {
    let (out, _) = compress(
        "not_upscaled",
        "transparent.png",
        TRANSPARENT,
        &opts(OutputFormat::Same, 80, Some(4000)),
    );

    assert_eq!((out.width, out.height), (512, 512));
}

#[test]
fn exif_orientation_is_applied() {
    let (out, bytes) = compress(
        "exif_orientation",
        "rotated.jpg",
        ROTATED,
        &opts(OutputFormat::Same, 90, None),
    );

    // stored as 300x200 with orientation 6 -> displayed as 200x300
    assert_eq!((out.width, out.height), (200, 300));

    // the red square moves from top-left to top-right
    let img = ::image::load_from_memory(&bytes).unwrap().to_rgb8();
    let [r, g, b] = img.get_pixel(out.width - 5, 5).0;
    assert!(
        r > 180 && g < 80 && b < 80,
        "top-right corner should be red, got {:?}",
        (r, g, b)
    );
}

#[test]
fn transparent_png_to_jpeg_gets_white_background() {
    let (out, bytes) = compress(
        "transparent_to_jpeg",
        "transparent.png",
        TRANSPARENT,
        &opts(OutputFormat::To(ImageFormat::Jpeg), 90, None),
    );

    assert_eq!(out.format, ImageFormat::Jpeg);

    let img = ::image::load_from_memory(&bytes).unwrap().to_rgb8();
    let [r, g, b] = img.get_pixel(2, 2).0;
    // JPEG is lossy: allow a small tolerance instead of exact 255
    assert!(
        r > 245 && g > 245 && b > 245,
        "corner should be white, got {:?}",
        (r, g, b)
    );
}

#[test]
fn png_stays_png_and_reports_no_quality() {
    let (out, _) = compress(
        "png_stays_png",
        "transparent.png",
        TRANSPARENT,
        &opts(OutputFormat::Same, 70, None),
    );

    assert_eq!(out.format, ImageFormat::Png);
    assert_eq!(
        out.quality, None,
        "PNG is lossless and has no quality setting"
    );
}

#[test]
fn quality_is_clamped_and_reported() {
    let (high, _) = compress(
        "quality_high",
        "photo.jpg",
        PHOTO,
        &opts(OutputFormat::Same, 150, Some(400)),
    );
    assert_eq!(high.quality, Some(100));

    let (zero, _) = compress(
        "quality_zero",
        "photo.jpg",
        PHOTO,
        &opts(OutputFormat::Same, 0, Some(400)),
    );
    assert_eq!(zero.quality, Some(1));
}

#[test]
fn format_comes_from_content_not_extension() {
    let (out, _) = compress(
        "content_not_extension",
        "photo.png",
        PHOTO,
        &opts(OutputFormat::Same, 80, Some(400)),
    );
    assert_eq!(out.format, ImageFormat::Jpeg);

    let dir = scratch("detect_format");
    let path = dir.join("photo.png");
    fs::write(&path, PHOTO).unwrap();
    assert_eq!(detect_kind(&path), Ok(PhotoKind::Jpeg));
}

#[test]
fn max_bytes_output_fits_target() {
    let (out, bytes) = compress(
        "max_bytes_fits",
        "photo.jpg",
        PHOTO,
        &max_bytes(OutputFormat::Same, 200_000),
    );

    assert!(bytes.len() <= 200_000, "got {} bytes", bytes.len());
    assert!(out.quality.is_some());
}

#[test]
fn generous_target_uses_max_quality_and_full_size() {
    let (out, _) = compress(
        "generous_target",
        "photo.jpg",
        PHOTO,
        &max_bytes(OutputFormat::Same, 50_000_000),
    );

    assert_eq!(out.quality, Some(95));
    assert_eq!((out.width, out.height), (2000, 1500));
}

#[test]
fn png_target_forces_downscale() {
    // A noisy photo saved as PNG is ~6.8 MB at full size, so 2 MB requires downscaling.
    let (out, bytes) = compress(
        "png_target",
        "photo.jpg",
        PHOTO,
        &max_bytes(OutputFormat::To(ImageFormat::Png), 2_000_000),
    );

    assert_eq!(out.format, ImageFormat::Png);
    assert!(bytes.len() <= 2_000_000, "got {} bytes", bytes.len());
    assert!(out.width < 2000, "PNG can only shrink by downscaling");
    assert_eq!(out.quality, None);
}

#[test]
fn impossible_target_reports_smallest_size() {
    match fail(
        "impossible_target",
        PHOTO,
        &max_bytes(OutputFormat::Same, 1_000),
    ) {
        ErrorKind::SizeLimitUnreachable { best } => assert!(best > 1_000),
        other => panic!("expected SizeLimitUnreachable, got {other:?}"),
    }
}

#[test]
fn flat_png_below_minimum_is_unreachable() {
    let kind = fail(
        "flat_png",
        TRANSPARENT,
        &max_bytes(OutputFormat::Same, 3_000),
    );

    assert!(matches!(kind, ErrorKind::SizeLimitUnreachable { .. }));
}

#[test]
fn non_image_input_is_unsupported() {
    let kind = fail(
        "non_image",
        b"this is not an image",
        &opts(OutputFormat::Same, 80, None),
    );

    assert_eq!(kind, ErrorKind::UnsupportedFormat);
}

#[test]
fn broken_jpeg_is_damaged() {
    let mut broken = PHOTO[..4].to_vec();
    broken.extend_from_slice(&[0x55; 64]);

    let kind = fail("broken_jpeg", &broken, &opts(OutputFormat::Same, 80, None));

    assert_eq!(kind, ErrorKind::Damaged);
}

#[test]
fn missing_original_is_reported() {
    let dir = scratch("missing_original");

    let outcome = compress_image(
        &dir.join("gone.jpg"),
        &dir.join("result.jpg"),
        &opts(OutputFormat::Same, 80, None),
        &mut |_| {},
        &CancelToken::new(),
    );

    assert_eq!(outcome.unwrap_err(), ErrorKind::OriginalMissing);
    assert_eq!(file_count(&dir), 0);
}

#[test]
fn existing_output_is_never_overwritten() {
    let dir = scratch("existing_output");
    let original = dir.join("photo.jpg");
    let output = dir.join("photo (compressed).jpg");
    fs::write(&original, PHOTO).unwrap();
    fs::write(&output, b"keep me").unwrap();

    let outcome = compress_image(
        &original,
        &output,
        &opts(OutputFormat::Same, 80, Some(400)),
        &mut |_| {},
        &CancelToken::new(),
    );

    assert_eq!(outcome.unwrap_err(), ErrorKind::CannotWrite);
    assert_eq!(fs::read(&output).unwrap(), b"keep me");
    assert_eq!(file_count(&dir), 2);
}

#[test]
fn progress_goes_up_and_ends_at_one() {
    let mut seen = Vec::new();
    let (outcome, _) = run(
        "progress",
        "photo.jpg",
        PHOTO,
        &max_bytes(OutputFormat::Same, 200_000),
        &mut |p| seen.push(p),
        &CancelToken::new(),
    );
    outcome.unwrap();

    let fractions: Vec<f32> = seen
        .iter()
        .map(|p| match p {
            Progress::Fraction(f) => *f,
            other => panic!("expected a fraction, got {other:?}"),
        })
        .collect();
    assert!(
        fractions.len() > 5,
        "the size search should report each step"
    );
    assert!(
        fractions.windows(2).all(|w| w[0] <= w[1]),
        "got {fractions:?}"
    );
    assert_eq!(fractions.last(), Some(&1.0));
}

#[test]
fn cancel_before_start_leaves_nothing() {
    let cancel = CancelToken::new();
    cancel.cancel();

    let (outcome, result) = run(
        "cancel_before_start",
        "photo.jpg",
        PHOTO,
        &opts(OutputFormat::Same, 80, None),
        &mut |_| {},
        &cancel,
    );

    assert_eq!(outcome.unwrap_err(), ErrorKind::Cancelled);
    assert!(result.is_none());
}

#[test]
fn cancel_during_size_search_stops_it() {
    let cancel = CancelToken::new();
    let mut steps_after_cancel = 0;
    let mut progress = |p| {
        if cancel.is_cancelled() {
            steps_after_cancel += 1;
        }
        // Cancel in the middle of the size search.
        if matches!(p, Progress::Fraction(f) if f > 0.5) {
            cancel.cancel();
        }
    };

    let (outcome, result) = run(
        "cancel_during_search",
        "photo.jpg",
        PHOTO,
        &max_bytes(OutputFormat::Same, 200_000),
        &mut progress,
        &cancel,
    );

    assert_eq!(outcome.unwrap_err(), ErrorKind::Cancelled);
    assert!(result.is_none());
    assert_eq!(steps_after_cancel, 0, "no step may run after cancelling");
}

#[test]
fn photo_info_reads_the_header() {
    let dir = scratch("photo-info");
    let path = dir.join("photo.jpg");
    fs::write(&path, PHOTO).unwrap();

    let info = rantakit_image::photo_info(&path).unwrap();
    assert_eq!(info.kind, PhotoKind::Jpeg);
    assert!(info.width > 0 && info.height > 0);
}

#[test]
fn thumbnails_are_small_jpgs() {
    let dir = scratch("thumbnail");
    let path = dir.join("transparent.png");
    fs::write(&path, TRANSPARENT).unwrap();

    let bytes = rantakit_image::thumbnail(&path, 96).unwrap();
    let thumb = ::image::load_from_memory(&bytes).unwrap();
    assert!(thumb.width().max(thumb.height()) <= 96);
    assert_eq!(detect_format_of(&bytes), ImageFormat::Jpeg);
}

fn detect_format_of(bytes: &[u8]) -> ImageFormat {
    match ::image::guess_format(bytes).unwrap() {
        ::image::ImageFormat::Jpeg => ImageFormat::Jpeg,
        _ => ImageFormat::Png,
    }
}

#[test]
fn transparency_becomes_the_chosen_background_in_a_jpg() {
    let dir = scratch("background");
    let input = dir.join("logo.png");
    fs::write(&input, TRANSPARENT).unwrap();
    let output = dir.join("logo.jpg");
    let black = ImageOptions {
        background: [0, 0, 0],
        ..opts(OutputFormat::To(ImageFormat::Jpeg), 95, None)
    };

    compress_image(&input, &output, &black, &mut |_| {}, &CancelToken::new()).unwrap();

    let corner = ::image::open(&output).unwrap().to_rgb8().get_pixel(0, 0).0;
    assert!(
        corner.iter().all(|&c| c < 40),
        "expected a dark corner, got {corner:?}"
    );
}
