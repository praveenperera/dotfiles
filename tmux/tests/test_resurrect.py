from pathlib import Path
import os
import subprocess


ROOT = Path(__file__).resolve().parents[2]
FILTER = ROOT / "tmux" / "reject-empty-save.sh"
SAFE_RESTORE = ROOT / "tmux" / "safe-restore.sh"
FIXTURES = Path(__file__).parent / "fixtures"


def write_fake_tmux(bin_dir: Path) -> None:
    tmux = bin_dir / "tmux"
    tmux.write_text(
        "#!/usr/bin/env bash\n"
        "if [[ $1 == list-sessions ]]; then\n"
        "  printf '%b' \"${FLEET_SESSIONS:-}\"\n"
        "elif [[ $1 == list-panes ]]; then\n"
        "  printf '%b' \"${FLEET_PANES:-}\"\n"
        "elif [[ $1 == show-option ]]; then\n"
        "  printf '%s\\n' \"${FLEET_RESTORE_COMMAND:-}\"\n"
        "elif [[ $1 == run-shell ]]; then\n"
        "  printf '%s\\n' \"$3\" > \"$TMUX_RUN_LOG\"\n"
        "fi\n"
    )
    tmux.chmod(0o755)


def environment(tmp_path: Path) -> dict[str, str]:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    write_fake_tmux(bin_dir)
    home = tmp_path / "home"
    (home / ".tmux" / "resurrect").mkdir(parents=True)
    return {**os.environ, "HOME": str(home), "PATH": f"{bin_dir}:{os.environ['PATH']}"}


def test_filters_managed_rows_and_managed_active_state(tmp_path: Path) -> None:
    env = environment(tmp_path)
    env["FLEET_SESSIONS"] = (
        "workspace\\tworkspace\\n__fleet_view_1\\tview\\n"
        "__fleet_service\\tservice\\n__fleet_display\\tdisplay\\n"
    )
    snapshot = tmp_path / "snapshot.txt"
    snapshot.write_bytes((FIXTURES / "resurrect-mixed.txt").read_bytes())

    subprocess.run([FILTER, snapshot], env=env, check=True)

    assert snapshot.read_bytes() == (FIXTURES / "resurrect-filtered.txt").read_bytes()
    assert b"ssh code" not in snapshot.read_bytes()


def test_filters_display_session_and_managed_outer_launcher(tmp_path: Path) -> None:
    env = environment(tmp_path)
    env["FLEET_SESSIONS"] = "fleet\\tworkspace\\n__fleet_display\\tdisplay\\n"
    env["FLEET_PANES"] = (
        "fleet\\t2\\t0\\t%40\\tcode:123:@9\\n"
        "fleet\\t2\\t1\\t%41\\tcode:123:@9\\n"
        "fleet\\t2\\t2\\t%42\\tcode:123:@9\\n"
    )
    snapshot = tmp_path / "snapshot.txt"
    snapshot.write_bytes((FIXTURES / "resurrect-managed-outer.txt").read_bytes())

    subprocess.run([FILTER, snapshot], env=env, check=True)

    assert snapshot.read_bytes() == (FIXTURES / "resurrect-managed-outer-filtered.txt").read_bytes()
    assert b"ssh code" not in snapshot.read_bytes()
    assert b":exit" in snapshot.read_bytes()


def test_removes_outer_window_when_it_has_only_its_launcher(tmp_path: Path) -> None:
    env = environment(tmp_path)
    env["FLEET_SESSIONS"] = "fleet\\tworkspace\\n"
    env["FLEET_PANES"] = "fleet\\t2\\t0\\t%40\\tcode:123:@9\\n"
    snapshot = tmp_path / "snapshot.txt"
    snapshot.write_text(
        "pane\tfleet\t0\t1\t:*\t0\tlocal\t:/repo\t1\tzsh\t:zsh\n"
        "window\tfleet\t0\tlocal\t1\t:*\tlocal-layout\toff\n"
        "pane\tfleet\t2\t0\t:\t0\tremote\t:/repo\t1\tssh\t:ssh code\n"
        "window\tfleet\t2\tremote\t0\t:\tremote-layout\toff\n"
    )

    subprocess.run([FILTER, snapshot], env=env, check=True)

    assert snapshot.read_text() == (
        "pane\tfleet\t0\t1\t:*\t0\tlocal\t:/repo\t1\tzsh\t:zsh\n"
        "window\tfleet\t0\tlocal\t1\t:*\tlocal-layout\toff\n"
    )


def test_filtered_empty_save_keeps_previous_canonical_snapshot(tmp_path: Path) -> None:
    env = environment(tmp_path)
    env["FLEET_SESSIONS"] = "__fleet_view_1\\tview\\n__fleet_service\\tservice\\n"
    snapshot = tmp_path / "snapshot.txt"
    snapshot.write_text(
        "pane\t__fleet_view_1\t0\t1\t:*\t0\tremote\t:/repo\t1\tssh\t:ssh code\n"
        "window\t__fleet_view_1\t0\tremote\t1\t:*\tlayout\toff\n"
        "state\t__fleet_view_1\t__fleet_service\n"
    )
    last = Path(env["HOME"]) / ".tmux" / "resurrect" / "last"
    last.write_bytes((FIXTURES / "resurrect-filtered.txt").read_bytes())

    subprocess.run([FILTER, snapshot], env=env, check=True)

    assert snapshot.read_bytes() == (FIXTURES / "resurrect-filtered.txt").read_bytes()


def test_filtered_empty_save_without_fallback_stays_empty(tmp_path: Path) -> None:
    env = environment(tmp_path)
    env["FLEET_SESSIONS"] = "__fleet_service\\tservice\\n"
    snapshot = tmp_path / "snapshot.txt"
    snapshot.write_text("pane\t__fleet_service\t0\t1\t:*\t0\tsync\t:/repo\t1\tcmd\t:cmd sync\n")

    subprocess.run([FILTER, snapshot], env=env, check=True)

    last = Path(env["HOME"]) / ".tmux" / "resurrect" / "last"
    assert snapshot.read_bytes() == b""
    assert last.read_bytes() == b""


def test_post_restore_rebuild_uses_configured_command(tmp_path: Path) -> None:
    env = environment(tmp_path)
    log = tmp_path / "run.log"
    env.update(TMUX_RUN_LOG=str(log), FLEET_RESTORE_COMMAND="custom rebuild --safe")

    subprocess.run([SAFE_RESTORE, "--rebuild-managed"], env=env, check=True)

    assert log.read_text().strip() == "custom rebuild --safe"
