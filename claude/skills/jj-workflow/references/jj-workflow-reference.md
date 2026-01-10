# jj Workflow Reference (Human-Readable)

A reference guide for understanding jj workflows, concepts, and the slash commands.

---

## How jj Differs from Git

| Aspect | Git | jj |
|--------|-----|-----|
| **Working copy** | Uncommitted changes separate from commits | Working copy IS a commit (@) |
| **Staging** | `git add` to stage | No staging - all changes auto-tracked |
| **Committing** | `git commit` creates commit | `jj describe` names existing commit |
| **Branches** | Auto-move with HEAD | Bookmarks stay fixed; move explicitly |
| **History editing** | `git rebase -i` (interactive) | Direct commands: `jj rebase`, `jj squash` |
| **Undo** | Complex reflog archaeology | Simple `jj undo` or `jj op restore` |
| **Conflicts** | Must resolve immediately | First-class; can defer resolution |

---

## Key Mental Model

1. **Working copy (@) is always a commit** - no uncommitted state
2. **Change IDs persist across rebases** - commit hashes change, change IDs don't
3. **Descendants auto-rebase** - edit commit A, and B/C/D rebase automatically
4. **Bookmarks don't auto-move** - must explicitly `jj bookmark set`
5. **Operations are logged** - every command creates a checkpoint

---

## Checkpoint Behavior

jj snapshots at the **start of every `jj` command**, not continuously:

```
1. You edit files (jj doesn't know yet)
2. You run any jj command (jj status, jj log, etc.)
3. jj reads file changes → records them in @
4. Operation saved to op log ← this is your checkpoint
```

**Implication:** Run `jj status` before risky work to ensure current edits are checkpointed.

---

## `/jj-pr` Command
**Purpose:** Publish commits as GitHub PRs.

**Modes:**
- `stacked` - commits depend on each other, PRs target previous PR
- `independent` - each commit on master, PRs all target master
- `hybrid` - mix of stacked and independent

**Key operations:**
- `jj bookmark create <name> -r <commit>` - create bookmark (becomes Git branch)
- `jj git push` - push all bookmarks
- `gh pr create --head <branch> --base <base>` - create PR

---

## Stacked vs Independent PRs

**Stacked PRs** (dependent chain):
```
master ─── A ─── B ─── C
         │     │     │
         │     │     └── PR #3 (base: B)
         │     └──────── PR #2 (base: A)
         └────────────── PR #1 (base: master)
```
- Must merge in order: A, then B, then C
- Good when features truly depend on each other

**Independent PRs** (parallel):
```
         ┌── A ── PR #1 (base: master)
master ────┼── B ── PR #2 (base: master)
         └── C ── PR #3 (base: master)
```
- Merge in any order
- Good for unrelated features

---

## After Squash-Merge (Stacked PRs)

When GitHub squash-merges your stacked PR, you need to rebase remastering commits onto the new master:

**Scenario:** You had A → B → C stacked. PR for A was squash-merged.

```bash
# 1. Fetch the updated master
jj git fetch

# 2. Rebase B (and its descendants C) onto master
jj rebase -s B -d master@origin

# 3. Update bookmarks
jj bookmark set branch-b -r B
jj bookmark set branch-c -r C

# 4. Force push (commit hashes changed)
jj git push --allow-new
```

**Key insight:** Use `-s` (source) not `-r` (revision) to bring descendants along.

**If B had conflicts with squashed A:**
- jj will mark B as conflicted after rebase
- Edit files to resolve
- Run `jj status` to confirm resolved
- The conflict markers are tracked in the commit until resolved

---

## Common Pitfalls

1. **Bookmark didn't move** - jj bookmarks don't auto-move. Use `jj bookmark set`.

2. **Forgot `jj new` after edit** - After `jj edit`, run `jj new` when done, or future changes keep amending that commit.

3. **File edits not checkpointed** - jj only snapshots when you run a command. Run `jj status` to force a checkpoint.

4. **Can't push conflicts** - jj won't push conflicted commits. Resolve first.

5. **Undo doesn't undo push** - `jj undo` is local only. To undo a push, move bookmark back and force-push.

6. **Independent PRs aren't independent** - If B uses types from A, they're not truly independent.

7. **No non-interactive hunk splitting** - Must use TUI or edit files manually.

---

## Hunk-Level Splitting (Limitation)

jj doesn't have non-interactive hunk selection. Workarounds:

**Method 1: Edit file to isolate**
1. Edit file to contain ONLY feature A code
2. `jj split <file>` - captures current state
3. Edit file to have ONLY feature B code
4. `jj describe -m "feat B"`

**Method 2: Build from scratch**
1. `jj new master`
2. Write feature A code
3. `jj describe -m "feat A"` && `jj new`
4. Write feature B code
5. `jj describe -m "feat B"`

---

## Quick Command Reference

| Command | Purpose |
|---------|---------|
| `jj status` | Show modified files + trigger snapshot |
| `jj diff` | Show all changes |
| `jj log` | Show commit graph |
| `jj log -r 'master..@'` | Show commits since master |
| `jj describe -m "msg"` | Set commit message |
| `jj new` | Create new empty commit |
| `jj new master` | Create commit based on master |
| `jj split <files>` | Split files into separate commit |
| `jj edit <id>` | Edit an older commit |
| `jj rebase -r X -d Y` | Move commit X onto Y |
| `jj bookmark create X -r Y` | Create bookmark at commit |
| `jj bookmark set X -r Y` | Move bookmark to commit |
| `jj git fetch` | Fetch from remote |
| `jj git push` | Push all tracking bookmarks |
| `jj undo` | Undo last operation |
| `jj op log` | Show operation history |
| `jj op restore <id>` | Restore to operation |

---

## Revset Shortcuts

| Expression | Meaning |
|------------|---------|
| `@` | Working copy commit |
| `@-` | Parent of @ |
| `@--` | Grandparent |
| `master` | Main bookmark |
| `master@origin` | Remote master |
| `master..@` | Commits since master |

---

## Sources

- [GitHub - jj-vcs/jj](https://github.com/jj-vcs/jj)
- [Jujutsu Documentation](https://jj-vcs.github.io/jj/latest/)
- [Operation log docs](https://jj-vcs.github.io/jj/latest/operation-log/)
- [Bookmarks docs](https://jj-vcs.github.io/jj/latest/bookmarks/)
- [Git compatibility](https://jj-vcs.github.io/jj/latest/git-compatibility/)
