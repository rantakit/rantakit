use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage};
use imageproc::drawing::draw_polygon_mut;
use imageproc::point::Point;
use rantakit_image::{Corners, PhotoEdit, find_paper_in, page_photo};

/// A sheet of white paper lying skewed on a brown table, like a phone photo.
fn paper_on_table(width: u32, height: u32, corners: [(i32, i32); 4]) -> RgbImage {
    let mut img = RgbImage::from_pixel(width, height, Rgb([140, 105, 75]));
    // Wood grain, so the table is not one flat color.
    for y in (0..height).step_by(37) {
        for x in 0..width {
            img.put_pixel(x, y, Rgb([125, 95, 66]));
        }
    }
    let points = corners.map(|(x, y)| Point::new(x, y));
    draw_polygon_mut(&mut img, &points, Rgb([246, 246, 242]));
    // Lines of text on the paper.
    for i in 0..8 {
        let y = corners[0].1 + 60 + i * 40;
        let row = [Point::new(corners[0].0 + 40, y), Point::new(corners[1].0 - 60, y - 10), Point::new(corners[1].0 - 60, y - 4), Point::new(corners[0].0 + 40, y + 6)];
        draw_polygon_mut(&mut img, &row, Rgb([90, 100, 115]));
    }
    img
}

fn jpeg_of(img: &DynamicImage, quality: u8) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    img.write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality)).unwrap();
    bytes.into_inner()
}

fn near(found: Corners, expected: [(f32, f32); 4]) -> bool {
    found.iter().zip(expected).all(|(f, e)| (f[0] - e.0).abs() < 0.03 && (f[1] - e.1).abs() < 0.03)
}

#[test]
fn the_paper_is_found_with_its_corners_clockwise_from_the_top_left() {
    let (w, h) = (900, 1200);
    let corners = [(160, 140), (760, 100), (800, 1080), (110, 1110)];
    let img = DynamicImage::ImageRgb8(paper_on_table(w, h, corners));

    let found = find_paper_in(&img).expect("the paper is found");

    let expected = corners.map(|(x, y)| (x as f32 / w as f32, y as f32 / h as f32));
    assert!(near(found, expected), "found {found:?}, expected about {expected:?}");
}

#[test]
fn a_photo_without_paper_edges_finds_nothing() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(800, 600, Rgb([140, 105, 75])));
    assert_eq!(find_paper_in(&img), None);
}

#[test]
fn something_small_on_the_table_is_not_taken_for_the_paper() {
    let img = DynamicImage::ImageRgb8(paper_on_table(900, 1200, [(400, 500), (520, 500), (520, 640), (400, 640)]));
    assert_eq!(find_paper_in(&img), None);
}

#[test]
fn straightening_keeps_only_the_paper() {
    let (w, h) = (900, 1200);
    let corners = [(160, 140), (760, 100), (800, 1080), (110, 1110)];
    let original = jpeg_of(&DynamicImage::ImageRgb8(paper_on_table(w, h, corners)), 92);
    let edit = PhotoEdit {
        corners: Some(corners.map(|(x, y)| [x as f32 / (w - 1) as f32, y as f32 / (h - 1) as f32])),
        quarter_turns: 0,
        clean: false,
    };

    let page = page_photo(&original, &edit, 2400).unwrap();

    let img = image::load_from_memory(&page.jpeg).unwrap().to_luma8();
    assert!(page.height > page.width, "the page stays upright: {}x{}", page.width, page.height);
    // No table is left: every edge of the page is paper, not brown wood.
    let edge_is_paper = |x: u32, y: u32| img.get_pixel(x, y).0[0] > 200;
    let (pw, ph) = (img.width(), img.height());
    for (x, y) in [(5, ph / 2), (pw - 6, ph / 2), (pw / 2, 5), (pw / 2, ph - 6)] {
        assert!(edge_is_paper(x, y), "edge pixel ({x}, {y}) is {:?}", img.get_pixel(x, y));
    }
}

#[test]
fn a_turn_swaps_the_sides() {
    let original = jpeg_of(&DynamicImage::ImageRgb8(RgbImage::new(400, 300)), 90);
    let edit = PhotoEdit { corners: None, quarter_turns: 1, clean: false };

    let page = page_photo(&original, &edit, 2400).unwrap();

    assert_eq!((page.width, page.height), (300, 400));
}

