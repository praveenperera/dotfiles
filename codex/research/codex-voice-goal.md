# Codex Voice Goal

## End Goal

Build a Rust CLI named `codex-voice` that can be launched from tmux while focused on a Codex TUI pane. The CLI reads the latest long Codex plan or explanation aloud through OpenAI Realtime voice, lets Praveen discuss it by voice, and produces one paste-ready prompt to send back to Codex.

Phase 1 is read-only against Codex. It must not mutate the live Codex TUI process. The final prompt is copied to the clipboard and printed for manual paste.

## Why This Exists

Long Codex responses are often useful but slow to read and process in a terminal. The first useful version should optimize for understanding: faithful readout, conversational clarification, and a clean next prompt. Directly writing into the active Codex TUI can wait until the session-sharing problem is solved.

## Phase 1 Scope

`codex-voice` should:

- run as a Rust CLI using `clap`
- work from a tmux key binding
- identify the focused tmux pane
- resolve the related Codex thread
- fetch the latest assistant-readable Codex item
- prefer `Plan` items over regular assistant messages when both are present
- read the selected message aloud using OpenAI Realtime
- allow a spoken follow-up discussion
- produce one final prompt in Praveen's voice
- copy the final prompt to the clipboard
- print the final prompt to stdout

`codex-voice` should not:

- write directly into Codex
- scrape terminal text as the primary source of truth
- require a GUI
- require Codex to be launched in remote app-server mode
- summarize or rewrite the Codex message before readout unless explicitly asked

## CLI Interface

Default behavior:

```sh
codex-voice
```

Equivalent to:

```sh
codex-voice ask
```

Commands:

```sh
codex-voice context
codex-voice read
codex-voice read --json
codex-voice ask
codex-voice prompt
```

Shared options:

```sh
codex-voice --pane "$TMUX_PANE"
codex-voice --thread <thread-id>
codex-voice --debug
```

`context` prints the detected tmux and Codex context. `read` prints the selected Codex message without starting audio. `ask` starts the full voice flow. `prompt` can run a voice session focused only on producing a paste-ready prompt after context has been read.

## Tmux Flow

Expected tmux binding:

```tmux
bind-key v run-shell 'codex-voice --pane "#{pane_id}"'
```

The CLI should collect pane metadata with:

```sh
tmux display-message -p -t "$pane" '#{pane_id}\t#{pane_current_path}\t#{pane_current_command}\t#{pane_title}'
```

Thread resolution priority:

1. explicit `--thread`
2. thread or session id parsed from `pane_title`
3. latest interactive Codex thread matching `pane_current_path`
4. clear error with candidates if multiple plausible threads exist

The terminal-title path should become the preferred reliable route. Cwd matching is a fallback and may be ambiguous when multiple Codex sessions are open in the same repo.

## Codex Read Path

Use Codex app-server or persisted thread state as structured data. Do not scrape terminal output for the main implementation.

The selected readable item should include:

```json
{
  "thread_id": "...",
  "turn_id": "...",
  "kind": "plan",
  "text": "...",
  "source": "terminal_title | cwd_latest | explicit_thread"
}
```

Selection rules:

1. walk turns newest to oldest
2. within each turn, inspect completed items newest to oldest
3. choose the latest `Plan` item first
4. otherwise choose the latest `AgentMessage`
5. ignore command output, file changes, and tool calls for Phase 1
6. preserve original text for readout

## Realtime Voice Behavior

Use OpenAI Realtime directly from the CLI for Phase 1. Prefer WebSocket first because it is simpler for a Rust CLI than WebRTC.

The voice session has three phases:

1. readout
2. discussion
3. final prompt

System prompt:

```text
You are a voice companion for a Codex CLI session.

First, read the provided Codex message aloud in a clear, faithful way. Preserve meaning, ordering, and technical detail. You may lightly adapt bullets or code references for speech, but do not add new technical claims.

After reading, answer the user's spoken questions about the message. Help them decide what they want Codex to do next.

When the user is ready, produce one concise prompt they can paste into Codex. Write it directly to Codex in the user's voice. Include all important decisions from the conversation. Do not include meta commentary.
```

The final prompt should be copied to the clipboard and printed.

## Rust Project Shape

Use Rust 2018+ module layout. Do not use `mod.rs`.

```text
src/
  main.rs
  cli.rs
  config.rs
  tmux.rs
  clipboard.rs
  history.rs

  codex.rs
  codex/
    app_server.rs
    latest_message.rs
    thread_resolver.rs
    types.rs

  realtime.rs
  realtime/
    audio.rs
    events.rs
    prompt.rs
    session.rs
    websocket.rs
```

Responsibilities:

- `cli.rs`: `clap` parser and command types
- `tmux.rs`: pane metadata discovery
- `codex/app_server.rs`: Codex app-server JSON-RPC client
- `codex/thread_resolver.rs`: thread selection from explicit id, title, or cwd
- `codex/latest_message.rs`: latest readable item selection
- `codex/types.rs`: newtypes and shared DTOs
- `realtime/websocket.rs`: Realtime WebSocket transport
- `realtime/events.rs`: Realtime protocol event structs
- `realtime/audio.rs`: microphone and speaker handling
- `realtime/prompt.rs`: session instructions and prompt construction
- `realtime/session.rs`: readout, discussion, and final prompt orchestration
- `clipboard.rs`: clipboard write
- `history.rs`: optional local prompt/readout history

## Rust Style Rules

- use `color-eyre` for CLI errors
- use `tracing` for diagnostics
- use `println!` only for user-facing CLI output
- avoid `unwrap()` in production code
- add `.wrap_err()` at process and network boundaries
- use named imports instead of wildcard imports
- prefer structs with methods for stateful components
- use newtypes for domain identifiers like `ThreadId`, `PaneId`, and `TurnId`
- use enums instead of boolean mode flags
- collapse nested `if let` statements with if-let chains where useful
- keep inline comments lowercase and only explain why
- capitalize higher-level doc comments

## Suggested Dependencies

Use current crate versions when implementing rather than copying stale versions from this document. Expected dependency set:

- `clap`
- `tokio`
- `serde`
- `serde_json`
- `color-eyre`
- `tracing`
- `tracing-subscriber`
- `tokio-tungstenite`
- `futures-util`
- `cpal`
- `rodio`
- `arboard`
- `directories`
- `uuid`
- `time`

## Acceptance Criteria

Phase 1 is complete when:

- `codex-voice context` prints useful tmux and Codex thread context
- `codex-voice read` prints the latest plan or assistant message for the focused Codex pane
- `codex-voice ask` reads that message aloud
- the user can ask spoken questions after readout
- the tool can produce a single paste-ready Codex prompt
- the prompt is copied to the clipboard
- the prompt is printed to stdout
- failures are actionable and do not panic
- implementation passes `just fmt` and `just clippy`, or `cargo fmt` and `cargo clippy` if no justfile exists
