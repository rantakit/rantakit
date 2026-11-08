//! Reading photos: JPG and PNG, and HEIC and WebP, which Rantakit reads but
//! never writes (O1). A photo is turned upright as it is read, and its color
//! profile is kept for the Result.

use std::io::Cursor;

use ::image::{
    DynamicImage, ImageDecoder, ImageError, ImageFormat as RawFormat, ImageReader, RgbImage,
    RgbaImage,
};
use rantakit_core::{ErrorKind, Result};

use crate::options::ImageFormat;

/// The largest HEIC Rantakit opens, in pixels: four times a 48 MP iPhone
/// photo, so a damaged or hostile file can't ask for gigabytes.
const HEIC_MAX_PIXELS: u64 = 200_000_000;

/// A kind of photo Rantakit reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoKind {
    Jpeg,
    Png,
    /// iPhone photos. Read only: the Result is a JPG unless PNG is chosen.
    Heic,
    /// Read only, like HEIC.
    Webp,
}

impl PhotoKind {
    /// The kind from the first bytes of a file, not its name.
    pub fn of(start: &[u8]) -> Option<PhotoKind> {
        let at =
            |offset: usize, bytes: &[u8]| start.get(offset..offset + bytes.len()) == Some(bytes);
        if at(0, &[0xFF, 0xD8, 0xFF]) {
            Some(PhotoKind::Jpeg)
        } else if at(0, b"\x89PNG\r\n\x1a\n") {
            Some(PhotoKind::Png)
        } else if at(0, b"RIFF") && at(8, b"WEBP") {
            Some(PhotoKind::Webp)
        } else if at(4, b"ftyp") && HEIC_BRANDS.iter().any(|brand| at(8, brand)) {
            Some(PhotoKind::Heic)
        } else {
            None
        }
    }

    /// The format a Result keeps when no other is chosen: PNG stays PNG,
    /// everything else becomes JPG.
    pub fn saved_as(self) -> ImageFormat {
        match self {
            PhotoKind::Png => ImageFormat::Png,
            PhotoKind::Jpeg | PhotoKind::Heic | PhotoKind::Webp => ImageFormat::Jpeg,
        }
    }
}

/// The major brands of a HEIF file holding an HEVC picture.
const HEIC_BRANDS: &[&[u8]] = &[
    b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1", b"msf1",
];

/// A photo read and turned upright.
pub(crate) struct Decoded {
    pub img: DynamicImage,
    pub kind: PhotoKind,
    /// The Original's color profile, such as Display P3 on an iPhone, which
    /// goes into the Result so its colors look the same.
    pub icc: Option<Vec<u8>>,
}

pub(crate) fn decode(input: &[u8]) -> Result<Decoded> {
    let kind = PhotoKind::of(input).ok_or(ErrorKind::UnsupportedFormat)?;
    if kind == PhotoKind::Heic {
        return decode_heic(input);
    }
    let format = match kind {
        PhotoKind::Jpeg => RawFormat::Jpeg,
        PhotoKind::Png => RawFormat::Png,
        PhotoKind::Webp | PhotoKind::Heic => RawFormat::WebP,
    };
    let mut decoder = ImageReader::with_format(Cursor::new(input), format)
        .into_decoder()
        .map_err(decode_error)?;
    let orientation = decoder.orientation().map_err(decode_error)?;
    // A profile that can't be read is left out rather than failing the photo.
    let icc = decoder.icc_profile().ok().flatten();
    let mut img = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    img.apply_orientation(orientation);
    Ok(Decoded { img, kind, icc })
}

/// A HEIC turned upright: the decoder applies the HEIF rotation and mirror,
/// which HEIF uses in place of the EXIF orientation.
fn decode_heic(input: &[u8]) -> Result<Decoded> {
    let info = heic::ImageInfo::from_bytes(input).map_err(|_| ErrorKind::Damaged)?;
    let layout = if info.has_alpha {
        heic::PixelLayout::Rgba8
    } else {
        heic::PixelLayout::Rgb8
    };
    let mut limits = heic::Limits::default();
    limits.max_pixels = Some(HEIC_MAX_PIXELS);
    let config = heic::DecoderConfig::new();
    let out = config
        .decode_request(input)
        .with_output_layout(layout)
        .with_limits(&limits)
        .decode()
        .map_err(|_| ErrorKind::Damaged)?;
    let img = match layout {
        heic::PixelLayout::Rgba8 => {
            RgbaImage::from_raw(out.width, out.height, out.data).map(DynamicImage::ImageRgba8)
        }
        _ => RgbImage::from_raw(out.width, out.height, out.data).map(DynamicImage::ImageRgb8),
    }
    .ok_or(ErrorKind::Damaged)?;
    Ok(Decoded {
        img,
        kind: PhotoKind::Heic,
        icc: info.icc_profile,
    })
}

/// A HEIC's own small preview, when it has one: much quicker than decoding
/// the whole photo for a file row. Left out when it is turned another way
/// than the photo, since it may not carry the photo's rotation.
pub(crate) fn heic_thumbnail(input: &[u8]) -> Option<DynamicImage> {
    let info = heic::ImageInfo::from_bytes(input).ok()?;
    let out = heic::DecoderConfig::new()
        .decode_thumbnail(input, heic::PixelLayout::Rgb8)
        .ok()
        .flatten()?;
    let same_way = (out.width >= out.height) == (info.width >= info.height);
    same_way
        .then(|| RgbImage::from_raw(out.width, out.height, out.data))?
        .map(DynamicImage::ImageRgb8)
}

/// The EXIF and XMP a HEIC holds, for Hapus Info Lokasi. The EXIF is the
/// TIFF data itself, without HEIF's offset in front.
pub(crate) fn heic_info(input: &[u8]) -> Result<heic::ImageInfo> {
    heic::ImageInfo::from_bytes(input).map_err(|_| ErrorKind::Damaged)
}

pub(crate) fn decode_error(err: ImageError) -> ErrorKind {
    match err {
        ImageError::Unsupported(_) => ErrorKind::UnsupportedFormat,
        _ => ErrorKind::Damaged,
    }
}
