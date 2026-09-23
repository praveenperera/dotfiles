# Persistent sessions

## Choose whether to use tmux

Use a direct local shell or SSH command for short, non-interactive work that is safe to rerun, such as checking machine identity, resource availability, service state, configuration, or Git status. Do not create a tmux session only to run routine commands.

Use a named tmux session when work must survive a disconnect, can outlive the current interaction, needs an interactive prompt, or may require a user handoff. Agents, training jobs, and long-running builds normally meet these conditions. Create the session on the execution machine; a local tmux session that holds an SSH connection does not make remote work persistent.

Connect to `praveen@code.local` for coding and training. From the host, enter `code` with the command below:

```bash
incus exec code -- su - praveen
```

When tmux is needed, start or attach to it inside that container. Run agents and training as `praveen`, not root.

The remote `praveen` user must have systemd lingering enabled so tmux survives the final SSH disconnect. Check with `loginctl show-user praveen -p Linger`; the result must be `Linger=yes`. If it is not, repair the machine setup instead of replacing tmux with a transient service.

Use a task-specific session name. Inspect existing sessions before reuse. Never send commands into an active agent or job, or create a second copy after a connection failure. Inspect the existing session first.

SSH calls that create or inspect tmux are session control steps. Run the persistent or interactive task in the pane. Supporting checks can remain direct commands unless they need the same persistence or interactive context.

## Start and inspect

These examples use `code.local`. Change the destination for host administration. Choose an unused name:

```bash
ssh praveen@code.local 'tmux list-sessions'
ssh praveen@code.local 'tmux new-session -d -s fleet-myproject -P -F "#{pane_id}"'
```

A missing tmux server is not an SSH failure. Record the returned pane ID; do not assume pane numbering starts at zero. If the returned ID is `%3`:

```bash
ssh praveen@code.local 'tmux send-keys -t %3 -l -- "hostname; pwd; id -un"'
ssh praveen@code.local 'tmux send-keys -t %3 Enter'
ssh praveen@code.local 'tmux capture-pane -p -S -100 -t %3'
```

Use the actual pane ID. Preserve quoting across the local shell, SSH, and remote shell. For complex input, transfer a prompt or script file. Keep task notes and evidence in the repository's `_scratch/` directory.

For a local agent that needs persistence or handoff, use the same tmux commands without SSH. Create a separate pane or session; do not start it inside an active agent process.

## Run and verify

Inside the pane:

1. Confirm the machine, user, and repository path. Read `AGENTS.md` and inspect Git state.
2. Reuse a suitable checkout or create an isolated clone or worktree that follows the project's existing pattern. Do not assume Mac paths exist on Linux. Keep one writer per checkout.
3. Transfer only the required patch and untracked files, and record the base revision. Do not commit, push, overwrite a dirty checkout, or copy secrets merely to transfer work.
4. Check that the selected agent CLI is installed and authenticated. Give it the repository, task, scope, constraints, and required checks. Run it in the foreground with its normal permission controls.

The parent agent owns progress, review, and integration. Successful `send-keys` only confirms that input was sent. Inspect output, completion or exit status, the Git diff, and checks. For long tasks, retain logs and exit status in `_scratch/`. Silence or an SSH disconnect is not proof of success.

If a prompt needs Praveen, keep the session and give the attach command. Never put credentials in task files or captured logs.

## Handoff and cleanup

Give Praveen the machine, session name, repository path, status, and exact attach command:

```bash
ssh -t praveen@code.local 'tmux attach-session -t fleet-myproject'
```

For local work, use `tmux attach-session -t fleet-myproject`.

Keep active jobs and handoff sessions. When a short maintenance, benchmark, or verification task succeeds, record its result, then remove only its completed session and temporary files. Report retained changes and logs. Transfer and review the intended diff when local integration is needed; preserve unrelated changes.
