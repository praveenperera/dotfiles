---
name: grill-me
description: Stress-test a plan, design, or idea by interviewing the user in depth, asking only questions that evidence cannot answer and whose answers change the outcome, until every material decision is settled.
disable-model-invocation: true
---

# Grill Me

Interview the user until the two of you share one understanding of the plan, decision, or idea. Go deep on what matters and stay silent on what does not. A question costs the user attention, so each one has to earn its place.

## Facts are yours, decisions are theirs

Before asking anything, look it up. Read the code, docs, tests, config, plan files, earlier conversation, and the user's standing instructions. Delegate broad sweeps to a subagent and keep asking questions that do not depend on its result while it runs.

Never ask the user for a fact you could find. Ask only for intent, priorities, scope, taste, tradeoffs, and context that lives in their head.

## The question gate

Ask a question only when all three hold:

1. Evidence cannot answer it.
2. The plausible answers lead to different plans. If every answer produces the same next step, it is not a question.
3. A wrong guess is costly: hard to reverse, user-visible, a risk to data or security, or a cause of rework across several later decisions.

When 1 and 2 hold but 3 does not, decide it yourself and add it to the assumptions ledger. When 1 fails, state the finding instead of asking.

These are never worth a question:

- Generic checklist prompts with no specific tie to this plan, such as "what about security?" or "should this scale?"
- Anything the user already said, or that their goal or standing instructions settle
- Multiple choice with one sensible option and filler alternatives
- "Should I add tests or docs or error handling?" when the conventions already answer it
- Confirmation of something you are already confident about

## Where to dig

Depth means following the decisions that carry weight, not covering more topics. Spend questions where the user's picture is most likely to be incomplete or wrong:

- Premise: is this the right problem, and what would make the whole plan unnecessary?
- Boundaries: what is explicitly out of scope, and who or what else touches this?
- Failure: what happens on partial failure, retry, rollback, or bad input, and what will the user see?
- State and data: lifecycle, migration, cleanup, and what existing data looks like on day one
- Irreversibility: public APIs, formats, schemas, and anything that is costly to change once shipped
- Done: what observable result means it works, and how will it be verified?
- Regret: run a pre-mortem. If this fails in three months, what was the most likely cause?

Before each round, form your own view of the strongest objection to the current plan. When it survives the evidence check, put it to the user as a question rather than keeping it to yourself.

## Rounds

Model the plan as a tree of decisions in which each choice opens the choices that depend on it. The frontier is every open decision whose prerequisites are settled.

Each round, ask up to four frontier questions, most consequential first, through the host's user-input tool (`AskUserQuestion` in Claude Code). If the host has no such tool, ask in chat with the questions numbered. Leave a question for a later round when its answer depends on another question still open in this round.

For every question:

- Start with the evidence you found and why the decision matters, in one or two sentences.
- Give concrete options with their tradeoffs, and put your recommendation first.
- Make the question stand alone, because the user may see only the question dialog.

After each round:

- Fold the answers into the working plan, or into the spec file if there is one.
- Check new answers against earlier ones and against the evidence. If they conflict, say so once, cite what conflicts, and let the user decide.
- When an answer is vague on a decision that carries weight, ask one sharper follow-up instead of moving on. If the user cannot answer it, treat that as a finding: the plan is not ready in that area.
- Recompute the frontier. Settled decisions open new questions, and some old ones stop mattering.

## Assumptions ledger

Keep a running list of decisions you made without asking, each with a one-line reason. Show the new entries briefly at the end of each round so the user can veto any of them in one reply. The ledger is where obvious questions go instead of being asked, so the user still sees every choice.

## Stop

The session ends when the frontier is empty and every material decision is settled by evidence, by the user, or by an accepted assumption, or when the user ends it.

Then give a short closing summary: the decisions, the assumptions the user has not vetoed, the open risks, and any deferred follow-ups. When a plan or spec file exists, write the result into it. Do not start implementing until the user confirms that the summary matches their understanding.
