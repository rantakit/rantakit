//! Text over every page, such as Beri Watermark's (K1). The lines come laid
//! out already, for each page as a reader shows it; this only puts them on
//! the page, turned and cropped as it is, in an embedded TrueType font.
//! Nothing already on a page is drawn again, so text stays text.

use std::fmt::Write as _;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use rantakit_core::{ErrorKind, Result};

use crate::pages::PdfDocument;

/// Resource names for the font and the transparency, unlikely to be taken.
const FONT: &str = "RkStampF";
const STATE: &str = "RkStampGs";

/// A TrueType font to embed, with its measures in thousandths of its size.
#[derive(Debug, Clone, Copy)]
pub struct StampFont<'a> {
    pub name: &'a str,
    /// The whole TrueType file.
    pub file: &'a [u8],
    /// Advance widths of WinAnsi codes 32 to 255.
    pub widths: &'a [u16],
    pub ascent: i32,
    pub descent: i32,
    pub cap_height: i32,
    pub bbox: [i32; 4],
}

/// One line of text on a page as a reader shows it: its baseline starts at
/// `x`, `y` points from the top left (`y` down) and runs `angle` degrees
/// counterclockwise. `text` is WinAnsi, one byte per character.
#[derive(Debug, Clone, PartialEq)]
pub struct StampLine {
    pub text: Vec<u8>,
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

/// How the text looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StampStyle {
    pub color: [u8; 3],
    /// 0 is invisible, 1 covers what is under it.
    pub opacity: f32,
}

impl PdfDocument {
    /// Puts text on every page: `lines_for` gets each page's width and
    /// height as it is shown and gives the lines to draw there.
    pub fn stamp(
        &mut self,
        font: &StampFont,
        style: StampStyle,
        lines_for: &mut dyn FnMut(f32, f32) -> Vec<StampLine>,
    ) -> Result<()> {
        let doc = &mut self.doc;
        let font_id = embed_font(doc, font);
        let opacity = style.opacity.clamp(0.0, 1.0);
        let state_id = doc.add_object(dictionary! {
            "Type" => "ExtGState",
            "ca" => opacity,
            "CA" => opacity,
        });
        let [r, g, b] = style.color.map(|c| f32::from(c) / 255.0);

        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        for page_id in pages {
            let (area, turn) = shown_area(doc, page_id)?;
            let (width, height) = match turn {
                90 | 270 => (area[3] - area[1], area[2] - area[0]),
                _ => (area[2] - area[0], area[3] - area[1]),
            };
            let lines = lines_for(width, height);
            if lines.is_empty() {
                continue;
            }
            let mut content = format!("q /{STATE} gs {r:.4} {g:.4} {b:.4} rg BT\n");
            for line in &lines {
                let (x, y, angle) = to_page(area, turn, line.x, line.y, line.angle);
                let (sin, cos) = angle.to_radians().sin_cos();
                let _ = write!(
                    content,
                    "/{FONT} {:.3} Tf {cos:.5} {sin:.5} {:.5} {cos:.5} {x:.3} {y:.3} Tm <",
                    line.size, -sin
                );
                for byte in &line.text {
                    let _ = write!(content, "{byte:02X}");
                }
                content.push_str("> Tj\n");
            }
            content.push_str("ET Q\n");

            add_resource(doc, page_id, b"Font", FONT, font_id)?;
            add_resource(doc, page_id, b"ExtGState", STATE, state_id)?;
            wrap_contents(doc, page_id, content.into_bytes())?;
        }
        Ok(())
    }
}

/// The font, its descriptor, and its file, as objects; the font's id.
fn embed_font(doc: &mut Document, font: &StampFont) -> ObjectId {
    let file_id = doc.add_object(Stream::new(
        dictionary! { "Length1" => font.file.len() as i64 },
        font.file.to_vec(),
    ));
    let bbox: Vec<Object> = font
        .bbox
        .iter()
        .map(|&v| Object::Integer(v.into()))
        .collect();
    let descriptor_id = doc.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => Object::Name(font.name.as_bytes().to_vec()),
        // Nonsymbolic: its characters are the standard Latin ones.
        "Flags" => 32,
        "FontBBox" => bbox,
        "ItalicAngle" => 0,
        "Ascent" => font.ascent,
        "Descent" => font.descent,
        "CapHeight" => font.cap_height,
        "StemV" => 120,
        "FontFile2" => file_id,
    });
    let widths: Vec<Object> = font
        .widths
        .iter()
        .map(|&w| Object::Integer(w.into()))
        .collect();
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => Object::Name(font.name.as_bytes().to_vec()),
        "FirstChar" => 32,
        "LastChar" => 32 + widths.len() as i64 - 1,
        "Widths" => widths,
        "Encoding" => "WinAnsiEncoding",
        "FontDescriptor" => descriptor_id,
    })
}

