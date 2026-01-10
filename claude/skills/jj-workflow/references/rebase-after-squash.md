# Rebase Stack After Squash-Merge

When GitHub squash-merges your stacked PR, you need to rebase remaining commits onto the new master.

**This is one command in jj vs a painful mess in Git.**

---

## The Problem (in Git)

You have stacked PRs: A → B → C → D. GitHub squash-merges A into master. Now:
- B still has all the original A commits as its base
- `git rebase master` sees original A and squashed A as *different commits*
- Git tries to replay everything, causing conflicts and history mess

---

## The Solution (in jj)

jj doesn't care that A was squashed. It just moves B onto the new master:

```bash
jj git fetch
jj rebase -s <B> -d master@origin
```

That's it. The original A commits are abandoned cleanly. B, C, D all rebase onto master with no confusion.

---

## Full Procedure

```bash
# 1. Fetch updated master (with squash-merged A)
jj git fetch

# 2. See current state - find B (first commit after merged A)
jj log -r 'master@origin..@'

# 3. Rebase B and all descendants onto master
jj rebase -s <B-change-id> -d master@origin

# 4. Verify clean history
jj log -r 'master@origin..@'
```

If B has conflicts with the squashed changes:

```bash
jj status              # show conflicted files
# edit files to resolve
jj status              # verify resolved
```

```bash
# 5. Update bookmarks to rebased commits
jj bookmark set pr/feature-b -r <B>
jj bookmark set pr/feature-c -r <C>

# 6. Push (force-push happens automatically)
jj git push
```

---

## Why This Works

- jj uses **change IDs** that persist across rebases (not commit hashes)
- `-s` (source) moves the commit AND all descendants
- jj doesn't try to "replay" commits like Git - it just re-parents them
- Original commits are abandoned, not conflicted against

---

## Key Point

| Git | jj |
|-----|-----|
| Painful multi-step with conflicts | `jj rebase -s B -d master@origin` |
