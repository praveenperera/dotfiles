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

### Non-Interactive (for scripts/automation)

**Important:** Even when specifying files, `jj split` opens an editor for the commit description. Use `-m` to skip the editor entirely:

```bash
jj split -m "feat: description for first commit" path/to/file1.ts path/to/file2.ts
```

This is required for:
- Claude Code and other non-interactive environments
- CI/CD pipelines
- Shell scripts

---

## Multi-Commit Split

To split into 4 features:

```bash
# extract feature A (use -m to skip editor)
jj split -m "feat: feature A" "glob:src/feature-a/*"

# extract feature B from remainder
jj split -m "feat: feature B" "glob:src/feature-b/*"

# extract feature C
jj split -m "feat: feature C" "glob:src/feature-c/*"

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
