#!/usr/bin/env python3
"""Run a Grok review and extract only its final completed message."""

import argparse
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Mapping


class GrokReviewError(Exception):
    """Report invalid provider output that cannot establish a review result."""


@dataclass(frozen=True)
class ParsedEvent:
    """Store one JSON event with its source line for validation errors."""

    line_number: int
    payload: Mapping[str, object]


@dataclass(frozen=True)
class OutputChunk:
    """Identify one thought or message chunk in a model output stream."""

    line_number: int
    prompt_id: str
    stream_start_ms: int


@dataclass(frozen=True)
class MessageChunk:
    """Store one validated assistant message chunk from the provider boundary."""

    output: OutputChunk
    text: str


@dataclass(frozen=True)
class TurnCompletion:
    """Store the terminal state for one validated provider turn."""

    line_number: int
    prompt_id: str
    stop_reason: str


@dataclass(frozen=True)
class CompletedReview:
    """Represent the final assistant message from one successful Grok turn."""

    session_id: str
    prompt_id: str | None
    stream_start_ms: int | None
    stop_reason: str
    message: str
    request_id: str | None = None


def parse_review_events(lines: Iterable[str]) -> CompletedReview:
    """Parse Grok ACP or native streaming updates into a final message."""

    events = [
        ParsedEvent(line_number, _parse_json_object(raw_line, line_number))
        for line_number, raw_line in enumerate(lines, start=1)
        if raw_line.strip()
    ]

    has_acp_updates = any(_is_acp_session_update(event.payload) for event in events)
    if has_acp_updates:
        return _parse_acp_events(events)
    if any(event.payload.get("type") == "end" for event in events):
        return _parse_native_events(events)

    raise GrokReviewError("Grok output contains no session updates or native terminal event")


def _parse_acp_events(events: list[ParsedEvent]) -> CompletedReview:
    """Parse the ACP session/update representation."""

    session_id: str | None = None
    output_chunks: list[OutputChunk] = []
    message_chunks: list[MessageChunk] = []
    completions: list[TurnCompletion] = []

    for event in events:
        line_number = event.line_number
        payload = event.payload
        method = payload.get("method")
        if not isinstance(method, str) or not method.endswith("session/update"):
            continue

        params = _require_mapping(payload, "params", line_number)
        event_session_id = _require_string(params, "sessionId", line_number)
        session_id = _merge_session_id(session_id, event_session_id, line_number)
        update = _require_mapping(params, "update", line_number)
        update_type = update.get("sessionUpdate")

        if update_type in {"agent_message_chunk", "agent_thought_chunk"}:
            output_chunk = _parse_output_chunk(params, line_number)
            output_chunks.append(output_chunk)

            if update_type == "agent_message_chunk":
                message_chunks.append(
                    _parse_message_chunk(output_chunk, update, line_number)
                )
        elif update_type == "turn_completed":
            completions.append(_parse_turn_completion(update, line_number))

    if session_id is None:
        raise GrokReviewError("Grok output contains no session updates")
    if len(completions) != 1:
        raise GrokReviewError(
            f"Grok output must contain one completed turn, found {len(completions)}"
        )

    completion = completions[0]
    if completion.stop_reason != "end_turn":
        raise GrokReviewError(
            f"Grok turn did not complete successfully: {completion.stop_reason}"
        )

    eligible_output_chunks = [
        chunk
        for chunk in output_chunks
        if chunk.line_number < completion.line_number
        and chunk.prompt_id == completion.prompt_id
    ]
    if not eligible_output_chunks:
        raise GrokReviewError("Grok turn completed without model output")
    if any(chunk.line_number > completion.line_number for chunk in output_chunks):
        raise GrokReviewError("Grok output contains model output after turn completion")

    final_stream_start_ms = eligible_output_chunks[-1].stream_start_ms
    final_stream = [
        chunk
        for chunk in message_chunks
        if chunk.output.line_number < completion.line_number
        and chunk.output.prompt_id == completion.prompt_id
        and chunk.output.stream_start_ms == final_stream_start_ms
    ]
    if not final_stream:
        raise GrokReviewError("Grok turn completed without a final assistant message")

    message = "".join(chunk.text for chunk in final_stream).strip()
    if not message:
        raise GrokReviewError("Grok final assistant message is empty")

    return CompletedReview(
        session_id=session_id,
        prompt_id=completion.prompt_id,
        stream_start_ms=final_stream_start_ms,
        stop_reason=completion.stop_reason,
        message=message,
    )


