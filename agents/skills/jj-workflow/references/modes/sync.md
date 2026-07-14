# Sync After a Merge

Synchronize only when fetching remote state and rewriting the remaining local stack are both within scope. Fetching, rebasing, bookmark cleanup, pushing, and changing PR bases are separate actions.

## Inspect before fetching

```bash
jj status
jj diff --stat
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
jj git remote list
jj log -r 'trunk()..@ & conflicts()'
```

Record the first unmerged change ID, remaining descendants, current bookmark targets, and the trunk bookmark name.

## Refresh remote state

If fetching is authorized:

```bash
jj git fetch
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj bookmark list --all-remotes
```

Confirm that `trunk()` now contains the merged PR before rewriting anything. If the upstream PR was squash-merged, also read [rebase-after-squash.md](../rebase-after-squash.md).

## Rebase the remaining stack

If local rewriting is authorized, move the first unmerged change and its descendants onto trunk:

```bash
jj rebase -s <first-unmerged-change-id> -o 'trunk()'
```

Verify the full remaining stack and conflict set:

```bash
jj log -r 'trunk() | <first-unmerged-change-id>::'
jj log -r '(<first-unmerged-change-id>::) & conflicts()'
jj status
```

Run relevant checks at every remaining PR tip affected by the rebase.

## Reconcile publication state

Only when bookmark cleanup is authorized, inspect whether the merged bookmark is still needed before deleting it:

```bash
jj bookmark list --all-remotes
jj bookmark delete <merged-feature>
```

Move remaining bookmarks only when inspection shows they do not follow the intended rebased change IDs. Only when publication is authorized, push bookmarks individually:

```bash
jj git push --bookmark <remaining-feature>
jj bookmark list --all-remotes
```

Change a remaining PR's base to `<trunk-bookmark>` only when PR mutation is authorized. Verify the remote bookmark targets and PR bases after publication.
