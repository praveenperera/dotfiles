//! Quick generations fo (R)ust (M)ulti(P)latform
//!
use askama::Template as _;
use convert_case::{Case, Casing};
use eyre::{Context as _, Result};
use xshell::Shell;

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/manager.rs.j2")]
struct RustManagerTemplate {
    manager_name: String,
}

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/manager.swift.j2")]
struct SwiftManagerTemplate {
    manager_name: String,
}

pub fn generate(_sh: &Shell, lang: &str, module_name: &str) -> Result<()> {
    let module_name = module_name.to_case(Case::Pascal);
    let module_name = module_name.trim_end_matches("Manager");

    let template = match lang {
        "swift" => SwiftManagerTemplate {
            manager_name: format!("{module_name}Manager"),
        }
        .render(),

        "rs" => RustManagerTemplate {
            manager_name: format!("{module_name}Manager"),
        }
        .render(),

        _ => return Err(eyre::eyre!("unknown type")),
    };

    let file = template.wrap_err("failed to render view model")?;

    println!("{}", file);

    Ok(())
}
