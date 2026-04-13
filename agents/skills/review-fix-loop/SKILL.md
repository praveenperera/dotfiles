---
name: review-fix-loop
description: Recursive PR review and fix loop. Creates a self-sustaining review-fix cycle that runs the PR review toolkit, fixes issues, documents fixes, commits, then re-reviews until no issues remain. Use when the user invokes /review-fix-loop or wants an automated iterative code review and fix workflow.
hooks:
  Stop:
    - hooks:
        - type: command
          command: "$HOME/.claude/scripts/review-fix-loop-check.sh"
---

# Review-Fix Loop

Run the PR review toolkit, fix all issues found, commit, then re-review. Repeat until no new issues are found or the safety cap is reached.

## Workflow

### 1. Create Tracking Doc

Create `PR_REVIEW_FIXES.md` in the project root:

```markdown
# PR Review Fixes

## STATUS: IN_PROGRESS

## Iteration 1
```

### 2. Get Current Diff

Run `git diff` (or `git diff HEAD~1` on subsequent iterations) to get the changes to review.

### 3. Run PR Review

**Each iteration MUST spawn a fresh Agent** (new Agent tool call) to run the review. Do NOT reuse a prior agent or run the review inline — a clean context ensures the reviewer evaluates the code without bias from prior iterations.

Provide the agent with:
- The diff
- The full contents of `PR_REVIEW_FIXES.md` (on iterations 2+)
- Explicit instruction: "The following changes have already been made in previous iterations. Do NOT flag these as issues. Only report genuinely new problems."
- Instruction to use the `pr-review-toolkit:code-reviewer` skill

### 4. Fix Issues

For each issue found:
1. **Before fixing**: check the tracking doc history. If the fix would revert or undo a change from a previous iteration, **skip it** and log as "SKIPPED - would revert Iteration N fix"
2. If the same file+location has been changed 2+ times across iterations, **stop the cycle** - set STATUS: COMPLETE and note the oscillation
3. Otherwise, fix the issue

### 5. Update Tracking Doc

Add entries for each fix and skip using this format:

```markdown
## Iteration N

### Fixed
- **file.rs:42** - Changed `foo()` to `bar()` because [reason]. Was: `old code`. Now: `new code`
- **file.rs:88** - Removed unused import `baz`

### Skipped (would revert previous fix)
- **file.rs:42** - Reviewer suggested reverting to `foo()` but this was intentionally changed in Iteration 1

### No issues found
(appears when iteration found nothing new)
```

### 6. Commit

Create a git commit with a proper descriptive message summarizing what was fixed in this iteration. Reference the specific issues addressed. Do NOT make WIP commits.

### 7. Attempt to Stop

After committing, attempt to finish. The Stop hook will:
- **Allow stop** if STATUS: COMPLETE (no more issues)
- **Block stop** if STATUS: IN_PROGRESS (forces another iteration)

### 8. Next Iteration

On subsequent iterations:
- Review against the **last commit's diff** (`git diff HEAD~1`)
- Include the **full tracking doc** so the reviewer sees prior context
- Increment the iteration number in the tracking doc

### 9. Termination Conditions

Set `## STATUS: COMPLETE` when any of these are true:
- A review pass finds **zero new actionable issues** (excluding skipped reversions)
- **Oscillation detected**: same file+location changed 2+ times across iterations
- **5 iterations reached** (safety cap) - note any remaining issues in the doc

### 10. Cleanup

After completion, ask the user if they want to keep or delete `PR_REVIEW_FIXES.md`.
