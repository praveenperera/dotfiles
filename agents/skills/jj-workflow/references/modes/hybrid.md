# Hybrid PR Graphs

Use a hybrid graph when some changes depend on each other and others are independent. For example, A → B → D is a stack while C applies directly to trunk:

```text
          ┌─ A ── B ── D
trunk ────┤
          └─ C
```

## Inspect and model dependencies

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj log -r 'trunk()..@ & conflicts()'
jj log -r 'trunk()..@' --no-graph \
  -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

Write down the intended parent of every change before rewriting. Confirm that C does not consume A or B and that D depends on B rather than C.

## Arrange the graph

If the current graph is `trunk → A → B → C → D` and local rewriting is authorized:

```bash
jj rebase -r <C> -o 'trunk()'
jj rebase -r <D> -o <B>
```

Use stable change IDs because positional expressions can change after each operation. Verify the whole planned graph and conflict set:

```bash
jj log -r 'trunk() | <A> | <B> | <C> | <D>'
jj log -r '(<A> | <B> | <C> | <D>) & conflicts()'
```

Run relevant checks at B, C, and D. The graph alone cannot prove that C is independent.

## Name and publish

Only when bookmark mutation is authorized:

```bash
jj bookmark create <feature-a> -r <A>
jj bookmark create <feature-b> -r <B>
jj bookmark create <feature-c> -r <C>
jj bookmark create <feature-d> -r <D>
jj bookmark list --all-remotes
```

Only when publication and PR creation are authorized:

```bash
jj git push --bookmark <feature-a>
jj git push --bookmark <feature-b>
jj git push --bookmark <feature-c>
jj git push --bookmark <feature-d>

gh pr create --head <feature-a> --base <trunk-bookmark> --title "<title A>"
gh pr create --head <feature-b> --base <feature-a> --title "<title B>"
gh pr create --head <feature-c> --base <trunk-bookmark> --title "<title C>"
gh pr create --head <feature-d> --base <feature-b> --title "<title D>"
```

Determine `<trunk-bookmark>` during inspection. For combined local testing, create a merge working copy from `<D>` and `<C>` only when local mutation is in scope.
