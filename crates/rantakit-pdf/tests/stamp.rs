//! Text over every page, as a reader shows each page.

use lopdf::{Document, Object, Stream, dictionary};
use rantakit_pdf::{CancelToken, PdfDocument, PdfPages, StampFont, StampLine, StampStyle, Tracker};

const FONT_FILE: &[u8] = include_bytes!("../../rantakit-image/fonts/PlusJakartaSans-Bold.ttf");

fn font() -> StampFont<'static> {
    static WIDTHS: [u16; 224] = [650; 224];
    StampFont {
        name: "PlusJakartaSans-Bold",
        file: FONT_FILE,
        widths: &WIDTHS,
        ascent: 1038,
        descent: -222,
        cap_height: 745,
        bbox: [-80, -237, 1193, 1041],
    }
}

/// A 2-page PDF: an upright A4 page whose content is `(page 1)`, and the
/// same page turned a quarter clockwise, with its resources inline.
fn two_pages() -> PdfDocument {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let kids: Vec<Object> = [0, 90]
        .iter()
        .enumerate()
        .map(|(at, &turn)| {
            let content = format!("2 0 0 2 0 0 cm BT (page {}) Tj ET", at + 1);
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                "Rotate" => turn,
                "Resources" => dictionary! {},
            })
            .into()
        })
        .collect();
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    PdfDocument::from_bytes(&bytes).unwrap()
}

fn saved(pdf: PdfDocument, name: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join("rantakit-pdf-stamp");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let _ = std::fs::remove_file(&path);
    let cancel = CancelToken::new();
    let mut progress = |_| {};
    pdf.save(&path, &mut Tracker::new(&mut progress, &cancel))
        .unwrap();
    std::fs::read(&path).unwrap()
}

/// The dark pixels of a rendered page: their bounds, as shares of its size.
fn ink(page: &image::RgbImage) -> Option<[f32; 4]> {
    let (w, h) = page.dimensions();
    let dark: Vec<(u32, u32)> = page
        .enumerate_pixels()
        .filter(|(_, _, p)| p[0] < 128)
        .map(|(x, y, _)| (x, y))
        .collect();
    let min_x = dark.iter().map(|d| d.0).min()?;
    let max_x = dark.iter().map(|d| d.0).max()?;
    let min_y = dark.iter().map(|d| d.1).min()?;
    let max_y = dark.iter().map(|d| d.1).max()?;
    Some([
        min_x as f32 / w as f32,
        min_y as f32 / h as f32,
        max_x as f32 / w as f32,
        max_y as f32 / h as f32,
    ])
}

#[test]
fn text_goes_where_a_reader_sees_it_on_upright_and_turned_pages() {
    let mut pdf = two_pages();
    let mut sizes = Vec::new();
    pdf.stamp(
        &font(),
        StampStyle {
            color: [0, 0, 0],
            opacity: 1.0,
        },
        &mut |w, h| {
            sizes.push((w, h));
            // A straight line near the bottom right corner of the page as shown.
            vec![StampLine {
                text: b"HHHHHH".to_vec(),
                size: 40.0,
                x: w - 260.0,
                y: h - 40.0,
                angle: 0.0,
            }]
        },
    )
    .unwrap();
    assert_eq!(sizes, [(595.0, 842.0), (842.0, 595.0)]);

    let bytes = saved(pdf, "turned.pdf");
    let pages = PdfPages::from_bytes(bytes.clone()).unwrap();
    for index in 0..2 {
        let page = pages.render(index, 600).unwrap();
        let [left, top, right, bottom] =
            ink(&page).unwrap_or_else(|| panic!("page {} has no text", index + 1));
        assert!(left > 0.5 && top > 0.8, "page {}: {left} {top}", index + 1);
        assert!(right < 1.0 && bottom < 1.0);
        // Straight, so wider than tall as shown.
        let (w, h) = page.dimensions();
        assert!(
            (right - left) * w as f32 > 3.0 * (bottom - top) * h as f32,
            "page {}",
            index + 1
        );
    }

    // Each page's own drawing is still there, kept apart in q … Q.
    let doc = Document::load_mem(&bytes).unwrap();
    for (n, page_id) in doc.get_pages() {
        let content = String::from_utf8_lossy(&doc.get_page_content(page_id)).into_owned();
        assert!(content.starts_with("q\n"), "{content}");
        assert!(content.contains(&format!("(page {n}) Tj")));
        assert!(content.contains("<484848484848> Tj"));
        let fonts = doc.get_page_fonts(page_id).unwrap();
        assert!(fonts.values().any(
            |font| font.get(b"BaseFont").unwrap().as_name().unwrap() == b"PlusJakartaSans-Bold"
        ));
    }
}

#[test]
fn slanted_faint_text_shows_faint() {
    let mut pdf = two_pages();
    pdf.stamp(
        &font(),
        StampStyle {
            color: [0, 0, 0],
            opacity: 0.3,
        },
        &mut |w, h| {
            vec![StampLine {
                text: b"HHHHHHHH".to_vec(),
                size: 60.0,
                x: w / 2.0 - 200.0,
                y: h / 2.0 + 100.0,
                angle: 30.0,
            }]
        },
    )
    .unwrap();
    let pages = PdfPages::from_bytes(saved(pdf, "faint.pdf")).unwrap();
    let page = pages.render(0, 600).unwrap();
    let darkest = page.pixels().map(|p| p[0]).min().unwrap();
    // 30% black over white.
    assert!((165..=190).contains(&darkest), "{darkest}");
    // Rising to the right: the leftmost ink is lower than the rightmost.
    let ink_at = |column: u32| (0..page.height()).find(|&y| page.get_pixel(column, y)[0] < 230);
    let marked: Vec<u32> = (0..page.width()).filter(|&x| ink_at(x).is_some()).collect();
    let (first, last) = (marked[0], *marked.last().unwrap());
    assert!(ink_at(first).unwrap() > ink_at(last).unwrap());
}
