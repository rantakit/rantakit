//! Page edits that keep every page as it is: merging PDFs (Gabung PDF),
//! taking pages out (Pisah PDF), and turning, dropping, and reordering pages
//! (Atur Halaman PDF). Nothing on a page is drawn again, so text stays text.

use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};
use rantakit_core::{ErrorKind, Result, Tracker, read_original, write_result};

use crate::compress::{load_document, save_document};

/// What a page takes from the page tree above it when it does not say itself.
const INHERITED: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Catalog entries that point at particular pages; dropped when a page goes,
/// so they cannot keep a dropped page alive or lead nowhere.
const ABOUT_PAGES: [&[u8]; 5] = [
    b"Outlines",
    b"StructTreeRoot",
    b"MarkInfo",
    b"OpenAction",
    b"PageLabels",
];

/// One page of a Result: a page of the Original, counted from 0, turned by
/// `quarter_turns` clockwise on top of how it already stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageEdit {
    pub page: usize,
    pub quarter_turns: u8,
}

impl PageEdit {
    /// The page as it is.
    pub fn keep(page: usize) -> Self {
        Self {
            page,
            quarter_turns: 0,
        }
    }
}

/// A PDF open for page edits.
#[derive(Debug, Clone)]
pub struct PdfDocument {
    pub(crate) doc: Document,
}

impl PdfDocument {
    /// Opens the PDF at `path`. Fails with `PasswordProtected` for a locked
    /// PDF and `Damaged` for one that cannot be read.
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_bytes(&read_original(path)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut doc = load_document(bytes)?;
        flatten_pages(&mut doc);
        if doc.get_pages().is_empty() {
            return Err(ErrorKind::Damaged);
        }
        Ok(Self { doc })
    }

    /// A PDF built here, already in the shape every page edit expects.
    pub(crate) fn from_document(doc: Document) -> Self {
        Self { doc }
    }

    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// Every page of `parts`, one PDF after another, as one PDF. The first
    /// PDF's bookmarks and document info stay.
    pub fn merge(parts: Vec<PdfDocument>) -> Result<Self> {
        let mut parts = parts.into_iter();
        let Some(Self { doc: mut merged }) = parts.next() else {
            return Err(ErrorKind::Damaged);
        };
        let mut pages: Vec<ObjectId> = merged.get_pages().into_values().collect();
        for Self { mut doc } in parts {
            doc.renumber_objects_with(merged.max_id + 1);
            pages.extend(doc.get_pages().into_values());
            merged.max_id = merged.max_id.max(doc.max_id);
            // Its own catalog and page tree are left behind and pruned on save.
            merged.objects.extend(doc.objects);
        }
        // The first PDF's page numbering would not fit the pages after it.
        if let Ok(catalog) = merged.catalog_mut() {
            catalog.remove(b"PageLabels");
        }
        set_page_tree(&mut merged, &pages)?;
        Ok(Self { doc: merged })
    }

    /// A new PDF of the pages in `edits`, in that order. Only what those
    /// pages use is copied, so a few pages of a long PDF make a small file.
    /// A page past the end is `Damaged`.
    pub fn arrange(&self, edits: &[PageEdit]) -> Result<Self> {
        let source = &self.doc;
        let pages: Vec<ObjectId> = source.get_pages().into_values().collect();
        if edits.is_empty() || edits.iter().any(|edit| edit.page >= pages.len()) {
            return Err(ErrorKind::Damaged);
        }
        let tree = page_tree_ids(source);
        let root = root_pages(source)?;
        let catalog_id = catalog_id(source)?;

        let mut doc = Document::with_version(source.version.clone());
        doc.trailer = source.trailer.clone();
        doc.max_id = source.max_id;

        let kept: HashSet<usize> = edits.iter().map(|edit| edit.page).collect();
        let mut catalog = source.catalog().map_err(|_| ErrorKind::Damaged)?.clone();
        if kept.len() < pages.len() {
            for key in ABOUT_PAGES {
                catalog.remove(key);
            }
        } else if edits.iter().enumerate().any(|(at, edit)| edit.page != at) {
            catalog.remove(b"PageLabels");
        }

        let mut used = HashSet::new();
        let mut kids = Vec::with_capacity(edits.len());
        let mut to_copy: Vec<&Object> = catalog
            .iter()
            .filter(|(key, _)| key.as_slice() != b"Pages")
            .map(|(_, value)| value)
            .collect();
        for edit in edits {
            let original = pages[edit.page];
            let mut page = source
                .get_dictionary(original)
                .map_err(|_| ErrorKind::Damaged)?
                .clone();
            let turned = rotation(&page) + i64::from(edit.quarter_turns % 4) * 90;
            page.set("Rotate", turned.rem_euclid(360));
            page.set("Parent", root);
            // The same page twice needs its own object the second time.
            let id = if used.insert(original) {
                original
            } else {
                doc.max_id += 1;
                (doc.max_id, 0)
            };
            doc.objects.insert(id, Object::Dictionary(page));
            kids.push(id);
        }
        for edit in edits {
            let page = source.get_dictionary(pages[edit.page]).map_err(|_| ErrorKind::Damaged)?;
            to_copy.extend(
                page.iter()
                    .filter(|(key, _)| key.as_slice() != b"Parent")
                    .map(|(_, value)| value),
            );
        }
        copy_used(source, &mut doc, to_copy, &tree);

        doc.objects.insert(catalog_id, Object::Dictionary(catalog));
        set_page_tree(&mut doc, &kids)?;
        Ok(Self { doc })
    }

