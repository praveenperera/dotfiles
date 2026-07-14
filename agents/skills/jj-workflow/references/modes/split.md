# Split Mixed Work

`jj split` selects changes for the first commit; unselected changes remain in the second commit:

```text
before: parent ── @ (A + B)
after:  parent ── A ── B
```

## Inspect and define ownership

```bash
jj status
jj diff --stat
jj diff
jj log -r 'trunk() | trunk()..@ | bookmarks()'
jj log -r 'trunk()..@ & conflicts()'
```

Define the behavior or invariant owned by each resulting change before splitting. Record the source change ID and its descendants. Confirm that rewriting the source is authorized.

## Choose a split method

Filesets are positional arguments; there is no `--paths` or `--files` flag.

```bash
# interactive file and hunk selection
jj split

# select a fileset for the first commit
jj split -m "feat: first change" 'glob:src/auth/**'

# select explicit files for the first commit
jj split -m "feat: first change" path/to/file1 path/to/file2

# split an older revision; descendants rebase automatically
jj split -r <source-change-id>
```

Supplying `-m` avoids the description editor in non-interactive environments. For repeated splits, identify the remainder by its new stable change ID after each operation; do not assume `@-` still names the same change.

## Split hunks non-interactively

If `jju` is installed, preview before mutating:

```bash
command -v jju
jju sh --preview
jju sh --preview --file src/lib.rs
```

Then select by hunk, line range, or pattern:

```bash
jju sh -m "feat: first change" --hunks 0,2
jju sh -m "fix: bounded behavior" --file src/lib.rs --lines 10-50
jju sh -m "refactor: tracing" --pattern 'tracing::'
```

Use `--dry-run` for an additional preview. Use `-r <source-change-id>` for an older commit. Hunk indices and line numbers describe the current preview and may change after every split, so preview again before the next operation.

## Verify each result

After every split:

```bash
jj log -r 'trunk() | <first-change-id> | <remainder-change-id> | <affected-descendants>'
jj diff -r <first-change-id>
jj diff -r <remainder-change-id>
jj log -r '(<first-change-id> | <remainder-change-id> | <affected-descendants>) & conflicts()'
jj status
```

Check that each commit has one coherent responsibility and that later commits do not accidentally supply files needed by earlier commits. Run relevant verification at each intended PR tip. Do not create bookmarks or publish merely because the split succeeded; follow the selected PR mode after local verification.
