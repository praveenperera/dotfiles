#!/usr/bin/env python3
"""Resolve /ship words and repos.toml into one explicit, ordered step plan

The model never decides what "end to end" means: this script does, so the same
words in the same repo always produce the same plan, and anything it cannot
resolve is an error instead of a guess

Usage:
  ship_plan.py -- [WORD ...]            plan for the repo in the current directory
  ship_plan.py --repo OWNER/NAME -- ... plan for a named repo
  ship_plan.py --json -- ...            machine-readable plan
  ship_plan.py --task SLUG -- ...       also save the plan to _scratch/ship/SLUG/plan.json
  ship_plan.py --task SLUG --done STEP...  record finished steps in that plan
  ship_plan.py --check                  validate repos.toml
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from dataclasses import asdict, dataclass, field
from pathlib import Path

CONFIG_PATH = Path(__file__).resolve().parents[1] / "repos.toml"

# Canonical order; typed order never matters
STEPS = ["sol", "commit", "push", "pr", "greptile", "merge", "deploy", "publish", "clean", "settle"]

# Each step pulls in its prerequisite so `/ship merge` on a direct-to-master repo
# still opens the PR it merges
REQUIRES = {"pr": "push", "greptile": "pr", "merge": "pr"}

# Spoken and transcribed variants seen in Praveen's dictated prompts
STEP_ALIASES = {
    "soul": "sol",
    "review": "sol",
    "codex": "sol",
    "reptile": "greptile",
    "release": "deploy",
    "cleanup": "clean",
    "clean-up": "clean",
    "settled": "settle",
    "s": "settle",
    "set": "settle",
    "squash": "merge",
}
EFFORTS = {"high": "high", "xhigh": "xhigh", "xi": "xhigh", "x-high": "xhigh", "extra-high": "xhigh"}
BUMPS = {"patch", "minor", "major"}
STATUS_WORDS = {"?", "status"}

LIST_FIELDS = {"steps", "deploy", "deploy_paths", "deploy_otherwise", "publish"}
BOOL_FIELDS = {"greptile", "pr_draft"}
STR_FIELDS = {"merge", "pr_repo", "notes"}


class PlanError(Exception):
    """A word or config value that cannot be resolved without guessing"""


@dataclass
class Plan:
    """Resolved plan for one repo"""

    repo: str
    source: str
    external: bool
    pr_repo: str
    mode: str = "run"
    confirm: bool = False
    steps: list[str] = field(default_factory=list)
    implied: list[str] = field(default_factory=list)
    sol_effort: str = "xhigh"
    bump: str = "patch"
    merge_method: str = "squash"
    pr_draft: bool = False
    commands: dict[str, list[str]] = field(default_factory=dict)
    notes: str = ""

    def summary(self) -> str:
        labels = {
            "sol": f"sol 6.1 {self.sol_effort} review",
            "pr": f"{'draft ' if self.pr_draft else ''}pr ({self.pr_repo})",
            "greptile": "greptile 5/5",
            "merge": f"merge ({self.merge_method})",
        }
        return " → ".join(labels.get(step, step) for step in self.steps)


def load_config(path: Path = CONFIG_PATH) -> dict:
    with path.open("rb") as fh:
        return tomllib.load(fh)


def parse_github_remote(url: str) -> str | None:
    match = re.search(r"github\.com[:/]([^/]+/[^/]+?)(?:\.git)?/?$", url.strip())
    return match.group(1).lower() if match else None


def git(cwd: Path, *args: str) -> str | None:
    result = subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else None


def git_remote(cwd: Path, name: str) -> str | None:
    url = git(cwd, "remote", "get-url", name)
    return parse_github_remote(url) if url else None


def changed_paths(cwd: Path) -> list[str] | None:
    """Paths that differ from the remote default branch, committed or not"""
    base = git(cwd, "merge-base", "HEAD", "origin/HEAD")
    if base is None:
        return None

    tracked = git(cwd, "diff", "--name-only", base.strip()) or ""
    untracked = git(cwd, "ls-files", "--others", "--exclude-standard") or ""
    return sorted(set(tracked.split()) | set(untracked.split()))


def normalize_step(word: str, raw: str) -> str:
    step = STEP_ALIASES.get(word, word)
    if step not in STEPS:
        raise PlanError(f"unknown word '{raw}'; steps are: {', '.join(STEPS)}")

    return step


def resolve(
    config: dict,
    repo: str,
    words: list[str],
    upstream: str | None = None,
    changed: list[str] | None = None,
) -> Plan:
    """Build the plan for `repo` from its config entry and the typed words

    `upstream` is the upstream remote, used as the PR target only for unlisted
    repos; `changed` lists the paths this work touches, needed only by entries
    whose deploy depends on them
    """
    repo = repo.lower()
    entry = config.get("repos", {}).get(repo)
    owners = {owner.lower() for owner in config.get("owners", [])}

    # a listed repo states its PR target; an upstream remote must not silently redirect it
    pr_repo = ((entry or {}).get("pr_repo") or (upstream if entry is None else None) or repo).lower()
    external = pr_repo.split("/")[0] not in owners

    if entry is not None:
        source, base = "repos.toml", entry
    elif external:
        source, base = "external fallback", config["external"]
    else:
        source, base = "default fallback", config["default"]

    plan = Plan(
        repo=repo,
        source=source,
        external=external,
        pr_repo=pr_repo,
        merge_method="never" if base.get("merge") == "never" else "squash",
        pr_draft=bool(base.get("pr_draft", False)),
        notes=base.get("notes", ""),
    )

    added: set[str] = set()
    removed: set[str] = set()
    only = False
    effort_word = None
    bump_word = None

    for raw in words:
        word = raw.lower()

        if word in STATUS_WORDS:
            plan.mode = "status"
        elif word in ("+confirm", "confirm"):
            # the skill starts on its own unless something is unclear, so this opts into waiting
            plan.confirm = True
        elif word == "only":
            only = True
        elif word in EFFORTS:
            plan.sol_effort = EFFORTS[word]
            effort_word = word
        elif word in BUMPS:
            plan.bump = word
            bump_word = word
        elif word.startswith("-"):
            removed.add(normalize_step(word[1:], raw))
        else:
            added.add(normalize_step(word.removeprefix("+"), raw))

    if only and not added:
        raise PlanError("'only' needs at least one step")

    steps = (added if only else set(base["steps"]) | added) - removed

    # a PR target outside Praveen's orgs always gets a Sol review unless he removes it
    if external and "sol" not in removed:
        steps.add("sol")

    # an effort word is a Sol request, e.g. `/ship high`
    if effort_word:
        if "sol" in removed:
            raise PlanError(f"'{effort_word}' sets the Sol effort, but '-sol' removes the review")

        steps.add("sol")

    # where Greptile reviews, merging waits for its 5/5 unless he removes the gate
    requires = dict(REQUIRES)
    if base.get("greptile", False) and "greptile" not in removed:
        requires["merge"] = "greptile"

    # repeat until stable, since an implied step can need another, e.g. merge → greptile → pr → push
    missing = True
    while missing:
        missing = False

        # with a PR in the plan, the code to release only reaches the default branch by merging
        needs = dict(requires)
        if "pr" in steps:
            needs.update(deploy="merge", publish="merge")

        for step, needed in needs.items():
            if step in steps and needed not in steps:
                if needed in removed:
                    raise PlanError(f"'{step}' needs '{needed}', which was removed")

                steps.add(needed)
                plan.implied.append(needed)
                missing = True

    if "merge" in steps and plan.merge_method == "never":
        raise PlanError(f"{repo} never merges from here; someone else merges {pr_repo} PRs")

    if "greptile" in steps and not base.get("greptile", False):
        raise PlanError(f"Greptile does not review {repo}; set greptile = true in repos.toml once it does")

    for step in ("deploy", "publish"):
        if step not in steps:
            continue

        commands = base.get(step)
        if not commands:
            raise PlanError(f"{repo} has no {step} commands in repos.toml")

        if step == "deploy" and base.get("deploy_paths"):
            if changed is None:
                raise PlanError(f"{repo}'s deploy depends on the changed paths; run from its checkout")

            if not any(path.startswith(tuple(base["deploy_paths"])) for path in changed):
                commands = base["deploy_otherwise"]

        plan.commands[step] = [command.replace("<part>", plan.bump) for command in commands]

    if bump_word and not any("<part>" in command for command in base.get("deploy", []) + base.get("publish", [])):
        raise PlanError(f"'{bump_word}' has no effect: no {repo} command takes a bump level")

    plan.steps = [step for step in STEPS if step in steps]
    return plan


def task_dir(cwd: Path, task: str) -> Path:
    """The run's scratch directory under the checkout root, named by a path-safe slug"""
    root = git(cwd, "rev-parse", "--show-toplevel")
    if root is None:
        raise PlanError("not inside a git checkout")

    # branch names such as feature/x would otherwise nest directories
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", task).strip("-.")
    if not slug:
        raise PlanError(f"task '{task}' has no usable characters")

    return Path(root.strip()) / "_scratch" / "ship" / slug


