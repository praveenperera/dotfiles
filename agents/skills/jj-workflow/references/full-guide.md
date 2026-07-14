# Jujutsu Workflow Design

Use this reference for the concepts and safety boundaries shared by the mode-specific procedures. Use the files in `modes/` for concrete commands.

## Mental model

- The working copy `@` is a commit. jj snapshots tracked changes when commands run.
- Change IDs survive rewrites; commit IDs usually do not. Record change IDs before rearranging a graph.
- Rewriting a change automatically rebases its descendants.
- Conflicts can be stored in commits. A completed command does not imply a conflict-free graph.
- Bookmarks name revisions but do not follow the working copy automatically.
- `jj undo` and `jj op restore` affect local operations; they do not reverse a remote push or hosting-service action.

## Inspect before deciding

Start every workflow from evidence:

```bash
jj root
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj git remote list
jj log -r 'trunk()..@ & conflicts()'
```

Use `trunk()` rather than assuming `main`, `master`, a remote name, or a local tracking bookmark. When GitHub requires a branch name, derive the actual trunk bookmark from inspection.

Do not initialize a jj repository, fetch, rewrite, mutate bookmarks, publish, or alter PR state unless the request authorizes that capability. Inspection and planning can still proceed when mutation is out of scope.

## Design the graph before commands

Choose the graph from code dependencies, not convenience:

```text
stacked                 independent              hybrid

trunk─A─B─C             trunk─┬─A                 trunk─┬─A─B─D
                              ├─B                       └─C
                              └─C
```

- Stack changes when B requires A or merge order matters.
- Make changes independent only when each applies and passes checks directly on trunk.
- Use a hybrid graph when dependency groups differ.

Before rewriting, record each change ID, intended parent, intended PR bookmark, and intended PR base. Prefer `jj rebase -r` to move only named revisions and `jj rebase -s` to move a root with all descendants. Re-read `jj rebase --help` when the desired graph is more complex than those cases.

## Separate local shape from publication

Treat these as distinct phases:

1. Inspect repository and remote-tracking state.
2. Plan the intended graph and authorization boundary.
3. Rewrite local changes if authorized.
4. Verify graph, content, conflicts, and project behavior.
5. Create or move bookmarks if authorized.
6. Push exact bookmarks if authorized.
7. Create or edit PRs if authorized.

Do not use bare `jj git push` when only named PR bookmarks are in scope; it can publish every tracked bookmark. Prefer:

```bash
jj git push --bookmark <feature>
```

## Verify graph and content

For affected changes A, B, and C:

```bash
jj log -r 'trunk() | <A> | <B> | <C>'
jj diff -r <A>
jj diff -r <B>
jj diff -r <C>
jj log -r '(<A> | <B> | <C>) & conflicts()'
jj status
```

Graph inspection checks parentage. Diffs check ownership. The conflict revset checks revisions that may not be the working copy. Project checks at each intended PR tip establish behavioral independence and catch failures masked by later changes.

For conflicts:

```bash
jj log -r '(<affected-revset>) & conflicts()'
jj resolve --list -r <conflicted-change-id>
```

Resolve one conflicted change at a time, then repeat graph, diff, and conflict verification across its descendants.

## Work with bookmarks deliberately

Inspect local and remote bookmarks together:

```bash
jj bookmark list --all-remotes
```

Create bookmarks only after the graph is correct. After a rewrite, do not move a bookmark reflexively: a bookmark attached to a rewritten change may already identify the correct new commit. Move it only when inspection proves otherwise.

Before an authorized push, inspect what the bookmark contains relative to trunk:

```bash
jj log -r 'trunk() | trunk()..<feature>'
jj log -r '(trunk()..<feature>) & conflicts()'
```

After pushing, inspect all remote targets again. If a remote bookmark moved unexpectedly, fetch only with authorization, compare local and remote targets, and ask for direction when reconciling would overwrite someone else's work.

## Colocated Git interoperability

A colocated repository contains both `.jj/` and `.git/`. Git tools may show a detached HEAD; this is normal. Prefer read-only Git commands for integrations and use jj for history-changing operations.

Safe inspection examples include `git log`, `git diff`, `git show`, and `git remote -v`. Avoid mixing `git commit`, `git rebase`, `git merge`, `git stash`, or `git checkout` into a jj-managed workflow because they change the Git view behind jj's model.

If another contributor moves a remote branch, inspect the divergence rather than automatically overwriting it. Remote collaboration does not broaden authorization to fetch, force an outcome, or push.

## Recover locally

Use the operation log to understand a mistake:

```bash
jj op log
jj undo
```

Use `jj op restore <operation-id>` only after inspecting the target operation and confirming that restoring repository state is authorized. If a push already occurred, report the remote state separately; local recovery does not change it.

## Detailed procedures

- [Stacked PRs](modes/stacked.md)
- [Independent PRs](modes/independent.md)
- [Hybrid graphs](modes/hybrid.md)
- [Splitting work](modes/split.md)
- [Updating after review](modes/update.md)
- [Syncing after merges](modes/sync.md)
- [Rebasing after a squash merge](rebase-after-squash.md)
- [Command reference](quick-reference.md)