#[test]
fn a_jpg_left_as_it_is_goes_in_byte_for_byte() {
    let original = jpeg_of(&DynamicImage::ImageRgb8(RgbImage::from_pixel(640, 480, Rgb([200, 30, 30]))), 90);

    let page = page_photo(&original, &PhotoEdit::default(), 2400).unwrap();

    assert_eq!(page.jpeg, original);
    assert_eq!((page.width, page.height, page.gray), (640, 480, false));
}

#[test]
fn a_gray_jpg_stays_gray() {
    let original = jpeg_of(&DynamicImage::ImageLuma8(GrayImage::from_pixel(64, 64, Luma([120]))), 90);
    let page = page_photo(&original, &PhotoEdit::default(), 2400).unwrap();
    assert!(page.gray);
}

#[test]
fn a_big_photo_is_made_smaller() {
    let original = jpeg_of(&DynamicImage::ImageRgb8(RgbImage::new(3000, 2000)), 80);

    let page = page_photo(&original, &PhotoEdit::default(), 2400).unwrap();

    assert_eq!((page.width, page.height), (2400, 1600));
}

#[test]
fn a_png_becomes_a_jpg() {
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::new(100, 80)).write_to(&mut png, image::ImageFormat::Png).unwrap();

    let page = page_photo(png.get_ref(), &PhotoEdit::default(), 2400).unwrap();

    assert_eq!(image::guess_format(&page.jpeg).unwrap(), image::ImageFormat::Jpeg);
}

/// A page photographed under a lamp: paper that darkens to one side, a line
/// of dark text, and a red stamp.
fn shaded_page() -> RgbImage {
    RgbImage::from_fn(800, 1000, |x, y| {
        let light = 230.0 - 110.0 * x as f32 / 800.0;
        let paper = |share: f32| (light * share) as u8;
        if (300..330).contains(&y) && (100..700).contains(&x) {
            Rgb([paper(0.25), paper(0.25), paper(0.27)])
        } else if (600..700).contains(&y) && (500..640).contains(&x) {
            Rgb([paper(0.95), paper(0.3), paper(0.3)])
        } else {
            Rgb([paper(1.0), paper(0.98), paper(0.94)])
        }
    })
}

#[test]
fn cleaning_whitens_the_paper_and_keeps_writing_and_colors() {
    let original = jpeg_of(&DynamicImage::ImageRgb8(shaded_page()), 92);
    let edit = PhotoEdit {
        clean: true,
        ..PhotoEdit::default()
    };

    let page = page_photo(&original, &edit, 2400).unwrap();

    let img = image::load_from_memory(&page.jpeg).unwrap().to_rgb8();
    let at = |x, y| img.get_pixel(x, y).0;
    for (x, y) in [(50, 50), (400, 500), (760, 900), (780, 100)] {
        assert!(at(x, y).iter().all(|&c| c > 235), "paper at ({x}, {y}) is {:?}", at(x, y));
    }
    for x in [150, 400, 650] {
        assert!(at(x, 315).iter().all(|&c| c < 110), "text at ({x}, 315) is {:?}", at(x, 315));
    }
    let [r, g, b] = at(570, 650);
    assert!(r > 150 && r > g + 80 && r > b + 80, "the stamp stays red: {:?}", [r, g, b]);
}

#[test]
fn cleaning_removes_a_hard_edged_shadow() {
    // A hand's shadow over the left of the page: half as bright, with a sharp edge.
    let mut img = shaded_page();
    for (x, _, pixel) in img.enumerate_pixels_mut() {
        if x < 300 {
            pixel.0 = pixel.0.map(|c| c / 2);
        }
    }
    let original = jpeg_of(&DynamicImage::ImageRgb8(img), 92);
    let edit = PhotoEdit {
        clean: true,
        ..PhotoEdit::default()
    };

    let page = page_photo(&original, &edit, 2400).unwrap();

    let img = image::load_from_memory(&page.jpeg).unwrap().to_rgb8();
    for x in [50, 200, 270, 330, 500] {
        let pixel = img.get_pixel(x, 800).0;
        assert!(pixel.iter().all(|&c| c > 225), "paper at ({x}, 800) is {pixel:?}");
    }
}
