//! Quick generations fo (R)ust (M)ulti(P)latform
//!
use askama::Template as _;
use convert_case::{Case, Casing};
use eyre::{Context as _, Result};
use xshell::Shell;

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/view_model.rs.j2")]
struct RustViewModelTemplate {
    module_name: String,
}

#[derive(askama::Template)]
#[template(path = "rust-multiplatform/view_model.swift.j2")]
struct SwiftViewModelTemplate {
    module_name: String,
}

pub fn generate(_sh: &Shell, type_: &str, module_name: &str) -> Result<()> {
    let module_name = module_name.to_case(Case::Pascal);

    let template = match type_ {
        "swift" => SwiftViewModelTemplate {
            module_name: module_name.to_string(),
        }
        .render(),

        "rs" => RustViewModelTemplate {
            module_name: module_name.to_string(),
        }
        .render(),

        _ => return Err(eyre::eyre!("unknown type")),
    };

    let file = template.wrap_err("failed to render view model")?;

    println!("{}", file);

    Ok(())
}
