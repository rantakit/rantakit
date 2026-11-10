//! How long Kompres PDF takes on a 10-page scan, the speed bar in
//! docs/stories/v0.3.md (A4). Makes the scan, then compresses it with
//! "Kecilkan saja" (Seimbang) and with a 1 MB Size limit.
//!
//! On an Android emulator, build for its ABI, push the binary to
//! `/data/local/tmp`, and run it there:
//!
//! ```text
//! cargo build --release -p rantakit-tasks --example scan_speed --target x86_64-linux-android
//! adb push target/x86_64-linux-android/release/examples/scan_speed /data/local/tmp/
//! adb shell /data/local/tmp/scan_speed /data/local/tmp/scan-speed
//! ```

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use ::image::codecs::jpeg::JpegEncoder;
use ::image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use lopdf::{Document, Object, Stream, dictionary};
use rantakit_tasks::{CancelToken, Compression, Task, ToolOptions, run_task};

const PAGES: usize = 10;
/// An A4 page scanned at 300 dpi.
const WIDTH: u32 = 2480;
const HEIGHT: u32 = 3508;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("rantakit-scan-speed"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let original = dir.join("scan.pdf");
    fs::write(&original, scan_pdf()).unwrap();
    println!(
        "{PAGES}-page scan, {WIDTH}x{HEIGHT} per page: {}",
        megabytes(fs::metadata(&original).unwrap().len())
    );

    let runs = [
        ("Kecilkan saja (Seimbang)", Compression::default()),
        (
            "Size limit 1 MB",
            Compression {
                limit: Some(1024 * 1024),
                ..Default::default()
            },
        ),
    ];
    for (name, compression) in runs {
        let results = dir.join(name.replace(' ', "-"));
        fs::create_dir_all(&results).unwrap();
        let task = Task {
            tool: ToolOptions::CompressPdf(compression.pdf()),
            originals: vec![original.clone()],
            results_folder: Some(results),
        };
        let start = Instant::now();
        let outcome = run_task(&task, &mut |_| {}, &CancelToken::new()).remove(0);
        let seconds = start.elapsed().as_secs_f64();
        match outcome {
            Ok(result) => println!("{name}: {seconds:.2} s, {}", megabytes(result.size)),
            Err(error) => println!("{name}: failed after {seconds:.2} s: {error:?}"),
        }
    }
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

/// A page like a phone scan: off-white paper with sensor noise and rows of
/// dark "text", saved as a color JPEG.
fn scan_page(page: usize) -> Vec<u8> {
    let mut seed = 7 + page as u32;
    let img = RgbImage::from_fn(WIDTH, HEIGHT, |x, y| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = (seed >> 29) as u8;
        let in_text = y > 300 && y < HEIGHT - 300 && y % 70 < 28 && x > 250 && x < WIDTH - 250;
        let ink = in_text && !(x / 23 + y / 70 + page as u32).is_multiple_of(7);
        let paper = if ink { 40 } else { 220 };
        Rgb([paper + noise, paper + noise, paper - 8 + noise])
    });
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 85)
        .write_image(img.as_raw(), WIDTH, HEIGHT, ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

/// A PDF with one scanned page image per page, like a scanner app makes.
fn scan_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for page in 0..PAGES {
        let image_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => WIDTH,
                "Height" => HEIGHT,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            scan_page(page),
        ));
        // A4 in points.
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            b"q 595 0 0 842 0 0 cm /Im0 Do Q".to_vec(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
        });
        kids.push(page_id.into());
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => PAGES as i64,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}