/// The part of the page a reader shows, `[left, bottom, right, top]`, and
/// its clockwise turn: 0, 90, 180, or 270.
fn shown_area(doc: &Document, page_id: ObjectId) -> Result<([f32; 4], i64)> {
    let page = doc
        .get_dictionary(page_id)
        .map_err(|_| ErrorKind::Damaged)?;
    let number = |object: &Object| -> Option<f32> {
        let object = match object {
            Object::Reference(id) => doc.get_object(*id).ok()?,
            other => other,
        };
        object.as_float().ok()
    };
    let rect = |key: &[u8]| -> Option<[f32; 4]> {
        let object = page.get(key).ok()?;
        let object = match object {
            Object::Reference(id) => doc.get_object(*id).ok()?,
            other => other,
        };
        let values: Vec<f32> = object.as_array().ok()?.iter().filter_map(number).collect();
        let [a, b, c, d] = values[..] else {
            return None;
        };
        Some([a.min(c), b.min(d), a.max(c), b.max(d)])
    };
    let media = rect(b"MediaBox").unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let area = match rect(b"CropBox") {
        // The crop box is clipped to the media box.
        Some(crop) => [
            crop[0].max(media[0]),
            crop[1].max(media[1]),
            crop[2].min(media[2]),
            crop[3].min(media[3]),
        ],
        None => media,
    };
    if area[2] - area[0] < 1.0 || area[3] - area[1] < 1.0 {
        return Err(ErrorKind::Damaged);
    }
    let turn = page.get(b"Rotate").ok().and_then(number).unwrap_or(0.0) as i64;
    Ok((area, (turn.rem_euclid(360) / 90) * 90))
}

/// A point and angle on the page as shown, to the page's own space: `y`
/// up, before its turn.
fn to_page(area: [f32; 4], turn: i64, x: f32, y: f32, angle: f32) -> (f32, f32, f32) {
    let [left, bottom, right, top] = area;
    match turn {
        90 => (left + y, bottom + x, angle + 90.0),
        180 => (right - x, bottom + y, angle + 180.0),
        270 => (right - y, top - x, angle + 270.0),
        _ => (left + x, top - y, angle),
    }
}

/// Adds `name` → `value` to the page's `category` of resources (`Font`,
/// `ExtGState`). Pages may share their resources; the same name is then
/// added once more, harmlessly.
fn add_resource(
    doc: &mut Document,
    page_id: ObjectId,
    category: &[u8],
    name: &str,
    value: ObjectId,
) -> Result<()> {
    let resources_id = resources_id(doc, page_id)?;
    let held = doc
        .get_dictionary(resources_id)
        .map_err(|_| ErrorKind::Damaged)?
        .get(category)
        .ok()
        .cloned();
    match held {
        Some(Object::Reference(id)) => {
            let dict = doc.get_dictionary_mut(id).map_err(|_| ErrorKind::Damaged)?;
            dict.set(name, value);
        }
        Some(Object::Dictionary(mut dict)) => {
            dict.set(name, value);
            let resources = doc
                .get_dictionary_mut(resources_id)
                .map_err(|_| ErrorKind::Damaged)?;
            resources.set(category, dict);
        }
        _ => {
            let resources = doc
                .get_dictionary_mut(resources_id)
                .map_err(|_| ErrorKind::Damaged)?;
            resources.set(category, dictionary! { name => value });
        }
    }
    Ok(())
}

/// The page's resources as an object of their own, made so if they were
/// written inside the page or missing.
fn resources_id(doc: &mut Document, page_id: ObjectId) -> Result<ObjectId> {
    let held = doc
        .get_dictionary(page_id)
        .map_err(|_| ErrorKind::Damaged)?
        .get(b"Resources")
        .ok()
        .cloned();
    let id = match held {
        Some(Object::Reference(id)) if doc.get_dictionary(id).is_ok() => return Ok(id),
        Some(Object::Dictionary(dict)) => doc.add_object(dict),
        _ => doc.add_object(Dictionary::new()),
    };
    doc.get_dictionary_mut(page_id)
        .map_err(|_| ErrorKind::Damaged)?
        .set("Resources", id);
    Ok(id)
}

/// The page's own drawing, kept apart in `q … Q` so nothing it leaves set
/// moves the text, then `content` on top.
fn wrap_contents(doc: &mut Document, page_id: ObjectId, content: Vec<u8>) -> Result<()> {
    let held = doc
        .get_dictionary(page_id)
        .map_err(|_| ErrorKind::Damaged)?
        .get(b"Contents")
        .ok()
        .cloned();
    let mut parts: Vec<Object> = match held {
        Some(Object::Reference(id)) => match doc.get_object(id) {
            Ok(Object::Array(parts)) => parts.clone(),
            _ => vec![Object::Reference(id)],
        },
        Some(Object::Array(parts)) => parts,
        _ => Vec::new(),
    };
    if !parts.is_empty() {
        let open = doc.add_object(Stream::new(Dictionary::new(), b"q\n".to_vec()));
        parts.insert(0, Object::Reference(open));
        let mut closed = b"Q\n".to_vec();
        closed.extend_from_slice(&content);
        parts.push(Object::Reference(
            doc.add_object(Stream::new(Dictionary::new(), closed)),
        ));
    } else {
        parts.push(Object::Reference(
            doc.add_object(Stream::new(Dictionary::new(), content)),
        ));
    }
    doc.get_dictionary_mut(page_id)
        .map_err(|_| ErrorKind::Damaged)?
        .set("Contents", parts);
    Ok(())
}
