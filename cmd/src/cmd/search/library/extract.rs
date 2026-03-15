use eyre::{eyre, Result};
use std::path::Path;

/// Extract text from a PDF file
pub fn extract_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| eyre!("PDF text extraction failed: {e}"))
}
