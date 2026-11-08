//! Beri Watermark (K1): semi-transparent text over a photo, repeated on a
//! slant, in the middle, or in a corner. The layout is worked out here for
//! photos and PDFs alike, in the units of the page (pixels, or points), so
//! a PDF page looks like a photo of the same shape; `rantakit-pdf` only
//! draws the lines it is given.

use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

use ::image::{DynamicImage, GrayImage, Luma, RgbaImage};
use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont, point};
use rantakit_core::{
    CancelToken, ErrorKind, Progress, Result, Tracker, read_original, write_result,
};

use crate::options::{Compressed, ImageFormat};
use crate::{Decoded, decode, encode};

/// Plus Jakarta Sans Bold, the app's own font, Latin only (OFL, `fonts/OFL.txt`).
pub const FONT: &[u8] = include_bytes!("../fonts/PlusJakartaSans-Bold.ttf");
/// The name a PDF knows the font by.
pub const FONT_NAME: &str = "PlusJakartaSans-Bold";

/// A watermarked JPG is saved at this quality.
pub const JPEG_QUALITY: u8 = 92;

/// The slant of repeated and centered text, counterclockwise.
const ANGLE: f32 = 30.0;
/// A date line is this much of the text's size.
const DATE_SIZE: f32 = 0.8;
/// From one baseline to the next, in sizes.
const LINE_HEIGHT: f32 = 1.3;
/// Space around each repeated block, in sizes.
const GAP_ALONG: f32 = 3.0;
const GAP_ACROSS: f32 = 2.5;
/// Centered text is this much bigger than repeated text.
const CENTER_SCALE: f32 = 1.6;
/// A single line keeps this share of the page free around it.
const FIT: f32 = 0.9;

/// Progress once the Original is read, decoded, marked, and encoded.
const READ: f32 = 0.1;
const DECODED: f32 = 0.3;
const MARKED: f32 = 0.6;
const ENCODED: f32 = 0.9;

/// Where the text goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// "Berulang": a slanted pattern over the whole page.
    Repeated,
    /// "Tengah": one slanted block in the middle.
    Center,
    /// "Pojok kanan bawah": one straight block in the bottom right corner.
    Corner,
}

/// What to write and how it looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Watermark {
    /// The text, then the date if there is one. Characters the font or a
    /// PDF cannot show become "?".
    pub lines: Vec<String>,
    pub placement: Placement,
    /// The text's size as a share of the page's shorter side.
    pub size: f32,
    /// 0 is invisible, 1 covers what is under it.
    pub opacity: f32,
    pub color: [u8; 3],
}

/// One line of text where it goes on a page: its baseline starts at `x`,
/// `y` (from the top left, `y` down) and runs `angle` degrees
/// counterclockwise. `text` is in WinAnsi, one byte per character.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLine {
    pub text: Vec<u8>,
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

/// The font's measures for a PDF, in thousandths of its size.
#[derive(Debug, Clone, PartialEq)]
pub struct FontMetrics {
    /// Advance widths of WinAnsi codes 32 to 255.
    pub widths: Vec<u16>,
    pub ascent: i32,
    pub descent: i32,
    pub cap_height: i32,
    pub bbox: [i32; 4],
}

fn font() -> &'static FontRef<'static> {
    static PARSED: OnceLock<FontRef<'static>> = OnceLock::new();
    PARSED.get_or_init(|| FontRef::try_from_slice(FONT).expect("the bundled font is valid"))
}

/// ab_glyph's scale for text whose em is `size` units.
fn scale(size: f32) -> PxScale {
    let font = font();
    let em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size * font.height_unscaled() / em)
}

/// The width of `text` at `size`, without kerning, as a PDF draws it.
fn width(text: &str, size: f32) -> f32 {
    let scaled = font().as_scaled(scale(size));
    text.chars()
        .map(|c| scaled.h_advance(font().glyph_id(c)))
        .sum()
}