    /// Writes the PDF to `output`, which must not exist yet. Returns its size
    /// in bytes.
    pub fn save(self, output: &Path, tracker: &mut Tracker<'_>) -> Result<u64> {
        let bytes = save_document(self.doc)?;
        write_result(output, tracker, |file| file.write_all(&bytes))
    }
}

/// Gives every page what it would take from the page tree above it, so a
/// page can move to another tree as it is.
fn flatten_pages(doc: &mut Document) {
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for id in pages {
        let mut found: Vec<(&[u8], Object)> = Vec::new();
        let mut parent = doc
            .get_dictionary(id)
            .ok()
            .and_then(|page| page.get(b"Parent").and_then(Object::as_reference).ok());
        let Ok(page) = doc.get_dictionary(id) else {
            continue;
        };
        let mut missing: Vec<&[u8]> = INHERITED
            .into_iter()
            .filter(|key| !page.has(key))
            .collect();
        // A loop in a damaged tree stops after a sane depth.
        for _ in 0..64 {
            let Some(node_id) = parent else { break };
            let Ok(node) = doc.get_dictionary(node_id) else {
                break;
            };
            missing.retain(|key| match node.get(key) {
                Ok(value) => {
                    found.push((key, value.clone()));
                    false
                }
                Err(_) => true,
            });
            parent = node.get(b"Parent").and_then(Object::as_reference).ok();
        }
        if let Ok(page) = doc.get_dictionary_mut(id) {
            for (key, value) in found {
                page.set(key, value);
            }
        }
    }
}

/// Makes `kids` the whole page tree, under the catalog's own Pages node.
fn set_page_tree(doc: &mut Document, kids: &[ObjectId]) -> Result<()> {
    let root = root_pages(doc)?;
    for &kid in kids {
        if let Ok(page) = doc.get_dictionary_mut(kid) {
            page.set("Parent", root);
        }
    }
    let mut node = Dictionary::new();
    node.set("Type", "Pages");
    node.set(
        "Kids",
        kids.iter().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
    );
    node.set("Count", kids.len() as i64);
    doc.objects.insert(root, Object::Dictionary(node));
    Ok(())
}

fn catalog_id(doc: &Document) -> Result<ObjectId> {
    doc.trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(|_| ErrorKind::Damaged)
}

fn root_pages(doc: &Document) -> Result<ObjectId> {
    doc.catalog()
        .and_then(|catalog| catalog.get(b"Pages"))
        .and_then(Object::as_reference)
        .map_err(|_| ErrorKind::Damaged)
}

/// How far a page is already turned, as a number of degrees.
fn rotation(page: &Dictionary) -> i64 {
    page.get(b"Rotate")
        .ok()
        .and_then(|value| value.as_i64().ok().or_else(|| value.as_float().ok().map(|f| f as i64)))
        .unwrap_or(0)
}

/// Every page and Pages node: copying these would pull in the whole PDF.
fn page_tree_ids(doc: &Document) -> HashSet<ObjectId> {
    doc.objects
        .iter()
        .filter(|(_, object)| {
            object.as_dict().is_ok_and(|dict| {
                dict.get(b"Type")
                    .and_then(Object::as_name)
                    .is_ok_and(|name| name == b"Page" || name == b"Pages")
            })
        })
        .map(|(&id, _)| id)
        .collect()
}

/// Copies every object `start` leads to from `source` into `doc`, except
/// pages, which `doc` already has or leaves out. A link to a page left out
/// then leads nowhere, which readers treat as no link.
fn copy_used(source: &Document, doc: &mut Document, start: Vec<&Object>, tree: &HashSet<ObjectId>) {
    let mut found: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut stack: Vec<ObjectId> = Vec::new();
    for object in start {
        references(object, &mut stack);
    }
    while let Some(id) = stack.pop() {
        if tree.contains(&id) || found.contains_key(&id) || doc.objects.contains_key(&id) {
            continue;
        }
        let Some(object) = source.objects.get(&id) else {
            continue;
        };
        references(object, &mut stack);
        found.insert(id, object.clone());
    }
    doc.objects.extend(found);
}

fn references(object: &Object, out: &mut Vec<ObjectId>) {
    match object {
        Object::Reference(id) => out.push(*id),
        Object::Array(items) => items.iter().for_each(|item| references(item, out)),
        Object::Dictionary(dict) => dict.iter().for_each(|(_, value)| references(value, out)),
        Object::Stream(stream) => stream.dict.iter().for_each(|(_, value)| references(value, out)),
        _ => {}
    }
}
