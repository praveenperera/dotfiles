"""Check remote tmux routing and quoting without an SSH connection."""

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
        ssh = Path(self.directory.name) / "ssh"
        ssh.write_text(
            f"#!{sys.executable}\n"
            "import json, os, sys\n"
            "print(json.dumps(sys.argv[1:]))\n"
            "sys.exit(int(os.environ.get('FLEET_TEST_SSH_EXIT', '0')))\n"
        )
        ssh.chmod(0o755)
        self.environment = dict(os.environ)
        self.environment["PATH"] = self.directory.name + os.pathsep + os.environ["PATH"]

    def invoke(self, *arguments):
        return subprocess.run(
            ["zsh", "-f", "-c", 'source "$1"; shift; "$@"', "test", str(REMOTE_ZSH), *arguments],
            env=self.environment,
            text=True,
            capture_output=True,
        )

    def test_lists_without_allocating_a_terminal(self):
        for command, host in (("ttc", "code"), ("ttt", "training")):
            for arguments in ((), ("-l",), ("--list",)):
                with self.subTest(command=command, arguments=arguments):
                    result = self.invoke(command, *arguments)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(json.loads(result.stdout), ["--", f"praveen@{host}.local", "tmux list-sessions"])

    def test_create_or_attach_preserves_names_through_remote_shell(self):
        name = "two words'$(printf unsafe >&2)"
        result = self.invoke("ttc", name)
        self.assertEqual(result.returncode, 0, result.stderr)
        arguments = json.loads(result.stdout)
        self.assertEqual(arguments[:-1], ["-t", "--", "praveen@code.local"])
        remote = subprocess.run(
            ["/bin/sh", "-c", 'tmux() { printf "%s\\n" "$@"; }; ' + arguments[-1]],
            text=True,
            capture_output=True,
        )
        self.assertEqual(remote.returncode, 0, remote.stderr)
        self.assertEqual(remote.stderr, "")
        self.assertEqual(remote.stdout.splitlines(), ["new-session", "-A", "-s", name])

    def test_attach_only_uses_exact_session_target(self):
        result = self.invoke("ttt", "-a", "app")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), ["-t", "--", "praveen@training.local", "tmux attach-session -t '=app'"])

    def test_invalid_arguments_never_connect(self):
        for arguments in (("-a",), ("",), ("one", "two"), ("a.b",), ("a:b",), ("a\nb",), ("a\rb",), ("--unknown",)):
            with self.subTest(arguments=arguments):
                result = self.invoke("ttc", *arguments)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("Usage:", result.stderr)

    def test_help_never_connects(self):
        result = self.invoke("ttc", "--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Usage:", result.stdout)

    def test_ssh_failure_is_returned(self):
        self.environment["FLEET_TEST_SSH_EXIT"] = "255"
        for arguments in ((), ("app",), ("-a", "app")):
            with self.subTest(arguments=arguments):
                self.assertEqual(self.invoke("ttc", *arguments).returncode, 255)


if __name__ == "__main__":
    unittest.main()
