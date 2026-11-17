//! Frees a TCP port, behind the `killp` shell alias

use std::process::Command;

use eyre::{eyre, Result, WrapErr};

use crate::process::Process;

/// SIGKILLs every process listening on TCP `port`
///
/// Only listeners count, since a browser or client connected to the port also
/// shows up in `lsof -i` and must not be killed
pub fn kill(port: u16) -> Result<()> {
    let pids = listeners(port)?;

    if pids.is_empty() {
        println!("No process listening on port {port}");
        return Ok(());
    }

    for pid in pids {
        let name = Process::lookup(pid).map(|process| process.name().to_owned());

        // SAFETY: kill(2) takes plain integers and touches no memory
        if unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) } != 0 {
            let error = std::io::Error::last_os_error();
            return Err(eyre!("could not kill pid {pid}: {error}"));
        }

        let name = name.unwrap_or_else(|| "unknown".to_owned());
        println!("Killed pid {pid} ({name}) listening on port {port}");
    }

    Ok(())
}

fn listeners(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("lsof")
        .args(["-t", &format!("-iTCP:{port}"), "-sTCP:LISTEN"])
        .output()
        .wrap_err("could not run lsof")?;

    // lsof exits 1 when nothing matches, so read its output either way
    let mut pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();

    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}
