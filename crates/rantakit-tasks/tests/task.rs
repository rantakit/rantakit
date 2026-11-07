use std::fs;
use std::path::{Path, PathBuf};

use ::image::codecs::jpeg::JpegEncoder;
use ::image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use lopdf::{Document, Object, Stream, dictionary};
use rantakit_tasks::{
    Availability, CATALOGUE, CancelToken, Details, ErrorKind, FileOutcome, ImageFormat,
    ImageOptions, OutputFormat, PageEdit, PaperSize, PdfOptions, PhotoEdit, PhotosToPdf, Platform, Split, Target, Task, TaskEvent,
    ToolId, ToolOptions, WHITE, detect_kind, file_info, file_info_of, run_task, tools_for_files,
};

/// A noisy 600x400 photo, so compressing it really makes it smaller.
fn photo_jpg() -> Vec<u8> {
    let mut seed = 7u32;
    let img = RgbImage::from_fn(600, 400, |x, y| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = (seed >> 24) as u8 / 4;
        Rgb([(x / 4) as u8 + noise, (y / 3) as u8 + noise, 128 + noise])
    });
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 95)
        .write_image(img.as_raw(), 600, 400, ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

/// A one-page PDF that draws `photo_jpg` as an image.
fn scan_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let image_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 600,
            "Height" => 400,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode",
        },
        photo_jpg(),
    ));
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        b"q 600 0 0 400 0 0 cm /Im0 Do Q".to_vec(),
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 600.into(), 400.into()],
        "Contents" => content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn kompres_foto(target: Target) -> ToolOptions {
    ToolOptions::KompresFoto(ImageOptions {
        format: OutputFormat::Same,
        target,
        max_dimension: None,
        background: WHITE,
    })
}

fn kompres_pdf() -> ToolOptions {
    ToolOptions::KompresPdf(PdfOptions {
        quality: 50,
        max_dimension: None,
        max_bytes: None,
    })
}

/// An Original each Tool accepts. The match has no catch-all, so a new Tool
/// does not compile until it gets a fixture and is covered by the tests below.
fn original_for(tool: &ToolOptions) -> (&'static str, Vec<u8>) {
    match tool {
        ToolOptions::KompresFoto(_) => ("foto.jpg", photo_jpg()),
        ToolOptions::KompresPdf(_) => ("ijazah.pdf", scan_pdf()),
        ToolOptions::GabungPdf { .. } | ToolOptions::PisahPdf(_) | ToolOptions::AturHalamanPdf(_) => {
            ("ijazah.pdf", pages_pdf(3))
        }
        ToolOptions::UbahFormatFoto(_) | ToolOptions::UbahUkuranFoto(_) | ToolOptions::JpgKePdf(_) => {
            ("foto.jpg", photo_jpg())
        }
    }
}

/// Every Tool, with the options that exercise its different paths.
fn every_tool() -> Vec<ToolOptions> {
    vec![
        kompres_foto(Target::Quality(60)),
        kompres_foto(Target::MaxBytes(20_000)),
        ToolOptions::KompresFoto(ImageOptions {
            format: OutputFormat::To(ImageFormat::Png),
            target: Target::Quality(80),
            max_dimension: Some(300),
            background: WHITE,
        }),
        kompres_pdf(),
        ToolOptions::convert(ImageFormat::Png, 90, WHITE),
        ToolOptions::resize(300),
        gabung_pdf(),
        ToolOptions::PisahPdf(Split::Take(vec![1, 3])),
        ToolOptions::PisahPdf(Split::Every(1)),
        ToolOptions::AturHalamanPdf(vec![
            PageEdit {
                page: 2,
                quarter_turns: 1,
            },
            PageEdit::keep(0),
        ]),
        jpg_ke_pdf(PaperSize::A4, vec![]),
        jpg_ke_pdf(
            PaperSize::Photo,
            vec![PhotoEdit {
                corners: Some([[0.1, 0.1], [0.9, 0.1], [0.9, 0.9], [0.1, 0.9]]),
                quarter_turns: 1,
            }],
        ),
    ]
}

