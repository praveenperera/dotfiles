# Update a Change After Review

Apply review feedback to the change that owns the behavior. Do not create a caller-specific workaround in a later change merely to avoid rewriting the target.

## Inspect and preserve navigation points

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj log -r 'trunk()..@ & conflicts()'
jj log -r 'trunk()..@' --no-graph \
  -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

Identify the target change, the current working-copy change ID, all affected descendants, and any bookmarks on them. Confirm that local history rewriting is authorized. Choose one update method; do not combine both.

## Alternative A: fixup commit and squash

Prefer this method because it keeps the review edits isolated until they are ready.

```bash
# create an isolated child of the target
jj new <target-change-id> -m "fixup: address review feedback"

# edit files, then inspect the fixup
jj status
jj diff

# move the fixup into the target without opening an editor
jj squash --from @ --into <target-change-id> -u
```

The target and its descendants are rewritten. If you need to return to the prior working-copy commit, use its saved change ID:

```bash
jj edit <saved-working-copy-change-id>
```

Use `jj new <rebased-stack-tip>` instead when the old working-copy commit should not remain the workspace commit.

## Alternative B: edit the target directly

Use this method for a small, well-bounded edit when temporarily making the target the working copy is acceptable:

```bash
jj edit <target-change-id>

# edit files, then inspect the amended target
jj status
jj diff

# return to the exact workspace commit saved during inspection
jj edit <saved-working-copy-change-id>
```

Do not use a bare `jj new` to "return to the stack." It creates a child of whichever revision is currently being edited and may leave the working copy below rebased descendants.

## Verify the update

Inspect graph shape, target content, bookmarks, and conflicts across all affected descendants:

```bash
jj log -r 'trunk() | <target-change-id>::'
jj diff -r <target-change-id>
jj log -r '(<target-change-id>::) & conflicts()'
jj bookmark list --all-remotes
jj status
```

If conflicts appear, inspect each conflicted revision and its files before resolving:

```bash
jj log -r '(<target-change-id>::) & conflicts()'
jj resolve --list -r <conflicted-change-id>
```

Resolve only within the authorized rewrite scope, then repeat graph, diff, and conflict verification. Run relevant tests at the updated PR tip and at affected descendant PR tips.

Move a bookmark only if verification shows that it no longer identifies the intended PR tip and bookmark mutation is authorized. Push only the reviewed bookmark when publication is authorized:

```bash
jj bookmark set <feature> -r <intended-tip>
jj git push --bookmark <feature>
jj bookmark list --all-remotes
```

History rewriting may make GitHub inline comments appear outdated. Do not post comments or alter PR state unless requested.
