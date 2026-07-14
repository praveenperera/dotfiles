# Stacked PRs

Use a stack when each change depends on the previous change:

```text
trunk ── A ── B ── C
          \    \    \
           PR1  PR2  PR3
```

PR A targets the trunk bookmark, PR B targets A's bookmark, and PR C targets B's bookmark. Merge them in order.

## Inspect and plan

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj log -r 'trunk()..@ & conflicts()'
jj log -r 'trunk()..@' --no-graph \
  -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

Identify A, B, and C by change ID. Confirm that the existing or intended graph is `trunk → A → B → C`, and determine the trunk bookmark name for GitHub. Fetch only when current remote state is required and the request authorizes synchronization.

## Arrange the stack

If mixed work must be separated and local rewriting is authorized, follow [split.md](split.md). If the changes already form the planned chain, do not rewrite them merely to normalize the workflow.

Verify the result:

```bash
jj log -r 'trunk() | <A> | <B> | <C>'
jj log -r '(<A> | <B> | <C>) & conflicts()'
```

Run the project's relevant checks at each PR tip when later changes can mask a missing dependency or failure.

## Name and publish

Only when bookmark mutation is authorized:

```bash
jj bookmark create <feature-a> -r <A>
jj bookmark create <feature-b> -r <B>
jj bookmark create <feature-c> -r <C>
jj bookmark list --all-remotes
```

Only when remote publication is authorized, push each intended bookmark explicitly:

```bash
jj git push --bookmark <feature-a>
jj git push --bookmark <feature-b>
jj git push --bookmark <feature-c>
```

Create PRs only when requested, using the actual trunk bookmark name where `<trunk-bookmark>` appears:

```bash
gh pr create --head <feature-a> --base <trunk-bookmark> --title "<title A>"
gh pr create --head <feature-b> --base <feature-a> --title "<title B>"
gh pr create --head <feature-c> --base <feature-b> --title "<title C>"
```

After publication, re-run `jj bookmark list --all-remotes` and verify each PR's base. After a merge, follow [sync.md](sync.md); use [rebase-after-squash.md](../rebase-after-squash.md) for a squash merge.
