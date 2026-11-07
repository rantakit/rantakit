//! Photos laid out as PDF pages (D2): each photo whole, on a page of the
//! chosen paper turned the same way as the photo.

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use rantakit_core::{ErrorKind, Result};

use crate::pages::PdfDocument;

/// A photo ready for a page: JPG bytes and their pixel size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagePhoto {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// One gray channel rather than three colors.
    pub gray: bool,
}

/// "Ukuran kertas".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    #[default]
    A4,
    /// Folio, 215 × 330 mm, common in Indonesian offices.
    F4,
    /// "Ikuti ukuran foto": each page the shape of its photo.
    Photo,
}

impl PaperSize {
    /// Width and height in mm, upright.
    fn millimetres(self) -> (f32, f32) {
        match self {
            PaperSize::A4 | PaperSize::Photo => (210.0, 297.0),
            PaperSize::F4 => (215.0, 330.0),
        }
    }
}

/// The thin margin around each photo on A4 or F4.
const MARGIN_MM: f32 = 10.0;
const POINTS_PER_MM: f32 = 72.0 / 25.4;

/// Where a photo goes: the page size, and the photo's box on it, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub page: (f32, f32),
    /// Left, bottom, width, height.
    pub photo: (f32, f32, f32, f32),
}

/// The page for a `width` × `height` photo: upright or sideways as the
/// photo is, the photo fitted whole and centered.
pub fn placement(width: u32, height: u32, paper: PaperSize, margin: bool) -> Placement {
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    let (short, long) = paper.millimetres();
    let long = long * POINTS_PER_MM;
    if paper == PaperSize::Photo {
        // The photo's own shape, its long side as long as A4's.
        let scale = long / w.max(h);
        let page = (w * scale, h * scale);
        return Placement { page, photo: (0.0, 0.0, page.0, page.1) };
    }
    let short = short * POINTS_PER_MM;
    let page = if w > h { (long, short) } else { (short, long) };
    let edge = if margin { MARGIN_MM * POINTS_PER_MM } else { 0.0 };
    let (room_w, room_h) = (page.0 - 2.0 * edge, page.1 - 2.0 * edge);
    let scale = (room_w / w).min(room_h / h);
    let (photo_w, photo_h) = (w * scale, h * scale);
    Placement {
        page,
        photo: ((page.0 - photo_w) / 2.0, (page.1 - photo_h) / 2.0, photo_w, photo_h),
    }
}

impl PdfDocument {
    /// A new PDF with one page per photo, in order.
    pub fn from_photos(photos: Vec<PagePhoto>, paper: PaperSize, margin: bool) -> Result<Self> {
        if photos.is_empty() {
            return Err(ErrorKind::Damaged);
        }
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::with_capacity(photos.len());

        for photo in photos {
            let place = placement(photo.width, photo.height, paper, margin);
            let image = Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => photo.width,
                    "Height" => photo.height,
                    "ColorSpace" => if photo.gray { "DeviceGray" } else { "DeviceRGB" },
                    "BitsPerComponent" => 8,
                    "Filter" => "DCTDecode",
                },
                photo.jpeg,
            )
            .with_compression(false);
            let image_id = doc.add_object(image);

            let (x, y, w, h) = place.photo;
            let content = Content {
                operations: vec![
                    Operation::new("q", vec![]),
                    Operation::new("cm", vec![w.into(), 0.into(), 0.into(), h.into(), x.into(), y.into()]),
                    Operation::new("Do", vec![Object::Name(b"Photo".to_vec())]),
                    Operation::new("Q", vec![]),
                ],
            };
            let bytes = content.encode().map_err(|_| ErrorKind::Damaged)?;
            let content_id = doc.add_object(Stream::new(dictionary! {}, bytes));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), place.page.0.into(), place.page.1.into()],
                "Contents" => content_id,
                "Resources" => dictionary! { "XObject" => dictionary! { "Photo" => image_id } },
            });
            kids.push(page_id.into());
        }

        let count = kids.len() as i64;
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        Ok(PdfDocument::from_document(doc))
    }
}