def _parse_native_events(events: list[ParsedEvent]) -> CompletedReview:
    """Parse Grok CLI 1.0.5 native streaming-json events."""

    terminal_events = [
        event for event in events if event.payload.get("type") == "end"
    ]
    if len(terminal_events) != 1:
        raise GrokReviewError(
            "Grok native output must contain exactly one terminal end event"
        )

    terminal = terminal_events[0]
    terminal_index = events.index(terminal)
    if terminal_index != len(events) - 1:
        raise GrokReviewError(
            "Grok native output contains an event after the terminal end event"
        )

    session_id = _require_string(terminal.payload, "sessionId", terminal.line_number)
    request_id = _require_string(terminal.payload, "requestId", terminal.line_number)
    stop_reason = _require_string(
        terminal.payload, "stopReason", terminal.line_number
    )
    if stop_reason != "end_turn":
        raise GrokReviewError(
            f"Grok native turn did not complete successfully: {stop_reason}"
        )

    text_data: dict[int, str] = {}
    for event in events[:terminal_index]:
        event_type = _parse_native_event_type(event)
        if event_type in {"text", "thought"}:
            text_data[event.line_number] = _require_string(
                event.payload, "data", event.line_number
            )
        elif event_type not in {
            "available_commands",
            "tool_call",
            "tool_call_update",
            "usage",
        }:
            raise GrokReviewError(
                f"Grok native event on line {event.line_number} has unsupported type"
            )

    final_text_index = terminal_index - 1
    while (
        final_text_index >= 0
        and events[final_text_index].payload.get("type")
        in {"available_commands", "usage"}
    ):
        final_text_index -= 1

    if (
        final_text_index < 0
        or events[final_text_index].payload.get("type") != "text"
    ):
        raise GrokReviewError(
            "Grok native turn completed without a final assistant message"
        )

    first_text_index = final_text_index
    while (
        first_text_index >= 0
        and events[first_text_index].payload.get("type") == "text"
    ):
        first_text_index -= 1

    message = "".join(
        text_data[events[index].line_number]
        for index in range(first_text_index + 1, final_text_index + 1)
    ).strip()
    if not message:
        raise GrokReviewError("Grok native final assistant message is empty")

    return CompletedReview(
        session_id=session_id,
        prompt_id=None,
        stream_start_ms=None,
        stop_reason=stop_reason,
        message=message,
        request_id=request_id,
    )


def parse_review_file(path: Path) -> CompletedReview:
    """Parse a saved Grok streaming JSONL artifact."""

    with path.open(encoding="utf-8") as raw_stream:
        return parse_review_events(raw_stream)


def build_command(args: argparse.Namespace, repo: Path, prompt_file: Path) -> list[str]:
    """Build the fixed read-only Grok review command."""

    return [
        args.grok_bin,
        "--prompt-file",
        str(prompt_file),
        "--cwd",
        str(repo),
        "--model",
        args.model,
        "--reasoning-effort",
        args.reasoning_effort,
        "--always-approve",
        "--disallowed-tools",
        "search_replace,write,run_terminal_cmd,run_terminal_command",
        "--disable-web-search",
        "--no-subagents",
        "--no-plan",
        "--verbatim",
        "--output-format",
        "streaming-json",
    ]


def write_result(path: Path, review: CompletedReview) -> None:
    """Atomically write the validated provider result as JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None

    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            delete=False,
        ) as temporary_file:
            temporary_path = Path(temporary_file.name)
            json.dump(
                {
                    key: value
                    for key, value in asdict(review).items()
                    if value is not None
                },
                temporary_file,
                indent=2,
            )
            temporary_file.write("\n")
            temporary_file.flush()
            os.fsync(temporary_file.fileno())

        os.replace(temporary_path, path)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments for one fresh Grok review."""

    parser = argparse.ArgumentParser(
        description=(
            "Run Grok with streaming events and extract its final completed message"
        ),
    )
    parser.add_argument("--repo", required=True, help="Repository root for the review")
    parser.add_argument(
        "--prompt-file", required=True, help="Self-contained review prompt"
    )
    parser.add_argument(
        "--raw-file", required=True, help="Raw Grok streaming JSONL output"
    )
    parser.add_argument(
        "--result-file", required=True, help="Validated final result JSON"
    )
    parser.add_argument("--stderr-file", required=True, help="Raw Grok stderr output")
    parser.add_argument("--grok-bin", default="grok", help="Grok executable")
    parser.add_argument("--model", default="grok-4.6", help="Exact Grok model")
    parser.add_argument(
        "--reasoning-effort",
        default="high",
        help="Grok reasoning effort",
    )
    return parser.parse_args()


