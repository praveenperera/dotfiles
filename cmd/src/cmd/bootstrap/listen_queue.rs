//! Raises the macOS listen queue cap so local servers survive a browser's burst of connections
//!
//! macOS caps every listen queue at `kern.ipc.somaxconn`, 128 by default. Opening T3 Code
//! through Tailscale Serve makes about 200 connections to the local proxy at once, and the kernel
//! resets the ones past the cap, which the app reports as "Importing a module script failed".
//! macOS reads no sysctl.conf, so a root launch daemon reapplies the cap at every boot

use std::fs;

use colored::Colorize;
use eyre::{eyre, Result};
use xshell::{cmd, Shell};

/// The listen queue cap `kern.ipc.somaxconn` is raised to
const SOMAXCONN: u32 = 1024;
const LABEL: &str = "dev.praveen.somaxconn";
const PLIST_PATH: &str = "/Library/LaunchDaemons/dev.praveen.somaxconn.plist";

/// Installs the launch daemon and raises the cap now, asking for sudo only when either is missing
pub(super) fn ensure(sh: &Shell) -> Result<()> {
    let plist = plist();
    let installed = fs::read_to_string(PLIST_PATH).ok();
    if current_cap(sh)? >= SOMAXCONN && installed.as_deref() == Some(plist.as_str()) {
        return Ok(());
    }

    println!(
        "{}",
        format!("raising kern.ipc.somaxconn to {SOMAXCONN}").green()
    );

    // config also runs without a terminal, where sudo can't prompt; the rest of config still
    // applies, and the next run from a terminal installs the daemon
    if let Err(error) = install(sh, &plist) {
        println!(
            "{} {error}",
            "could not raise kern.ipc.somaxconn; rerun `cmd config` in a terminal:".yellow()
        );
    }

    Ok(())
}

fn current_cap(sh: &Shell) -> Result<u32> {
    let value = cmd!(sh, "sysctl -n kern.ipc.somaxconn").quiet().read()?;
    value
        .trim()
        .parse()
        .map_err(|error| eyre!("kern.ipc.somaxconn is not a number ({value:?}): {error}"))
}

fn install(sh: &Shell, plist: &str) -> Result<()> {
    let target = format!("system/{LABEL}");
    let setting = format!("kern.ipc.somaxconn={SOMAXCONN}");

    cmd!(sh, "sudo tee {PLIST_PATH}")
        .stdin(plist)
        .ignore_stdout()
        .run()?;
    cmd!(sh, "sudo chown root:wheel {PLIST_PATH}").run()?;
    cmd!(sh, "sudo chmod 0644 {PLIST_PATH}").run()?;

    // bootout fails when the daemon was never loaded, which is the fresh-install case
    let _ = cmd!(sh, "sudo launchctl bootout --wait {target}")
        .quiet()
        .ignore_stderr()
        .run();

    cmd!(sh, "sudo launchctl bootstrap system {PLIST_PATH}").run()?;
    cmd!(sh, "sudo sysctl -w {setting}").ignore_stdout().run()?;

    Ok(())
}

fn plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/sbin/sysctl</string>
    <string>-w</string>
    <string>kern.ipc.somaxconn={SOMAXCONN}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#
    )
}
