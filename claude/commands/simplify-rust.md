---
description: Simplify and refine Rust code for clarity and maintainability
argument-hint: [file-path or empty for recent changes]
model: opus
---

Invoke the rust-code-simplifier agent to analyze and simplify Rust code.

If a file path is provided ($ARGUMENTS), focus on that file.
Otherwise, focus on recently modified Rust files (use git diff to identify them).

Use the Task tool to launch the rust-code-simplifier agent with the appropriate scope.
