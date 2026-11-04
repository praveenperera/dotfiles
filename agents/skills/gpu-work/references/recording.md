# Recording decisions, results and lessons

Record as you go: at the end of each phase, after a revert or a parked attempt, after the user corrects you, and before the task ends. Compaction loses anything unrecorded. `R` = `~/code/research/gpu-kernels/`.

## Decision log

Every project keeps one decision log at `R/<topic>/decisions.md`. If the project has no topic yet, create one following `~/code/research/CLAUDE.md` and link the log from the topic README. `_scratch/` is wiped weekly, so it can't hold the log.

Add an entry when you choose between designs, keep, revert or park a change, kill an idea, change the plan, hit a trap that cost time, or get corrected by the user. A hill climb's `_scratch/<task>/LOG.md` stays the per-experiment notebook; copy each keep, kill or plan decision into the decision log when you make it. Newest entries go at the bottom, times in US Central:

```
## 2026-10-31 14:05 CT · Rewrite the attention tile instead of tuning it

- Why now: gap table row 2, 1.9x from the bandwidth floor
- Chose: one-pass fused tile over a launch-config sweep and split-K; sweep is capped at 2% by the ablation ceiling
- Evidence: ceiling 0.41 ms against 0.78 ms today on the RTX 5090, gpu0 job 3f2a91c0, commit abc1234
- Outcome: kept, 1.6x on batch 1-16; not measured on Ada
- Skill: rule (Track A) · "Fuse the tile before sweeping launch configs when the ceiling says ..."
```

- Fill in **Outcome** when it's known, and edit it if a later result changes it. Give every kill its scope (what, which shapes, which commit).
- **Skill** is `none`, `rule` (changes how the next project, or the next project of a track, is done), or `gotcha` (a trap in cuda-oxide, a library, a profiler or the queue), with a one-line proposed wording. Mark it whenever you're unsure; the user filters these.
- Commit and push the log with the topic's other notes as you go.

## Where each finding goes

1. **Decisions:** the decision log.
2. **Results** (numbers, tables, what shipped, what was rejected): the project's topic, such as `R/scriptrs-cuda/<subject>-<date>.md`, linked from the topic README. When the conclusion changes, update the README's status, date and summary.
3. **Transferable lessons with their evidence:** `R/cuda-kernels/cuda-kernel-lessons-general-2026-10-04.md`, in your project's section or a new `## N. Lessons from <project> (<dates>)`, under the existing bold sub-heads where they fit.
4. **Rules and gotchas:** the decision log's Skill lines. Never edit this skill during a task; the user decides what goes into it.
5. **Scripts, runbooks, brief templates and handoffs:** `~/code/private-workbench`; repo-specific commands, gates and rejected ideas go in its `ops/<repo>/hill-climb/README.md`.

## How to write a lesson

- Lead with the rule in bold, in the imperative: "**Bound per-shape caches by bytes.**"
- Follow with the evidence: before and after numbers, GPU, and a commit hash, PR or `path:line`. Copy the numbers in; a scratch path can't be the only evidence.
- Record negative results: what didn't pay, by how much, and why.
- Mark anything inferred or unverified, and say where you looked.
- Only quote timings from an idle-GPU queue run at the commit you name.

Before adding, grep for the key terms (`rg -i '<term>' ~/code/research ~/.claude/skills/gpu-work`) and extend an existing bullet rather than writing a near-duplicate. If a new result contradicts a lesson, edit the old bullet to say what's now true and when it changed. If a later change moves numbers you quoted, update every note that quotes them.

No raw logs, profiler output or task lists in the research repo; summarize them in Markdown. Commit and push; its pre-commit hook regenerates `INDEX.md` and checks the layout.
