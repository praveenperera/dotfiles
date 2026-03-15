use eyre::{bail, Result};
use std::path::Path;
use std::process::Command;

/// Extract text from a PDF file using pdftotext (poppler)
pub fn extract_text(path: &Path) -> Result<String> {
    let output = Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                eyre::eyre!("pdftotext not found — install poppler: brew install poppler")
            } else {
                eyre::eyre!("failed to run pdftotext: {e}")
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("pdftotext failed (exit {}): {stderr}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
