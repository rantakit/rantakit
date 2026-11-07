use std::path::PathBuf;

use lopdf::{Document, EncryptionState, EncryptionVersion, Object, Permissions, Stream, dictionary};
use rantakit_pdf::{CancelToken, ErrorKind, PageEdit, PdfDocument, PdfPages, Tracker};

/// A PDF of `count` pages whose content says which PDF and page it is, such
/// as `(cv 2)`. The page size and turn sit on the page tree, not the pages,
/// as many scanners write them.
fn marked(name: &str, count: usize, rotate: i64) -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let kids: Vec<Object> = (1..=count)
        .map(|n| {
            let content = format!("0 g 0 0 100 100 re f BT ({name} {n}) Tj ET");
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
            })
            .into()
        })
        .collect();
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count as i64,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Rotate" => rotate,
            "Resources" => dictionary! {},
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc
}

fn bytes(mut doc: Document) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn open(doc: Document) -> PdfDocument {
    PdfDocument::from_bytes(&bytes(doc)).unwrap()
}

/// Saves `pdf` to a new file and reads it back.
fn saved(pdf: PdfDocument, name: &str) -> (Document, Vec<u8>) {
    let dir = std::env::temp_dir().join("rantakit-pdf-page-edits");
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join(name);
    let _ = std::fs::remove_file(&path);
    let cancel = CancelToken::new();
    let mut progress = |_| {};
    let size = pdf.save(&path, &mut Tracker::new(&mut progress, &cancel)).unwrap();
    let written = std::fs::read(&path).unwrap();
    assert_eq!(size, written.len() as u64);
    (Document::load_mem(&written).unwrap(), written)
}

/// Each page's marker, in order, with its turn and whether it has a size.
fn pages_of(doc: &Document) -> Vec<(String, i64, bool)> {
    doc.get_pages()
        .into_values()
        .map(|id| {
            let content = String::from_utf8_lossy(&doc.get_page_content(id)).into_owned();
            let marker = content.split('(').nth(1).and_then(|s| s.split(')').next()).unwrap_or("").to_string();
            let page = doc.get_dictionary(id).unwrap();
            let rotate = page.get(b"Rotate").and_then(Object::as_i64).unwrap_or(0);
            (marker, rotate, page.has(b"MediaBox"))
        })
        .collect()
}

fn markers(doc: &Document) -> Vec<String> {
    pages_of(doc).into_iter().map(|(marker, ..)| marker).collect()
}

#[test]
fn merging_keeps_every_page_in_order() {
    let merged = PdfDocument::merge(vec![open(marked("cv", 2, 0)), open(marked("ijazah", 3, 0))]).unwrap();
    assert_eq!(merged.page_count(), 5);

    let (doc, written) = saved(merged, "merged.pdf");
    assert_eq!(markers(&doc), ["cv 1", "cv 2", "ijazah 1", "ijazah 2", "ijazah 3"]);
    assert!(pages_of(&doc).iter().all(|(_, _, sized)| *sized), "pages keep their size");
    assert_eq!(PdfPages::from_bytes(written).unwrap().count(), 5);
}

#[test]
fn taking_pages_keeps_only_those() {
    let pdf = open(marked("ijazah", 4, 0));
    let taken = pdf.arrange(&[PageEdit::keep(3), PageEdit::keep(1)]).unwrap();

    let (doc, _) = saved(taken, "taken.pdf");
    assert_eq!(markers(&doc), ["ijazah 4", "ijazah 2"]);
    let streams: Vec<String> = doc
        .objects
        .values()
        .filter_map(|object| object.as_stream().ok())
        .map(|stream| {
            let content = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());
            String::from_utf8_lossy(&content).into_owned()
        })
        .collect();
    assert!(streams.iter().any(|s| s.contains("ijazah 4")), "kept pages are there");
    assert!(
        !streams.iter().any(|s| s.contains("ijazah 1") || s.contains("ijazah 3")),
        "dropped pages are not in the file"
    );
    assert_eq!(pdf.page_count(), 4, "the open PDF is unchanged");
}

#[test]
fn turns_add_to_how_a_page_already_stands() {
    let pdf = open(marked("scan", 2, 90));
    let edited = pdf
        .arrange(&[
            PageEdit { page: 0, quarter_turns: 1 },
            PageEdit { page: 1, quarter_turns: 3 },
        ])
        .unwrap();

    let (doc, _) = saved(edited, "turned.pdf");
    let turns: Vec<i64> = pages_of(&doc).into_iter().map(|(_, rotate, _)| rotate).collect();
    assert_eq!(turns, [180, 0]);
}

#[test]
fn a_page_past_the_end_is_damaged() {
    let pdf = open(marked("cv", 2, 0));
    assert_eq!(pdf.arrange(&[PageEdit::keep(2)]).err(), Some(ErrorKind::Damaged));
    assert_eq!(pdf.arrange(&[]).err(), Some(ErrorKind::Damaged));
}

#[test]
fn a_locked_pdf_says_so() {
    let mut doc = marked("rahasia", 1, 0);
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
        PdfDocument::from_bytes(&bytes(doc)).err(),
        Some(ErrorKind::PasswordProtected)
    );
}

#[test]
fn rubbish_is_not_a_pdf() {
    assert_eq!(
        PdfDocument::from_bytes(b"bukan pdf").err(),
        Some(ErrorKind::UnsupportedFormat)
    );
}
