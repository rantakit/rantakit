use lopdf::{Document, EncryptionState, EncryptionVersion, Object, Permissions, Stream, dictionary};
use rantakit_pdf::{ErrorKind, PdfPages};

/// Two pages: an A4 portrait page with a black box in its top-left quarter
/// and a line of Helvetica, which PDFs often use without embedding it; then
/// a landscape page that is turned 90° when shown.
fn two_pages() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let page = |doc: &mut Document, media: [i64; 4], content: &[u8], rotate: i64| {
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.to_vec()));
        doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => media.iter().map(|&n| n.into()).collect::<Vec<Object>>(),
            "Rotate" => rotate,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        })
    };
    // PDF space starts bottom-left: this box covers the page's top-left quarter.
    let first = page(
        &mut doc,
        [0, 0, 595, 842],
        b"0 g 0 421 297 421 re f BT /F1 24 Tf 40 200 Td (Ijazah Sekolah Menengah) Tj ET",
        0,
    );
    let second = page(&mut doc, [0, 0, 842, 595], b"0 g 0 0 100 100 re f", 90);
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![first.into(), second.into()],
            "Count" => 2,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc
}

fn save(mut doc: Document) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn dark(pixel: &image::Rgb<u8>) -> bool {
    pixel.0.iter().all(|&c| c < 64)
}

#[test]
fn pages_are_drawn_on_white_within_the_size_asked() {
    let pages = PdfPages::from_bytes(save(two_pages())).unwrap();
    assert_eq!(pages.count(), 2);

    let first = pages.render(0, 200).unwrap();
    assert_eq!(first.height(), 200, "the longest side is the size asked");
    assert!((140..=142).contains(&first.width()), "A4 keeps its shape: {}", first.width());
    assert!(dark(first.get_pixel(10, 10)), "the box is in the top-left corner");
    assert_eq!(first.get_pixel(first.width() - 5, 5).0, [255, 255, 255], "the rest is white");

    // The text's baseline is 200 pt up a 842 pt page; look in the band above it.
    let baseline = (first.height() as f32 * (1.0 - 200.0 / 842.0)) as u32;
    let inked = (baseline - 5..baseline)
        .flat_map(|y| (0..first.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| first.get_pixel(x, y).0 != [255; 3])
        .count();
    assert!(inked > 0, "Helvetica text is drawn without being embedded");
}

#[test]
fn a_turned_page_is_drawn_as_a_reader_shows_it() {
    let pages = PdfPages::from_bytes(save(two_pages())).unwrap();
    let second = pages.render(1, 200).unwrap();
    assert!(second.height() > second.width(), "landscape turned 90° stands upright");
}

#[test]
fn thumbnails_are_small_jpgs() {
    let pages = PdfPages::from_bytes(save(two_pages())).unwrap();
    let jpg = pages.thumbnail(0, 160).unwrap();
    assert_eq!(&jpg[..3], &[0xFF, 0xD8, 0xFF]);
    assert!(jpg.len() < 20_000, "{} bytes", jpg.len());
}

#[test]
fn a_page_past_the_end_is_an_error_not_a_crash() {
    let pages = PdfPages::from_bytes(save(two_pages())).unwrap();
    assert_eq!(pages.render(2, 100).err(), Some(ErrorKind::Damaged));
}

#[test]
fn a_locked_pdf_says_so() {
    let mut doc = two_pages();
    doc.trailer.set(
        "ID",
        Object::Array(vec![
            Object::string_literal(b"0123456789abcdef"),
            Object::string_literal(b"fedcba9876543210"),
        ]),
    );
    let state = EncryptionState::try_from(EncryptionVersion::V2 {
        document: &doc,
        owner_password: "owner",
        user_password: "secret",
        key_length: 128,
        permissions: Permissions::all(),
    })
    .unwrap();
    doc.encrypt(&state).unwrap();

    assert_eq!(
        PdfPages::from_bytes(save(doc)).err(),
        Some(ErrorKind::PasswordProtected)
    );
}