def save_plan(directory: Path, plan: Plan) -> Path:
    """Write the plan with an empty `done` list; a new invocation starts a new plan"""
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "plan.json"
    path.write_text(json.dumps({**asdict(plan), "done": []}, indent=2) + "\n")
    return path


def mark_done(directory: Path, steps: list[str]) -> list[str]:
    """Add finished steps to a saved plan and return its `done` list in plan order"""
    path = directory / "plan.json"
    if not path.exists():
        raise PlanError(f"no saved plan at {path}; run with --task first")

    saved = json.loads(path.read_text())
    unknown = [step for step in steps if step not in saved["steps"]]
    if unknown:
        raise PlanError(f"not in the plan: {', '.join(unknown)}")

    done = set(saved.get("done", [])) | set(steps)
    saved["done"] = [step for step in saved["steps"] if step in done]
    path.write_text(json.dumps(saved, indent=2) + "\n")
    return saved["done"]


def check_section(name: str, section: object) -> list[str]:
    if not isinstance(section, dict):
        return [f"{name}: must be a table"]

    problems = []

    for key, value in section.items():
        if key in LIST_FIELDS:
            if not (isinstance(value, list) and all(isinstance(item, str) for item in value)):
                problems.append(f"{name}: {key} must be a list of strings")
        elif key in BOOL_FIELDS:
            if not isinstance(value, bool):
                problems.append(f"{name}: {key} must be true or false")
        elif key in STR_FIELDS:
            if not isinstance(value, str):
                problems.append(f"{name}: {key} must be a string")
        else:
            problems.append(f"{name}: unknown field {key}")

    steps = section.get("steps")
    if not isinstance(steps, list):
        return problems + [f"{name}: needs a steps list"]

    unknown_steps = set(steps) - set(STEPS)
    if unknown_steps:
        problems.append(f"{name}: unknown steps {sorted(unknown_steps)}")

    # publishing can't be undone, so it has to be asked for each time
    if "publish" in steps:
        problems.append(f"{name}: publish may not be a default step")

    if section.get("merge", "squash") not in ("squash", "never"):
        problems.append(f"{name}: merge must be squash or never")

    if bool(section.get("deploy_paths")) != bool(section.get("deploy_otherwise")):
        problems.append(f"{name}: deploy_paths and deploy_otherwise go together")

    return problems


