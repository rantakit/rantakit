//! Scan Dokumen on real photos, timed: find the paper, then make the page
//! straightened and cleaned as a PDF would get it.
//!
//!     cargo run --release -p rantakit-image --example scan_pages [--features ml] -- OUT_DIR PHOTO...
//!
//! Writes `OUT_DIR/<photo>.jpg` and prints, per photo, whether paper was
//! found and how long each step took.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rantakit_image::{PhotoEdit, find_paper, page_photo};

/// As rantakit-tasks' PAGE_PHOTO_SIDE.
const PAGE_SIDE: u32 = 2400;

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().expect("OUT_DIR PHOTO..."));
    std::fs::create_dir_all(&out_dir).unwrap();
    let mode = if cfg!(feature = "ml") { "ml" } else { "classic" };
    for photo in args {
        let original = std::fs::read(&photo).unwrap();
        let start = Instant::now();
        let corners = find_paper(&original).unwrap();
        let found_ms = start.elapsed().as_millis();
        let edit = PhotoEdit { corners, quarter_turns: 0, clean: true };
        let start = Instant::now();
        let page = page_photo(&original, &edit, PAGE_SIDE).unwrap();
        let page_ms = start.elapsed().as_millis();
        let name = Path::new(&photo).file_stem().unwrap().to_string_lossy();
        std::fs::write(out_dir.join(format!("{name}.jpg")), &page.jpeg).unwrap();
        println!(
            "{mode}\t{name}\t{}\tfind {found_ms} ms\tpage {page_ms} ms\t{}x{}\t{corners:.3?}",
            if corners.is_some() { "paper" } else { "whole" },
            page.width,
            page.height
        );
    }
}
