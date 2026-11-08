mod clean;
mod encode;
mod info;
pub mod paper;
pub mod options;
mod preview;
mod resize;
mod target_size;
pub mod watermark;

use std::io::{Cursor, Write};
use std::path::Path;

use ::image::{DynamicImage, ImageDecoder, ImageError, ImageFormat as RawFormat, ImageReader};
use rantakit_core::{Tracker, read_original, write_result};

pub use options::{Compressed, ImageFormat, ImageOptions, OutputFormat, Target, WHITE};
pub use watermark::{FontMetrics, Placement, PlacedLine, Watermark, font_metrics, shown, watermark_image, watermark_preview};
pub use info::{Cleaned, FoundInfo, found_info_of, remove_info};
pub use preview::{PhotoInfo, photo_info, photo_info_of, thumbnail};
pub use paper::{Corners, PagePhoto, PhotoEdit, edited_preview, find_paper, find_paper_in, page_photo};
pub use rantakit_core::{CancelToken, ErrorKind, Progress, Result};

/// Progress once the Original is read, decoded, resized, and encoded.
const READ: f32 = 0.1;
const DECODED: f32 = 0.3;
const RESIZED: f32 = 0.4;
const ENCODED: f32 = 0.9;

/// Compresses the photo at `input` and writes the Result to `output`, which
/// must not exist yet.
pub fn compress_image(
    input: &Path,
    output: &Path,
    opts: &ImageOptions,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> Result<Compressed> {
    let mut tracker = Tracker::new(progress, cancel);

    let original = read_original(input)?;
    tracker.step(READ)?;

    let (img, input_format) = decode(&original)?;
    drop(original);
    tracker.step(DECODED)?;

    let format = match opts.format {
        OutputFormat::Same => input_format,
        OutputFormat::To(f) => f,
    };

    let img = match opts.max_dimension {
        Some(max) => resize::fit_within(img, max),
        None => img,
    };
    let img = if format == ImageFormat::Jpeg && img.color().has_alpha() {
        DynamicImage::ImageRgb8(encode::flatten_on(&img, opts.background))
    } else {
        img
    };
    tracker.step(RESIZED)?;

    let fitted = match opts.target {
        Target::Quality(q) => target_size::Fitted {
            bytes: encode::encode(&img, format, q)?,
            width: img.width(),
            height: img.height(),
            quality: match format {
                ImageFormat::Jpeg => Some(encode::clamp_quality(q)),
                ImageFormat::Png => None,
            },
        },
        Target::MaxBytes(max) => target_size::compress_to_size(&img, format, max, &mut tracker)?,
    };
    tracker.step(ENCODED)?;

    let size = write_result(output, &mut tracker, |file| file.write_all(&fitted.bytes))?;

    Ok(Compressed {
        size,
        format,
        width: fitted.width,
        height: fitted.height,
        quality: fitted.quality,
    })
}

/// The format of the photo at `path`, from its content, not its extension.
pub fn detect_format(path: &Path) -> Result<ImageFormat> {
    let reader = ImageReader::open(path)
        .map_err(|e| ErrorKind::reading(&e))?
        .with_guessed_format()
        .map_err(|e| ErrorKind::reading(&e))?;

    format_of(reader.format())
}

pub(crate) fn decode(input: &[u8]) -> Result<(DynamicImage, ImageFormat)> {
    let reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(|e| ErrorKind::reading(&e))?;

    let format = format_of(reader.format())?;

    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    let orientation = decoder.orientation().map_err(decode_error)?;

    let mut img = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    img.apply_orientation(orientation);

    Ok((img, format))
}

pub(crate) fn format_of(raw: Option<RawFormat>) -> Result<ImageFormat> {
    match raw {
        Some(RawFormat::Jpeg) => Ok(ImageFormat::Jpeg),
        Some(RawFormat::Png) => Ok(ImageFormat::Png),
        _ => Err(ErrorKind::UnsupportedFormat),
    }
}

fn decode_error(err: ImageError) -> ErrorKind {
    match err {
        ImageError::Unsupported(_) => ErrorKind::UnsupportedFormat,
        _ => ErrorKind::Damaged,
    }
}
