"""Regression tests for the Grok review provider boundary."""

import importlib.util
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


SCRIPT_PATH = Path(__file__).parents[1] / "scripts" / "run_grok_review.py"
FIXTURE_PATH = Path(__file__).parent / "fixtures" / "grok-clean-after-draft.jsonl"
SPEC = importlib.util.spec_from_file_location("run_grok_review", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {SCRIPT_PATH}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class ParseReviewEventsTests(unittest.TestCase):
    """Protect the distinction between draft thoughts and the final review."""

    def test_uses_final_message_stream_after_draft_thoughts(self):
        with FIXTURE_PATH.open(encoding="utf-8") as fixture:
            review = MODULE.parse_review_events(fixture)

        self.assertEqual(review.message, "No P1/P2 findings")
        self.assertEqual(review.stop_reason, "end_turn")
        self.assertEqual(review.stream_start_ms, 200)

    def test_rejects_non_terminal_provider_stop(self):
        events = _minimal_stream(stop_reason="max_turns")

        with self.assertRaisesRegex(
            MODULE.GrokReviewError, "did not complete successfully: max_turns"
        ):
            MODULE.parse_review_events(events)

    def test_rejects_completed_turn_without_final_message(self):
        events = _minimal_stream(include_message=False)

        with self.assertRaisesRegex(MODULE.GrokReviewError, "without model output"):
            MODULE.parse_review_events(events)

    def test_does_not_use_progress_message_when_final_stream_has_only_thoughts(self):
        events = _minimal_stream(include_final_thought=True)

        with self.assertRaisesRegex(
            MODULE.GrokReviewError, "without a final assistant message"
        ):
            MODULE.parse_review_events(events)

    def test_rejects_malformed_json(self):
        with self.assertRaisesRegex(MODULE.GrokReviewError, "invalid JSON on line 1"):
            MODULE.parse_review_events(io.StringIO("{not-json}\n"))


class RunAdapterTests(unittest.TestCase):
    """Protect raw evidence and stale-result handling across provider runs."""

    def test_writes_result_only_after_stream_validation(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            paths = _adapter_paths(Path(temporary_directory))
            arguments = _adapter_arguments(paths)
            captured_command = []

            def complete_provider(command, **kwargs):
                captured_command.extend(command)
                kwargs["stdout"].write(FIXTURE_PATH.read_text(encoding="utf-8"))
                return MODULE.subprocess.CompletedProcess(command, 0)

            with patch.object(MODULE.subprocess, "run", side_effect=complete_provider):
                return_code = MODULE.run(arguments)

            result = json.loads(paths["result"].read_text(encoding="utf-8"))
            self.assertEqual(return_code, 0)
            self.assertEqual(result["message"], "No P1/P2 findings")
            self.assertIn("streaming-json", captured_command)
            self.assertEqual(
                paths["raw"].read_text(encoding="utf-8"),
                FIXTURE_PATH.read_text(encoding="utf-8"),
            )

    def test_provider_failure_removes_stale_result(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            paths = _adapter_paths(Path(temporary_directory))
            paths["result"].write_text("stale approval", encoding="utf-8")
            arguments = _adapter_arguments(paths)

            def fail_provider(command, **_kwargs):
                return MODULE.subprocess.CompletedProcess(command, 7)

            with patch.object(MODULE.subprocess, "run", side_effect=fail_provider):
                with redirect_stderr(io.StringIO()):
                    return_code = MODULE.run(arguments)

            self.assertEqual(return_code, 7)
            self.assertFalse(paths["result"].exists())


def _minimal_stream(
    *,
    stop_reason: str = "end_turn",
    include_message: bool = True,
    include_final_thought: bool = False,
) -> io.StringIO:
    events = []
    if include_message:
        events.append(
            {
                "method": "session/update",
                "params": {
                    "sessionId": "session",
                    "update": {
                        "sessionUpdate": "agent_message_chunk",
                        "content": {"type": "text", "text": "No P1/P2 findings"},
                    },
                    "_meta": {"promptId": "prompt", "streamStartMs": 10},
                },
            }
        )

    if include_final_thought:
        events.append(
            {
                "method": "session/update",
                "params": {
                    "sessionId": "session",
                    "update": {
                        "sessionUpdate": "agent_thought_chunk",
                        "content": {"type": "text", "text": "unfinished draft"},
                    },
                    "_meta": {"promptId": "prompt", "streamStartMs": 20},
                },
            }
        )

    events.append(
        {
            "method": "session/update",
            "params": {
                "sessionId": "session",
                "update": {
                    "sessionUpdate": "turn_completed",
                    "prompt_id": "prompt",
                    "stop_reason": stop_reason,
                },
            },
        }
    )
    return io.StringIO("".join(f"{json.dumps(event)}\n" for event in events))


def _adapter_paths(root: Path) -> dict[str, Path]:
    prompt = root / "prompt.md"
    prompt.write_text("Review the supplied snapshot", encoding="utf-8")
    return {
        "repo": root,
        "prompt": prompt,
        "raw": root / "raw.jsonl",
        "result": root / "final.json",
        "stderr": root / "stderr.txt",
    }


def _adapter_arguments(paths: dict[str, Path]) -> SimpleNamespace:
    return SimpleNamespace(
        repo=str(paths["repo"]),
        prompt_file=str(paths["prompt"]),
        raw_file=str(paths["raw"]),
        result_file=str(paths["result"]),
        stderr_file=str(paths["stderr"]),
        grok_bin="grok",
        model="grok-4.6",
        reasoning_effort="high",
    )


if __name__ == "__main__":
    unittest.main()
