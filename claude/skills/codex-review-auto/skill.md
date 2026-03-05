---
name: codex-review-auto
description: Autonomous Codex plan review. Claude resolves disagreements on its own, then presents a decision log and final plan for user approval.
user_invocable: true
---

# Codex Plan Review (Autonomous)

Send the current implementation plan to OpenAI Codex for review. Claude autonomously resolves all feedback and iterates until Codex approves. At the end, Claude writes a decision log file and enters plan mode for final user approval.

---

## When to Invoke

- When the user runs `/codex-review-auto` during or after plan mode
- When the user wants Codex review but doesn't want to be involved in each round

## Agent Instructions

When invoked, perform the following autonomous review loop:

### Step 1: Generate Session ID

```bash
REVIEW_ID=$(uuidgen | tr '[:upper:]' '[:lower:]' | head -c 8)
```

Use this for all file paths:
- `/tmp/claude-plan-${REVIEW_ID}.md` — the plan
- `/tmp/codex-review-${REVIEW_ID}.md` — Codex output
- `/tmp/codex-decisions-${REVIEW_ID}.md` — decision log (written at end)

### Step 2: Capture the Plan

Write the current plan to `/tmp/claude-plan-${REVIEW_ID}.md`. If there is no plan in the current context, ask the user what they want reviewed.

### Step 3: Initial Review (Round 1)

```bash
codex exec \
  -m gpt-5.4 \
  -s read-only \
  -o /tmp/codex-review-${REVIEW_ID}.md \
  "Review the implementation plan in /tmp/claude-plan-${REVIEW_ID}.md. Focus on:
1. Correctness - Will this plan achieve the stated goals?
2. Risks - What could go wrong? Edge cases? Data loss?
3. Missing steps - Is anything forgotten?
4. Alternatives - Is there a simpler or better approach?
5. Security - Any security concerns?

Be specific and actionable. If the plan is solid and ready to implement, end your review with exactly: VERDICT: APPROVED

If changes are needed, end with exactly: VERDICT: REVISE"
```

**Capture the Codex session ID** from the output. Store as `CODEX_SESSION_ID` for session resume.

**Notes:**
- Use `-m gpt-5.4` as default. Accept model override from user arguments (e.g., `/codex-review-auto o4-mini`).
- Use `-s read-only` so Codex cannot modify files.
- Use `-o` to capture output to a file.

### Step 4: Read Review & Check Verdict

1. Read `/tmp/codex-review-${REVIEW_ID}.md`
2. Check the verdict:
   - If **VERDICT: APPROVED** → go to Step 7 (Done)
   - If **VERDICT: REVISE** → go to Step 5 (Autonomous Triage & Revise)
   - If no clear verdict but feedback is all positive → treat as approved
   - If max rounds (5) reached → go to Step 7

### Step 5: Autonomous Triage & Revise

For each Codex finding, Claude decides autonomously:

1. **Triage each finding** into one of three categories:
   - **Accepted** — Claude agrees and will fix it
   - **Rejected** — Claude believes the finding is incorrect, impractical, or already handled
   - **Partially accepted** — Claude takes the spirit of the feedback but implements differently

2. **Record every decision** in an internal list with this structure per finding:
   - Codex finding (verbatim summary)
   - Severity (as Codex rated it)
   - Claude's decision: accepted / rejected / partially accepted
   - Claude's reasoning (1-2 sentences)
   - Action taken (what changed in the plan, or "no change")

3. **Revise the plan** — apply accepted/partially-accepted changes. Rewrite `/tmp/claude-plan-${REVIEW_ID}.md`.

4. Do NOT ask the user or enter plan mode between rounds. Keep iterating.

### Step 6: Re-submit to Codex (Rounds 2-5)

Resume the existing Codex session:

```bash
codex exec resume ${CODEX_SESSION_ID} \
  "I've revised the plan based on your feedback. The updated plan is in /tmp/claude-plan-${REVIEW_ID}.md.

Here's what I changed:
[List the specific changes made]

Here's what I intentionally did not change and why:
[List rejected findings with reasoning]

Please re-review. If the plan is now solid and ready to implement, end with: VERDICT: APPROVED
If more changes are needed, end with: VERDICT: REVISE" 2>&1 | tail -80
```

If `resume` fails, fall back to a fresh `codex exec` with prior round context.

Then go back to **Step 4**.

### Step 7: Write Decision Log & Present Results

**Write the decision log** to `/tmp/codex-decisions-${REVIEW_ID}.md` with this format:

```markdown
# Codex Review Decision Log

**Plan:** [brief plan title]
**Model:** gpt-5.4
**Rounds:** N
**Final verdict:** APPROVED / NOT APPROVED

## Round 1

### Finding 1: [short title]
- **Severity:** HIGH/MEDIUM/LOW
- **Codex said:** [verbatim summary]
- **Decision:** Accepted / Rejected / Partially accepted
- **Reasoning:** [why Claude made this call]
- **Action:** [what changed, or "no change"]

### Finding 2: ...

## Round 2
...

## Summary
- **Accepted:** N findings
- **Rejected:** N findings
- **Partially accepted:** N findings
```

**Then present to user:**

```
## Codex Review — Complete (model: gpt-5.4)

**Status:** ✅ Approved after N round(s) | ⚠️ Max rounds reached

**Decision summary:**
- Accepted N findings, rejected N, partially accepted N
- [1-line summary of most important accepted change]
- [1-line summary of most notable rejection, if any]

Full decision log: /tmp/codex-decisions-${REVIEW_ID}.md
```

**Then enter plan mode** so the user can review the final plan and approve/deny before implementation.

### Step 8: Cleanup

After the user approves or denies:

```bash
rm -f /tmp/claude-plan-${REVIEW_ID}.md /tmp/codex-review-${REVIEW_ID}.md
```

**Keep** `/tmp/codex-decisions-${REVIEW_ID}.md` — do not delete the decision log. The user may want to reference it.

## Loop Summary

```
Round 1: Claude sends plan → Codex reviews → REVISE?
Round 2: Claude autonomously triages + revises → Codex re-reviews → REVISE?
Round 3: Claude autonomously triages + revises → Codex re-reviews → APPROVED ✅
Final:   Write decision log → Enter plan mode → User approves
```

## Rules

- Claude **autonomously resolves all disagreements** — no AskUserQuestion during the review loop
- Claude **must record every decision** (accept/reject/partial) with reasoning — no silent changes or silent rejections
- The **decision log file is mandatory** — always write it before presenting results
- **Enter plan mode once at the end** so the user gets final approval over the revised plan
- Do NOT delete the decision log during cleanup
- Default model is `gpt-5.4`. Accept model override from user arguments
- Always use read-only sandbox mode
- Max 5 review rounds
- If Codex CLI is not installed or fails, inform the user and suggest `npm install -g @openai/codex`
- If a revision contradicts the user's explicit requirements, reject it and note it in the decision log
