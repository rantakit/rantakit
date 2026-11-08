use std::borrow::Cow;
use std::io::Cursor;

use image::codecs::jpeg::{JpegDecoder, JpegEncoder};
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngDecoder, PngEncoder};
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, Limits};

use crate::package::NewPicture;

/// Decoding one picture may take at most this much memory.
const MAX_DECODE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PictureFormat {
    Jpeg,
    Png,
}

impl PictureFormat {
    /// The format a media part's name says it has; the content is checked
    /// when it is decoded.
    pub fn from_name(name: &str) -> Option<Self> {
        let (_, extension) = name.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            _ => None,
        }
    }
}

/// A picture decoded and fit within a longest side, ready to be saved at
/// any quality.
pub(crate) struct Prepared {
    image: DynamicImage,
    icc: Option<Vec<u8>>,
    gray: bool,
    format: PictureFormat,
    /// A PNG with no transparent pixel, which may become a JPG.
    opaque: bool,
    /// The PNG saved again, when it was made smaller and that saved bytes.
    png: Option<Vec<u8>>,
    /// The original's bytes in the document's ZIP.
    stored_len: u64,
}

/// `bytes` decoded and fit within `side`. `None` for a picture this engine
/// leaves as it is: one it can't read, a JPG turned by its EXIF orientation
/// (Office would show it turned differently once that is gone), or a CMYK
/// JPG, whose colors would change.
pub(crate) fn prepare(bytes: &[u8], format: PictureFormat, side: Option<u32>, stored_len: u64) -> Option<Prepared> {
    let (image, icc) = match format {
        PictureFormat::Jpeg => {
            if !matches!(jpeg_components(bytes), Some(1 | 3)) {
                return None;
            }
            let mut decoder = JpegDecoder::new(Cursor::new(bytes)).ok()?;
            decoder.set_limits(limits()).ok()?;
            if decoder.orientation().ok()? != Orientation::NoTransforms {
                return None;
            }
            let icc = decoder.icc_profile().ok().flatten();
            (DynamicImage::from_decoder(decoder).ok()?, icc)
        }
        PictureFormat::Png => {
            let mut decoder = PngDecoder::new(Cursor::new(bytes)).ok()?;
            decoder.set_limits(limits()).ok()?;
            let icc = decoder.icc_profile().ok().flatten();
            (DynamicImage::from_decoder(decoder).ok()?, icc)
        }
    };
    let opaque = format == PictureFormat::Png && is_opaque(&image);
    let gray = !image.color().has_color();

    let longest = image.width().max(image.height());
    let resized = side.is_some_and(|side| side > 0 && longest > side);
    let image = match side {
        Some(side) if resized => image.resize(side, side, FilterType::Lanczos3),
        _ => image,
    };
    // A PNG saved again without being made smaller rarely shrinks, and
    // saving it at the strongest level is slow.
    let png = (format == PictureFormat::Png && resized)
        .then(|| encode_png(&image, icc.as_deref()))
        .flatten()
        .filter(|png| (png.len() as u64) < stored_len);

    Some(Prepared {
        image,
        icc,
        gray,
        format,
        opaque,
        png,
        stored_len,
    })
}

impl Prepared {
    /// The picture to put in the document at JPG `quality`, or `None` to keep
    /// the original. A JPG is kept only when it is smaller. An opaque PNG
    /// becomes a JPG only when that halves it, which photos do and
    /// screenshots, whose text would blur, don't.
    pub fn at(&self, quality: u8) -> Option<NewPicture> {
        match self.format {
            PictureFormat::Jpeg => self
                .jpeg(quality)
                .filter(|jpeg| (jpeg.len() as u64) < self.stored_len)
                .map(|bytes| NewPicture {
                    bytes,
                    became_jpeg: false,
                }),
            PictureFormat::Png => {
                let png_len = self.png.as_ref().map_or(self.stored_len, |png| png.len() as u64);
                let jpeg = self
                    .opaque
                    .then(|| self.jpeg(quality))
                    .flatten()
                    .filter(|jpeg| 2 * (jpeg.len() as u64) < png_len);
                match jpeg {
                    Some(bytes) => Some(NewPicture {
                        bytes,
                        became_jpeg: true,
                    }),
                    None => self.png.clone().map(|bytes| NewPicture {
                        bytes,
                        became_jpeg: false,
                    }),
                }
            }
        }
    }

    /// The bytes the picture takes in the Result at `quality`.
    pub fn len_at(&self, quality: u8) -> u64 {
        self.at(quality)
            .map_or(self.stored_len, |picture| picture.bytes.len() as u64)
    }

