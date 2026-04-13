# Updating After PR Review

Make changes to a commit after receiving review feedback.

---

## Procedure

### 1. Find the commit to edit

```bash
jj log -r 'master..@'
```

### 2. Edit the commit

```bash
jj edit <change-id>
```

### 3. Make your changes

Files modified are auto-tracked.

### 4. Create new working copy when done

```bash
jj new
```

**Critical:** Without this, future changes keep amending the edited commit!

### 5. Squash review changes into the target commit

If you made review fixes in a new commit on top, squash them back:

```bash
jj squash -u  # keeps the original commit message, skips editor
```

### 6. Verify descendants auto-rebased

```bash
jj log -r 'master..@'
```

### 7. Update bookmarks if needed

```bash
jj bookmark set feature -r <change-id>
```

### 8. Push updates

```bash
jj git push
```

Force-push happens automatically. jj handles this safely.

**Note:** Force-push may collapse GitHub review comments into "outdated diff". Consider using `gh pr comment` to summarize addressed feedback.

---

## If Descendants Have Conflicts

After editing, descendants may conflict. Check:

```bash
jj log  # conflicted commits shown
jj status
```

To resolve:

```bash
jj edit <conflicted-commit>
# fix conflicts in files
jj new
```
