//! "Bersihkan" for Scan Dokumen (D4): a photographed page made to look
//! scanned. The paper's own shade, shadows included, is estimated and
//! divided out, so the paper turns white while ink and stamps keep their
//! colors; then the contrast is raised a little.

use ::image::imageops::{FilterType, resize};
use ::image::{Rgb, RgbImage};
use imageproc::filter::gaussian_blur_f32;

/// The background is estimated on a copy about this long, where strokes of
/// text are thinner than a pixel; a smaller photo is used as it is. Being
/// the same share of every page, the filters below act alike on a preview
/// and on a full photo.
const SMALL_SIDE: u32 = 150;
/// Passes of the brightest-neighbor filter, which wipes out what is left
/// of the text on the small copy, and then as many of the darkest-neighbor
/// filter, which brings a shadow's edge back to where it was.
const LIFTS: usize = 10;
/// Below this is ink, above it paper, after dividing out the background.
const INK: f32 = 16.0;
const PAPER: f32 = 232.0;

/// `img` with its paper whitened and its writing made clearer.
pub fn cleaned(img: &RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    if w < 2 || h < 2 {
        return img.clone();
    }
    let background = background(img);
    let mut out = img.clone();
    for (pixel, shade) in out.pixels_mut().zip(background.pixels()) {
        for c in 0..3 {
            let divided = pixel[c] as f32 / shade[c].max(1) as f32 * 255.0;
            let stretched = (divided - INK) / (PAPER - INK) * 255.0;
            pixel[c] = stretched.clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// The paper's shade at each pixel of `img`, without what is written on it.
fn background(img: &RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let scale = (SMALL_SIDE as f32 / w.max(h) as f32).min(1.0);
    let (sw, sh) = (
        ((w as f32 * scale) as u32).max(1),
        ((h as f32 * scale) as u32).max(1),
    );
    let mut small = resize(img, sw, sh, FilterType::Triangle);
    for _ in 0..LIFTS {
        small = nearby(&small, u8::max);
    }
    for _ in 0..LIFTS {
        small = nearby(&small, u8::min);
    }
    let smooth = gaussian_blur_f32(&small, 1.0);
    resize(&smooth, w, h, FilterType::Triangle)
}

/// Each pixel takes the brightest or darkest value (`pick`), per color, of
/// its 3×3 neighbors.
fn nearby(img: &RgbImage, pick: fn(u8, u8) -> u8) -> RgbImage {
    let (w, h) = img.dimensions();
    RgbImage::from_fn(w, h, |x, y| {
        let mut picked = img.get_pixel(x, y).0;
        for ny in y.saturating_sub(1)..(y + 2).min(h) {
            for nx in x.saturating_sub(1)..(x + 2).min(w) {
                let Rgb(p) = *img.get_pixel(nx, ny);
                for c in 0..3 {
                    picked[c] = pick(picked[c], p[c]);
                }
            }
        }
        Rgb(picked)
    })
}
