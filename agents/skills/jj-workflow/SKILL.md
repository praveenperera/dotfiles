---
name: jj-workflow
description: Plan and execute Jujutsu (jj) version-control workflows, including inspecting repository state, splitting or updating changes, arranging stacked, independent, or hybrid PR graphs, syncing after merges, and publishing bookmarks. Use for jj commands, change IDs, revsets, bookmarks, history rewriting, Git interoperability, or preparing commits and pull requests in a jj repository.
---

# jj workflow

Treat the working copy (`@`) as a commit, use stable change IDs while rewriting, and remember that bookmarks do not move automatically.

## Start with inspection

Inspect before choosing a workflow or running a mutating command:

```bash
jj root
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj git remote list
jj log -r 'trunk()..@ & conflicts()'
```

Use `trunk()` for the repository's configured trunk revision. Determine the actual trunk bookmark name from the bookmark output only when another tool, such as `gh pr create --base`, requires a branch name.

Record the relevant change IDs before rewriting:

```bash
jj log -r 'trunk()..@' --no-graph \
  -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

If the repository is not a jj repository, report that fact. Do not initialize or colocate it unless requested.

## Establish mutation scope

Infer the smallest scope authorized by the request and state any material assumption. Keep these capabilities separate:

- network synchronization: `jj git fetch`
- local history rewrites: `jj split`, `jj squash`, `jj rebase`, `jj edit`, `jj abandon`, and `jj duplicate`
- bookmark mutations: `jj bookmark create`, `set`, `move`, `delete`, `forget`, or `track`
- remote publication: `jj git push`, PR creation, and PR base edits

Do not fetch merely to inspect. Do not rewrite history, mutate bookmarks, push, or change PRs unless that capability is within the user's requested scope. A request to prepare or reorganize local commits does not authorize publishing. A request to push one bookmark does not authorize pushing every tracking bookmark; prefer `jj git push --bookmark <name>`.

Before a rewrite, show or summarize the current graph, the intended graph, affected change IDs, and expected bookmark effects. Use `jj op log` and `jj undo` for local recovery, but never imply that `jj undo` reverses a remote push.

## Choose a workflow

- Read [stacked.md](references/modes/stacked.md) for dependent changes and ordered PR bases.
- Read [independent.md](references/modes/independent.md) for changes that each apply directly to trunk.
- Read [hybrid.md](references/modes/hybrid.md) for a graph containing both dependent and independent work.
- Read [split.md](references/modes/split.md) to separate mixed changes by file or hunk.
- Read [update.md](references/modes/update.md) to address feedback in an existing change.
- Read [sync.md](references/modes/sync.md) after a merge or when explicitly asked to synchronize.
- Read [rebase-after-squash.md](references/rebase-after-squash.md) when an upstream PR was squash-merged.
- Read [quick-reference.md](references/quick-reference.md) for command and revset syntax.
- Read [full-guide.md](references/full-guide.md) for graph design, safety boundaries, and Git interoperability.

Use the templates in `examples/` only after replacing every placeholder and confirming the mutation and publication scope. They intentionally stop after inspection by default.

## Verify every mutation

After each rewrite, inspect the exact affected graph rather than relying only on command success:

```bash
jj log -r 'trunk() | <affected-revset>'
jj diff -r <change-id>
jj log -r '(<affected-revset>) & conflicts()'
jj status
```

No output from the conflict query means the selected revisions are conflict-free. If conflicts exist, list the conflicted revisions and files with:

```bash
jj log -r '(<affected-revset>) & conflicts()'
jj resolve --list -r <conflicted-change-id>
```

After bookmark mutations, run `jj bookmark list --all-remotes`. Before an authorized push, inspect the exact bookmark and its range, then push only that bookmark. After a push, inspect local and remote bookmark targets again.

## Non-interactive commands

Avoid commands that open editors in automation:

```bash
jj split -m "feat: description" path/to/file
jj squash --from <source> --into <destination> -u
```

Use `jju sh` for non-interactive hunk selection only after `command -v jju` confirms it is installed. Preview before splitting:

```bash
jju sh --preview
jju sh -m "feat: description" --hunks 0,2
```

For command behavior that may vary by installed jj version, inspect `jj <command> --help` instead of guessing.
