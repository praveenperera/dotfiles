//! Quick generations fo (R)ust (M)ulti(P)latform
//!
use askama::Template as _;
use convert_case::{Case, Casing};
use eyre::{Context as _, Result};
use xshell::Shell;

#[derive(Debug)]
pub struct RmpFlags {
    pub lang: String,
    pub module_name: String,
    pub app: Option<String>,
}

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/manager.rs.j2")]
struct RustManagerTemplate {
    manager_name: String,
    app_name: String,
}

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/manager.swift.j2")]
struct SwiftManagerTemplate {
    manager_name: String,
    app_name: String,
}

pub fn generate(_sh: &Shell, flags: &RmpFlags) -> Result<()> {
    let module_name = flags.module_name.to_case(Case::Pascal);
    let module_name = module_name.trim_end_matches("Manager");

    let template = match flags.lang.as_str() {
        "swift" => SwiftManagerTemplate {
            manager_name: format!("{module_name}Manager"),
            app_name: flags.app.as_deref().unwrap_or("cove").to_string(),
        }
        .render(),

        "rs" => RustManagerTemplate {
            manager_name: format!("{module_name}Manager"),
            app_name: flags.app.as_deref().unwrap_or("cove").to_string(),
        }
        .render(),

        _ => return Err(eyre::eyre!("unknown type")),
    };

    let file = template.wrap_err("failed to render view model")?;

    println!("{file}");

    Ok(())
}