/// WinAnsi codes 128 to 159, which are not Latin-1.
const WIN_ANSI_HIGH: [(u8, char); 27] = [
    (0x80, '€'),
    (0x82, '‚'),
    (0x83, 'ƒ'),
    (0x84, '„'),
    (0x85, '…'),
    (0x86, '†'),
    (0x87, '‡'),
    (0x88, 'ˆ'),
    (0x89, '‰'),
    (0x8A, 'Š'),
    (0x8B, '‹'),
    (0x8C, 'Œ'),
    (0x8E, 'Ž'),
    (0x91, '‘'),
    (0x92, '’'),
    (0x93, '“'),
    (0x94, '”'),
    (0x95, '•'),
    (0x96, '–'),
    (0x97, '—'),
    (0x98, '˜'),
    (0x99, '™'),
    (0x9A, 'š'),
    (0x9B, '›'),
    (0x9C, 'œ'),
    (0x9E, 'ž'),
    (0x9F, 'Ÿ'),
];

/// The WinAnsi code of `c`, the one-byte encoding every PDF reader knows.
fn win_ansi(c: char) -> Option<u8> {
    match c as u32 {
        0x20..=0x7E | 0xA0..=0xFF => Some(c as u8),
        _ => WIN_ANSI_HIGH
            .iter()
            .find(|(_, high)| *high == c)
            .map(|(code, _)| *code),
    }
}

/// The character of WinAnsi `code`.
fn from_win_ansi(code: u8) -> Option<char> {
    match code {
        0x20..=0x7E | 0xA0..=0xFF => Some(code as char),
        _ => WIN_ANSI_HIGH
            .iter()
            .find(|(high, _)| *high == code)
            .map(|(_, c)| *c),
    }
}

