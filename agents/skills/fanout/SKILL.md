---
name: fanout
description: Launch one new T3 thread per item, each in its own new worktree, with a prompt written as if the user sent it. Use when the user invokes $fanout or /fanout.
disable-model-invocation: true
---

# Fanout

Start one top-level T3 thread for each item in the list the user gave, or in the list
under discussion. Give each thread a new worktree by setting `workspaceStrategy`.
Do not create worktrees in the shell.

Write each prompt in the user's voice, as if they typed it themselves:

- Cover only that item's scope: the problem, where it shows up, the evidence or
  measurements so far, and what "done" looks like.
- Do not mention yourself, this thread, delegation, or reporting back.
- Do not mention the other items or threads.
- If the item likely needs GPU time, such as training, inference benchmarks, or CUDA
  builds, say "use homebased for anything that needs the GPU".
- Add other standing instructions only if the user asked for them.

Then stop. Don't monitor, poll, or message the threads. Reply with each thread's link
and a one-line scope for each.
