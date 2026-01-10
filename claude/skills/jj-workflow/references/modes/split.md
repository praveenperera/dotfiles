# Splitting Mixed Work

Split one commit with mixed changes into logical commits.

**Mental model:** You select what goes into the FIRST commit. Everything else stays in SECOND.

```
BEFORE: master ─── @ (mixed: fileA, fileB, fileC)
AFTER:  master ─── first (fileA) ─── second (fileB, fileC)
```

---

## Methods

### Interactive TUI

```bash
jj split
```

Opens editor to select files/hunks for first commit.

### By file pattern

```bash
jj split "glob:src/auth/*"
```

### Specific files

```bash
jj split path/to/file1.ts path/to/file2.ts
```

---

## Multi-Commit Split

To split into 4 features:

```bash
# extract feature A
jj split "glob:src/feature-a/*"
jj describe @- -m "feat: feature A"

# extract feature B from remainder
jj split "glob:src/feature-b/*"
jj describe @- -m "feat: feature B"

# extract feature C
jj split "glob:src/feature-c/*"
jj describe @- -m "feat: feature C"

# remainder is feature D
jj describe -m "feat: feature D"

# result: master → A → B → C → D
```

---

## Split Older Commit

```bash
jj split -r <change-id>
# descendants auto-rebase
```

---

## Limitation: No Non-Interactive Hunk Selection

Workaround for same-file splits:

1. Edit file to contain ONLY feature A code
2. `jj split <file>` - captures current state
3. Edit file to have ONLY feature B code
4. `jj describe -m "feat B"`

Or build from scratch:

1. `jj new master`
2. Write feature A code
3. `jj describe -m "feat A"` && `jj new`
4. Write feature B code
