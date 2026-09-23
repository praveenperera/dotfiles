"""Check that the public remote tmux shell functions remain thin and exact."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REMOTE_ZSH = Path(__file__).resolve().parents[1] / "remote.zsh"


class RemoteTmuxTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        command = Path(self.directory.name) / "cmd"
        command.write_text(
            f"#!{sys.executable}\n"
            "import json, os, sys\n"
            "print(json.dumps(sys.argv[1:]))\n"
            "sys.exit(int(os.environ.get('FLEET_TEST_CMD_EXIT', '0')))\n"
        )
        command.chmod(0o755)
        self.environment = dict(os.environ)
        self.environment["PATH"] = self.directory.name + os.pathsep + os.environ["PATH"]

    def invoke(self, *arguments):
        return subprocess.run(
            ["zsh", "-f", "-c", 'source "$1"; shift; "$@"', "test", str(REMOTE_ZSH), *arguments],
            env=self.environment,
            text=True,
            capture_output=True,
        )

    def test_routes_public_command_to_code(self):
        result = self.invoke("ttc")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), ["tmux", "remote", "code"])

    def test_forwards_legacy_and_explicit_mode_arguments_exactly(self):
        cases = (
            ("ttc", "two words'$(printf unsafe >&2)"),
            ("ttc", "-a", "app"),
            ("ttc", "--attach", "app"),
            ("ttc", "--list"),
            ("ttc", "--mode", "minimal", "app"),
            ("ttc", "--mode", "normal", "app"),
            ("ttc", "--help"),
        )
        for arguments in cases:
            with self.subTest(arguments=arguments):
                result = self.invoke(*arguments)
                self.assertEqual(result.returncode, 0, result.stderr)
                expected = ["tmux", "remote", "code", *arguments[1:]]
                self.assertEqual(json.loads(result.stdout), expected)

    def test_cmd_failure_is_returned(self):
        self.environment["FLEET_TEST_CMD_EXIT"] = "255"
        for arguments in (("ttc",), ("ttc", "app"), ("ttc", "-a", "app")):
            with self.subTest(arguments=arguments):
                self.assertEqual(self.invoke(*arguments).returncode, 255)


if __name__ == "__main__":
    unittest.main()
