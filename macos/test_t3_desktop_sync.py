"""Check update selection without changing the real T3 service"""

import fcntl
import importlib.util
import json
import plistlib
import sqlite3
import tempfile
import time
import unittest
from contextlib import closing
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "t3_desktop_sync", Path(__file__).with_name("t3-desktop-sync.py")
)
sync = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sync)


class DesktopSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.app = self.home / "T3 Code.app"
        self.state = self.home / ".t3/runtime/service-state.json"
        self.unit = self.home / "Library/LaunchAgents" / f"{sync.SERVICE}.plist"
        self.info = self.app / "Contents/Info.plist"
        self.settings = self.home / sync.DESKTOP_SETTINGS
        self.configure("0.0.43", "0.0.42")
        self.write_settings({"localEnvironmentEnabled": False})

    def write_settings(self, settings):
        self.settings.parent.mkdir(parents=True, exist_ok=True)
        self.settings.write_text(json.dumps(settings))

    def read_settings(self):
        return json.loads(self.settings.read_text())

    def configure(self, desktop, active, pending=False, launcher=None):
        for path in [self.info, self.unit, self.state]:
            path.parent.mkdir(parents=True, exist_ok=True)
        self.info.write_bytes(
            plistlib.dumps(
                {"CFBundleShortVersionString": desktop, "CFBundleExecutable": "T3 Code"}
            )
        )
        runtime = self.install_runtime(active)
        self.unit.write_bytes(
            plistlib.dumps(
                {"ProgramArguments": launcher or [str(runtime), "__service-launcher"]}
            )
        )
        self.state.write_text(
            json.dumps(
                {
                    "activeVersion": active,
                    "update": {"status": "pending" if pending else "committed"},
                }
            )
        )

    def install_runtime(self, version):
        runtime = self.home / ".t3/runtime/versions" / version / "t3"
        runtime.parent.mkdir(parents=True, exist_ok=True)
        runtime.touch(mode=0o755)
        return runtime

    def set_sessions(self, *statuses):
        database = self.home / ".t3/userdata/state.sqlite"
        database.parent.mkdir(parents=True, exist_ok=True)
        with closing(sqlite3.connect(database)) as db, db:
            db.execute(
                "create table if not exists projection_thread_sessions"
                " (thread_id text primary key, status text not null)"
            )
            db.execute("delete from projection_thread_sessions")
            db.executemany(
                "insert into projection_thread_sessions values (?, ?)",
                [(str(index), status) for index, status in enumerate(statuses)],
            )

    def test_equal_older_and_pending_do_not_update(self):
        for desktop, active, pending in [
            ("0.0.43", "0.0.43", False),
            ("0.0.42", "0.0.43", False),
            ("0.0.43", "0.0.42", True),
        ]:
            with self.subTest(desktop=desktop, active=active, pending=pending):
                self.configure(desktop, active, pending)
                with patch.object(sync, "run") as run:
                    sync.check(self.home, self.app)
                    run.assert_not_called()

    def test_new_desktop_updates_exact_version_and_repairs_service(self):
        commands = []

        def run(command, **_kwargs):
            commands.append(command)
            self.state.write_text(json.dumps({"activeVersion": "0.0.43"}))

        with (
            patch.object(sync.shutil, "which", return_value="/fake/npx"),
            patch.object(sync, "run", side_effect=run),
        ):
            sync.check(self.home, self.app)
        self.assertEqual(
            commands[0],
            ["/fake/npx", "--yes", "t3@0.0.43", "update", "0.0.43", "--yes"],
        )
        self.assertEqual(commands[1][1:], ["service", "install"])

    def test_failed_update_does_not_restart_or_mark_success(self):
        with (
            patch.object(sync.shutil, "which", return_value="/fake/npx"),
            patch.object(sync, "run", side_effect=RuntimeError("offline")) as run,
            self.assertRaisesRegex(RuntimeError, "offline"),
        ):
            sync.check(self.home, self.app)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(json.loads(self.state.read_text())["activeVersion"], "0.0.42")

    def test_same_version_keeps_launcher_from_older_release(self):
        # t3 update leaves the launcher that installed the job in place
        launcher = self.install_runtime("0.0.42")
        self.configure(
            "0.0.43", "0.0.43", launcher=[str(launcher), "__service-launcher"]
        )
        with patch.object(sync, "run") as run:
            sync.check(self.home, self.app)
            run.assert_not_called()

    def test_running_turns_defer_update_once(self):
        self.set_sessions("running", "ready")
        with patch.object(sync, "run") as run, patch.object(sync, "log") as log:
            sync.check(self.home, self.app)
            sync.check(self.home, self.app)
            run.assert_not_called()
        self.assertEqual(log.call_count, 1)

    def test_idle_service_clears_deferral_and_updates(self):
        self.set_sessions("starting")
        with patch.object(sync, "run") as run:
            sync.check(self.home, self.app)
            run.assert_not_called()
        self.set_sessions("ready", "stopped", "error")
        with (
            patch.object(sync.shutil, "which", return_value="/fake/npx"),
            patch.object(sync, "run") as run,
            self.assertRaisesRegex(RuntimeError, "did not activate"),
        ):
            sync.check(self.home, self.app)
        self.assertEqual(
            run.call_args_list[0].args[0][-3:], ["update", "0.0.43", "--yes"]
        )
        marker = self.home / "Library/Caches" / sync.LABEL / "deferred-since"
        self.assertFalse(marker.exists())

    def test_expired_deferral_updates_busy_service(self):
        self.set_sessions("running")
        marker = self.home / "Library/Caches" / sync.LABEL / "deferred-since"
        marker.parent.mkdir(parents=True)
        marker.write_text(str(time.time() - sync.MAX_DEFER_SECONDS - 1))
        with (
            patch.object(sync.shutil, "which", return_value="/fake/npx"),
            patch.object(sync, "run") as run,
            self.assertRaisesRegex(RuntimeError, "did not activate"),
        ):
            sync.check(self.home, self.app)
        self.assertTrue(run.called)

    def test_same_version_repairs_stale_launcher(self):
        self.configure("0.0.43", "0.0.43", launcher=["node", "service-launcher.mjs"])
        runtime = self.home / ".t3/runtime/versions/0.0.43/t3"
        with patch.object(sync, "run") as run:
            sync.check(self.home, self.app)
        self.assertEqual(run.call_args_list[0].args[0][0], str(runtime))

    def test_invalid_or_nightly_bundle_version_is_refused(self):
        for version in ["../0.0.43", "latest", "0.0.43-nightly.20260926.2282"]:
            self.configure(version, "0.0.42")
            with patch.object(sync, "run") as run, self.assertRaises(ValueError):
                sync.check(self.home, self.app)
            run.assert_not_called()

    def test_closed_app_gets_local_server_turned_off(self):
        self.configure("0.0.43", "0.0.43")
        for settings in [None, {"serverExposureMode": "network-accessible"}]:
            with self.subTest(settings=settings):
                if settings is None:
                    self.settings.unlink()
                else:
                    self.write_settings(settings)
                with (
                    patch.object(sync, "app_processes", return_value=[]),
                    patch.object(sync, "run") as run,
                ):
                    sync.check(self.home, self.app)
                run.assert_not_called()
                expected = {**(settings or {}), "localEnvironmentEnabled": False}
                self.assertEqual(self.read_settings(), expected)

    def test_running_app_quits_before_local_server_is_turned_off(self):
        # the app overwrites the file from memory, so it must stop first
        self.configure("0.0.43", "0.0.43")
        self.write_settings({"localEnvironmentEnabled": True})
        main = f"{self.app}/Contents/MacOS/T3 Code"
        running = [[(41, main), (42, f"{self.app}/Contents/Frameworks/Helper")], []]
        with (
            patch.object(sync, "app_processes", side_effect=lambda _app: running[0]),
            patch.object(
                sync.os, "kill", side_effect=lambda *_: running.pop(0)
            ) as kill,
            patch.object(sync, "run") as run,
        ):
            sync.check(self.home, self.app)
        kill.assert_called_once_with(41, sync.signal.SIGTERM)
        self.assertFalse(self.read_settings()["localEnvironmentEnabled"])
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0][:3], ["launchctl", "kickstart", "-k"])
        self.assertTrue(commands[0][3].endswith(f"/{sync.SERVICE}"))
        self.assertEqual(commands[1], ["open", "-a", str(self.app)])

    def test_running_turns_keep_app_local_server_until_idle(self):
        self.configure("0.0.43", "0.0.43")
        self.write_settings({"localEnvironmentEnabled": True})
        self.set_sessions("running")
        main = f"{self.app}/Contents/MacOS/T3 Code"
        with (
            patch.object(sync, "app_processes", return_value=[(41, main)]),
            patch.object(sync.os, "kill") as kill,
            patch.object(sync, "run") as run,
        ):
            sync.check(self.home, self.app)
        kill.assert_not_called()
        run.assert_not_called()
        self.assertTrue(self.read_settings()["localEnvironmentEnabled"])

    def test_version_order_is_numeric(self):
        self.assertGreater(sync.Version.parse("0.0.100"), sync.Version.parse("0.0.99"))

    def test_overlapping_check_does_not_update(self):
        lock_dir = self.home / "Library/Caches" / sync.LABEL
        lock_dir.mkdir(parents=True)
        with (
            (lock_dir / "check.lock").open("w") as lock,
            patch.object(sync, "run") as run,
        ):
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            sync.check(self.home, self.app)
            run.assert_not_called()

    def test_service_tool_paths_survive_update(self):
        config = sync.read_plist(self.unit)
        config["EnvironmentVariables"] = {"PATH": "/existing/provider/bin"}
        self.unit.write_bytes(plistlib.dumps(config))
        with (
            patch.object(sync.shutil, "which", return_value="/fake/npx"),
            patch.object(sync, "run") as run,
            self.assertRaisesRegex(RuntimeError, "did not activate"),
        ):
            sync.check(self.home, self.app)
        for call in run.call_args_list:
            self.assertIn("/existing/provider/bin", call.kwargs["env"]["PATH"])


if __name__ == "__main__":
    unittest.main()
