# Independent PRs Workflow

Each commit rebased onto master. All PRs target master. Merge in any order.

```
         ┌── A ──────── PR #1 (base: master)
         │
master ──┼── B ──────── PR #2 (base: master)
         │
         └── C ──────── PR #3 (base: master)
```

---

## Procedure

### 1. Start with commits (may be stacked initially)

```bash
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

### 2. Rebase each onto master (except first which is already there)

```bash
jj rebase -r <change-id-B> -d master
jj rebase -r <change-id-C> -d master
```

### 3. Create bookmarks

```bash
jj bookmark create pr/<feature-a> -r <change-id-A>
jj bookmark create pr/<feature-b> -r <change-id-B>
jj bookmark create pr/<feature-c> -r <change-id-C>
```

### 4. Push and create PRs

```bash
jj git push

gh pr create --head pr/<feature-a> --base master --title "<title>"
gh pr create --head pr/<feature-b> --base master --title "<title>"
gh pr create --head pr/<feature-c> --base master --title "<title>"
```

### 5. Optional: dev merge for combined testing

```bash
jj new pr/<feature-a> pr/<feature-b> pr/<feature-c> -m "dev: combined"
# now @ contains all features merged
```

---

## Pitfall: Not Actually Independent

If B uses types/exports from A, they're not truly independent.

**Test each in isolation:**
```bash
jj new master -m "test B alone"
jj squash --from <B> --into @
# run tests - if they fail, B depends on A
```

If they depend on each other, use stacked PRs instead.
