use ::image::DynamicImage;
use rantakit_core::{ErrorKind, Result, Tracker};

use crate::encode::encode;
use crate::options::ImageFormat;
use crate::resize::scale_by;
use crate::{ENCODED, RESIZED};

const MIN_QUALITY: u8 = 10;
const MAX_QUALITY: u8 = 95;
const SCALE_STEP: f32 = 0.85;
const MIN_SIDE: u32 = 320;

pub(crate) struct Fitted {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub quality: Option<u8>,
}

pub(crate) fn compress_to_size(
    img: &DynamicImage,
    format: ImageFormat,
    max_bytes: u64,
    icc: Option<&[u8]>,
    tracker: &mut Tracker<'_>,
) -> Result<Fitted> {
    let mut search = Search {
        tracker,
        icc,
        attempts: 0,
    };
    let mut factor: f32 = 1.0;
    let mut smallest = u64::MAX;

    loop {
        let scaled;
        let current = if factor < 1.0 {
            search.step()?;
            scaled = scale_by(img, factor);
            &scaled
        } else {
            img
        };

        let found = match format {
            ImageFormat::Jpeg => search_jpeg(current, max_bytes, &mut smallest, &mut search)?,
            ImageFormat::Png => try_png(current, max_bytes, &mut smallest, &mut search)?,
        };

        if let Some((bytes, quality)) = found {
            return Ok(Fitted {
                bytes,
                width: current.width(),
                height: current.height(),
                quality,
            });
        }

        let longest = current.width().max(current.height());
        if (longest as f32 * SCALE_STEP) < MIN_SIDE as f32 {
            return Err(ErrorKind::SizeLimitUnreachable { best: smallest });
        }

        factor *= SCALE_STEP;
    }
}

/// Reports progress and stops when cancelled before every encode or resize.
struct Search<'t, 'a> {
    tracker: &'t mut Tracker<'a>,
    /// The Original's color profile, in every try so the sizes are true.
    icc: Option<&'t [u8]>,
    attempts: i32,
}

impl Search<'_, '_> {
    fn step(&mut self) -> Result<()> {
        self.attempts += 1;
        // The number of attempts is not known ahead, so progress moves
        // toward ENCODED by a smaller step each time without reaching it.
        let share = 1.0 - 0.8_f32.powi(self.attempts);
        self.tracker.step(RESIZED + (ENCODED - RESIZED) * share)
    }

    fn encode(&mut self, img: &DynamicImage, format: ImageFormat, quality: u8) -> Result<Vec<u8>> {
        self.step()?;
        encode(img, format, quality, self.icc)
    }
}

fn search_jpeg(
    img: &DynamicImage,
    max_bytes: u64,
    smallest: &mut u64,
    search: &mut Search<'_, '_>,
) -> Result<Option<(Vec<u8>, Option<u8>)>> {
    let floor = search.encode(img, ImageFormat::Jpeg, MIN_QUALITY)?;
    *smallest = (*smallest).min(floor.len() as u64);
    if floor.len() as u64 > max_bytes {
        return Ok(None);
    }

    let mut best = (floor, MIN_QUALITY);
    let (mut lo, mut hi) = (MIN_QUALITY + 1, MAX_QUALITY);

    while lo <= hi {
        let q = lo + (hi - lo) / 2;
        let bytes = search.encode(img, ImageFormat::Jpeg, q)?;

        if bytes.len() as u64 <= max_bytes {
            best = (bytes, q);
            lo = q + 1;
        } else {
            hi = q - 1;
        }
    }

    Ok(Some((best.0, Some(best.1))))
}

fn try_png(
    img: &DynamicImage,
    max_bytes: u64,
    smallest: &mut u64,
    search: &mut Search<'_, '_>,
) -> Result<Option<(Vec<u8>, Option<u8>)>> {
    let bytes = search.encode(img, ImageFormat::Png, 0)?;
    *smallest = (*smallest).min(bytes.len() as u64);

    Ok((bytes.len() as u64 <= max_bytes).then_some((bytes, None)))
}
