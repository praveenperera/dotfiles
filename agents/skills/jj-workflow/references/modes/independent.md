# Independent PRs

Use independent PRs only when each change applies and passes verification directly on trunk:

```text
          ┌─ A
trunk ────┼─ B
          └─ C
```

All PRs target the trunk bookmark and may merge in any order.

## Inspect and test the model

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj log -r 'trunk()..@ & conflicts()'
jj log -r 'trunk()..@' --no-graph \
  -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

Check imports, generated files, schema changes, and tests for hidden dependencies. If B needs anything introduced by A, use [stacked.md](stacked.md) or [hybrid.md](hybrid.md).

## Arrange independent roots

If A, B, and C currently form a chain and local rewriting is authorized, move only the revisions that should become independent:

```bash
jj rebase -r <B> -o 'trunk()'
jj rebase -r <C> -o 'trunk()'
```

Use stable change IDs, not `@-` positions that change after the first rebase. Verify the parentage visually and check the entire affected set for conflicts:

```bash
jj log -r 'trunk() | <A> | <B> | <C>'
jj log -r '(<A> | <B> | <C>) & conflicts()'
jj diff -r <A>
jj diff -r <B>
jj diff -r <C>
```

Run the project's relevant verification from each independent tip. A clean rebase proves graph shape, not behavioral independence.

## Name and publish

Only when bookmark mutation is authorized:

```bash
jj bookmark create <feature-a> -r <A>
jj bookmark create <feature-b> -r <B>
jj bookmark create <feature-c> -r <C>
jj bookmark list --all-remotes
```

Only when publication is authorized:

```bash
jj git push --bookmark <feature-a>
jj git push --bookmark <feature-b>
jj git push --bookmark <feature-c>

gh pr create --head <feature-a> --base <trunk-bookmark> --title "<title A>"
gh pr create --head <feature-b> --base <trunk-bookmark> --title "<title B>"
gh pr create --head <feature-c> --base <trunk-bookmark> --title "<title C>"
```

Determine `<trunk-bookmark>` during inspection. Do not substitute `trunk()` in a `gh` command because GitHub expects a branch name.

For combined local testing, an authorized `jj new <A> <B> <C> -m "test: combined changes"` creates a merge working copy. Treat it as local scaffolding, not a PR tip, unless explicitly requested.
