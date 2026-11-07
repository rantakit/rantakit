use std::borrow::Cow;
use std::io::Write;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use ::image::codecs::jpeg::JpegEncoder;
use ::image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat as RawFormat};
use lopdf::{Document, LoadOptions, Object, ObjectId, SaveOptions, Stream};
use rantakit_core::{
    CancelToken, ErrorKind, Progress, Result, Tracker, read_original, write_result,
};

use crate::resize::fit_within;

const MAX_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

/// Progress once the PDF is loaded, its images are decoded, and it is repacked.
const LOADED: f32 = 0.1;
const DECODED: f32 = 0.3;
const REPACKED: f32 = 0.9;

/// Lowest image quality tried for a Size limit.
const MIN_QUALITY: u8 = 10;
/// Qualities closer than this look the same, so the Size limit search stops there.
const QUALITY_STEP: u8 = 5;
/// Images decoded or encoded at once, one per core on a typical phone.
const MAX_THREADS: usize = 4;
/// Images shrunk at once. Shrinking one page holds about 90 MB of working
/// memory, so more at once would cost a low-end phone more than it has spare.
const RESIZE_THREADS: usize = 2;

/// Longest image sides tried, in order, when a Size limit can't be reached at
/// the original resolution: 300, 200, 150, 100 and 72 dpi on an A4 page.
const SHRINK_STEPS: [u32; 5] = [3508, 2339, 1754, 1170, 842];

#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// JPEG quality for images inside the PDF. With `max_bytes`, the highest
    /// quality tried.
    pub quality: u8,
    /// Longest side of images inside the PDF, in pixels.
    pub max_dimension: Option<u32>,
    /// A Size limit: search image quality, then resolution, until the PDF fits.
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CompressedPdf {
    /// Size of the Result in bytes.
    pub size: u64,
    pub images: usize,
    pub images_recompressed: usize,
}

/// Compresses the PDF at `input` and writes the Result to `output`, which
/// must not exist yet. Progress is a fraction, because the work goes image by
/// image, not page by page.
pub fn compress_pdf(
    input: &Path,
    output: &Path,
    opts: &PdfOptions,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> Result<CompressedPdf> {
    let mut tracker = Tracker::new(progress, cancel);

    let original = read_original(input)?;
    let doc = load_document(&original)?;
    drop(original);
    tracker.step(LOADED)?;

    let images = doc
        .objects
        .values()
        .filter(|object| matches!(object, Object::Stream(stream) if is_image(stream)))
        .count();
    let candidates = decode_images(&doc, &mut tracker)?;
    let quality = opts.quality.clamp(1, 100);

    let packed = match opts.max_bytes {
        None => {
            let images = at_side(&candidates, opts.max_dimension);
            let packed = pack(doc, &candidates, &images, quality)?;
            tracker.step(REPACKED)?;
            packed
        }
        Some(limit) => fit_limit(
            doc,
            &candidates,
            quality,
            opts.max_dimension,
            limit,
            &mut tracker,
        )?,
    };

    let size = write_result(output, &mut tracker, |file| file.write_all(&packed.bytes))?;
    Ok(CompressedPdf {
        size,
        images,
        images_recompressed: packed.recompressed,
    })
}

/// How many pages the PDF at `path` has, for the Files step.
pub fn page_count(path: &Path) -> Result<usize> {
    page_count_of(&read_original(path)?)
}

/// `page_count` of a PDF already in memory, such as one read from a file
/// Android handed over without a path.
pub fn page_count_of(original: &[u8]) -> Result<usize> {
    if !is_pdf(original) {
        return Err(ErrorKind::UnsupportedFormat);
    }
    let load = LoadOptions {
        max_decompressed_size: Some(MAX_DECOMPRESSED_BYTES),
        ..Default::default()
    };
    let doc = Document::load_mem_with_options(original, load).map_err(load_error)?;
    Ok(doc.get_pages().len())
}

/// An image the engine can recompress, decoded once for every attempt.
struct Candidate {
    id: ObjectId,
    image: DynamicImage,
    gray: bool,
    /// Masked images keep their size so the mask still lines up.
    masked: bool,
    original_len: usize,
}

fn decode_images(doc: &Document, tracker: &mut Tracker<'_>) -> Result<Vec<Candidate>> {
    let ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter(|(_, object)| matches!(object, Object::Stream(stream) if is_image(stream)))
        .map(|(id, _)| *id)
        .collect();
    let mut candidates = Vec::new();
    // A few at a time, so progress still moves and Cancel is heard between them.
    for (chunk, ids_now) in ids.chunks(MAX_THREADS).enumerate() {
        let done = chunk * MAX_THREADS;
        tracker.step(LOADED + (DECODED - LOADED) * done as f32 / ids.len() as f32)?;
        candidates.extend(
            each_at_once(ids_now, MAX_THREADS, |id| {
                let Some(Object::Stream(stream)) = doc.objects.get(id) else {
                    return None;
                };
                let (image, gray) = decode_jpeg(stream)?;
                Some(Candidate {
                    id: *id,
                    image,
                    gray,
                    masked: stream.dict.has(b"SMask") || stream.dict.has(b"Mask"),
                    original_len: stream.content.len(),
                })
            })
            .into_iter()
            .flatten(),
        );
    }
    tracker.step(DECODED)?;
    Ok(candidates)
}

struct Packed {
    bytes: Vec<u8>,
    recompressed: usize,
}

/// Each candidate's image with its longest side at most `max_dimension`, made
/// once for every quality tried at that size. Masked images keep their size.
fn at_side(candidates: &[Candidate], max_dimension: Option<u32>) -> Vec<Cow<'_, DynamicImage>> {
    let Some(max) = max_dimension else {
        return candidates.iter().map(|c| Cow::Borrowed(&c.image)).collect();
    };
    each_at_once(candidates, RESIZE_THREADS, |candidate| {
        if candidate.masked {
            Cow::Borrowed(&candidate.image)
        } else {
            fit_within(&candidate.image, max)
        }
    })
}

