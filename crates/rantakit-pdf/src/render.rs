//! PDF pages as pictures, rendered with `hayro`: the page thumbnails every
//! PDF Tool shows, and PDF ke JPG's photos (O2).

use std::fs;
use std::path::Path;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::{LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, RgbImage};
use rantakit_core::{ErrorKind, Result};

/// Thumbnails are small; this quality keeps them sharp and a few KB each.
const THUMBNAIL_QUALITY: u8 = 80;
/// No page is drawn larger than this on its longest side, whatever is asked.
const MAX_SIDE: u32 = 4096;
/// A page for PDF ke JPG may be bigger, for printing, but no more than this,
/// so a poster at 600 dpi can't ask a phone for gigabytes.
const MAX_PHOTO_SIDE: u32 = 8000;
/// PDF sizes are in points, 72 to the inch.
const POINTS_PER_INCH: f32 = 72.0;

/// An open PDF whose pages can be drawn one at a time. Opening reads the
/// whole file once; drawing a page reads only what that page needs, so a
/// 200-page PDF opens at once.
pub struct PdfPages {
    pdf: Pdf,
}

impl PdfPages {
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_bytes(fs::read(path).map_err(|e| ErrorKind::reading(&e))?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let pdf = Pdf::new(bytes).map_err(|err| match err {
            LoadPdfError::Decryption(_) => ErrorKind::PasswordProtected,
            LoadPdfError::Invalid => ErrorKind::Damaged,
        })?;
        Ok(Self { pdf })
    }

    pub fn count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Page `index`, counting from 0, at most `max_side` pixels on its
    /// longest side, on white: a page keeps its own white in light and dark
    /// themes. Rotated and cropped pages are drawn the way a reader shows them.
    pub fn render(&self, index: usize, max_side: u32) -> Result<RgbImage> {
        self.render_scaled(index, |longest| max_side.clamp(1, MAX_SIDE) as f32 / longest)
    }

    /// Page `index`, counting from 0, at `dpi` dots per inch of the paper, for
    /// PDF to JPG: 150 dpi makes an A4 page 1240 × 1754 pixels. A page that
    /// would be larger than 8000 pixels is drawn smaller to fit.
    pub fn render_at(&self, index: usize, dpi: u32) -> Result<RgbImage> {
        self.render_scaled(index, |longest| {
            let scale = dpi.max(1) as f32 / POINTS_PER_INCH;
            scale.min(MAX_PHOTO_SIDE as f32 / longest)
        })
    }

    /// Page `index` drawn at the scale `scale_for` picks from its longest side
    /// in points.
    fn render_scaled(&self, index: usize, scale_for: impl Fn(f32) -> f32) -> Result<RgbImage> {
        let pages = self.pdf.pages();
        let page = pages.get(index).ok_or(ErrorKind::Damaged)?;
        let (width, height) = page.render_dimensions();
        let longest = width.max(height);
        if !longest.is_finite() || longest < 1.0 {
            return Err(ErrorKind::Damaged);
        }
        let scale = scale_for(longest);
        let pixmap = hayro::render(
            page,
            &RenderCache::new(),
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let (w, h) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        // The background is opaque, so premultiplied and plain RGBA agree.
        let rgb = pixmap
            .data_as_u8_slice()
            .chunks_exact(4)
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        RgbImage::from_raw(w, h, rgb).ok_or(ErrorKind::Damaged)
    }

    /// [`render`](Self::render) as a small JPG, for the page thumbnails.
    pub fn thumbnail(&self, index: usize, max_side: u32) -> Result<Vec<u8>> {
        let page = self.render(index, max_side)?;
        page_jpeg(&page, THUMBNAIL_QUALITY)
    }
}

/// A page drawn as a JPG at `quality`.
pub fn page_jpeg(page: &RgbImage, quality: u8) -> Result<Vec<u8>> {
    let mut jpg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpg, quality)
        .encode(page.as_raw(), page.width(), page.height(), ExtendedColorType::Rgb8)
        .map_err(|_| ErrorKind::Damaged)?;
    Ok(jpg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubbish_is_damaged() {
        assert_eq!(
            PdfPages::from_bytes(b"not a pdf".to_vec()).err(),
            Some(ErrorKind::Damaged)
        );
    }
}