fn gabung_pdf() -> ToolOptions {
    ToolOptions::GabungPdf {
        date: "2026-11-07".into(),
    }
}

fn jpg_ke_pdf(paper: PaperSize, edits: Vec<PhotoEdit>) -> ToolOptions {
    ToolOptions::JpgKePdf(PhotosToPdf {
        date: "2026-11-07".into(),
        paper,
        margin: true,
        edits,
    })
}

/// A PDF of `count` plain pages.
fn pages_pdf(count: usize) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let kids: Vec<Object> = (1..=count)
        .map(|n| {
            let content = format!("BT (halaman {n}) Tj ET").into_bytes();
            let content_id = doc.add_object(Stream::new(dictionary! {}, content));
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
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
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

/// How many pages the PDF at `path` has.
fn pages_in(path: &Path) -> usize {
    Document::load(path).unwrap().get_pages().len()
}

/// A fresh, empty folder for one test.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rantakit-tasks-tests").join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn task(tool: ToolOptions, originals: Vec<PathBuf>) -> Task {
    Task {
        tool,
        originals,
        results_folder: None,
    }
}

fn run(task: &Task) -> Vec<FileOutcome> {
    run_task(task, &mut |_| {}, &CancelToken::new())
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn the_original_is_never_modified_moved_or_overwritten() {
    for (n, tool) in every_tool().into_iter().enumerate() {
        let dir = scratch(&format!("original_untouched_{n}"));
        let (name, bytes) = original_for(&tool);
        let original = dir.join(name);
        fs::write(&original, &bytes).unwrap();
        let modified = fs::metadata(&original).unwrap().modified().unwrap();

        // Twice, so the second Result has to find a free name next to the first.
        for _ in 0..2 {
            let outcomes = run(&task(tool.clone(), vec![original.clone()]));
            let result = outcomes[0].as_ref().unwrap();
            assert_ne!(result.path, original);
        }

        assert_eq!(
            fs::read(&original).unwrap(),
            bytes,
            "{tool:?} changed the Original"
        );
        assert_eq!(
            fs::metadata(&original).unwrap().modified().unwrap(),
            modified,
            "{tool:?} touched the Original"
        );
        assert_eq!(file_names(&dir).len(), 3, "the Original and two Results");
    }
}

#[test]
fn results_get_new_names_next_to_the_original() {
    let dir = scratch("names");
    let foto = dir.join("foto.jpeg");
    fs::write(&foto, photo_jpg()).unwrap();
    let ijazah = dir.join("ijazah.pdf");
    fs::write(&ijazah, scan_pdf()).unwrap();

    let photos = task(kompres_foto(Target::Quality(60)), vec![foto]);
    let first = run(&photos).remove(0).unwrap();
    let second = run(&photos).remove(0).unwrap();
    let pdf = run(&task(kompres_pdf(), vec![ijazah])).remove(0).unwrap();

    assert_eq!(first.path, dir.join("foto (compressed).jpg"));
    assert_eq!(second.path, dir.join("foto (compressed) (2).jpg"));
    assert_eq!(pdf.path, dir.join("ijazah (compressed).pdf"));
    assert!(matches!(pdf.details, Details::Pdf { images: 1, .. }));
}

#[test]
fn a_failed_file_does_not_stop_the_others() {
    let dir = scratch("failed_file");
    let good = dir.join("a.jpg");
    let broken = dir.join("b.jpg");
    let missing = dir.join("c.jpg");
    let also_good = dir.join("d.jpg");
    fs::write(&good, photo_jpg()).unwrap();
    fs::write(&broken, b"not a photo").unwrap();
    fs::write(&also_good, photo_jpg()).unwrap();

    let mut done = Vec::new();
    let outcomes = run_task(
        &task(
            kompres_foto(Target::Quality(60)),
            vec![good, broken, missing, also_good],
        ),
        &mut |event| {
            if let TaskEvent::FileDone { file, .. } = event {
                done.push(file);
            }
        },
        &CancelToken::new(),
    );

    assert!(outcomes[0].is_ok());
    assert_eq!(outcomes[1], Err(ErrorKind::UnsupportedFormat));
    assert_eq!(outcomes[2], Err(ErrorKind::OriginalMissing));
    assert!(outcomes[3].is_ok());
    assert_eq!(
        done,
        [0, 1, 2, 3],
        "every file reports how it ended, in order"
    );
}

#[test]
fn results_go_into_the_chosen_folder() {
    let dir = scratch("results_folder");
    let chosen = dir.join("Hasil");
    fs::create_dir_all(&chosen).unwrap();
    let foto = dir.join("foto.jpg");
    fs::write(&foto, photo_jpg()).unwrap();

    let outcomes = run(&Task {
        tool: kompres_foto(Target::Quality(60)),
        originals: vec![foto],
        results_folder: Some(chosen.clone()),
    });

    assert_eq!(
        outcomes[0].as_ref().unwrap().path,
        chosen.join("foto (compressed).jpg")
    );
    assert_eq!(file_names(&dir), ["Hasil", "foto.jpg"]);
}

#[test]
fn a_folder_that_cannot_be_written_to_ends_with_cannot_write() {
    let dir = scratch("cannot_write");
    let foto = dir.join("foto.jpg");
    fs::write(&foto, photo_jpg()).unwrap();

    let outcomes = run(&Task {
        tool: kompres_foto(Target::Quality(60)),
        originals: vec![foto],
        results_folder: Some(dir.join("does not exist")),
    });

    assert_eq!(outcomes[0], Err(ErrorKind::CannotWrite));
    assert_eq!(file_names(&dir), ["foto.jpg"]);
}

#[test]
fn cancel_keeps_finished_results_and_skips_the_rest() {
    let dir = scratch("cancel");
    let originals: Vec<PathBuf> = ["a.jpg", "b.jpg", "c.jpg"]
        .iter()
        .map(|name| {
            let path = dir.join(name);
            fs::write(&path, photo_jpg()).unwrap();
            path
        })
        .collect();
    let cancel = CancelToken::new();

    let outcomes = run_task(
        &task(kompres_foto(Target::MaxBytes(20_000)), originals),
        &mut |event| match event {
            // Cancel while the second file is in its size search.
            TaskEvent::Progress { file: 1, .. } => cancel.cancel(),
            TaskEvent::Progress { file, .. } => assert!(file == 0, "file {file} ran after cancel"),
            TaskEvent::FileDone { .. } => {}
        },
        &cancel,
    );

    assert!(outcomes[0].is_ok());
    assert_eq!(outcomes[1], Err(ErrorKind::Cancelled));
    assert_eq!(outcomes[2], Err(ErrorKind::Cancelled));
    assert_eq!(
        file_names(&dir),
        ["a (compressed).jpg", "a.jpg", "b.jpg", "c.jpg"]
    );
}

#[test]
fn every_tool_that_runs_is_shipped_and_accepts_its_original() {
    let dir = scratch("catalogue_matches_tasks");
    for tool in every_tool() {
        let (name, bytes) = original_for(&tool);
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();
        let facts = CATALOGUE.iter().find(|t| t.id == tool.id()).unwrap();

        assert_eq!(facts.availability, Availability::Shipped, "{tool:?}");
        assert!(
            facts.accepts.contains(&detect_kind(&path).unwrap()),
            "{tool:?}"
        );
    }
}

#[test]
fn dropped_files_fit_tools_by_content_not_extension() {
    let dir = scratch("tools_for_files");
    let photo_named_pdf = dir.join("ktp.pdf");
    let png = dir.join("pas foto.png");
    let pdf = dir.join("ijazah.pdf");
    let text = dir.join("catatan.txt");
    fs::write(&photo_named_pdf, photo_jpg()).unwrap();
    fs::write(&png, b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(&pdf, scan_pdf()).unwrap();
    fs::write(&text, b"bukan foto").unwrap();
    let on = Platform::current();

    assert_eq!(
        tools_for_files(&[&photo_named_pdf, &png], on),
        [
            ToolId::KompresFoto,
            ToolId::UbahFormatFoto,
            ToolId::UbahUkuranFoto,
            ToolId::JpgKePdf
        ]
    );
    assert_eq!(
        tools_for_files(&[&pdf], on),
        [ToolId::KompresPdf, ToolId::PisahPdf, ToolId::AturHalamanPdf]
    );
    assert!(tools_for_files(&[&photo_named_pdf, &pdf], on).is_empty());
    assert!(tools_for_files(&[&pdf, &text], on).is_empty());
    assert!(tools_for_files(&[dir.join("hilang.jpg")], on).is_empty());
}

#[test]
fn a_photo_already_in_the_chosen_format_is_left_unchanged() {
    let dir = scratch("unchanged_format");
    let foto = dir.join("foto.jpg");
    fs::write(&foto, photo_jpg()).unwrap();

    let to_jpg = task(
        ToolOptions::convert(ImageFormat::Jpeg, 90, WHITE),
        vec![foto.clone()],
    );
    assert_eq!(run(&to_jpg), [Err(ErrorKind::Unchanged)]);
    assert_eq!(
        file_names(&dir).len(),
        1,
        "no Result for an unchanged photo"
    );

    let to_png = task(
        ToolOptions::convert(ImageFormat::Png, 90, WHITE),
        vec![foto],
    );
    let result = run(&to_png).remove(0).unwrap();
    assert!(result.path.ends_with("foto (converted).png"));
}

#[test]
fn photos_are_made_smaller_but_never_larger() {
    let dir = scratch("resize");
    let foto = dir.join("foto.jpg");
    fs::write(&foto, photo_jpg()).unwrap();

    let already_small = task(ToolOptions::resize(800), vec![foto.clone()]);
    assert_eq!(run(&already_small), [Err(ErrorKind::Unchanged)]);
    assert_eq!(file_names(&dir).len(), 1, "no Result for a small photo");

    let result = run(&task(ToolOptions::resize(300), vec![foto]))
        .remove(0)
        .unwrap();
    assert!(result.path.ends_with("foto (resized).jpg"));
    match result.details {
        Details::Photo { width, height, .. } => assert_eq!((width, height), (300, 200)),
        other => panic!("expected a photo, got {other:?}"),
    }
}

#[test]
fn photo_sizes_are_plain_longest_sides() {
    use rantakit_tasks::PhotoSize;
    assert_eq!(PhotoSize::default(), PhotoSize::Medium);
    assert_eq!(
        [PhotoSize::Large, PhotoSize::Medium, PhotoSize::Small].map(PhotoSize::side),
        [1920, 1280, 800]
    );
}

#[test]
fn an_open_file_tells_as_much_as_its_path() {
    // Android hands over chosen files as open files without a path.
    let dir = scratch("open-file-info");
    for (name, bytes) in [("foto.jpg", photo_jpg()), ("scan.pdf", scan_pdf())] {
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();

        let info = file_info_of(fs::File::open(&path).unwrap());
        assert_eq!(info, file_info(&path), "{name}");
        assert!(info.pages.is_some() || info.longest_side.is_some(), "{name}");
    }
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn merging_makes_one_result_from_every_file_in_order() {
    let dir = scratch("merge_in_order");
    let cv = write(&dir, "cv.pdf", &pages_pdf(2));
    let ijazah = write(&dir, "ijazah.pdf", &pages_pdf(3));

    let outcomes = run(&task(gabung_pdf(), vec![cv.clone(), ijazah.clone()]));

    let merged = dir.join("merged 2026-11-07.pdf");
    for (outcome, original) in outcomes.iter().zip([&cv, &ijazah]) {
        let result = outcome.as_ref().unwrap();
        assert_eq!(result.path, merged, "one Result for every file");
        assert_eq!(result.original_size, fs::metadata(original).unwrap().len());
        assert_eq!(
            result.details,
            Details::Pages {
                pages: 5,
                skipped: vec![]
            }
        );
    }
    assert_eq!(pages_in(&merged), 5);
    assert_eq!(file_names(&dir), ["cv.pdf", "ijazah.pdf", "merged 2026-11-07.pdf"]);
}

#[test]
fn a_file_that_cannot_be_read_stops_a_merge_before_anything_is_written() {
    let dir = scratch("merge_stops");
    let cv = write(&dir, "cv.pdf", &pages_pdf(2));
    let broken = write(&dir, "rusak.pdf", b"%PDF-1.5 bukan isi pdf");
    let ijazah = write(&dir, "ijazah.pdf", &pages_pdf(1));

    let outcomes = run(&task(gabung_pdf(), vec![cv, broken, ijazah]));

    assert_eq!(
        outcomes.into_iter().map(Result::err).collect::<Vec<_>>(),
        [
            Some(ErrorKind::Cancelled),
            Some(ErrorKind::Damaged),
            Some(ErrorKind::Cancelled)
        ]
    );
    assert_eq!(file_names(&dir), ["cv.pdf", "ijazah.pdf", "rusak.pdf"]);
}

#[test]
fn photos_become_one_pdf_a_page_each_in_order() {
    let dir = scratch("photos_to_pdf");
    let ktp = write(&dir, "ktp.jpg", &photo_jpg());
    let kk = write(&dir, "kk.jpg", &photo_jpg());
    let edits = vec![
        PhotoEdit::default(),
        PhotoEdit {
            corners: Some([[0.3, 0.1], [0.7, 0.1], [0.7, 0.9], [0.3, 0.9]]),
            quarter_turns: 0,
        },
    ];

    let outcomes = run(&task(jpg_ke_pdf(PaperSize::A4, edits), vec![ktp.clone(), kk.clone()]));

    let scan = dir.join("scan 2026-11-07.pdf");
    for (outcome, original) in outcomes.iter().zip([&ktp, &kk]) {
        let result = outcome.as_ref().unwrap();
        assert_eq!(result.path, scan, "one Result for every photo");
        assert_eq!(result.original_size, fs::metadata(original).unwrap().len());
        assert_eq!(result.details, Details::Pages { pages: 2, skipped: vec![] });
    }
    let doc = Document::load(&scan).unwrap();
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    let media_box = |id| -> Vec<f32> {
        let page = doc.get_dictionary(id).unwrap();
        page.get(b"MediaBox").unwrap().as_array().unwrap().iter().map(|n| n.as_float().unwrap()).collect()
    };
    assert!(media_box(pages[0])[2] > media_box(pages[0])[3], "the wide photo lies sideways");
    assert!(media_box(pages[1])[3] > media_box(pages[1])[2], "its tall crop stands upright");
    assert_eq!(file_names(&dir), ["kk.jpg", "ktp.jpg", "scan 2026-11-07.pdf"]);
}

#[test]
fn a_photo_that_cannot_be_read_stops_the_pdf_before_anything_is_written() {
    let dir = scratch("photos_to_pdf_stops");
    let ktp = write(&dir, "ktp.jpg", &photo_jpg());
    let broken = write(&dir, "rusak.jpg", b"bukan foto");

    let outcomes = run(&task(jpg_ke_pdf(PaperSize::F4, vec![]), vec![ktp, broken]));

    assert_eq!(
        outcomes.into_iter().map(Result::err).collect::<Vec<_>>(),
        [Some(ErrorKind::Cancelled), Some(ErrorKind::UnsupportedFormat)]
    );
    assert_eq!(file_names(&dir), ["ktp.jpg", "rusak.jpg"]);
}

#[test]
fn taking_pages_skips_the_ones_a_pdf_does_not_have() {
    let dir = scratch("take_pages");
    let original = write(&dir, "ijazah.pdf", &pages_pdf(3));
    let take = |pages: Vec<usize>| {
        run(&task(ToolOptions::PisahPdf(Split::Take(pages)), vec![original.clone()])).remove(0)
    };

    let result = take(vec![2, 5]).unwrap();
    assert_eq!(result.path, dir.join("ijazah (page 2).pdf"));
    assert_eq!(
        result.details,
        Details::Pages {
            pages: 1,
            skipped: vec![5]
        }
    );
    assert_eq!(pages_in(&result.path), 1);

    let result = take(vec![1, 2, 3, 2]).unwrap();
    assert_eq!(result.path, dir.join("ijazah (pages 1-3, 2).pdf"));
    assert_eq!(pages_in(&result.path), 4);

    assert_eq!(take(vec![7, 8]).err(), Some(ErrorKind::NoSuchPages));
    assert_eq!(take(vec![1, 2, 3]).err(), Some(ErrorKind::Unchanged));
}

#[test]
fn splitting_makes_a_folder_of_parts() {
    let dir = scratch("split_every");
    let original = write(&dir, "ijazah.pdf", &pages_pdf(3));
    let split = |every| {
        run(&task(ToolOptions::PisahPdf(Split::Every(every)), vec![original.clone()])).remove(0)
    };

    let result = split(1).unwrap();
    assert_eq!(result.path, dir.join("ijazah (pages)"));
    assert_eq!(
        file_names(&result.path),
        ["ijazah (page 1).pdf", "ijazah (page 2).pdf", "ijazah (page 3).pdf"]
    );
    let Details::Folder { files } = &result.details else {
        panic!("a folder Result");
    };
    assert_eq!(result.size, files.iter().map(|file| file.size).sum::<u64>());

    let result = split(2).unwrap();
    assert_eq!(result.path, dir.join("ijazah (pages) (2)"));
    assert_eq!(
        file_names(&result.path),
        ["ijazah (page 3).pdf", "ijazah (pages 1-2).pdf"]
    );

    assert_eq!(split(3).err(), Some(ErrorKind::Unchanged));
}

#[test]
fn a_cancelled_split_leaves_no_folder() {
    let dir = scratch("split_cancelled");
    let original = write(&dir, "ijazah.pdf", &pages_pdf(3));
    let cancel = CancelToken::new();
    let stopper = cancel.clone();
    let mut parts = 0;
    let outcomes = run_task(
        &task(ToolOptions::PisahPdf(Split::Every(1)), vec![original]),
        &mut |event| {
            if let TaskEvent::Progress { .. } = event {
                parts += 1;
                if parts == 2 {
                    stopper.cancel();
                }
            }
        },
        &cancel,
    );

    assert_eq!(outcomes[0].as_ref().err(), Some(&ErrorKind::Cancelled));
    assert_eq!(file_names(&dir), ["ijazah.pdf"]);
}

#[test]
fn arranging_pages_makes_an_edited_copy() {
    let dir = scratch("arrange_pages");
    let original = write(&dir, "ijazah.pdf", &pages_pdf(3));
    let arrange = |edits: Vec<PageEdit>| {
        run(&task(ToolOptions::AturHalamanPdf(edits), vec![original.clone()])).remove(0)
    };

    let result = arrange(vec![PageEdit::keep(2), PageEdit::keep(0)]).unwrap();
    assert_eq!(result.path, dir.join("ijazah (edited).pdf"));
    assert_eq!(pages_in(&result.path), 2);

    let as_it_is = (0..3).map(PageEdit::keep).collect();
    assert_eq!(arrange(as_it_is).err(), Some(ErrorKind::Unchanged));
}
