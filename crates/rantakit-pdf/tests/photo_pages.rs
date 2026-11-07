use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, RgbImage};
use lopdf::{Document, Object};
use rantakit_pdf::{CancelToken, PagePhoto, PaperSize, PdfDocument, Tracker, placement};

const A4: (f32, f32) = (595.28, 841.89);
const MARGIN: f32 = 28.35;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.1
}

fn photo(width: u32, height: u32) -> PagePhoto {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::new(width, height))
        .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, 80))
        .unwrap();
    PagePhoto { jpeg: bytes.into_inner(), width, height, gray: false }
}

#[test]
fn an_upright_photo_goes_on_an_upright_page_with_a_thin_margin() {
    let place = placement(1000, 2000, PaperSize::A4, true);

    assert!(close(place.page.0, A4.0) && close(place.page.1, A4.1), "{place:?}");
    let (x, y, w, h) = place.photo;
    // Taller than the page is wide: it fills the height inside the margin.
    assert!(close(h, A4.1 - 2.0 * MARGIN) && close(w, h / 2.0), "{place:?}");
    assert!(close(y, MARGIN) && close(x, (A4.0 - w) / 2.0), "{place:?}");
}

#[test]
fn a_sideways_photo_turns_the_page_sideways() {
    let place = placement(3000, 2000, PaperSize::F4, true);
    let folio = (215.0 * 72.0 / 25.4, 330.0 * 72.0 / 25.4);
    assert!(close(place.page.0, folio.1) && close(place.page.1, folio.0), "{place:?}");
}

#[test]
fn without_a_margin_the_photo_reaches_the_edge() {
    let place = placement(1000, 2000, PaperSize::A4, false);
    assert!(close(place.photo.3, A4.1), "{place:?}");
}

#[test]
fn match_photo_size_gives_each_page_the_photo_shape() {
    let place = placement(1200, 900, PaperSize::Photo, true);
    assert!(close(place.page.0, A4.1) && close(place.page.1, A4.1 * 0.75), "{place:?}");
    assert_eq!(place.photo, (0.0, 0.0, place.page.0, place.page.1));
}

#[test]
fn every_photo_becomes_a_page_in_order_and_keeps_its_jpg() {
    let photos = vec![photo(400, 600), photo(600, 400)];
    let first_jpeg = photos[0].jpeg.clone();
    let dir = std::env::temp_dir().join(format!("rantakit-photo-pages-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let output = dir.join("scan.pdf");
    let _ = std::fs::remove_file(&output);

    let pdf = PdfDocument::from_photos(photos, PaperSize::A4, true).unwrap();
    let cancel = CancelToken::new();
    let mut progress = |_| {};
    pdf.save(&output, &mut Tracker::new(&mut progress, &cancel)).unwrap();

    let doc = Document::load(&output).unwrap();
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    assert_eq!(pages.len(), 2);
    let media_box = |id| -> Vec<f32> {
        let page = doc.get_dictionary(id).unwrap();
        page.get(b"MediaBox").unwrap().as_array().unwrap().iter().map(|n| n.as_float().unwrap()).collect()
    };
    assert!(media_box(pages[0])[3] > media_box(pages[0])[2], "the first page is upright");
    assert!(media_box(pages[1])[2] > media_box(pages[1])[3], "the second page is sideways");

    let images: Vec<_> = doc
        .objects
        .values()
        .filter_map(|object| match object {
            Object::Stream(stream) if stream.dict.get(b"Subtype").and_then(|s| s.as_name()).ok() == Some(b"Image") => {
                Some(stream)
            }
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2);
    assert!(images.iter().any(|image| image.content == first_jpeg), "the JPG goes in as it is");
    std::fs::remove_dir_all(&dir).unwrap();
}
