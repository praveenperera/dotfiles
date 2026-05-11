use color_eyre::eyre::{Result, WrapErr};

pub fn copy(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().wrap_err("Failed to open system clipboard")?;
    clipboard
        .set_text(text.to_owned())
        .wrap_err("Failed to copy prompt to clipboard")
}
