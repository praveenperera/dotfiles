# PR stacks

Read this when the run coordinates stacked PRs. [ship](../../ship/SKILL.md) covers the per-PR steps: watch-pr, Greptile, Sol, merge and deploy.

## READY bar

- Write the bar every PR must meet in one file before the first delegate starts, and link to it from the state file and from every brief. A PR counts as READY only when it meets every item. Track progress as items met over items planned.
- Typical items:
  - **Stacked:** each base is the PR below it, and each lower head is an ancestor of the branch above (`git merge-base --is-ancestor`)
  - **Complete:** the PR body is done, and no review threads are left unresolved
  - **Reviewed:** one fresh review, plus a delta review for every code commit after it
  - **CI:** passes on the current head SHA
- Use the repository's own bar file when it has one, and do not copy it into the state file.

## Ownership

- One delegate owns one stack, or one layer of a stack. Two delegates never write to the same branch.
- Before work starts, record any files shared across stacks in the state file, and say which delegate goes first.
- The root decides rebases and merge order. A fix that belongs in a lower PR goes there, and the root has the PRs above it rebased.
- Only the user merges unless the user says otherwise. Record the merge command and order in the state file for the user.
- Link every layer to the thread with `link_pull_requests`.