    fn jpeg(&self, quality: u8) -> Option<Vec<u8>> {
        // Converted only when the pixels aren't in the encoder's layout already.
        let (raw, color): (Cow<'_, [u8]>, _) = match (&self.image, self.gray) {
            (DynamicImage::ImageLuma8(luma), true) => (Cow::Borrowed(luma.as_raw()), ExtendedColorType::L8),
            (image, true) => (Cow::Owned(image.to_luma8().into_raw()), ExtendedColorType::L8),
            (DynamicImage::ImageRgb8(rgb), false) => (Cow::Borrowed(rgb.as_raw()), ExtendedColorType::Rgb8),
            (image, false) => (Cow::Owned(image.to_rgb8().into_raw()), ExtendedColorType::Rgb8),
        };
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality.clamp(1, 100));
        if let Some(icc) = &self.icc {
            let _ = encoder.set_icc_profile(icc.clone());
        }
        encoder
            .write_image(&raw, self.image.width(), self.image.height(), color)
            .ok()?;
        Some(bytes)
    }
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

fn encode_png(image: &DynamicImage, icc: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, PngFilter::Adaptive);
    if let Some(icc) = icc {
        let _ = encoder.set_icc_profile(icc.to_vec());
    }
    encoder
        .write_image(image.as_bytes(), image.width(), image.height(), image.color().into())
        .ok()?;
    Some(bytes)
}

/// Whether no pixel of the picture lets what's behind it show through.
fn is_opaque(image: &DynamicImage) -> bool {
    match image {
        DynamicImage::ImageRgba8(rgba) => rgba.pixels().all(|p| p[3] == u8::MAX),
        DynamicImage::ImageLumaA8(luma) => luma.pixels().all(|p| p[1] == u8::MAX),
        image if image.color().has_alpha() => image.to_rgba8().pixels().all(|p| p[3] == u8::MAX),
        _ => true,
    }
}

/// How many color components a JPG has, from its frame header: 1 for gray,
/// 3 for color, 4 for CMYK.
fn jpeg_components(bytes: &[u8]) -> Option<u8> {
    let mut at = 2;
    while at + 4 <= bytes.len() {
        if bytes[at] != 0xFF {
            return None;
        }
        let marker = bytes[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        let len = u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
        let frame = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if frame {
            return bytes.get(at + 9).copied();
        }
        at += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    use super::*;

    fn noisy(width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| {
            let n = (x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)).wrapping_mul(2_654_435_761);
            Rgb([(n >> 8) as u8, (n >> 16) as u8, (n >> 24) as u8])
        })
    }

    fn png_of(image: &DynamicImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).unwrap();
        bytes
    }

    #[test]
    fn a_photo_saved_as_png_becomes_a_jpg() {
        let png = png_of(&DynamicImage::ImageRgb8(noisy(200, 150)));
        let prepared = prepare(&png, PictureFormat::Png, None, png.len() as u64).unwrap();
        let picture = prepared.at(70).unwrap();
        assert!(picture.became_jpeg);
        assert!(picture.bytes.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn a_flat_screenshot_stays_png() {
        let flat = RgbImage::from_fn(200, 150, |x, _| if x < 100 { Rgb([255, 255, 255]) } else { Rgb([20, 60, 200]) });
        let png = png_of(&DynamicImage::ImageRgb8(flat));
        let prepared = prepare(&png, PictureFormat::Png, None, png.len() as u64).unwrap();
        assert!(prepared.at(70).is_none(), "kept as it is");
    }

    #[test]
    fn transparent_pngs_stay_png_and_shrink_when_smaller() {
        let photo = noisy(200, 150);
        let image = RgbaImage::from_fn(200, 150, |x, y| {
            let Rgb([r, g, b]) = *photo.get_pixel(x, y);
            Rgba([r, g, b, if x < 10 { 0 } else { 255 }])
        });
        let png = png_of(&DynamicImage::ImageRgba8(image));
        let prepared = prepare(&png, PictureFormat::Png, Some(100), png.len() as u64).unwrap();
        let picture = prepared.at(70).unwrap();
        assert!(!picture.became_jpeg);
        assert!(picture.bytes.starts_with(b"\x89PNG"));
        let small = image::load_from_memory(&picture.bytes).unwrap();
        assert_eq!((small.width(), small.height()), (100, 75));
    }

    #[test]
    fn jpgs_shrink_only_when_smaller() {
        let image = DynamicImage::ImageRgb8(noisy(300, 200));
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 95)
            .write_image(image.as_bytes(), 300, 200, ExtendedColorType::Rgb8)
            .unwrap();
        assert_eq!(jpeg_components(&jpeg), Some(3));
        let prepared = prepare(&jpeg, PictureFormat::Jpeg, Some(150), jpeg.len() as u64).unwrap();
        assert!(prepared.len_at(60) < jpeg.len() as u64);
        let small = prepare(&jpeg, PictureFormat::Jpeg, None, 10).unwrap();
        assert!(small.at(60).is_none(), "already smaller than any new JPG");
    }

    #[test]
    fn unreadable_pictures_are_left_alone() {
        assert!(prepare(b"not a picture", PictureFormat::Jpeg, None, 13).is_none());
        assert!(prepare(b"not a picture", PictureFormat::Png, None, 13).is_none());
    }
}