/// `work` on every item, up to `max_threads` at once, in the items' order.
/// Images are independent of each other, so a phone's cores share them.
fn each_at_once<'a, T: Sync, R: Send>(
    items: &'a [T],
    max_threads: usize,
    work: impl Fn(&'a T) -> R + Sync,
) -> Vec<R> {
    let threads = thread::available_parallelism()
        .map_or(1, NonZero::get)
        .min(max_threads)
        .min(items.len());
    if threads <= 1 {
        return items.iter().map(work).collect();
    }
    // Each thread takes the next item when it is free, since pages differ in size.
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, R)> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else {
                            return done;
                        };
                        done.push((index, work(item)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("image worker panicked"))
            .collect()
    });
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, result)| result).collect()
}

/// The whole PDF with every candidate's image, from `at_side`, re-encoded at
/// `quality`, wherever that makes the image smaller.
fn pack(
    mut doc: Document,
    candidates: &[Candidate],
    images: &[Cow<'_, DynamicImage>],
    quality: u8,
) -> Result<Packed> {
    let pairs: Vec<_> = candidates.iter().zip(images).collect();
    let encoded = each_at_once(&pairs, MAX_THREADS, |(candidate, image)| {
        encode_jpeg(image, quality, candidate.gray)
            .filter(|bytes| bytes.len() < candidate.original_len)
    });
    let mut recompressed = 0;
    for ((candidate, image), bytes) in pairs.into_iter().zip(encoded) {
        let Some(bytes) = bytes else {
            continue;
        };
        let Some(Object::Stream(stream)) = doc.objects.get_mut(&candidate.id) else {
            continue;
        };
        stream.dict.set("Width", image.width() as i64);
        stream.dict.set("Height", image.height() as i64);
        stream.dict.set("BitsPerComponent", 8);
        stream.dict.remove(b"DecodeParms");
        stream.set_content(bytes);
        recompressed += 1;
    }

    let bytes = save_document(doc)?;
    Ok(Packed {
        bytes,
        recompressed,
    })
}

/// The best quality that fits `limit`, at the original resolution first and
/// then at each smaller step. Fails with the smallest size reached.
fn fit_limit(
    doc: Document,
    candidates: &[Candidate],
    max_quality: u8,
    max_dimension: Option<u32>,
    limit: u64,
    tracker: &mut Tracker<'_>,
) -> Result<Packed> {
    let fits = |packed: &Packed| packed.bytes.len() as u64 <= limit;
    let min_quality = MIN_QUALITY.min(max_quality);
    // A step that shrinks no image would only repeat the attempt before it.
    let largest = candidates
        .iter()
        .filter(|c| !c.masked)
        .map(|c| c.image.width().max(c.image.height()))
        .max()
        .unwrap_or(0);
    let mut sides = vec![max_dimension];
    sides.extend(
        SHRINK_STEPS
            .iter()
            .filter(|&&side| max_dimension.is_none_or(|max| side < max) && side < largest)
            .map(|&side| Some(side)),
    );
    // Without images to work on, every attempt gives the same file.
    if candidates.is_empty() {
        sides.truncate(1);
    }

    let mut best = u64::MAX;
    for (step, side) in sides.iter().enumerate() {
        tracker.step(DECODED + (REPACKED - DECODED) * step as f32 / sides.len() as f32)?;
        let images = at_side(candidates, *side);
        let smallest = pack(doc.clone(), candidates, &images, min_quality)?;
        best = best.min(smallest.bytes.len() as u64);
        if !fits(&smallest) {
            continue;
        }

        // About the highest quality that still fits; `found` always fits at `low`.
        let (mut low, mut high, mut found) = (min_quality, max_quality, smallest);
        while high - low >= QUALITY_STEP {
            tracker.check()?;
            let mid = low + (high - low).div_ceil(2);
            let packed = pack(doc.clone(), candidates, &images, mid)?;
            if fits(&packed) {
                low = mid;
                found = packed;
            } else {
                high = mid - 1;
            }
        }
        tracker.step(REPACKED)?;
        return Ok(found);
    }
    Err(ErrorKind::SizeLimitUnreachable { best })
}

/// Reads a PDF for changing: it must be a PDF and must not be locked.
pub(crate) fn load_document(original: &[u8]) -> Result<Document> {
    if !is_pdf(original) {
        return Err(ErrorKind::UnsupportedFormat);
    }
    let load = LoadOptions {
        max_decompressed_size: Some(MAX_DECOMPRESSED_BYTES),
        ..Default::default()
    };
    let doc = Document::load_mem_with_options(original, load).map_err(load_error)?;
    if doc.was_encrypted() || doc.is_encrypted() {
        return Err(ErrorKind::PasswordProtected);
    }
    Ok(doc)
}

/// The PDF as bytes, without objects nothing refers to, using object streams.
pub(crate) fn save_document(mut doc: Document) -> Result<Vec<u8>> {
    doc.prune_objects();
    doc.compress();
    let save = SaveOptions::builder()
        .use_object_streams(true)
        .use_xref_streams(true)
        .build();
    let mut bytes = Vec::new();
    doc.save_with_options(&mut bytes, save)
        .map_err(|_| ErrorKind::Damaged)?;
    Ok(bytes)
}

/// PDF readers accept the `%PDF-` header anywhere in the first 1024 bytes.
fn is_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|window| window == b"%PDF-")
}

