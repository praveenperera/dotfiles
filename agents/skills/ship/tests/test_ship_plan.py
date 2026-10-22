"""Behavior tests for the /ship plan resolver"""

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).parents[1] / "scripts" / "ship_plan.py"
SPEC = importlib.util.spec_from_file_location("ship_plan", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {SCRIPT_PATH}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

CONFIG = tomllib.loads(
    """
owners = ["me"]

[default]
steps = ["commit", "push", "clean"]

[external]
steps = ["sol", "commit", "push", "pr", "clean"]

[repos."me/app"]
steps = ["commit", "push", "deploy", "clean"]
deploy = ["just bump <part>", "just release"]

[repos."me/gated"]
steps = ["commit", "push", "pr", "greptile", "merge", "clean"]
greptile = true

[repos."me/tool"]
steps = ["commit", "push", "deploy", "clean"]
deploy = ["release tool"]
deploy_paths = ["tool/"]
deploy_otherwise = ["pull config"]

[repos."me/fork"]
steps = ["commit", "push", "pr", "clean"]
pr_repo = "them/upstream"
merge = "never"
"""
)


class ResolveTests(unittest.TestCase):
    def test_typed_order_and_dictation_aliases_resolve_to_canonical_order(self):
        plan = MODULE.resolve(CONFIG, "me/app", ["settle", "soul", "release", "xi"])

        self.assertEqual(plan.steps, ["sol", "commit", "push", "deploy", "clean", "settle"])
        self.assertEqual(plan.sol_effort, "xhigh")

    def test_short_settle_aliases(self):
        for word in ("+s", "+set", "s", "set"):
            self.assertEqual(MODULE.resolve(CONFIG, "me/app", [word]).steps[-1], "settle")

    def test_merge_on_direct_repo_implies_pr(self):
        plan = MODULE.resolve(CONFIG, "me/app", ["merge"])

        self.assertEqual(plan.steps[:4], ["commit", "push", "pr", "merge"])
        self.assertEqual(plan.implied, ["pr"])

    def test_removing_a_prerequisite_of_a_kept_step_is_an_error(self):
        with self.assertRaisesRegex(MODULE.PlanError, "'pr' needs 'push'"):
            MODULE.resolve(CONFIG, "me/gated", ["-push"])

    def test_release_after_a_pr_waits_for_the_merge(self):
        plan = MODULE.resolve(CONFIG, "me/app", ["pr"])

        self.assertEqual(plan.steps, ["commit", "push", "pr", "merge", "deploy", "clean"])

        with self.assertRaisesRegex(MODULE.PlanError, "'deploy' needs 'merge'"):
            MODULE.resolve(CONFIG, "me/app", ["pr", "-merge"])

    def test_confirm_opts_into_the_go_ahead_without_changing_steps(self):
        default = MODULE.resolve(CONFIG, "me/app", [])
        confirmed = MODULE.resolve(CONFIG, "me/app", ["confirm"])

        self.assertFalse(default.confirm)
        self.assertTrue(confirmed.confirm)
        self.assertEqual(confirmed.steps, default.steps)

    def test_only_replaces_the_default_but_keeps_prerequisites(self):
        plan = MODULE.resolve(CONFIG, "me/app", ["only", "merge"])

        self.assertEqual(plan.steps, ["push", "pr", "merge"])

    def test_only_keeps_the_greptile_gate_and_external_sol(self):
        gated = MODULE.resolve(CONFIG, "me/gated", ["only", "merge"])
        ungated = MODULE.resolve(CONFIG, "me/gated", ["merge", "only", "-greptile"])
        fork = MODULE.resolve(CONFIG, "me/fork", ["only", "pr"])

        self.assertEqual(gated.steps, ["push", "pr", "greptile", "merge"])
        self.assertEqual(ungated.steps, ["push", "pr", "merge"])
        self.assertEqual(fork.steps, ["sol", "push", "pr"])

    def test_repo_names_match_regardless_of_case(self):
        plan = MODULE.resolve(CONFIG, "Me/App", [])

        self.assertEqual(plan.source, "repos.toml")
        self.assertFalse(plan.external)

    def test_upstream_remote_does_not_redirect_a_listed_repo(self):
        plan = MODULE.resolve(CONFIG, "me/gated", [], upstream="them/gated")

        self.assertEqual(plan.pr_repo, "me/gated")
        self.assertFalse(plan.external)

    def test_deploy_depends_on_touched_paths_when_configured(self):
        touched = MODULE.resolve(CONFIG, "me/tool", [], changed=["tool/src/main.rs"])
        untouched = MODULE.resolve(CONFIG, "me/tool", [], changed=["docs/readme.md"])

        self.assertEqual(touched.commands["deploy"], ["release tool"])
        self.assertEqual(untouched.commands["deploy"], ["pull config"])

        with self.assertRaisesRegex(MODULE.PlanError, "run from its checkout"):
            MODULE.resolve(CONFIG, "me/tool", [])

    def test_pr_target_outside_owners_always_gets_sol(self):
        listed_fork = MODULE.resolve(CONFIG, "me/fork", [])
        unlisted_fork = MODULE.resolve(CONFIG, "me/unlisted", [], upstream="them/upstream")
        unlisted_owned = MODULE.resolve(CONFIG, "me/unlisted", [])

        self.assertEqual(listed_fork.steps[0], "sol")
        self.assertEqual(listed_fork.pr_repo, "them/upstream")
        self.assertEqual(unlisted_fork.source, "external fallback")
        self.assertEqual(unlisted_fork.steps, ["sol", "commit", "push", "pr", "clean"])
        self.assertEqual(unlisted_owned.steps, ["commit", "push", "clean"])

    def test_steps_the_repo_cannot_run_are_errors(self):
        with self.assertRaisesRegex(MODULE.PlanError, "never merges"):
            MODULE.resolve(CONFIG, "me/fork", ["merge"])

        with self.assertRaisesRegex(MODULE.PlanError, "Greptile does not review"):
            MODULE.resolve(CONFIG, "me/app", ["greptile"])

        with self.assertRaisesRegex(MODULE.PlanError, "no deploy commands"):
            MODULE.resolve(CONFIG, "me/gated", ["deploy"])

        with self.assertRaisesRegex(MODULE.PlanError, "unknown word 'e2e'"):
            MODULE.resolve(CONFIG, "me/app", ["e2e"])

    def test_bump_level_fills_deploy_commands(self):
        plan = MODULE.resolve(CONFIG, "me/app", ["minor"])

        self.assertEqual(plan.commands["deploy"], ["just bump minor", "just release"])

        with self.assertRaisesRegex(MODULE.PlanError, "'major' has no effect"):
            MODULE.resolve(CONFIG, "me/gated", ["major"])


class RepoTableTests(unittest.TestCase):
    def test_repos_toml_is_valid(self):
        self.assertEqual(MODULE.check(MODULE.load_config()), [])


class CliTests(unittest.TestCase):
    def run_cli(self, arguments):
        return subprocess.run(
            [sys.executable, "-I", str(SCRIPT_PATH), "--repo", "praveenperera/homelab", "--", arguments],
            capture_output=True,
            text=True,
            check=True,
        ).stdout

    def test_skill_arguments_arrive_as_one_string(self):
        output = self.run_cli("soul high")

        self.assertIn("plan: sol 6.1 high review → commit → push → deploy → clean\n", output)

    def test_a_lone_removal_word_is_not_parsed_as_an_option(self):
        output = self.run_cli("+settle")

        self.assertIn("plan: commit → push → deploy → clean → settle\n", output)


class SavedPlanTests(unittest.TestCase):
    def setUp(self):
        self.checkout = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.checkout)
        subprocess.run(["git", "init", "-q", str(self.checkout)], check=True)
        subprocess.run(
            ["git", "-C", str(self.checkout), "remote", "add", "origin", "git@github.com:praveenperera/homelab.git"],
            check=True,
        )

    def run_cli(self, *arguments):
        return subprocess.run(
            [sys.executable, "-I", str(SCRIPT_PATH), *arguments],
            cwd=self.checkout,
            capture_output=True,
            text=True,
        )

    def saved(self):
        return json.loads((self.checkout / "_scratch/ship/feature-x/plan.json").read_text())

    def test_one_run_prints_and_saves_the_plan_and_done_steps_keep_plan_order(self):
        output = self.run_cli("--task", "feature/x", "--").stdout

        self.assertIn("plan: commit → push → deploy → clean\n", output)
        self.assertIn("saved: ", output)
        self.assertEqual(self.saved()["steps"], ["commit", "push", "deploy", "clean"])
        self.assertEqual(self.saved()["done"], [])

        self.assertEqual(self.run_cli("--task", "feature/x", "--done", "push", "commit").stdout, "done: commit, push\n")
        self.assertEqual(self.saved()["done"], ["commit", "push"])

    def test_done_rejects_a_step_outside_the_plan(self):
        self.run_cli("--task", "feature/x", "--")
        result = self.run_cli("--task", "feature/x", "--done", "merge")

        self.assertEqual(result.returncode, 2)
        self.assertIn("not in the plan: merge", result.stderr)
        self.assertEqual(self.saved()["done"], [])

    def test_status_mode_leaves_the_saved_plan_alone(self):
        self.run_cli("--task", "feature/x", "--")
        self.run_cli("--task", "feature/x", "--done", "commit")
        self.run_cli("--task", "feature/x", "--", "?")

        self.assertEqual(self.saved()["done"], ["commit"])


if __name__ == "__main__":
    unittest.main()
