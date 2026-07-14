# Rebase a Stack After a Squash Merge

Suppose A → B → C is a local stack and the hosting service replaces A with one squash commit on trunk. Rebase B and its descendants onto the refreshed `trunk()`; do not replay A.

## Inspect and refresh

Before fetching:

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj log -r 'trunk()..@ & conflicts()'
```

Record B's stable change ID, all remaining PR tips, and their bookmarks. If remote synchronization is authorized:

```bash
jj git fetch
jj log -r 'trunk() | trunk()..@ | bookmarks()'
```

Inspect the refreshed graph and confirm that `trunk()` contains the squash result. Do not infer this only from the hosting service's merge status.

## Rebase the remaining stack

If local history rewriting is authorized:

```bash
jj rebase -s <B-change-id> -o 'trunk()'
```

`-s` moves B and all descendants. Their change IDs remain stable while commit IDs change.

Verify the resulting graph, diffs, and conflict set:

```bash
jj log -r 'trunk() | <B-change-id>::'
jj diff -r <B-change-id>
jj log -r '(<B-change-id>::) & conflicts()'
jj status
```

If conflicts exist, inspect each conflicted revision with `jj resolve --list -r <change-id>`, resolve it within scope, and repeat the verification. Run relevant checks at every remaining PR tip.

## Reconcile bookmarks and PRs

Because bookmarks identify changes rather than a current branch, first inspect whether they already point to the intended rebased changes:

```bash
jj bookmark list --all-remotes
```

Move or delete bookmarks only when necessary and authorized. Push each remaining bookmark explicitly only when publication is authorized:

```bash
jj git push --bookmark <feature-b>
jj git push --bookmark <feature-c>
jj bookmark list --all-remotes
```

Update B's PR base to the actual trunk bookmark only when PR mutation is authorized. Verify remote targets and PR bases afterward.
