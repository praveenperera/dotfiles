# Send a direct Codex-thread message

`homebased message send` sends one message to a Codex thread on this or another
Fleet machine. It does not send a prompt to a detached Homebased worker.

## Choose the destination

Use `--machine` with exactly one destination selector:

```bash
homebased --json message send \
  --machine code \
  --thread <thread-uuid> \
  --message "Please review the cluster design."
```

Or select a thread by its working directory on the receiving machine:

```bash
homebased --json message send \
  --machine code \
  --cwd '~/code/homebased' \
  --message "Please review the cluster design."
```

`--machine` accepts a known machine name or stable machine UUID. `--thread`
must be an exact thread UUID. `--cwd` must be an absolute path or start with
`~/`. The receiving daemon expands `~/` with its own home directory and picks
the most recently active Codex session with that exact working directory. The
receipt includes the resolved thread UUID and working directory. No match
returns `agent_thread_not_found`.

You can also send to the origin thread of a known task. Use `--task` alone,
without `--machine`, `--thread`, or `--cwd`:

```bash
homebased --json message send \
  --task <task-uuid> \
  --message "The build is ready for review."
```

The local daemon finds the task route in the known Fleet and sends to its
origin machine and original thread. If it cannot check a peer that may hold the
route, it returns `cluster_lookup_incomplete` instead of claiming no route
exists.

## Choose the source and conversation

By default, the CLI uses `HOMEBASED_TASK_ID` as the source task. Otherwise, it
uses `CODEX_THREAD_ID`, then `CODEX_SESSION_ID`, as the source thread. Pass one
explicit source when these values are not set or when you need a different
reply route:

```bash
--source-thread <thread-uuid>
--source-task <task-uuid>
```

The flags are mutually exclusive. The queued `HOMEBASED_MESSAGE` JSON has a
`source` route with the source machine and thread or task. The receiver runs
`codex queue` for the destination thread. A recipient can use the source route
to reply.

`--reply-to <message-uuid>` links a reply to an earlier message. Use
`--conversation <uuid>` to set a shared conversation UUID. By default, the
conversation UUID is the message UUID.

## Retry safely

Delivery is synchronous. The destination daemon saves the message identity,
request content, and resolved destination before it runs `codex queue`. After
queue success, it saves a receipt before it returns success. It does not keep
an offline mailbox or resume an unfinished message attempt after restart.
The sender also saves the selected machine UUID before delivery. A retry with
the same message UUID cannot move to another machine if a name changes owners.

The CLI creates a message UUID unless you pass `--message-id <uuid>`. Choose
the UUID before the first send if you may need to retry after a lost response.
Retry with the same message UUID, destination, source, body, and other options.
The receiver returns a saved receipt without another queue call when it has
one. It uses the saved thread when it retries a `--cwd` destination. Reusing a
message UUID with different request content returns `message_conflict`.

```bash
message_id="$(uuidgen)"
homebased --json message send \
  --machine code \
  --thread <thread-uuid> \
  --source-thread <source-thread-uuid> \
  --message "Please review the cluster design." \
  --message-id "$message_id"
```

Queue delivery is at-least-once, not exactly-once. If Codex accepts a message
and the receiver stops before it saves the receipt, an explicit retry can
queue the same message again. Include the message UUID in the conversation
handling and ignore a duplicate. The destination must be reachable for each
attempt. A failed attempt returns a typed error; Homebased does not retry a
message in the background.

`message_outcome_unknown` means the receiver may have queued the message. Retry
with the same message UUID and request. The sender uses the saved machine UUID
and recipient, even if the original machine name now resolves elsewhere.
`message_delivery_failed`
means the receiver did not return a success receipt; retry explicitly with the
same request identity. A saved receipt suppresses a repeat after a lost HTTP
response.