fn load_error(err: lopdf::Error) -> ErrorKind {
    match err {
        lopdf::Error::Decryption(_) | lopdf::Error::InvalidPassword => ErrorKind::PasswordProtected,
        _ => ErrorKind::Damaged,
    }
}

fn is_image(stream: &Stream) -> bool {
    matches!(
        stream.dict.get(b"Subtype").and_then(Object::as_name),
        Ok(b"Image")
    )
}

/// The image as pixels, and whether it is gray, when it is a plain RGB or
/// gray JPEG this engine knows how to re-encode.
fn decode_jpeg(stream: &Stream) -> Option<(DynamicImage, bool)> {
    match stream.filters() {
        Ok(filters) if filters == [b"DCTDecode".as_slice()] => {}
        _ => return None,
    }
    let gray = match stream.dict.get(b"ColorSpace").and_then(Object::as_name) {
        Ok(b"DeviceRGB") => false,
        Ok(b"DeviceGray") => true,
        _ => return None,
    };
    if stream.dict.has(b"Decode") {
        return None;
    }
    let image = ::image::load_from_memory_with_format(&stream.content, RawFormat::Jpeg).ok()?;
    Some((image, gray))
}

fn encode_jpeg(img: &DynamicImage, quality: u8, gray: bool) -> Option<Vec<u8>> {
    // Converted only when the pixels aren't already in the encoder's layout,
    // so that several images at once don't each hold a second copy.
    let (raw, color): (Cow<'_, [u8]>, _) = match (img, gray) {
        (DynamicImage::ImageLuma8(luma), true) => {
            (Cow::Borrowed(luma.as_raw()), ExtendedColorType::L8)
        }
        (_, true) => (Cow::Owned(img.to_luma8().into_raw()), ExtendedColorType::L8),
        (DynamicImage::ImageRgb8(rgb), false) => {
            (Cow::Borrowed(rgb.as_raw()), ExtendedColorType::Rgb8)
        }
        (_, false) => (
            Cow::Owned(img.to_rgb8().into_raw()),
            ExtendedColorType::Rgb8,
        ),
    };

    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, quality)
        .write_image(&raw, img.width(), img.height(), color)
        .ok()?;
    Some(buf)
}
