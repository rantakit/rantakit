use std::io;

/// Why an engine has no Result for a file. The frontend turns each kind into a
/// message in the user's language; the text here is for logs only.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorKind {
    #[error("the file is locked with a password")]
    PasswordProtected,

    #[error("the file is damaged or incomplete")]
    Damaged,

    #[error("this kind of file is not supported")]
    UnsupportedFormat,

    /// `best` is the smallest size the engine reached, in bytes.
    #[error("size limit unreachable, smallest was {best} bytes")]
    SizeLimitUnreachable { best: u64 },

    #[error("not enough storage space")]
    NotEnoughSpace,

    #[error("the Result could not be saved")]
    CannotWrite,

    #[error("the Original was moved or deleted")]
    OriginalMissing,

    /// None of the pages asked for are in the file, such as page 5 of a
    /// 3-page PDF.
    #[error("none of the pages asked for are in the file")]
    NoSuchPages,

    /// The cancel token was set. An outcome, not a problem.
    #[error("cancelled")]
    Cancelled,

    /// The file is already what the Tool would make, such as a JPG for
    /// "Ubah ke JPG". No Result is written. An outcome, not a problem.
    #[error("already as asked, nothing to change")]
    Unchanged,
}

pub type Result<T> = std::result::Result<T, ErrorKind>;

impl ErrorKind {
    /// The stable code the frontend turns into a message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PasswordProtected => "password-protected",
            Self::Damaged => "damaged",
            Self::UnsupportedFormat => "unsupported-format",
            Self::SizeLimitUnreachable { .. } => "size-limit-unreachable",
            Self::NotEnoughSpace => "not-enough-space",
            Self::CannotWrite => "cannot-write",
            Self::OriginalMissing => "original-missing",
            Self::NoSuchPages => "no-such-pages",
            Self::Cancelled => "cancelled",
            Self::Unchanged => "unchanged",
        }
    }

    /// The kind for an I/O error while reading an Original.
    pub fn reading(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::OriginalMissing,
            _ => Self::Damaged,
        }
    }

    /// The kind for an I/O error while writing a Result.
    pub fn writing(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::StorageFull => Self::NotEnoughSpace,
            _ => Self::CannotWrite,
        }
    }
}