/// `text` as the watermark can show it: one line, every character the
/// font and WinAnsi both have, and "?" for the others.
pub fn shown(text: &str) -> String {
    let font = font();
    text.chars()
        .filter(|c| !c.is_control())
        .map(|c| match win_ansi(c) {
            Some(_) if font.glyph_id(c) != GlyphId(0) || c == ' ' => c,
            _ => '?',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// The bundled font's measures, for embedding it in a PDF.
pub fn font_metrics() -> FontMetrics {
    let font = font();
    let em = font.units_per_em().unwrap_or(1000.0);
    let thousandths = |units: f32| (units * 1000.0 / em).round() as i32;
    let widths = (32..=255u8)
        .map(|code| {
            from_win_ansi(code)
                .filter(|&c| font.glyph_id(c) != GlyphId(0))
                .map(|c| thousandths(font.h_advance_unscaled(font.glyph_id(c))).max(0) as u16)
                .unwrap_or(0)
        })
        .collect();
    // Drawn at an em of 1000 from the origin: bounds are y down, in thousandths.
    let bounds = |c: char| {
        let glyph = font
            .glyph_id(c)
            .with_scale_and_position(scale(1000.0), point(0.0, 0.0));
        font.outline_glyph(glyph)
            .map(|outlined| outlined.px_bounds())
    };
    let mut bbox = [0i32; 4];
    for b in (32..=255u8).filter_map(from_win_ansi).filter_map(bounds) {
        bbox[0] = bbox[0].min(b.min.x.floor() as i32);
        bbox[1] = bbox[1].min(-b.max.y.ceil() as i32);
        bbox[2] = bbox[2].max(b.max.x.ceil() as i32);
        bbox[3] = bbox[3].max(-b.min.y.floor() as i32);
    }
    let cap_height = bounds('H').map(|b| -b.min.y.round() as i32).unwrap_or(700);
    FontMetrics {
        widths,
        ascent: thousandths(font.ascent_unscaled()),
        descent: thousandths(font.descent_unscaled()),
        cap_height,
        bbox,
    }
}

/// A block of lines, measured: each line's size, width, and where its
/// baseline starts from the block's center, in the block's own direction.
struct Block {
    lines: Vec<BlockLine>,
    width: f32,
    height: f32,
}

struct BlockLine {
    text: String,
    size: f32,
    width: f32,
    /// From the block's center to the start of its baseline, `dy` down.
    dx: f32,
    dy: f32,
}

impl Watermark {
    /// The lines of the block, set at `size` for the first, left to right
    /// centered, or right aligned in a corner.
    fn block(&self, size: f32) -> Block {
        let font = font();
        let em = font.units_per_em().unwrap_or(1000.0);
        let ascent = font.ascent_unscaled() / em;
        let mut lines: Vec<BlockLine> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, text)| !text.is_empty())
            .map(|(at, text)| {
                let size = if at == 0 { size } else { size * DATE_SIZE };
                BlockLine {
                    width: width(text, size),
                    text: text.clone(),
                    size,
                    dx: 0.0,
                    dy: 0.0,
                }
            })
            .collect();
        let width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
        let height: f32 = lines.iter().map(|line| line.size * LINE_HEIGHT).sum();
        let mut top = -height / 2.0;
        for line in &mut lines {
            let gap = line.size * (LINE_HEIGHT - 1.0) / 2.0;
            line.dy = top + gap + line.size * ascent;
            line.dx = match self.placement {
                Placement::Corner => width / 2.0 - line.width,
                _ => -line.width / 2.0,
            };
            top += line.size * LINE_HEIGHT;
        }
        Block {
            lines,
            width,
            height,
        }
    }

    /// Every line to draw on a page `width` × `height` as it is shown.
    pub fn layout(&self, width: f32, height: f32) -> Vec<PlacedLine> {
        let base = self.size * width.min(height);
        if base <= 0.0 || self.lines.iter().all(String::is_empty) {
            return Vec::new();
        }
        let (block, centers, angle) = match self.placement {
            Placement::Repeated => {
                let block = self.block(base);
                let centers = repeated_centers(&block, base, width, height);
                (block, centers, ANGLE)
            }
            Placement::Center => {
                let first = self.block(base * CENTER_SCALE);
                let (sin, cos) = ANGLE.to_radians().sin_cos();
                let across = first.width * cos + first.height * sin;
                let down = first.width * sin + first.height * cos;
                let fit = (FIT * width / across).min(FIT * height / down).min(1.0);
                let block = self.block(base * CENTER_SCALE * fit);
                (block, vec![(width / 2.0, height / 2.0)], ANGLE)
            }
            Placement::Corner => {
                let first = self.block(base);
                let margin = base;
                let fit = ((width - 2.0 * margin) / first.width).clamp(0.0, 1.0);
                let block = self.block(base * fit);
                let center = (
                    width - margin - block.width / 2.0,
                    height - margin - block.height / 2.0,
                );
                (block, vec![center], 0.0)
            }
        };
        let (sin, cos) = angle.to_radians().sin_cos();
        let mut placed = Vec::with_capacity(centers.len() * block.lines.len());
        for (cx, cy) in centers {
            for line in &block.lines {
                // Along the text is (cos, -sin) with y down; across it, (sin, cos).
                placed.push(PlacedLine {
                    text: line.text.chars().filter_map(win_ansi).collect(),
                    size: line.size,
                    x: cx + line.dx * cos + line.dy * sin,
                    y: cy - line.dx * sin + line.dy * cos,
                    angle,
                });
            }
        }
        placed
    }
}

/// Centers of the repeated blocks: rows along the slant, each row moved
/// half a block from the last, from the page's center out past its corners.
fn repeated_centers(block: &Block, size: f32, width: f32, height: f32) -> Vec<(f32, f32)> {
    let along = block.width + GAP_ALONG * size;
    let across = block.height + GAP_ACROSS * size;
    let reach = (width * width + height * height).sqrt() / 2.0 + along;
    let (sin, cos) = ANGLE.to_radians().sin_cos();
    let rows = (reach / across).ceil() as i32;
    let columns = (reach / along).ceil() as i32 + 1;
    let half = block.width.max(block.height);
    let mut centers = Vec::new();
    for row in -rows..=rows {
        let shift = if row % 2 == 0 { 0.0 } else { along / 2.0 };
        for column in -columns..=columns {
            let u = column as f32 * along + shift;
            let v = row as f32 * across;
            let x = width / 2.0 + u * cos + v * sin;
            let y = height / 2.0 - u * sin + v * cos;
            if x > -half && x < width + half && y > -half && y < height + half {
                centers.push((x, y));
            }
        }
    }
    centers
}

/// Draws `watermark` over `img`, which is shown as it is stored.
pub fn mark(img: &mut RgbaImage, watermark: &Watermark) {
    let (w, h) = img.dimensions();
    let lines = watermark.layout(w as f32, h as f32);
    let alpha = watermark.opacity.clamp(0.0, 1.0);
    let [r, g, b] = watermark.color.map(f32::from);
    // Lines repeat, so each is drawn once into its own mask and stamped.
    let mut masks: Vec<(Vec<u8>, f32, Mask)> = Vec::new();
    for line in &lines {
        let at = match masks
            .iter()
            .position(|(text, size, _)| *text == line.text && *size == line.size)
        {
            Some(at) => at,
            None => {
                masks.push((
                    line.text.clone(),
                    line.size,
                    line_mask(&line.text, line.size),
                ));
                masks.len() - 1
            }
        };
        let mask = &masks[at].2;
        stamp(img, mask, line, |pixel, coverage| {
            let a = alpha * coverage;
            let below = pixel[3] as f32 / 255.0;
            let out = a + below * (1.0 - a);
            if out <= 0.0 {
                return;
            }
            for (c, top) in [r, g, b].into_iter().enumerate() {
                let mixed = (top * a + pixel[c] as f32 * below * (1.0 - a)) / out;
                pixel[c] = mixed.round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = (out * 255.0).round() as u8;
        });
    }
}

/// One line drawn flat: its coverage, and where its baseline starts in it.
struct Mask {
    coverage: GrayImage,
    origin_x: f32,
    origin_y: f32,
}

fn line_mask(text: &[u8], size: f32) -> Mask {
    let font = font();
    let scale = scale(size);
    let scaled = font.as_scaled(scale);
    let pad = 2.0;
    let text: String = text
        .iter()
        .filter_map(|&code| from_win_ansi(code))
        .collect();
    let width = width(&text, size);
    let ascent = scaled.ascent();
    let descent = -scaled.descent();
    let (mw, mh) = (
        (width + 2.0 * pad + size * 0.2).ceil() as u32,
        (ascent + descent + 2.0 * pad).ceil() as u32,
    );
    let mut coverage = GrayImage::new(mw.max(1), mh.max(1));
    let (origin_x, origin_y) = (pad, pad + ascent);
    let mut x = origin_x;
    for c in text.chars() {
        let id = font.glyph_id(c);
        let glyph = id.with_scale_and_position(scale, point(x, origin_y));
        x += scaled.h_advance(id);
        let Some(outlined) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outlined.px_bounds();
        outlined.draw(|gx, gy, c| {
            let px = bounds.min.x as i32 + gx as i32;
            let py = bounds.min.y as i32 + gy as i32;
            if px >= 0 && py >= 0 && (px as u32) < mw && (py as u32) < mh {
                let old = coverage.get_pixel(px as u32, py as u32)[0];
                let new = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                coverage.put_pixel(px as u32, py as u32, Luma([old.max(new)]));
            }
        });
    }
    Mask {
        coverage,
        origin_x,
        origin_y,
    }
}

/// Puts `mask` on `img` with its baseline start at `line`'s point, turned
/// by its angle, calling `blend` for each pixel it covers.
fn stamp(
    img: &mut RgbaImage,
    mask: &Mask,
    line: &PlacedLine,
    mut blend: impl FnMut(&mut ::image::Rgba<u8>, f32),
) {
    let (w, h) = img.dimensions();
    let (mw, mh) = mask.coverage.dimensions();
    let (sin, cos) = line.angle.to_radians().sin_cos();
    // A page point p is mask point (ux, uy) = R⁻¹(p - start) + origin.
    let to_mask = |px: f32, py: f32| {
        let (dx, dy) = (px - line.x, py - line.y);
        (
            dx * cos - dy * sin + mask.origin_x,
            dx * sin + dy * cos + mask.origin_y,
        )
    };
    // The mask's corners on the page bound the pixels to visit.
    let corners = [
        (0.0, 0.0),
        (mw as f32, 0.0),
        (0.0, mh as f32),
        (mw as f32, mh as f32),
    ]
    .map(|(ux, uy)| {
        let (ux, uy) = (ux - mask.origin_x, uy - mask.origin_y);
        (line.x + ux * cos + uy * sin, line.y - ux * sin + uy * cos)
    });
    let min_x = corners
        .iter()
        .map(|c| c.0)
        .fold(f32::MAX, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_x = corners
        .iter()
        .map(|c| c.0)
        .fold(f32::MIN, f32::max)
        .ceil()
        .min(w as f32) as u32;
    let min_y = corners
        .iter()
        .map(|c| c.1)
        .fold(f32::MAX, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_y = corners
        .iter()
        .map(|c| c.1)
        .fold(f32::MIN, f32::max)
        .ceil()
        .min(h as f32) as u32;
    for py in min_y..max_y {
        for px in min_x..max_x {
            let (ux, uy) = to_mask(px as f32 + 0.5, py as f32 + 0.5);
            let coverage = sample(&mask.coverage, ux - 0.5, uy - 0.5);
            if coverage > 0.0 {
                blend(img.get_pixel_mut(px, py), coverage);
            }
        }
    }
}

/// Bilinear coverage at `(x, y)`, 0 outside the mask.
fn sample(mask: &GrayImage, x: f32, y: f32) -> f32 {
    let (w, h) = mask.dimensions();
    if x <= -1.0 || y <= -1.0 || x >= w as f32 || y >= h as f32 {
        return 0.0;
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |x: f32, y: f32| -> f32 {
        if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
            0.0
        } else {
            mask.get_pixel(x as u32, y as u32)[0] as f32 / 255.0
        }
    };
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
    let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Writes the photo at `input` with `watermark` on it to `output`, which
/// must not exist yet, in the Original's format. The Result has no photo
/// info: it is a fresh picture, turned upright first.
pub fn watermark_image(
    input: &Path,
    output: &Path,
    watermark: &Watermark,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> Result<Compressed> {
    let mut tracker = Tracker::new(progress, cancel);
    let original = read_original(input)?;
    tracker.step(READ)?;
    let Decoded { img, kind, icc } = decode(&original)?;
    let format = kind.saved_as();
    drop(original);
    tracker.step(DECODED)?;

    let had_alpha = img.color().has_alpha();
    let mut rgba = img.into_rgba8();
    mark(&mut rgba, watermark);
    tracker.step(MARKED)?;

    // Transparency stays in a PNG; a JPG gets it on white, from the encoder.
    let marked = if had_alpha {
        DynamicImage::ImageRgba8(rgba)
    } else {
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(rgba).into_rgb8())
    };
    let bytes = encode::encode(&marked, format, JPEG_QUALITY, icc.as_deref())?;
    tracker.step(ENCODED)?;
    let size = write_result(output, &mut tracker, |file| file.write_all(&bytes))?;
    Ok(Compressed {
        size,
        format,
        width: marked.width(),
        height: marked.height(),
        quality: (format == ImageFormat::Jpeg).then_some(JPEG_QUALITY),
    })
}

/// A small JPG or PNG `picture`, such as a thumbnail or a PDF page, with
/// `watermark` on it, as a JPG: the live preview.
pub fn watermark_preview(picture: &[u8], watermark: &Watermark) -> Result<Vec<u8>> {
    let img = ::image::load_from_memory(picture).map_err(|_| ErrorKind::Damaged)?;
    let mut rgba = img.into_rgba8();
    mark(&mut rgba, watermark);
    let flat = DynamicImage::ImageRgb8(encode::flatten_on(
        &DynamicImage::ImageRgba8(rgba),
        crate::WHITE,
    ));
    encode::encode(&flat, ImageFormat::Jpeg, 85, None)
}
