#!/usr/bin/python3
"""Keep the Mac T3 Code background service at the installed desktop version"""

import argparse
import fcntl
import json
import os
import plistlib
import re
import shutil
import sqlite3
import subprocess
import sys
import time
from contextlib import closing
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo

LABEL = "dev.praveen.t3-desktop-sync"
SERVICE = "com.t3tools.t3code.service"
APP = Path("/Applications/T3 Code (Alpha).app")
# a crashed server leaves its sessions marked running until it starts again, so
# a busy service still updates once this much time has passed
MAX_DEFER_SECONDS = 6 * 60 * 60


@dataclass(frozen=True, order=True)
class Version:
    major: int
    minor: int
    patch: int

    @classmethod
    def parse(cls, value):
        # bundle versions must identify exact releases, not paths or npm tags
        if not isinstance(value, str) or not re.fullmatch(
            r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value
        ):
            raise ValueError(f"Unsupported release version: {value!r}")
        return cls(*map(int, value.split(".")))

    def __str__(self):
        return f"{self.major}.{self.minor}.{self.patch}"


def log(message):
    stamp = datetime.now(ZoneInfo("America/Chicago")).strftime("%Y-%m-%d %H:%M:%S %Z")
    print(f"{stamp} {message}", flush=True)


def read_plist(path):
    with path.open("rb") as source:
        return plistlib.load(source)


def run(command, *, env=None):
    subprocess.run(command, check=True, timeout=900, env=env)


def installed_launcher(home, launcher):
    # t3 update keeps the launcher that installed the job and that launcher
    # starts the active release, so any installed release's launcher is current
    if not isinstance(launcher, list) or len(launcher) != 2:
        return False
    if launcher[1] != "__service-launcher":
        return False
    binary = Path(launcher[0])
    versions = home / ".t3/runtime/versions"
    return (
        binary.name == "t3"
        and binary.parent.parent == versions
        and os.access(binary, os.X_OK)
    )


def running_turns(home):
    """Count agent sessions that a service restart would interrupt"""
    database = home / ".t3/userdata/state.sqlite"
    if not database.exists():
        return 0
    # the desktop and the service share this database, so both count
    with closing(
        sqlite3.connect(f"{database.as_uri()}?mode=ro", uri=True, timeout=5)
    ) as db:
        (count,) = db.execute(
            "select count(*) from projection_thread_sessions"
            " where status in ('running', 'starting')"
        ).fetchone()
    return count


def defer_for_turns(home):
    """Return True while running agent turns should postpone a restart"""
    marker = home / "Library/Caches" / LABEL / "deferred-since"
    busy = running_turns(home)
    if busy == 0:
        marker.unlink(missing_ok=True)
        return False
    now = time.time()
    try:
        since = float(marker.read_text())
    except (OSError, ValueError):
        marker.write_text(str(now))
        log(f"Deferring the T3 Code service restart while {busy} agent sessions run")
        return True
    if now - since < MAX_DEFER_SECONDS:
        return True
    log(f"Restarting the T3 Code service after deferring for {int(now - since)}s")
    marker.unlink(missing_ok=True)
    return False


def sync(home, app):
    unit = home / "Library/LaunchAgents" / f"{SERVICE}.plist"
    state_path = home / ".t3/runtime/service-state.json"
    info = app / "Contents/Info.plist"
    if not info.exists() or not unit.exists():
        return

    target = Version.parse(read_plist(info).get("CFBundleShortVersionString"))
    state = json.loads(state_path.read_text())
    if not isinstance(state, dict):
        raise TypeError("Invalid T3 Code service state")
    active = Version.parse(state.get("activeVersion"))
    update = state.get("update") or {}
    if not isinstance(update, dict):
        raise TypeError("Invalid T3 Code update state")
    if update.get("status") == "pending" or target < active:
        return

    runtime = home / ".t3/runtime/versions" / str(target) / "t3"
    unit_config = read_plist(unit)
    launcher = unit_config.get("ProgramArguments")
    if target == active and installed_launcher(home, launcher):
        return
    # the update and service install below restart the service and every agent
    # turn in it
    if defer_for_turns(home):
        return

    # execute the target release's updater so a stale launcher cannot block repair
    if os.access(runtime, os.X_OK):
        cli = [str(runtime)]
    else:
        npx = shutil.which("npx")
        if npx is None:
            raise RuntimeError("Cannot find npx to download the desktop release")
        cli = [npx, "--yes", f"t3@{target}"]

    log(f"Updating T3 Code service from {active} to desktop version {target}")
    # service installation captures PATH; retain provider paths from the existing job
    environment = os.environ.copy()
    service_path = unit_config.get("EnvironmentVariables", {}).get("PATH", "")
    environment["PATH"] = f"{environment.get('PATH', '')}:{service_path}"
    environment["T3CODE_HOME"] = str(home / ".t3")
    run(cli + ["update", str(target), "--yes"], env=environment)
    # an interrupted update can leave the new runtime installed but the job unloaded
    run([str(runtime), "service", "install"], env=environment)
    current = json.loads(state_path.read_text())
    if current.get("activeVersion") != str(target):
        raise RuntimeError("The service did not activate the desktop version")
    run([str(runtime), "service", "status"])
    log(f"T3 Code service updated to {target}")


def check(home, app):
    # launchd serializes its own runs; this lock also covers manual invocations
    lock_dir = home / "Library/Caches" / LABEL
    lock_dir.mkdir(parents=True, exist_ok=True)
    with (lock_dir / "check.lock").open("w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return
        sync(home, app)


def install(home, app):
    destination = home / ".local/bin/t3-desktop-sync.py"
    destination.parent.mkdir(parents=True, exist_ok=True)
    if Path(__file__).resolve() != destination.resolve():
        shutil.copyfile(__file__, destination)
    destination.chmod(0o755)
    agent_dir = home / "Library/LaunchAgents"
    agent_dir.mkdir(parents=True, exist_ok=True)
    log_dir = home / "Library/Logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / "t3-desktop-sync.log"
    plist = agent_dir / f"{LABEL}.plist"
    config = {
        "Label": LABEL,
        "ProgramArguments": ["/usr/bin/python3", str(destination), "--app", str(app)],
        "RunAtLoad": True,
        "StartInterval": 300,
        "ProcessType": "Background",
        "WorkingDirectory": str(home),
        "EnvironmentVariables": {
            "HOME": str(home),
            "PATH": f"{home}/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        },
        "StandardOutPath": str(log_path),
        "StandardErrorPath": str(log_path),
    }
    with plist.open("wb") as output:
        plistlib.dump(config, output)
    target = f"gui/{os.getuid()}/{LABEL}"
    # macOS does not support bootout --wait; unload only this check's own job
    subprocess.run(["launchctl", "bootout", target], capture_output=True, check=False)
    run(["launchctl", "enable", target])
    run(["launchctl", "bootstrap", f"gui/{os.getuid()}", str(plist)])
    log("Automatic T3 Code service checks enabled every five minutes")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=["check", "install"], default="check", nargs="?"
    )
    parser.add_argument("--app", type=Path, default=APP)
    args = parser.parse_args()
    if sys.platform != "darwin" or os.getuid() == 0:
        parser.error("Run this command on macOS as the T3 Code service user")
    try:
        if args.command == "install":
            install(Path.home(), args.app.resolve())
        else:
            check(Path.home(), args.app.resolve())
    except (
        OSError,
        ValueError,
        TypeError,
        RuntimeError,
        sqlite3.Error,
        subprocess.SubprocessError,
    ) as error:
        log(f"T3 Code service check failed: {error}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