def run(args: argparse.Namespace) -> int:
    """Run one provider process and write a result only after strict validation."""

    repo = _resolve_existing_directory(args.repo, "--repo")
    prompt_file = _resolve_existing_file(args.prompt_file, "--prompt-file")
    raw_file = _resolve_output_file(args.raw_file)
    result_file = _resolve_output_file(args.result_file)
    stderr_file = _resolve_output_file(args.stderr_file)
    _require_distinct_artifacts(prompt_file, raw_file, result_file, stderr_file)

    raw_file.parent.mkdir(parents=True, exist_ok=True)
    result_file.parent.mkdir(parents=True, exist_ok=True)
    stderr_file.parent.mkdir(parents=True, exist_ok=True)
    result_file.unlink(missing_ok=True)

    command = build_command(args, repo, prompt_file)
    try:
        with raw_file.open("w", encoding="utf-8") as raw_stream, stderr_file.open(
            "w", encoding="utf-8"
        ) as stderr_stream:
            provider = subprocess.run(
                command,
                cwd=repo,
                stdout=raw_stream,
                stderr=stderr_stream,
                text=True,
                check=False,
            )
    except OSError as error:
        print(f"Cannot run Grok review: {error}", file=sys.stderr)
        return 1

    if provider.returncode != 0:
        error_message = (
            f"Grok review failed with exit code {provider.returncode}; "
            f"see {stderr_file}"
        )
        print(
            error_message,
            file=sys.stderr,
        )
        return provider.returncode if 0 < provider.returncode < 256 else 1

    try:
        review = parse_review_file(raw_file)
    except GrokReviewError as error:
        print(f"Grok review output is invalid: {error}", file=sys.stderr)
        return 1

    try:
        write_result(result_file, review)
    except OSError as error:
        print(f"Cannot write Grok review result: {error}", file=sys.stderr)
        return 1

    return 0


def main() -> int:
    """Run the Grok review adapter command."""

    return run(parse_args())


def _is_acp_session_update(event: Mapping[str, object]) -> bool:
    method = event.get("method")
    return isinstance(method, str) and method.endswith("session/update")


def _parse_native_event_type(event: ParsedEvent) -> str:
    event_type = event.payload.get("type")
    if not isinstance(event_type, str) or not event_type:
        raise GrokReviewError(
            f"Grok native event on line {event.line_number} has invalid type"
        )
    return event_type


def _parse_json_object(raw_line: str, line_number: int) -> Mapping[str, object]:
    try:
        event = json.loads(raw_line)
    except json.JSONDecodeError as error:
        message = f"invalid JSON on line {line_number}: {error.msg}"
        raise GrokReviewError(message) from error

    if not isinstance(event, dict):
        raise GrokReviewError(f"Grok event on line {line_number} is not an object")
    return event


def _require_mapping(
    value: Mapping[str, object], key: str, line_number: int
) -> Mapping[str, object]:
    nested = value.get(key)
    if not isinstance(nested, dict):
        raise GrokReviewError(f"Grok event on line {line_number} has invalid {key}")
    return nested


def _require_string(value: Mapping[str, object], key: str, line_number: int) -> str:
    text = value.get(key)
    if not isinstance(text, str) or not text:
        raise GrokReviewError(f"Grok event on line {line_number} has invalid {key}")
    return text


def _require_integer(value: Mapping[str, object], key: str, line_number: int) -> int:
    number = value.get(key)
    if not isinstance(number, int) or isinstance(number, bool):
        raise GrokReviewError(f"Grok event on line {line_number} has invalid {key}")
    return number


def _merge_session_id(
    current_session_id: str | None, event_session_id: str, line_number: int
) -> str:
    if current_session_id is not None and current_session_id != event_session_id:
        raise GrokReviewError(f"Grok output changes session on line {line_number}")
    return event_session_id


def _parse_output_chunk(
    params: Mapping[str, object], line_number: int
) -> OutputChunk:
    metadata = _require_mapping(params, "_meta", line_number)
    return OutputChunk(
        line_number=line_number,
        prompt_id=_require_string(metadata, "promptId", line_number),
        stream_start_ms=_require_integer(metadata, "streamStartMs", line_number),
    )


def _parse_message_chunk(
    output: OutputChunk, update: Mapping[str, object], line_number: int
) -> MessageChunk:
    content = _require_mapping(update, "content", line_number)
    if _require_string(content, "type", line_number) != "text":
        raise GrokReviewError(f"Grok message on line {line_number} is not text")

    return MessageChunk(
        output=output,
        text=_require_string(content, "text", line_number),
    )


def _parse_turn_completion(
    update: Mapping[str, object], line_number: int
) -> TurnCompletion:
    return TurnCompletion(
        line_number=line_number,
        prompt_id=_require_string(update, "prompt_id", line_number),
        stop_reason=_require_string(update, "stop_reason", line_number),
    )


def _resolve_existing_directory(path: str, label: str) -> Path:
    resolved = Path(path).expanduser().resolve()
    if not resolved.is_dir():
        raise SystemExit(f"{label} is not a directory: {resolved}")
    return resolved


def _resolve_existing_file(path: str, label: str) -> Path:
    resolved = Path(path).expanduser().resolve()
    if not resolved.is_file():
        raise SystemExit(f"{label} is not a file: {resolved}")
    return resolved


def _resolve_output_file(path: str) -> Path:
    return Path(path).expanduser().resolve()


def _require_distinct_artifacts(*paths: Path) -> None:
    if len(set(paths)) != len(paths):
        raise SystemExit("prompt, raw, result, and stderr files must be distinct")


if __name__ == "__main__":
    sys.exit(main())
