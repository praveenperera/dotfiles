---
name: maintain-verification-skill
description: Audit a project-local application verification skill against current source and live behavior, then repair its instructions, harness, or feature map. Use when a verify skill has drifted, misses features, or no longer drives the application reliably. Do not fix product defects as part of this maintenance pass.
---

# Maintain a verification skill

Keep the project's `.agents/skills/verify-*` instructions and feature map accurate.

## Choose the target

Find the project-local skill that contains launch, doctor, drive, evidence, cleanup, and a feature map. If several match and the user did not name one, ask which application is in scope. If none match, use `create-verification-skill`.

Only edit the selected verification skill. Report product failures separately.

## Audit the source

Read the feature index and each feature file. For every mapped feature:

- trace its current source entry points
- verify its route, selector, command, or protocol input
- check prerequisites and observable results
- identify user-visible features that the map omits

Use a source path before adding a missing feature. Fix dead, duplicate, or misleading index entries.

## Drive every feature

Follow the verification skill's own isolation and launch model. Use one healthy long-lived instance for a UI or service when safe. Use a fresh isolated session for a short-lived CLI.

Run doctor before the first drive and after surprising behavior. Reset or relaunch a wedged instance instead of continuing from unknown state. Keep evidence from completed drives under `_scratch/verification/`, and remove only the processes and temporary state that this run created.

Classify each result:

- **Verified:** the documented path and result match the live application
- **Skill drift:** the application works but the instructions, harness, or map are wrong
- **Product defect:** the documented user behavior fails in the application
- **Unreachable:** a named prerequisite prevents the drive

Fix skill drift and drive the corrected path again. Report product defects without changing product code or rewriting the map to hide them.

## Finish with one outcome

- **Clean:** every mapped feature received source and live coverage, with no skill changes
- **Changed:** validated corrections were made to the verification skill
- **Blocked:** coverage could not finish, with the exact unmet prerequisite or failure

Validate changed skill files with `skill-creator`. Return feature coverage, evidence paths, changes, product defects, and the final outcome.
