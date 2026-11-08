//! Beri Watermark (K1) on photos.

use std::fs;
use std::path::PathBuf;

use image::{Rgba, RgbaImage};
use rantakit_image::watermark::mark;
use rantakit_image::{
    CancelToken, ImageFormat, Placement, Watermark, font_metrics, shown, watermark_image,
    watermark_preview,
};

const PHOTO: &[u8] = include_bytes!("fixtures/photo.jpg");
const TRANSPARENT: &[u8] = include_bytes!("fixtures/transparent.png");
const ROTATED: &[u8] = include_bytes!("fixtures/rotated_exif6.jpg");

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-image-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn watermark(placement: Placement) -> Watermark {
    Watermark {
        lines: vec!["Hanya untuk verifikasi Bank X".into(), "08/11/2026".into()],
        placement,
        size: 0.05,
        opacity: 0.5,
        color: [0, 0, 0],
    }
}

/// A white page with `watermark` on it, and which of its pixels changed.
fn marked(width: u32, height: u32, watermark: &Watermark) -> Vec<(u32, u32)> {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 255]));
    mark(&mut img, watermark);
    img.enumerate_pixels()
        .filter(|(_, _, p)| p[0] < 250)
        .map(|(x, y, _)| (x, y))
        .collect()
}

#[test]
fn the_layout_is_the_same_share_of_any_page_of_that_shape() {
    for placement in [Placement::Repeated, Placement::Center, Placement::Corner] {
        let big = watermark(placement).layout(1712.0, 1080.0);
        let small = watermark(placement).layout(856.0, 540.0);
        assert_eq!(big.len(), small.len(), "{placement:?}");
        for (big, small) in big.iter().zip(&small) {
            assert_eq!(big.text, small.text);
            assert!((big.x - small.x * 2.0).abs() < 0.01, "{placement:?}");
            assert!((big.y - small.y * 2.0).abs() < 0.01, "{placement:?}");
            assert!((big.size - small.size * 2.0).abs() < 0.01, "{placement:?}");
        }
    }
}

#[test]
fn repeated_text_covers_the_whole_photo() {
    let changed = marked(600, 400, &watermark(Placement::Repeated));
    for (qx, qy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let inside = changed
            .iter()
            .filter(|(x, y)| x / 300 == qx && y / 200 == qy)
            .count();
        assert!(
            inside > 500,
            "quarter {qx},{qy} has only {inside} marked pixels"
        );
    }
    let lines = watermark(Placement::Repeated).layout(600.0, 400.0);
    assert!(lines.iter().all(|line| line.angle == 30.0));
}

#[test]
fn centered_text_stays_in_the_middle_and_a_corner_in_its_corner() {
    let center = marked(600, 400, &watermark(Placement::Center));
    assert!(!center.is_empty());
    assert!(
        center
            .iter()
            .all(|(x, y)| (30..570).contains(x) && (20..380).contains(y))
    );

    let corner = marked(600, 400, &watermark(Placement::Corner));
    assert!(!corner.is_empty());
    assert!(corner.iter().all(|(x, y)| *x >= 300 && *y >= 300));
    let lines = watermark(Placement::Corner).layout(600.0, 400.0);
    assert!(lines.iter().all(|line| line.angle == 0.0));
    // The date is smaller and under the text.
    assert!(lines[1].size < lines[0].size && lines[1].y > lines[0].y);
}

#[test]
fn opacity_and_color_show() {
    let mut faint = watermark(Placement::Center);
    faint.opacity = 0.15;
    faint.color = [200, 0, 0];
    let mut img = RgbaImage::from_pixel(600, 400, Rgba([255, 255, 255, 255]));
    mark(&mut img, &faint);
    let darkest = img.pixels().map(|p| p[1]).min().unwrap();
    let red = img.pixels().find(|p| p[1] == darkest).unwrap();
    // Faint: 15% of the way to the color, and red stays reddest.
    assert!((210..=225).contains(&darkest), "{darkest}");
    assert!(red[0] > red[1] && red[1] == red[2]);
}

#[test]
fn a_photo_is_watermarked_upright_in_its_own_format() {
    let dir = scratch("watermark_photo");
    let cases = [
        ("ktp.jpg", PHOTO, ImageFormat::Jpeg),
        ("rotated.jpg", ROTATED, ImageFormat::Jpeg),
        ("logo.png", TRANSPARENT, ImageFormat::Png),
    ];
    for (name, bytes, format) in cases {
        let input = dir.join(name);
        fs::write(&input, bytes).unwrap();
        let output = dir.join(format!("{name}.out"));
        let out = watermark_image(
            &input,
            &output,
            &watermark(Placement::Repeated),
            &mut |_| {},
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(out.format, format, "{name}");
        let result = image::load_from_memory(&fs::read(&output).unwrap()).unwrap();
        assert_eq!((result.width(), result.height()), (out.width, out.height));
        if name == "rotated.jpg" {
            // Stored 300x200 with orientation 6, so the Result is 200x300.
            assert_eq!((out.width, out.height), (200, 300));
        }
        if format == ImageFormat::Png {
            assert!(result.color().has_alpha(), "a PNG keeps its transparency");
        }
        assert_eq!(
            fs::read(&input).unwrap(),
            bytes,
            "the Original is untouched"
        );
    }
}

#[test]
fn the_preview_is_a_jpg_of_the_same_size() {
    let preview = watermark_preview(PHOTO, &watermark(Placement::Repeated)).unwrap();
    let original = image::load_from_memory(PHOTO).unwrap();
    let shown = image::load_from_memory(&preview).unwrap();
    assert_eq!(
        (shown.width(), shown.height()),
        (original.width(), original.height())
    );
}

#[test]
fn text_the_font_cannot_show_becomes_a_question_mark() {
    assert_eq!(
        shown("  Hanya untuk verifikasi Bank X  "),
        "Hanya untuk verifikasi Bank X"
    );
    assert_eq!(shown("Rp 5.000 – “lunas” é"), "Rp 5.000 – “lunas” é");
    assert_eq!(shown("KTP 日本\tok"), "KTP ??ok");
}

#[test]
fn the_font_measures_fit_a_pdf() {
    let metrics = font_metrics();
    assert_eq!(metrics.widths.len(), 224);
    let width = |c: char| metrics.widths[c as usize - 32];
    assert!(width(' ') > 150 && width('W') > width('i'));
    assert!(metrics.ascent > metrics.cap_height && metrics.cap_height > 600 && metrics.descent < 0);
    assert!(metrics.bbox[2] > 800 && metrics.bbox[1] < 0);
}
