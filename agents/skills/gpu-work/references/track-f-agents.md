# Track F: agent-run kernel work

Running workers, advisors or reviewers on kernel tasks. Shorthands as in SKILL.md.

- The root reads this skill and copies the rules that apply into `PW/playbooks/kernel-worker-brief-template.md`, so workers get a bounded brief with stop rules, attempt caps and known risks, and the ambitious design to build first.
- The root owns the decision log. Workers report decisions and rule candidates; the root logs them, and the user decides what goes into the skill.
- Sol has been thorough and conservative; Opus reverse-engineers and takes bolder numerical routes that need audits. Run both on different routes for hard problems. Give advisors the same blind baseline and score them against measurements; use xhigh for Sol, and for Opus only when it can read raw traces.
- Workers fix their own scratch bugs and rerun without asking. Diff every worker change against its brief, and integrate with every dependency on day one, against the real gate.
- A delegate that queues a GPU job submits it, reports waiting and exits; the job's events resume it, even after it reported done. Don't edit its files meanwhile; route follow-ups through it. In LivePortrait the root and a delegate rewrote the same script within a minute.
- Give each session that builds and times GPU binaries its own worktree. Two sam3rs sessions in one tree froze binaries holding each other's half-finished edits. Before freezing, check that every file changed since the last freeze is yours, or snapshot the source you built. When workers each regenerate shared PTX, merge their kernel sources and rebuild once, then diff that only their kernels changed.

Read: lessons §4, §5 and §6 ("Workers and models"), `R/cuda-kernels/speakrs-cuda-advisor-comparison-2026-10-08.md` §8, `~/code/research/agents/agent-models/README.md`.
