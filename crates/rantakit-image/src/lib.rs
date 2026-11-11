mod clean;
mod encode;
mod info;
#[cfg(feature = "ml")]
mod ml;
mod photo;
pub mod paper;
pub mod options;
mod preview;
mod resize;
mod target_size;
pub mod watermark;

use std::io::{Read, Write};
use std::path::Path;

use ::image::DynamicImage;
use rantakit_core::{Tracker, read_original, write_result};

pub use options::{Compressed, ImageFormat, ImageOptions, OutputFormat, Target, WHITE};
pub use photo::PhotoKind;
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

    let Decoded { img, kind, icc } = decode(&original)?;
    drop(original);
    tracker.step(DECODED)?;

    let format = match opts.format {
        OutputFormat::Same => kind.saved_as(),
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
            bytes: encode::encode(&img, format, q, icc.as_deref())?,
            width: img.width(),
            height: img.height(),
            quality: match format {
                ImageFormat::Jpeg => Some(encode::clamp_quality(q)),
                ImageFormat::Png => None,
            },
        },
        Target::MaxBytes(max) => target_size::compress_to_size(&img, format, max, icc.as_deref(), &mut tracker)?,
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

/// The kind of photo at `path`, from its content, not its extension.
pub fn detect_kind(path: &Path) -> Result<PhotoKind> {
    let mut start = [0u8; 16];
    let mut file = std::fs::File::open(path).map_err(|e| ErrorKind::reading(&e))?;
    let read = file.read(&mut start).map_err(|e| ErrorKind::reading(&e))?;
    PhotoKind::of(&start[..read]).ok_or(ErrorKind::UnsupportedFormat)
}

pub(crate) use photo::{Decoded, decode};
