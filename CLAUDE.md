# Dotfiles

## Committing

- Commit with `cmd git commit-paths <paths> -m "<subject>"` from the checkout you changed, right after the change. It commits only those paths, rebases and pushes without touching other threads' uncommitted files. Other threads keep uncommitted work in this tree, so never reset, clean, stash, pull or check out over it.
- Any agent may commit any uncommitted change here, including files another thread left behind. Don't ask whose they are or leave them for their thread; commit and push them. Run `just fmt` and `just clippy` first when the change touches `cmd/`.
- Only files that don't belong in git stay out: secrets, large files, archives, binaries and build output. Add a `.gitignore` pattern for them in the same commit so the next agent doesn't trip on them.
- If `commit-paths` stops with the commit made but not pushed, resolve it yourself instead of reporting it: rebase in a separate worktree, keep both sides' changes, take the newer version where they truly conflict, and push.
