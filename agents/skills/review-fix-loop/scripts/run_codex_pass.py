#!/usr/bin/env python3
"""Run a review-fix prompt in a fresh Codex exec session."""

import argparse
import shlex
import subprocess
import sys
from pathlib import Path


def parse_args():
    parser = argparse.ArgumentParser(
        description="Run codex exec with a prompt file and no resume path.",
    )
    parser.add_argument("--repo", default=".", help="Repository root for codex --cd")
    parser.add_argument("--prompt-file", required=True, help="Markdown prompt to pass on stdin")
    parser.add_argument(
        "--output-file",
        required=True,
        help="File where Codex writes the last assistant message",
    )
    parser.add_argument("--codex-bin", default="codex", help="Codex executable")
    parser.add_argument("--model", help="Optional Codex model")
    parser.add_argument("--profile", help="Optional Codex config profile")
    parser.add_argument(
        "--config",
        action="append",
        default=[],
        help="Optional Codex config override, repeatable; passed as --config key=value",
    )
    parser.add_argument(
        "--sandbox",
        choices=["read-only", "workspace-write", "danger-full-access"],
        help="Optional Codex sandbox mode",
    )
    parser.add_argument(
        "--bypass-approvals-and-sandbox",
        action="store_true",
        help="Pass Codex's dangerous bypass flag",
    )
    parser.add_argument("--json", action="store_true", help="Ask Codex to emit JSONL events")
    parser.add_argument("--dry-run", action="store_true", help="Print the command and exit")
    return parser.parse_args()


def resolve_existing_dir(path, label):
    resolved = Path(path).expanduser().resolve()
    if not resolved.is_dir():
        raise SystemExit(f"{label} is not a directory: {resolved}")
    return resolved


def resolve_existing_file(path, label):
    resolved = Path(path).expanduser().resolve()
    if not resolved.is_file():
        raise SystemExit(f"{label} is not a file: {resolved}")
    return resolved


def build_command(args, repo, output_file):
    command = [args.codex_bin, "exec", "--cd", str(repo)]
    for config in args.config:
        command.extend(["--config", config])
    if args.model:
        command.extend(["--model", args.model])
    if args.profile:
        command.extend(["--profile", args.profile])
    if args.bypass_approvals_and_sandbox:
        if args.sandbox:
            raise SystemExit("use either --sandbox or --bypass-approvals-and-sandbox, not both")
        command.append("--dangerously-bypass-approvals-and-sandbox")
    elif args.sandbox:
        command.extend(["--sandbox", args.sandbox])
    if args.json:
        command.append("--json")
    command.extend(["--output-last-message", str(output_file), "-"])
    return command


def main():
    args = parse_args()
    repo = resolve_existing_dir(args.repo, "--repo")
    prompt_file = resolve_existing_file(args.prompt_file, "--prompt-file")
    output_file = Path(args.output_file).expanduser().resolve()
    output_file.parent.mkdir(parents=True, exist_ok=True)

    command = build_command(args, repo, output_file)
    if args.dry_run:
        printable = " ".join(shlex.quote(part) for part in command)
        print(f"{printable} < {shlex.quote(str(prompt_file))}")
        return 0

    prompt = prompt_file.read_text()
    result = subprocess.run(command, input=prompt, text=True, cwd=repo)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