def check(config: dict) -> list[str]:
    """Return every problem in the config; empty means valid"""
    problems = []

    if not config.get("owners"):
        problems.append("owners must list at least one GitHub org or user")

    for name in ("default", "external"):
        problems += check_section(f"[{name}]", config.get(name))

    for repo, entry in config.get("repos", {}).items():
        # remotes are lowercased before lookup, so a mixed-case key would never match
        if not re.fullmatch(r"[a-z0-9_.-]+/[a-z0-9_.-]+", repo):
            problems.append(f"{repo}: key must be a lowercase owner/name")

        section_problems = check_section(repo, entry)
        problems += section_problems

        if not section_problems:
            try:
                resolve(config, repo, [], changed=[])
            except PlanError as error:
                problems.append(f"{repo}: {error}")

    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("words", nargs="*")
    parser.add_argument("--repo", help="owner/name; defaults to the origin remote of the current directory")
    parser.add_argument("--config", type=Path, default=CONFIG_PATH)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--task", help="save the plan to _scratch/ship/TASK/plan.json in the checkout")
    parser.add_argument("--done", nargs="+", metavar="STEP", help="record finished steps in the saved plan; needs --task")
    args = parser.parse_args()

    config = load_config(args.config)

    if args.check:
        problems = check(config)
        for problem in problems:
            print(problem, file=sys.stderr)

        print(f"{len(config.get('repos', {}))} repos, {len(problems)} problems")
        return 1 if problems else 0

    cwd = Path.cwd()

    if args.done:
        if not args.task:
            print("ship: --done needs --task", file=sys.stderr)
            return 2

        try:
            done = mark_done(task_dir(cwd, args.task), args.done)
        except PlanError as error:
            print(f"ship: {error}", file=sys.stderr)
            return 2

        print(f"done: {', '.join(done)}")
        return 0

    origin = git_remote(cwd, "origin")
    repo = args.repo.lower() if args.repo else origin
    if repo is None:
        print("ship: no GitHub origin remote here; pass --repo OWNER/NAME", file=sys.stderr)
        return 2

    # remotes and changed paths describe the current checkout, so use them only when it is the repo
    in_checkout = repo == origin
    upstream = git_remote(cwd, "upstream") if in_checkout else None
    changed = changed_paths(cwd) if in_checkout else None

    # skills pass their arguments as one string, so split it here
    words = [word for arg in args.words for word in arg.split()]

    try:
        plan = resolve(config, repo, words, upstream, changed)
    except PlanError as error:
        print(f"ship: {error}", file=sys.stderr)
        return 2

    # status mode changes nothing, so it leaves any saved plan alone
    saved = None
    if args.task and plan.mode != "status":
        try:
            saved = save_plan(task_dir(cwd, args.task), plan)
        except PlanError as error:
            print(f"ship: {error}", file=sys.stderr)
            return 2

    if args.json:
        print(json.dumps(asdict(plan), indent=2))
        return 0

    tag = ", external" if plan.external and plan.source == "repos.toml" else ""
    print(f"ship: {plan.repo} ({plan.source}{tag})")

    if plan.mode == "status":
        print("mode: status only, change nothing")
    elif plan.confirm:
        print("confirm: wait for go-ahead")

    print(f"plan: {plan.summary()}")

    if plan.implied:
        print(f"implied: {', '.join(plan.implied)}")

    templates = config.get("repos", {}).get(repo, {})
    if any("<part>" in command for step in plan.commands for command in templates.get(step, [])):
        print(f"bump: {plan.bump}")

    for step, commands in plan.commands.items():
        print(f"{step}:")
        for command in commands:
            print(f"  {command}")

    if plan.notes:
        print(f"notes: {plan.notes}")

    if saved:
        print(f"saved: {saved}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
