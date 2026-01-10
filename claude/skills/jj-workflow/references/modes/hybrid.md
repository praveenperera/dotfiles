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
jj rebase -r <change-id-C> -d master
```

### 4. Rebase dependent commits to correct parent

```bash
jj rebase -r <change-id-D> -d <change-id-B>
```

### 5. Create bookmarks and push

```bash
jj bookmark create pr/feat-a -r <A>
jj bookmark create pr/feat-b -r <B>
jj bookmark create pr/feat-c -r <C>
jj bookmark create pr/feat-d -r <D>
jj git push
```

### 6. Create PRs with correct bases

```bash
gh pr create --head pr/feat-a --base master --title "A"
gh pr create --head pr/feat-b --base pr/feat-a --title "B"
gh pr create --head pr/feat-c --base master --title "C (independent)"
gh pr create --head pr/feat-d --base pr/feat-b --title "D"
```

---

## Dev Merge for Combined Work

```bash
jj new pr/feat-d pr/feat-c -m "dev: all features"
# work here; edits to specific commits via jj edit
```
