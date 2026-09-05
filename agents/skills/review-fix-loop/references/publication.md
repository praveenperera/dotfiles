# Publication and PR Writes

Read this reference only when publication or PR writes are requested. Running the review-fix loop authorizes review, local fixes, and local verification; it does not authorize publication or PR mutations.

## Independent authorization

Record each permission during preflight. Do not combine permissions or infer one from another.

| Action | Required authorization |
| --- | --- |
| Create a commit | Explicit request to commit the identified local changes |
| Push | Explicit request to push the identified branch; commit permission does not imply push permission |
| Post a PR comment | Explicit request to post that comment; push permission does not imply comment permission |
| Add or remove a label | Explicit request naming or clearly selecting the label action; a comment request does not imply it |
| Resolve review threads | Explicit request to resolve threads; fixing a finding or posting a comment does not imply it |

Ask when the requested scope or timing is ambiguous. Authorization for a final push does not authorize an interim push for a hosted reviewer. Keep all authorized writes in the orchestrator; fix agents must not commit, push, resolve PR threads, apply labels, or post comments.

## Authorized writes

After local success, perform only the individually authorized actions. Follow repository commit instructions. A push requires an existing authorized commit containing the intended changes; otherwise request commit authorization. For a pushed result, poll required CI with a finite timeout. Resolve only threads whose findings are demonstrably addressed and only when thread resolution was authorized.

## Optional PR audit comment

Create this only when PR commenting is independently authorized. Keep it concise and include:

- local result and published/CI result as separate fields
- commit, branch, and PR identifiers when applicable
- enabled provider/model history and which runs saw the final code
- total fix passes used out of the single run budget, with effort and verification result
- optional review-gate results
- remaining issues or `none`

Do not claim that a label was applied, threads were resolved, code was pushed, or CI passed unless that exact action or state was verified.
