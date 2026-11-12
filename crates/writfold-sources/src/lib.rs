//! Pure, byte-preserving authored-document and citation syntax codecs.

mod citation_fields;
pub mod citations;
mod document_edit;
mod document_lines;
pub mod documents;
pub mod frontmatter;
mod line_range;
pub mod material;
pub mod splicery;
pub mod splicery_refusal;
mod unicode;
mod unicode_data;

/// Syntax refusal, separate from source acquisition or authority admission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct CodecError {
    message: String,
}

impl CodecError {
    pub(crate) fn malformed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
