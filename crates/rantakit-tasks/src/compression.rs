//! How Kompres Foto and Kompres PDF turn what the user chose into engine
//! options. Every number the user never sees lives here, in one place, to be
//! tuned with real documents.

use rantakit_image::{ImageOptions, OutputFormat, Target, WHITE};
use rantakit_office::OfficeOptions;
use rantakit_pdf::PdfOptions;

/// "Kecilkan saja": how small, in plain words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Level {
    /// "Paling kecil"
    Smallest,
    /// "Seimbang"
    #[default]
    Balanced,
    /// "Paling jernih"
    Clearest,
}

/// Photo quality and longest side for each Level; `None` keeps the size.
fn photo_level(level: Level) -> (u8, Option<u32>) {
    match level {
        Level::Smallest => (60, Some(1600)),
        Level::Balanced => (75, Some(2560)),
        Level::Clearest => (88, None),
    }
}

/// PDF image quality and longest image side for each Level.
fn pdf_level(level: Level) -> (u8, Option<u32>) {
    match level {
        Level::Smallest => (50, Some(dpi_to_side(150))),
        Level::Balanced => (70, Some(dpi_to_side(200))),
        Level::Clearest => (85, None),
    }
}

/// The longest image side, in pixels, of an A4 page (11.69 in) at `dpi`.
pub fn dpi_to_side(dpi: u32) -> u32 {
    (dpi as f32 * 11.69).round() as u32
}

/// The highest image quality tried when searching for a PDF Size limit.
const PDF_LIMIT_QUALITY: u8 = 90;

/// What the user asked for in a compress Tool's Options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compression {
    /// "Sesuaikan batas ukuran": each Result at or under this many bytes.
    /// `None` compresses by `level` instead.
    pub limit: Option<u64>,
    pub level: Level,
    /// The exact quality from Advanced options; wins over the limit and the Level.
    pub quality: Option<u8>,
    /// The longest side from Advanced options, in pixels; wins over the Level.
    pub max_dimension: Option<u32>,
}

impl Compression {
    pub fn photo(&self, format: OutputFormat) -> ImageOptions {
        let (level_quality, level_side) = photo_level(self.level);
        let (target, max_dimension) = match (self.quality, self.limit) {
            (Some(quality), Some(_)) => (Target::Quality(quality), self.max_dimension),
            (None, Some(limit)) => (Target::MaxBytes(limit), self.max_dimension),
            (quality, None) => (
                Target::Quality(quality.unwrap_or(level_quality)),
                self.max_dimension.or(level_side),
            ),
        };
        ImageOptions {
            format,
            target,
            max_dimension,
            background: WHITE,
        }
    }

    /// Kompres Dokumen: the pictures in a document follow Kompres PDF's numbers,
    /// since both are sized for a page.
    pub fn document(&self) -> OfficeOptions {
        let pdf = self.pdf();
        OfficeOptions {
            quality: pdf.quality,
            max_dimension: pdf.max_dimension,
            max_bytes: pdf.max_bytes,
        }
    }

    pub fn pdf(&self) -> PdfOptions {
        let (level_quality, level_side) = pdf_level(self.level);
        match (self.quality, self.limit) {
            (Some(quality), Some(_)) => PdfOptions {
                quality,
                max_dimension: self.max_dimension,
                max_bytes: None,
            },
            (None, Some(limit)) => PdfOptions {
                quality: PDF_LIMIT_QUALITY,
                max_dimension: self.max_dimension,
                max_bytes: Some(limit),
            },
            (quality, None) => PdfOptions {
                quality: quality.unwrap_or(level_quality),
                max_dimension: self.max_dimension.or(level_side),
                max_bytes: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limit_searches_for_the_size() {
        let compression = Compression {
            limit: Some(200 * 1024),
            ..Default::default()
        };
        assert_eq!(
            compression.photo(OutputFormat::Same).target,
            Target::MaxBytes(200 * 1024)
        );
        assert_eq!(compression.pdf().max_bytes, Some(200 * 1024));
    }

    #[test]
    fn an_exact_quality_wins_over_the_limit() {
        let compression = Compression {
            limit: Some(200 * 1024),
            quality: Some(40),
            ..Default::default()
        };
        assert_eq!(
            compression.photo(OutputFormat::Same).target,
            Target::Quality(40)
        );
        let pdf = compression.pdf();
        assert_eq!((pdf.quality, pdf.max_bytes), (40, None));
    }

    #[test]
    fn levels_pick_quality_and_size() {
        let level = |level| Compression {
            level,
            ..Default::default()
        };
        let smallest = level(Level::Smallest).photo(OutputFormat::Same);
        assert_eq!(
            (smallest.target, smallest.max_dimension),
            (Target::Quality(60), Some(1600))
        );
        let clearest = level(Level::Clearest).photo(OutputFormat::Same);
        assert_eq!(
            (clearest.target, clearest.max_dimension),
            (Target::Quality(88), None)
        );
        let balanced = level(Level::Balanced).pdf();
        assert_eq!((balanced.quality, balanced.max_dimension), (70, Some(2338)));
    }

    #[test]
    fn advanced_overrides_win_over_the_level() {
        let photo = Compression {
            level: Level::Smallest,
            quality: Some(95),
            max_dimension: Some(800),
            ..Default::default()
        }
        .photo(OutputFormat::Same);
        assert_eq!(
            (photo.target, photo.max_dimension),
            (Target::Quality(95), Some(800))
        );
    }
}
