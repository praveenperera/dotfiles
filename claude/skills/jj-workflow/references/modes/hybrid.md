# Hybrid Workflow

Some commits stacked, some independent.

Example: A→B stacked, C independent, D depends on B

```
         ┌── A ─── B ─── D   (stacked: A→B→D)
master ──┤
         └── C               (independent)
```

---

## Procedure

### 1. Get change IDs

```bash
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

### 2. Decide which are stacked vs independent

Ask: does this commit depend on another's code?

### 3. Rebase independent commits onto master

```bash
jj rebase -r <change-id-C> -o master
```

### 4. Rebase dependent commits to correct parent

```bash
jj rebase -r <change-id-D> -o <change-id-B>
```

### 5. Create bookmarks and push

```bash
jj bookmark create feat-a -r <A>
jj bookmark create feat-b -r <B>
jj bookmark create feat-c -r <C>
jj bookmark create feat-d -r <D>
jj git push
```

### 6. Create PRs with correct bases

```bash
gh pr create --head feat-a --base master --title "A"
gh pr create --head feat-b --base feat-a --title "B"
gh pr create --head feat-c --base master --title "C (independent)"
gh pr create --head feat-d --base feat-b --title "D"
```

---

## Dev Merge for Combined Work

```bash
jj new feat-d feat-c -m "dev: all features"
# work here; edits to specific commits via jj edit
```
