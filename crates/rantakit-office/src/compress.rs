use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use image::ImageReader;
use rantakit_core::{CancelToken, ErrorKind, Progress, Result, Tracker, read_original, write_result};

use crate::package::{NewPicture, Package, entry_len};
use crate::picture::{PictureFormat, Prepared, prepare};

/// Progress once the document is read and its other parts deflated again,
/// and once the Result is packed.
const LOADED: f32 = 0.15;
const PACKED: f32 = 0.95;

/// Lowest picture quality tried for a Size limit.
const MIN_QUALITY: u8 = 10;
/// Qualities tried for a Size limit, this far apart. Every picture is saved
/// at each of them, so fewer steps keep a document of many photos quick.
const QUALITY_STEP: u8 = 10;
/// Pictures worked on at once. A phone photo takes about 50 MB decoded, and a
/// low-end phone has little to spare.
const THREADS: usize = 2;
/// Longest picture sides tried, in order, when a Size limit can't be reached
/// at the original size: 300, 200, 150, 100 and 72 dpi on an A4 page, as in
/// Kompres PDF.
const SHRINK_STEPS: [u32; 5] = [3508, 2339, 1754, 1170, 842];

#[derive(Debug, Clone)]
pub struct OfficeOptions {
    /// JPG quality for the pictures. With `max_bytes`, the highest tried.
    pub quality: u8,
    /// Longest side of the pictures, in pixels.
    pub max_dimension: Option<u32>,
    /// A Size limit: search picture quality, then size, until the document fits.
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedDocument {
    /// Size of the Result in bytes.
    pub size: u64,
    /// JPG and PNG pictures in the document.
    pub pictures: usize,
    /// Pictures made smaller.
    pub pictures_recompressed: usize,
}

/// A picture part of the original document.
struct Source {
    index: usize,
    name: String,
    format: PictureFormat,
    stored_len: u64,
    bytes: Vec<u8>,
}

/// Makes the pictures in the Word, Excel, or PowerPoint document at `input`
/// smaller and writes the Result to `output`, which must not exist yet.
/// Every other part is kept. A document that can't be made smaller, or
/// already fits the Size limit, ends with `Unchanged` and no Result.
pub fn compress_document(
    input: &Path,
    output: &Path,
    opts: &OfficeOptions,
    progress: &mut dyn FnMut(Progress),
    cancel: &CancelToken,
) -> Result<CompressedDocument> {
    let mut tracker = Tracker::new(progress, cancel);
    let original = read_original(input)?;
    let mut package = Package::open(&original)?;
    if opts.max_bytes.is_some_and(|limit| original.len() as u64 <= limit) {
        return Err(ErrorKind::Unchanged);
    }

    let mut sources = Vec::new();
    for index in 0..package.entries.len() {
        let entry = &package.entries[index];
        if let Some(format) = entry.picture {
            let (name, stored_len) = (entry.name.clone(), entry.stored_len);
            sources.push(Source {
                index,
                name,
                format,
                stored_len,
                bytes: package.read(index)?,
            });
        }
    }
    tracker.step(LOADED)?;

    let quality = opts.quality.clamp(1, 100);
    let pictures = match opts.max_bytes {
        None => {
            let pictures = shrink(&sources, opts.max_dimension, quality, &mut |done| {
                tracker.step(LOADED + (PACKED - LOADED) * done)
            })?;
            let bytes = package.pack(&pictures)?;
            (bytes, pictures.len())
        }
        Some(limit) => fit_limit(&mut package, &sources, quality, opts.max_dimension, limit, &mut tracker)?,
    };
    let (bytes, recompressed) = pictures;
    tracker.step(PACKED)?;
    if bytes.len() >= original.len() {
        return Err(ErrorKind::Unchanged);
    }

    let size = write_result(output, &mut tracker, |file| file.write_all(&bytes))?;
    Ok(CompressedDocument {
        size,
        pictures: sources.len(),
        pictures_recompressed: recompressed,
    })
}

/// Every picture that gets smaller at `side` and `quality`, by entry index.
/// `done` hears the fraction of pictures worked through.
fn shrink(
    sources: &[Source],
    side: Option<u32>,
    quality: u8,
    done: &mut dyn FnMut(f32) -> Result<()>,
) -> Result<HashMap<usize, NewPicture>> {
    let mut pictures = HashMap::new();
    for (chunk, now) in sources.chunks(THREADS).enumerate() {
        done((chunk * THREADS) as f32 / sources.len() as f32)?;
        let shrunk = each_at_once(now, |source| {
            prepare(&source.bytes, source.format, side, source.stored_len).and_then(|prepared| prepared.at(quality))
        });
        for (source, picture) in now.iter().zip(shrunk) {
            if let Some(picture) = picture {
                pictures.insert(source.index, picture);
            }
        }
    }
    done(1.0)?;
    Ok(pictures)
}

/// The best quality that fits `limit`, at the original size first and then
/// at each smaller step. Each picture is decoded once per step and saved at
/// every quality tried, so only the sizes are kept; the document is packed
/// once a quality fits. Fails with the smallest size reached.
fn fit_limit(
    package: &mut Package<'_>,
    sources: &[Source],
    max_quality: u8,
    max_dimension: Option<u32>,
    limit: u64,
    tracker: &mut Tracker<'_>,
) -> Result<(Vec<u8>, usize)> {
    let min_quality = MIN_QUALITY.min(max_quality);
    let mut qualities: Vec<u8> = (min_quality..=max_quality).rev().step_by(QUALITY_STEP as usize).collect();
    if qualities.last() != Some(&min_quality) {
        qualities.push(min_quality);
    }

    // A step that shrinks no picture would only repeat the one before it.
    let largest = sources.iter().filter_map(|source| longest_side(&source.bytes)).max().unwrap_or(0);
    let mut sides = vec![max_dimension];
    sides.extend(
        SHRINK_STEPS
            .iter()
            .filter(|&&side| max_dimension.is_none_or(|max| side < max) && side < largest)
            .map(|&side| Some(side)),
    );
    if sources.is_empty() {
        sides.truncate(1);
    }

    let base = package.base_len();
    let mut best = u64::MAX;
    let steps = sides.len() as f32;
    for (step, side) in sides.iter().enumerate() {
        let from = LOADED + (PACKED - LOADED) * step as f32 / steps;
        let span = (PACKED - LOADED) / steps;
        let mut totals = vec![base; qualities.len()];
        for (chunk, now) in sources.chunks(THREADS).enumerate() {
            tracker.step(from + span * (chunk * THREADS) as f32 / sources.len().max(1) as f32)?;
            let lens = each_at_once(now, |source| {
                let prepared: Option<Prepared> = prepare(&source.bytes, source.format, *side, source.stored_len);
                qualities
                    .iter()
                    .map(|&quality| prepared.as_ref().map_or(source.stored_len, |p| p.len_at(quality)))
                    .collect::<Vec<u64>>()
            });
            for (source, lens) in now.iter().zip(lens) {
                for (total, len) in totals.iter_mut().zip(lens) {
                    *total += entry_len(&source.name, len);
                }
            }
        }
        best = best.min(*totals.last().unwrap_or(&u64::MAX));

        for (quality, total) in qualities.iter().zip(&totals) {
            if *total > limit {
                continue;
            }
            let pictures = shrink(sources, *side, *quality, &mut |_| tracker.check())?;
            let bytes = package.pack(&pictures)?;
            best = best.min(bytes.len() as u64);
            // The sizes above are close but not exact; a miss tries lower.
            if bytes.len() as u64 <= limit {
                return Ok((bytes, pictures.len()));
            }
        }
    }
    Err(ErrorKind::SizeLimitUnreachable { best })
}

/// The longest side of a picture, read from its header only.
fn longest_side(bytes: &[u8]) -> Option<u32> {
    let (width, height) = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    Some(width.max(height))
}

/// `work` on every item, up to [`THREADS`] at once, in the items' order.
fn each_at_once<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = thread::available_parallelism()
        .map_or(1, NonZero::get)
        .min(THREADS)
        .min(items.len());
    if threads <= 1 {
        return items.iter().map(work).collect();
    }
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
            .flat_map(|worker| worker.join().expect("picture worker panicked"))
            .collect()
    });
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, result)| result).collect()
}
