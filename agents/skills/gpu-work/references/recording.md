# Recording decisions, results and lessons

Record as you go: at the end of each phase, after a revert or a parked attempt, after the user corrects you, and before the task ends. Long sessions get compacted, and unrecorded findings are lost. `R` = `~/code/research/gpu-kernels/`.

## Decision log

Every project keeps one decision log in its research topic, `R/<topic>/decisions.md`. If the project has no topic yet, create one following `~/code/research/CLAUDE.md` and link the log from the topic README. The log exists so the next session, the final write-up and the skill update can see why the work went the way it did; `_scratch/` is wiped weekly, so it can't hold this.

Add an entry when you choose between designs, keep, revert or park a change, kill an idea, change the plan, hit a trap that cost time, or get corrected by the user. A hill climb's `_scratch/<task>/LOG.md` stays the per-experiment notebook; copy each keep, kill or plan decision from it into the decision log when you make it. Newest entries go at the bottom, times in US Central:

```
## 2026-10-31 14:05 CT · Rewrite the attention tile instead of tuning it

- Why now: gap table row 2, 1.9x from the bandwidth floor
- Chose: one-pass fused tile over a launch-config sweep and split-K; sweep is capped at 2% by the ablation ceiling
- Evidence: ceiling 0.41 ms against 0.78 ms today on the RTX 5090, gpu0 job 3f2a91c0, commit abc1234
- Outcome: kept, 1.6x on batch 1-16; not measured on Ada
- Skill: rule (Track A) · "Fuse the tile before sweeping launch configs when the ceiling says ..."
```

- Fill in **Outcome** when it's known, and edit it if a later result changes it. Give every kill its scope (what, which shapes, which commit), so a later reader doesn't widen it.
- **Skill** is `none`, `rule` (would change how the next project, or the next project of a track, is done), or `gotcha` (a trap in cuda-oxide, a library, a profiler or the queue), with a one-line proposed wording. Mark it whenever you're unsure; the end-of-task pass filters.
- Workers with a brief don't write to the log. They list decisions and candidates in their report, and the root copies them in.
- Commit and push the log with the topic's other notes as you go.

## Where each finding goes

1. **Decisions** go in the decision log, as above.
2. **Results** (numbers, tables, what shipped, what was rejected) go in the project's topic, such as `R/scriptrs-cuda/<subject>-<date>.md`, linked from the topic README. When the conclusion changes, update the README's status, date and summary.
3. **Transferable lessons with their evidence** go in the general lessons file `R/cuda-kernels/cuda-kernel-lessons-general-2026-10-04.md`. Add to your project's section, or start a new numbered section, `## N. Lessons from <project> (<dates>)`, with a one-line source note. Group bullets under the existing bold sub-heads where they fit (kernel design, precision, parity, measurement, build, agents).
4. **Rules and gotchas** for every project, or for every project of a track, go in this skill, as in "Update this skill" below. The skill holds the rule; the numbers stay in research.
5. **Scripts, runbooks, brief templates and handoffs** go in `~/code/private-workbench`, and repo-specific commands, gates and rejected ideas go in the repo sheet `ops/<repo>/hill-climb/README.md` there. A reusable analysis script that isn't tied to one repo can go in this skill's `scripts/`.

## How to write a lesson

- Lead with the rule in bold, in the imperative: "**Bound per-shape caches by bytes.**"
- Follow with the evidence: before and after numbers, GPU, and a commit hash, PR or `path:line`. Copy the numbers in: `_scratch/` directories are wiped weekly, so a scratch path can't be the only evidence.
- Record negative results: what didn't pay, by how much, and why. They save the most time.
- Mark anything inferred or unverified, and say where you looked.
- Only quote timings from an idle-GPU queue run at the commit you name.

Before adding, grep for the key terms (`rg -i '<term>' ~/code/research ~/.claude/skills/gpu-work`) and extend an existing bullet rather than writing a near-duplicate. If a new result contradicts an existing lesson, edit the old bullet to say what's now true and when it changed; don't leave both standing. If a later change moves numbers you quoted, update every note that quotes them, including topic summaries.

No raw logs, profiler output or task lists in the research repo; summarize them in Markdown. Commit and push; its pre-commit hook regenerates `INDEX.md` and checks the layout.

## Update this skill

Before finishing, the root goes through the decision log entries marked `rule` or `gotcha` and the lessons added during the task, and folds what generalizes into this skill:

- **Placement:** a rule for every project goes in SKILL.md "Be ambitious", "Every project" or "Kernels"; a rule for one kind of project goes in that track's `references/track-*.md`; a cuda-oxide trap or per-build check goes in `references/cuda-oxide.md` with the commit it was seen at.
- **Keep it short:** merge into or replace an existing bullet rather than adding one. If the new rule contradicts an existing one, replace the old one and say in the decision log what changed. Leave out anything specific to one repo; that belongs in its sheet.
- **Wording:** the rule in the imperative, one clause of evidence (the number that justifies it), and a pointer to the lessons section or research note that holds the rest.
- **Where to edit:** `~/code/dotfiles/agents/skills/gpu-work/` (`~/.claude/skills/gpu-work` and `~/.codex/skills/gpu-work` link there). Commit only the skill's files, staged by hunk, in their own commit, and push.
- **Close the loop:** set each processed entry's Skill line to `added`, `merged into <section>` or `rejected: <reason>`, so the next pass doesn't redo it.

If nothing generalizes, say so in the final report rather than adding a weak rule.
