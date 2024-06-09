use std::path::PathBuf;

use askama::Template as _;
use convert_case::{Case, Casing};
use eyre::{Context as _, Result};
use xshell::Shell;

static RUST_VERSION: &str = "1.78.0";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Os {
    Ios,
    IosSim,
    Macos,
}

pub fn run(sh: &Shell, args: &[&str]) -> Result<()> {
    match args {
        [] => eprintln!("need args"),

        ["swift", name, identifier] => {
            generate_swift(sh, name, identifier, ".")?;
        }

        ["swift", name, identifier, path] => {
            generate_swift(sh, name, identifier, path)?;
        }

        cmd => {
            eprintln!("generate command not implemented: {cmd:?}");
        }
    }

    Ok(())
}

fn generate_swift(sh: &Shell, package_name: &str, identifier: &str, out_path: &str) -> Result<()> {
    log::info!("Generating Swift package {package_name} ({identifier}) in {out_path}");

    let out_path = PathBuf::from(out_path);

    let name = package_name.replace('-', "");
    let base_name = name.replace("ffi", "");
    let module_name = base_name.to_case(Case::Pascal);

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
    };

    sh.write_file(
        out_path.join("Package.swift"),
        package_swift
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
}

#[derive(askama::Template)]
#[template(path = "xcframework/umbrella.h.j2")]
struct UmbrellaHeader {
    name: String,
}
