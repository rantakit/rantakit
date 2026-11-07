//! A photo of paper made into a clean, straight page (D1): finding the
//! paper's edges, straightening it from four corners, and turning it.

use std::io::Cursor;

use ::image::codecs::jpeg::JpegEncoder;
use ::image::metadata::Orientation;
use ::image::{ColorType, DynamicImage, ImageDecoder, ImageFormat as RawFormat, ImageReader, Rgb, RgbImage};
use imageproc::contours::{BorderType, find_contours};
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;
use imageproc::geometric_transformations::{Border, Interpolation, Projection, warp_into};
use imageproc::geometry::{approximate_polygon_dp, arc_length, contour_area, convex_hull};
use imageproc::morphology::dilate_mut;
use imageproc::distance_transform::Norm;
use imageproc::point::Point;
use rantakit_core::{ErrorKind, Result};

use crate::clean::cleaned;
use crate::{decode, resize};

/// Where the paper is in a photo: four corners as fractions (0–1) of the
/// upright photo's width and height, clockwise from the top left.
pub type Corners = [[f32; 2]; 4];

/// How one photo becomes a page: cropped and straightened to `corners`,
/// or whole when there are none, then turned a quarter clockwise
/// `quarter_turns` times, and with `clean`, made to look scanned
/// ("Bersihkan", Scan Dokumen).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhotoEdit {
    pub corners: Option<Corners>,
    pub quarter_turns: u8,
    pub clean: bool,
}

impl PhotoEdit {
    fn is_identity(&self) -> bool {
        self.corners.is_none() && self.quarter_turns.is_multiple_of(4) && !self.clean
    }
}

/// A photo ready to be a PDF page: JPG bytes and their pixel size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagePhoto {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// One gray channel rather than three colors.
    pub gray: bool,
}

/// Edges are looked for on a copy this size, which is enough to place
/// the corners and fast on a low-end phone.
const FIND_SIDE: u32 = 600;
/// Paper smaller than this share of the photo is more likely something
/// on it, such as a photo on an ID card, than the paper itself.
const SMALLEST_PAPER: f64 = 0.2;
/// JPG quality for a photo that had to be made again.
const PAGE_QUALITY: u8 = 85;
const WHITE: Rgb<u8> = Rgb([255, 255, 255]);

/// The corners of the paper in the photo `original`, or None when no
/// clear four-sided edge is found.
pub fn find_paper(original: &[u8]) -> Result<Option<Corners>> {
    let (img, _) = decode(original)?;
    Ok(find_paper_in(&img))
}

/// `find_paper` on a photo already decoded and upright.
pub fn find_paper_in(img: &DynamicImage) -> Option<Corners> {
    let small = img.thumbnail(FIND_SIDE, FIND_SIDE).to_luma8();
    let (width, height) = small.dimensions();
    if width < 16 || height < 16 {
        return None;
    }
    let blurred = gaussian_blur_f32(&small, 2.0);
    let mut edges = canny(&blurred, 20.0, 50.0);
    // Close small gaps, so the paper's outline is one unbroken border.
    dilate_mut(&mut edges, Norm::LInf, 1);

    let whole = f64::from(width) * f64::from(height);
    let mut best: Option<(f64, [Point<i32>; 4])> = None;
    for contour in find_contours::<i32>(&edges) {
        if contour.border_type != BorderType::Outer || contour.points.len() < 20 {
            continue;
        }
        let hull = convex_hull(contour.points);
        let Some(quad) = four_sided(&hull) else { continue };
        let quad = sharpened(&hull, quad);
        let area = contour_area(&quad);
        if area >= SMALLEST_PAPER * whole && best.as_ref().is_none_or(|(most, _)| area > *most) {
            best = Some((area, quad));
        }
    }
    let (_, quad) = best?;
    let points = quad.map(|p| [p.x as f32 / (width - 1) as f32, p.y as f32 / (height - 1) as f32]);
    Some(clockwise(points))
}

/// The outline simplified to four corners, when it has four.
fn four_sided(hull: &[Point<i32>]) -> Option<[Point<i32>; 4]> {
    let perimeter = arc_length(hull, true);
    [0.02, 0.03, 0.05].into_iter().find_map(|share| {
        let simple = approximate_polygon_dp(hull, share * perimeter, true);
        <[Point<i32>; 4]>::try_from(simple).ok()
    })
}

/// The simplified corners moved onto the paper's real corners.
///
/// Simplifying keeps a point within its tolerance of the outline, which
/// can be a few percent short of a corner. On a convex outline the real
/// corner is the point between its two neighbours that lies farthest from
/// the line joining them; doing that twice settles every corner.
fn sharpened(hull: &[Point<i32>], mut quad: [Point<i32>; 4]) -> [Point<i32>; 4] {
    let n = hull.len();
    for _ in 0..2 {
        for i in 0..4 {
            let (a, b) = (quad[(i + 3) % 4], quad[(i + 1) % 4]);
            let (Some(from), Some(to)) = (hull.iter().position(|p| *p == a), hull.iter().position(|p| *p == b)) else {
                continue;
            };
            let Some(at) = hull.iter().position(|p| *p == quad[i]) else { continue };
            // The arc from `a` to `b` that passes through this corner.
            let forward = (at + n - from) % n <= (to + n - from) % n;
            let arc = if forward { (from, to) } else { (to, from) };
            let len = (arc.1 + n - arc.0) % n;
            let away = |p: &Point<i32>| {
                let (dx, dy) = (f64::from(b.x - a.x), f64::from(b.y - a.y));
                (dx * f64::from(p.y - a.y) - dy * f64::from(p.x - a.x)).abs()
            };
            if let Some(corner) = (1..len).map(|k| hull[(arc.0 + k) % n]).max_by(|p, q| away(p).total_cmp(&away(q))) {
                quad[i] = corner;
            }
        }
    }
    quad
}

