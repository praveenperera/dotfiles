# Agent and command sessions

## Place the session with the work

Start a named tmux session on the execution machine. A tmux session on the Mac mini that only holds an SSH connection does not replace a remote tmux session. For a local agent, use local tmux. For coding on ai5090, connect directly to `praveen@code.local`.

If already on `ai5090.local`, enter either container as `praveen` without SSH:

```bash
incus exec code -- su - praveen
incus exec training -- su - praveen
```

Run only the command for the required container. Then start or attach to a named tmux session inside that container. The login shell uses `praveen`'s home and environment; do not run coding agents or training as root.

Use a task-specific name such as `fleet-myproject`. Inspect an existing session before reuse. Do not send commands to a pane that is running another agent or job. Keep the session after the job ends so Praveen can inspect it.

SSH calls that create or inspect tmux are the session control steps. Run the actual shell commands, checks, builds, and agents inside its panes. Use an interactive shell pane so password prompts and agent prompts remain accessible.

## Start and inspect

Examples below use `code.local`; change the destination for training or host administration. Choose an unused session name before creation:

```bash
ssh praveen@code.local 'tmux list-sessions'
ssh praveen@code.local 'tmux new-session -d -s fleet-myproject -P -F "#{pane_id}"'
```

Record the returned pane ID. Use it for subsequent commands; do not assume that window or pane numbering starts at zero. If there is no tmux server yet, `list-sessions` can fail normally. Distinguish this from an SSH failure.

For example, if the returned ID is `%3`, send a command as literal text, then send Enter separately:

```bash
ssh praveen@code.local 'tmux send-keys -t %3 -l -- "hostname; pwd; id -un"'
ssh praveen@code.local 'tmux send-keys -t %3 Enter'
ssh praveen@code.local 'tmux capture-pane -p -S -100 -t %3'
```

Use the actual returned ID, not `%3` without checking it. Keep shell quoting intact across the local shell, SSH, and the remote shell. For a long task, transfer a prompt or script file instead of building a deeply quoted command. Put task notes and captured evidence in the relevant repository's `_scratch/` directory.

On the Mac mini, use the same tmux commands without SSH. Create a new pane or session for a new local agent; do not nest it inside an active agent process.

## Prepare the checkout and agent

Inside the target pane:

1. Confirm the machine, user, and repository path. Read the checkout's `AGENTS.md` and inspect its Git state.
2. Reuse a suitable checkout or create an isolated checkout/worktree for the task. Do not assume a Mac path exists on Linux. Keep one writer per working tree.
3. If local changes are needed remotely, transfer the intended patch and required untracked files. Record the base revision. Do not commit, push, overwrite a dirty checkout, or copy secrets merely to transfer work.
4. Check that the chosen agent CLI is installed and authenticated. The configured `code` workspace includes Codex, Claude Code, Grok Build, T3 Code nightly, Rust, Node.js, Python, and build tools; verify availability before use.
5. Give the agent a self-contained task with the repository, scope, constraints, and required checks. Start the chosen CLI in the foreground inside the pane. Preserve its normal permission controls.

The parent agent remains responsible for progress, result review, and integration. A successful `send-keys` call means only that input was sent. Inspect output, check exit status or explicit completion evidence, and review the resulting Git diff and checks. For long tasks, retain logs and record the command's exit status in the task's `_scratch/` directory. Do not treat silence or a disconnected SSH client as success.

If authentication or a password needs Praveen, leave the session running and provide the attach command. Do not enter credentials into task files or captured logs.

## Attach and hand back results

Give Praveen the execution machine, session name, repository path, task status, and exact attach command:

```bash
ssh -t praveen@code.local 'tmux attach-session -t fleet-myproject'
```

For a local agent:

```bash
tmux attach-session -t fleet-myproject
```

Detaching keeps the work running. Do not kill the session as automatic cleanup. Report where changes and logs remain. When the task requires local integration, transfer and review the intended diff without replacing unrelated local changes. Do not create a second copy of an active job after a connection failure; inspect its session first.
