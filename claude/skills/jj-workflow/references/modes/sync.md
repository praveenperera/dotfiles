# Syncing After PR Merges

Update your local state after a PR is merged on GitHub.

---

## After a PR Merges

### 1. Fetch latest

```bash
jj git fetch
```

### 2. Rebase remaining stack onto new master

```bash
jj rebase -o master@origin -s <next-feature>
```

**Key:** Use `-s` (source) to bring descendants along, not `-r` (revision).

### 3. Delete merged bookmark

```bash
jj bookmark delete <merged-feature>
```

### 4. Update PR base on GitHub

If PR #2 was targeting PR #1's branch, update it to target master.

### 5. Push updated stack

```bash
jj git push
```

---

## After Squash-Merge (Special Case)

If GitHub squash-merged your PR, see: `references/rebase-after-squash.md`

The key difference: your original commits are gone, replaced by a single squashed commit. jj handles this cleanly with the same rebase command.

---

## Staying Up to Date

Regular sync workflow:

```bash
jj git fetch
jj log -r 'master@origin..@'  # see your commits
jj rebase -o master@origin    # rebase onto latest
jj git push                   # update PRs
```