/// The corners from the top left, clockwise.
fn clockwise(points: [[f32; 2]; 4]) -> Corners {
    let by = |key: fn(&[f32; 2]) -> f32, most: bool| {
        let pick = points.iter().copied().max_by(|a, b| {
            let order = key(a).total_cmp(&key(b));
            if most { order } else { order.reverse() }
        });
        pick.unwrap_or_default()
    };
    [
        by(|p| p[0] + p[1], false), // top left: smallest x + y
        by(|p| p[0] - p[1], true),  // top right: largest x - y
        by(|p| p[0] + p[1], true),  // bottom right: largest x + y
        by(|p| p[0] - p[1], false), // bottom left: smallest x - y
    ]
}

/// The photo cropped, straightened, turned and cleaned as `edit` says, at
/// most `max_side` pixels on its longest side.
pub fn edited(img: &DynamicImage, edit: &PhotoEdit, max_side: u32) -> DynamicImage {
    let straight = match edit.corners {
        Some(corners) => DynamicImage::ImageRgb8(straighten(&img.to_rgb8(), &corners, max_side)),
        None => resize::fit_within(img.clone(), max_side),
    };
    let turned = match edit.quarter_turns % 4 {
        1 => straight.rotate90(),
        2 => straight.rotate180(),
        3 => straight.rotate270(),
        _ => straight,
    };
    if edit.clean {
        DynamicImage::ImageRgb8(cleaned(&turned.to_rgb8()))
    } else {
        turned
    }
}

/// The paper inside `corners`, made rectangular.
fn straighten(img: &RgbImage, corners: &Corners, max_side: u32) -> RgbImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let at = corners.map(|[x, y]| (x.clamp(0.0, 1.0) * (w - 1.0), y.clamp(0.0, 1.0) * (h - 1.0)));
    let length = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let across = length(at[0], at[1]).max(length(at[3], at[2]));
    let down = length(at[0], at[3]).max(length(at[1], at[2]));
    let scale = (max_side as f32 / across.max(down)).min(1.0);
    let (out_w, out_h) = (((across * scale).round() as u32).max(1), ((down * scale).round() as u32).max(1));
    let (right, bottom) = ((out_w - 1) as f32, (out_h - 1) as f32);

    let mut out = RgbImage::from_pixel(out_w, out_h, WHITE);
    // Corners all in a line make no shape; the photo is kept whole then.
    let Some(projection) = Projection::from_control_points(at, [(0.0, 0.0), (right, 0.0), (right, bottom), (0.0, bottom)])
    else {
        return resize::fit_within(DynamicImage::ImageRgb8(img.clone()), max_side).to_rgb8();
    };
    warp_into(img, projection, Interpolation::Bilinear, Border::Constant(WHITE), &mut out);
    out
}

/// A small JPG of the photo `original` as `edit` makes it, for file rows.
pub fn edited_preview(original: &[u8], edit: &PhotoEdit, max_side: u32) -> Result<Vec<u8>> {
    let (img, _) = decode(original)?;
    // Straightening from a smaller copy is enough for a preview, and quicker.
    let small = img.thumbnail(max_side * 2, max_side * 2);
    jpeg(&edited(&small, edit, max_side), 80)
}

/// The photo in `original` as a PDF page wants it, at most `max_side`
/// pixels on its longest side. A JPG left as it is, already upright and
/// small enough, is kept byte for byte, so it loses nothing.
pub fn page_photo(original: &[u8], edit: &PhotoEdit, max_side: u32) -> Result<PagePhoto> {
    if edit.is_identity()
        && let Some(kept) = kept_as_is(original, max_side)
    {
        return Ok(kept);
    }
    let (img, _) = decode(original)?;
    let page = edited(&img, edit, max_side);
    Ok(PagePhoto {
        jpeg: jpeg(&page, PAGE_QUALITY)?,
        width: page.width(),
        height: page.height(),
        gray: false,
    })
}

/// The JPG as it is, when nothing about it needs changing.
fn kept_as_is(original: &[u8], max_side: u32) -> Option<PagePhoto> {
    let reader = ImageReader::new(Cursor::new(original)).with_guessed_format().ok()?;
    if reader.format() != Some(RawFormat::Jpeg) {
        return None;
    }
    let mut decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    let upright = decoder.orientation().ok()? == Orientation::NoTransforms;
    // CMYK and other JPGs a PDF reader may draw wrongly are made again.
    let gray = match decoder.color_type() {
        ColorType::L8 => true,
        ColorType::Rgb8 => false,
        _ => return None,
    };
    (upright && width.max(height) <= max_side).then(|| PagePhoto {
        jpeg: original.to_vec(),
        width,
        height,
        gray,
    })
}

fn jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    img.to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality))
        .map_err(|_| ErrorKind::Damaged)?;
    Ok(bytes.into_inner())
}
