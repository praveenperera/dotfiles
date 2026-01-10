# Stacked PRs Workflow

Commits depend on each other. PRs target the previous PR's branch.

```
master ─── A ─── B ─── C
         │     │     │
         │     │     └── PR #3 (base: B)
         │     └──────── PR #2 (base: A)
         └────────────── PR #1 (base: master)
```

**Must merge in order:** A, then B, then C.

---

## Procedure

### 1. Ensure up-to-date with master

```bash
jj git fetch
jj rebase -o master@origin  # if needed
```

### 2. Split work if needed

```bash
jj split "glob:src/feature-a/*"
jj describe @- -m "feat: feature A"
# repeat for each feature...
```

### 3. Get change IDs

```bash
jj log -r 'master..@-' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
```

### 4. Create bookmarks

```bash
jj bookmark create pr/<feature-a> -r <change-id-A>
jj bookmark create pr/<feature-b> -r <change-id-B>
jj bookmark create pr/<feature-c> -r <change-id-C>
```

### 5. Push

```bash
jj git push
```

### 6. Create PRs with correct bases

```bash
gh pr create --head pr/<feature-a> --base master --title "<title A>"
gh pr create --head pr/<feature-b> --base pr/<feature-a> --title "<title B>"
gh pr create --head pr/<feature-c> --base pr/<feature-b> --title "<title C>"
```

---

## Pitfall: Large Stacks

10+ stacked PRs become unwieldy - slow rebases, cascading conflicts.

**Mitigation:**
- Keep stacks to 3-5 PRs max
- Use hybrid approach: some stacked, some independent
- Merge PRs incrementally as approved
- Break large features into parallel stacks

---

## After First PR Merges

See: `references/modes/sync.md` or `references/rebase-after-squash.md`
