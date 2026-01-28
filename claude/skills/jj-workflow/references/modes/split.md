# Splitting Mixed Work

Split one commit with mixed changes into logical commits.

**Mental model:** You select what goes into the FIRST commit. Everything else stays in SECOND.

```
BEFORE: master ─── @ (mixed: fileA, fileB, fileC)
AFTER:  master ─── first (fileA) ─── second (fileB, fileC)
```

---

## Methods

**⚠️ Syntax Warning:** Files are positional arguments, NOT flags. There is no `--paths` or `--files` flag.
- ✓ `jj split file1.ts file2.ts`
- ✗ `jj split --paths file1.ts file2.ts`

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

## Non-Interactive Hunk Selection with `jju sh`

The `jju sh` (split-hunk) command enables non-interactive hunk-level splitting, useful for Claude Code and automation.

### Preview hunks

```bash
jju sh --preview                     # all files
jju sh --preview --file src/lib.rs   # specific file
```

Output shows hunks with indices:
```
[0]  modified (lines 1-8):
     ...
[1]  added (lines 60-105):
     ...
```

### Split by hunk index

```bash
jju sh -m "Feature A" --hunks 0,2
jju sh -m "Bug fix" --file src/lib.rs --hunks 1
```

### Split by line range

```bash
jju sh -m "Feature A" --file src/lib.rs --lines 10-50
jju sh -m "Feature B" --lines 100-150,200-250
```

### Split by pattern

```bash
jju sh -m "Add logging" --pattern "log::|tracing::"
jju sh -m "Error handling" --file src/lib.rs --pattern "Error|Result"
```

### Invert selection

Use `--invert` to exclude matched hunks instead of including:

```bash
jju sh -m "Everything except logging" --pattern "log::" --invert
```

### Dry run

Preview what would be committed without making changes:

```bash
jju sh --dry-run -m "Test" --hunks 0,2
```

### Workflow Example

```bash
# 1. preview hunks to see indices
jju sh --preview

# 2. split feature A (hunks 0 and 2)
jju sh -m "feat: feature A" --hunks 0,2

# 3. split feature B (lines 100-200)
jju sh -m "feat: feature B" --lines 100-200

# 4. remaining changes stay in @
jj describe -m "feat: feature C"
```

---

## Manual Hunk Splitting (without jju)

If `jju` is not available, use this workaround:

1. Edit file to contain ONLY feature A code
2. `jj split <file>` - captures current state
3. Edit file to have ONLY feature B code
4. `jj describe -m "feat B"`

Or build from scratch:

1. `jj new master`
2. Write feature A code
3. `jj describe -m "feat A"` && `jj new`
4. Write feature B code
