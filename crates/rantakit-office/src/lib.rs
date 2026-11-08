//! Word, Excel, and PowerPoint files for Rantakit: telling them apart by
//! content, and Kompres Dokumen (O3), which makes the pictures inside them
//! smaller. A `.docx`, `.xlsx`, or `.pptx` is a ZIP of XML parts; only its
//! JPG and PNG pictures change, and every other part is kept as it is.

mod compress;
mod package;
mod picture;
mod sniff;

pub use compress::{CompressedDocument, OfficeOptions, compress_document};
pub use package::{DocumentInfo, document_info, document_info_of};
pub use sniff::{OfficeKind, Sniffed, sniff};
