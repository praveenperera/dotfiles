#!/usr/bin/env python3

import argparse
import re
import sys
from pathlib import Path


REQUIRED_FILES = ("original-spec.md", "spec.md", "progress.md", "audit.md")
HOT_LIMITS = {
    "spec.md": 200,
    "progress.md": 60,
    "audit.md": 150,
}
SUPPORT_DIRS = ("phases", "audits", "context", "decisions", "evidence")
REQUIRED_SUPPORT_HEADERS = ("Read when:", "Do not read when:", "Temperature:")
FINAL_ONLY_RE = re.compile(
    r"(Supporting files consulted(?: for final audit)?|Original spec coverage|"
    r"Counter-evidence scan|Completion criteria|Final completion review|"
    r"Terminal manual intervention|Residual risk):\s*(\S.*)$",
    re.IGNORECASE,
)


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def hot_text(root: Path) -> str:
    chunks = []
    for name in HOT_LIMITS:
        path = root / name
        if path.exists():
            chunks.append(path.read_text(encoding="utf-8"))
    return "\n".join(chunks)


def audit_status(audit: Path) -> str:
    if not audit.exists():
        return ""
    for line in audit.read_text(encoding="utf-8").splitlines():
        if line.lower().startswith("status:"):
            return line.split(":", 1)[1].strip().lower()
    return ""


def is_pending(value: str) -> bool:
    normalized = value.strip().lower()
    return normalized in {"pending", "tbd", "todo", "not started", "-"}


def support_files(root: Path) -> list[Path]:
    files = []
    for dirname in SUPPORT_DIRS:
        directory = root / dirname
        if directory.exists():
            files.extend(sorted(directory.rglob("*.md")))
    return files


def validate(root: Path) -> tuple[list[str], list[str]]:
    errors = []
    warnings = []
    root = root.resolve()

    for name in REQUIRED_FILES:
        path = root / name
        if not path.exists():
            errors.append(f"{name}: missing required hot file")

    for name, limit in HOT_LIMITS.items():
        path = root / name
        if not path.exists():
            continue
        count = line_count(path)
        if count > limit:
            warnings.append(f"{name}: {count} lines exceeds target {limit}")

    audit = root / "audit.md"
    if audit.exists():
        text = audit.read_text(encoding="utf-8")
        passed_after_count = len(re.findall(r"\bpassed after\b", text, re.IGNORECASE))
        command_log_count = len(re.findall(r"(?m)^-\s+`[^`]+`:\s+", text))
        if passed_after_count:
            errors.append(
                f"audit.md: contains {passed_after_count} 'passed after' command-log entries"
            )
        if command_log_count > 8:
            errors.append(
                f"audit.md: contains {command_log_count} command-result bullets; use stable verification rows"
            )
        status = audit_status(audit)
        if not status:
            errors.append("audit.md: missing Status line")
        elif status not in {"complete", "completed", "terminal"}:
            for line_number, line in enumerate(text.splitlines(), start=1):
                match = FINAL_ONLY_RE.search(line)
                if match and not is_pending(match.group(2)):
                    errors.append(
                        f"audit.md:{line_number}: final-only field is filled before final audit"
                    )

    hot = hot_text(root)
    for path in support_files(root):
        rel = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        header_block = "\n".join(text.splitlines()[:12])
        for header in REQUIRED_SUPPORT_HEADERS:
            if header not in header_block:
                errors.append(f"{rel}: missing support header '{header}'")
        if rel not in hot:
            errors.append(f"{rel}: not referenced from hot files or Read Map")

    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate goal-ready plan files for token-efficient audit hygiene"
    )
    parser.add_argument("plan_dir", type=Path)
    args = parser.parse_args()

    errors, warnings = validate(args.plan_dir)
    for warning in warnings:
        print(f"WARN: {warning}", file=sys.stderr)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    if warnings:
        print("OK: plan file hygiene checks passed with warnings")
    else:
        print("OK: plan file hygiene checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
