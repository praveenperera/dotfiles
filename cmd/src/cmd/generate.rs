pub mod rmp;

use askama::Template;
use clap::{Parser, Subcommand};
use convert_case::{Case, Casing};
use eyre::{Context as _, Result};
use log::{debug, info};
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;
use xshell::Shell;

use crate::util::hex_to_rgb;

#[derive(Debug, Clone, Parser)]
pub struct Generate {
    #[command(subcommand)]
    pub subcommand: GenerateCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GenerateCmd {
    /// Rust multi platform
    #[command(arg_required_else_help = true)]
    Rmp {
        /// either `swift` or `rs`
        lang: String,

        /// name of the module name ex: `MyModule`
        module_name: String,

        /// the name of the app, default to `cove`
        #[arg(short, long)]
        app: Option<String>,
    },

    /// Swift related generators
    #[command(arg_required_else_help = true)]
    Swift {
        name: String,
        identifier: String,
        path: Option<String>,
        #[arg(trailing_var_arg = true)]
        rest: Vec<String>,
    },

    /// Swift Colors
    #[command(arg_required_else_help = true)]
    SwiftColor {
        name: String,
        light_hex: String,
        dark_hex: Option<String>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Os {
    Ios,
    IosSim,
    Macos,
}

static RUST_VERSION: &str = "1.85.0";

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Generate::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Generate) -> Result<()> {
    match flags.subcommand {
        GenerateCmd::Rmp {
            lang,
            module_name,
            app,
        } => {
            let rmp_flags = rmp::RmpFlags {
                lang,
                module_name,
                app,
            };
            rmp::generate(sh, &rmp_flags)?;
        }

        GenerateCmd::Swift {
            name,
            identifier,
            path,
            rest,
        } => {
            let path = path.as_deref().unwrap_or(".");
            let rest = rest.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
            generate_swift(sh, &name, &identifier, path, &rest)?;
        }

        GenerateCmd::SwiftColor {
            name,
            light_hex,
            dark_hex,
        } => {
            generate_swift_color(sh, &name, &light_hex, dark_hex.as_deref())?;
        }
    }

    Ok(())
}

fn generate_swift(
    sh: &Shell,
    package_name: &str,
    identifier: &str,
    out_path: &str,
    flags: &[&str],
) -> Result<()> {
    info!("Generating Swift package {package_name} ({identifier}) in {out_path}");
    debug!("flags: {flags:?}");

    let out_path = PathBuf::from(out_path);

    let name = package_name.replace('-', "");
    let base_name = name.replace("ffi", "");
    let module_name = base_name.to_case(Case::Pascal);

    // release repo, create only github action and Package.swift
    if flags.contains(&"--release") || flags.contains(&"-r") {
        let github_actions_dir = out_path.join(".github").join("workflows");
        sh.create_dir(&github_actions_dir)?;

        let github_workflow = GithubActionsTemplate {
            package_name: package_name.to_string(),
            name: name.to_string(),
            module_name: module_name.clone(),
            base_name: base_name.clone(),
            rust_version: RUST_VERSION.to_string(),
        };

        let github_actions = github_actions_dir.join("publish-spm.yml");
        sh.write_file(
            github_actions,
            github_workflow
                .render()
                .wrap_err("failed to render github workflow")?,
        )?;

        return Ok(());
    }

    let xcframework_name = format!("{module_name}.xcframework");
    let framework_name = format!("{name}FFI.framework");

    let xcframework_path = out_path.join(xcframework_name);

    // Dirs for each platform
    let dirs = vec![
        (
            Os::Ios,
            xcframework_path.join("ios-arm64").join(&framework_name),
        ),
        (
            Os::IosSim,
            xcframework_path
                .join("ios-arm64_x86_64-simulator")
                .join(&framework_name),
        ),
        (
            Os::Macos,
            xcframework_path
                .join("macos-arm64_x86_64")
                .join(&framework_name),
        ),
    ];

    // Create the output directory
    sh.create_dir(&out_path)?;

    // Write the files
    // build.sh
    let build_sh = BuildShTemplate {
        base_name: base_name.to_string(),
        package_name: package_name.to_string(),
        rust_version: RUST_VERSION.to_string(),
        name: name.to_string(),
        module_name: module_name.clone(),
    };

    sh.write_file(
        out_path.join("build.sh"),
        build_sh.render().wrap_err("failed to render build.sh")?,
    )?;

    // Info.plist
    let info_plist = InfoPlistTemplate {
        name: name.to_string(),
    };

    sh.write_file(
        xcframework_path.join("Info.plist"),
        info_plist
            .render()
            .wrap_err("failed to render Info.plist")?,
    )?;

    // Package.swift
    let package_swift = PackageSwiftTemplate {
        name: name.to_string(),
        module_name: module_name.clone(),
        is_template: false,
    };

    sh.write_file(
        out_path.join("Package.swift"),
        package_swift
            .render()
            .wrap_err("failed to render Package.swift")?,
    )?;

    // Package.swift.txt
    let package_swift_template = PackageSwiftTemplate {
        name: name.to_string(),
        module_name: module_name.clone(),
        is_template: true,
    };

    sh.write_file(
        out_path.join("Package.swift.txt"),
        package_swift_template
            .render()
            .wrap_err("failed to render Package.swift")?,
    )?;

    // files for each platform
    for (os, dir) in &dirs {
        sh.create_dir(dir)?;

        let modules_dir = dir.join("Modules");
        sh.create_dir(&modules_dir)?;

        let headers_dir = dir.join("Headers");
        sh.create_dir(&headers_dir)?;

        // Inner Info.plist
        let inner_info_plist = InnerInfoPlistTemplate {
            name: name.to_string(),
            bundle_id: identifier.to_string(),
            os: *os,
        };

        sh.write_file(
            dir.join("Info.plist"),
            inner_info_plist
                .render()
                .wrap_err("failed to render Info.plist")?,
        )?;

        // Module.modulemap
        let module_map = ModuleMapTemplate {
            name: name.to_string(),
        };

        sh.write_file(
            modules_dir.join("module.modulemap"),
            module_map
                .render()
                .wrap_err("failed to render module.modulemap")?,
        )?;

        // Umbrella header
        let umbrella_header = UmbrellaHeader {
            name: name.to_string(),
        };

        sh.write_file(
            headers_dir.join(format!("{name}FFI-umbrella.h")),
            umbrella_header
                .render()
                .wrap_err("failed to render umbrella header")?,
        )?;
    }

    Ok(())
}

fn generate_swift_color(
    _sh: &Shell,
    color_name: &str,
    light_hex: &str,
    dark_hex: Option<&str>,
) -> Result<()> {
    info!("Generating Swift Color Set for {color_name}");
    let dark_hex = dark_hex.unwrap_or(light_hex);
    let light_rgb = hex_to_rgb(light_hex)?;
    let dark_rgb = hex_to_rgb(dark_hex)?;

    let colorset = json!({
      "colors": [
        {
          "color": {
            "color-space": "srgb",
            "components": {
              "alpha": "1.000",
              "blue": format!("{:.4}", light_rgb.2),
              "green": format!("{:.4}", light_rgb.1),
              "red": format!("{:.4}", light_rgb.0)
            }
          },
          "idiom": "universal"
        },
        {
          "appearances": [
            {
              "appearance": "luminosity",
              "value": "dark"
            }
          ],
          "color": {
            "color-space": "srgb",
            "components": {
              "alpha": "1.000",
              "blue": format!("{:.4}", dark_rgb.2),
              "green": format!("{:.4}", dark_rgb.1),
              "red": format!("{:.4}", dark_rgb.0)
            }
          },
          "idiom": "universal"
        }
      ],
      "info": {
        "author": "xcode",
        "version": 1
      }
    });

    println!("{colorset}");
    let current_dir = std::env::current_dir()?;
    let folder_name = format!("{color_name}.colorset");
    let output_path = current_dir.join(folder_name).join("Contents.json");

    let _ = std::fs::remove_file(&output_path);
    std::fs::create_dir_all(output_path.parent().unwrap())?;
    std::fs::write(output_path, colorset.to_string())?;

    Ok(())
}

// TEMPLATES
#[derive(askama::Template)]
#[template(path = "xcframework/build.sh.j2")]
struct BuildShTemplate {
    name: String,
    rust_version: String,
    base_name: String,
    package_name: String,
    module_name: String,
}

#[derive(askama::Template)]
#[template(path = "xcframework/info.plist.j2")]
struct InfoPlistTemplate {
    name: String,
}

#[derive(askama::Template)]
#[template(path = "xcframework/inner_info.plist.j2")]
struct InnerInfoPlistTemplate {
    name: String,
    bundle_id: String,
    os: Os,
}

#[derive(askama::Template)]
#[template(path = "xcframework/module.modulemap.j2")]
struct ModuleMapTemplate {
    name: String,
}

#[derive(askama::Template)]
#[template(path = "xcframework/package.swift.j2")]
struct PackageSwiftTemplate {
    name: String,
    module_name: String,
    is_template: bool,
}

#[derive(askama::Template)]
#[template(path = "xcframework/umbrella.h.j2")]
struct UmbrellaHeader {
    name: String,
}

#[derive(askama::Template)]
#[template(
    path = "xcframework/github_action.yaml.custom",
    ext = "custom",
    syntax = "custom"
)]
struct GithubActionsTemplate {
    package_name: String,
    name: String,
    module_name: String,
    base_name: String,
    rust_version: String,
}
