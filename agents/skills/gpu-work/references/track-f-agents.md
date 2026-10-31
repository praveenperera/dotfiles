# Track F: agent-run kernel work

Running workers, advisors or reviewers on kernel tasks. `R` = `~/code/research/gpu-kernels/`, `PW` = `~/code/private-workbench/`.

- Use `PW/playbooks/kernel-worker-brief-template.md`. It bakes in stop rules, attempt caps and known risks.
- The root reads this skill and copies the rules that apply into the brief, so workers get a bounded brief instead of the whole skill. Name the ambitious design the worker should build first.
- The root owns the decision log and the skill. Workers report their decisions and rule candidates; the root logs them and decides what goes into the skill.
- Codex workers can't be messaged mid-run; Claude workers can. Route time-sensitive findings to Claude workers.
- So far (one tier run plus the advisor comparison), Sol has been thorough and conservative, while Opus reverse-engineers and takes bolder numerical routes that need audits. Run both on different routes for hard problems.
- Workers fix their own scratch bugs and rerun without asking. Diff every worker change against its brief.
- Give advisors the same blind baseline, and score them against measurements. Use xhigh for Sol, and for Opus only when it can read raw traces.
- Integrate with every dependency on day one, against the real gate.
- Run Claude subagents that submit GPU queue jobs in the background. A job's callback arrives in the root session as a user turn and interrupts any foreground tool call there, including a foreground subagent, which then stops mid-task (twice in the cuda-oxide gotcha validation, 2026-10-31). The subagent waits by polling `homebased resource job show`, not with a callback.

Read: lessons §4, §5 and §6 ("Workers and models"), `R/cuda-kernels/speakrs-cuda-advisor-comparison-2026-10-08.md` §8, `~/code/research/agents/agent-models/README.md`.
