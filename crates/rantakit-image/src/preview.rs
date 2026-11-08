use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use ::image::{DynamicImage, ImageFormat as RawFormat, ImageReader};
use rantakit_core::{ErrorKind, Result, read_original};

use crate::options::ImageFormat;
use crate::photo::{self, PhotoKind};
use crate::{Decoded, decode, encode};

/// What the Files step shows about a photo before anything runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhotoInfo {
    pub kind: PhotoKind,
    /// Pixel size as stored, before any EXIF rotation; the longest side is the same either way.
    pub width: u32,
    pub height: u32,
}

/// Kind and pixel size from the file's header, without decoding the photo.
pub fn photo_info(path: &Path) -> Result<PhotoInfo> {
    let file = File::open(path).map_err(|e| ErrorKind::reading(&e))?;
    photo_info_of(BufReader::new(file))
}

/// `photo_info` read from an open file, such as one Android handed over
/// without a path.
pub fn photo_info_of(mut file: impl BufRead + Seek) -> Result<PhotoInfo> {
    let mut start = [0u8; 16];
    file.read_exact(&mut start).map_err(|_| ErrorKind::UnsupportedFormat)?;
    file.seek(SeekFrom::Start(0)).map_err(|e| ErrorKind::reading(&e))?;
    let kind = PhotoKind::of(&start).ok_or(ErrorKind::UnsupportedFormat)?;
    let (width, height) = match kind {
        // A HEIC's size sits in boxes that may be anywhere in the file.
        PhotoKind::Heic => {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map_err(|e| ErrorKind::reading(&e))?;
            let info = photo::heic_info(&bytes)?;
            (info.width, info.height)
        }
        PhotoKind::Jpeg | PhotoKind::Png | PhotoKind::Webp => {
            let format = match kind {
                PhotoKind::Jpeg => RawFormat::Jpeg,
                PhotoKind::Png => RawFormat::Png,
                _ => RawFormat::WebP,
            };
            ImageReader::with_format(file, format)
                .into_dimensions()
                .map_err(|_| ErrorKind::Damaged)?
        }
    };
    Ok(PhotoInfo { kind, width, height })
}

/// A small JPG of the photo at `path`, at most `max_side` pixels on its
/// longest side and turned upright, for file rows and Result previews. A
/// HEIC's own preview is used when it is big enough, which is much quicker.
pub fn thumbnail(path: &Path, max_side: u32) -> Result<Vec<u8>> {
    let original = read_original(path)?;
    let quick = (PhotoKind::of(&original) == Some(PhotoKind::Heic))
        .then(|| photo::heic_thumbnail(&original))
        .flatten()
        .filter(|small| small.width().max(small.height()) >= max_side);
    let (img, icc) = match quick {
        Some(small) => (small, photo::heic_info(&original).ok().and_then(|info| info.icc_profile)),
        None => {
            let Decoded { img, icc, .. } = decode(&original)?;
            (img, icc)
        }
    };
    drop(original);
    let small = DynamicImage::ImageRgb8(img.thumbnail(max_side, max_side).to_rgb8());
    encode::encode(&small, ImageFormat::Jpeg, 80, icc.as_deref())
}
