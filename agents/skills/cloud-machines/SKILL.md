---
name: cloud-machines
description: Run CPU batch work on temporary cloud machines through homebased pools and cloudlease, and choose the provider and machine class by effective cost. Use when many independent CPU jobs cannot finish in time on Praveen's Fleet machines, when an agent considers renting cloud machines, or when choosing between DigitalOcean, phoenixNAP, Latitude.sh, and AWS spot. Do not use for GPU work (homebased GPU queue), for builds and tests that fit the Fleet's CI classes, for container image builds (rb), or for work the Fleet can finish by its deadline.
---

# Cloud machines

A homebased pool is a CI class whose machines cloudlease creates while the pool's jobs wait and deletes when they go idle, stall, or reach the pool's deadline. cloudlease alone holds the provider tokens and decides whether a machine exists, so the work is: decide that the Fleet is not enough, pick the cheapest safe class, get approval, create the pool, submit one job per unit of work, and confirm afterwards that every machine is gone.

The mechanics of pools are in the `homebased` skill's [pools.md](../homebased/references/pools.md). This skill adds when to use them, how to choose, and the limits that cost money.

## Requires

- homebased with `homebased pool` (from avencera/homebased#25). If `homebased pool list` is an unknown command, stop and say so.
- The pool coordinator: the daemon with `[pools]` and `[fleet.iroh]` config, `code` today. Run `pool` commands and submit pool jobs there; only it reaches pool machines.
- A machine class in the deployed cloudlease machines Worker that fits the run, and Worker caps that allow it. See [Classes and caps](references/providers.md#classes-and-caps). Adding a class or raising a cap is an operator change by the runbook, with its own approval.

## When to use

1. GPU work never goes here. Send it to the homebased GPU queue.
2. Builds and test suites go to the Fleet's CI classes (`homebased ci run --class rust|android|xcode|sim`), not to a pool.
3. CPU batch work runs on the Fleet when it can finish there by its deadline: `code` costs nothing. Estimate its wall time on `code` from one timed unit and the memory it can have (the `fleet` skill). For the afv-clean reference run, `code` needs about 22 hours while GPU jobs share it and about 11 hours without them.
4. Use a pool only when the Fleet cannot finish in time, or Praveen asks for one. Say which in the approval request.

## Ask Praveen first

Creating a pool creates billable machines. Before `pool create`, get Praveen's approval for: the class, `--max`, `--deadline`, `--spend-cap`, your effective-cost estimate, and the expected wall time. Skip the request only when he already approved this run with these limits or looser ones; a higher limit is a new request.

- In a root thread, ask in the turn's final message and end the turn.
- In a child thread, send the request to the parent with `homebased message send --machine <machine> --thread <parent thread>`, or report `blocked`; the root asks Praveen.
- In a worker, report `waiting` or `blocked` with the request in the notes.
- When the run is time-bound and Praveen may be away, also send one line with `homebased notify test --message "<request>"`. It is homebased's only ntfy send command today and adds the machine name.

Do not create anything while you wait. A Worker change (new class, higher caps, redeploy) also needs approval, and follows `~/code/private-workbench/ops/cloudlease/machines-deploy.md`.

## Choose the provider and class

Compare effective cost, not list price. Each provider rounds billing differently, so the same work can cost twice as much on more machines. The formula, the provider facts, and a worked example are in [providers.md](references/providers.md).

1. **Measure the work.** Time one typical unit on `code` or on one machine. Write down: number of jobs, work per job, peak disk and memory per job, data uploaded, and the deadline.
2. **List the candidates.** Use only classes in the deployed Worker config, and stay within its caps: machines per run, hours per run, the class's `max_hours`, dollars per run, and dollars per hour across runs.
3. **Check the account limits and stock.** The tables in providers.md show each provider's account limit and the stock last seen. A class with no stock fails its create, and that failure bills nothing on phoenixNAP or Latitude.sh, but the pool stops after three failed creates in a row.
4. **Cost each candidate.** For each machine, billed time = boot + work + idle tail, rounded to the provider's billing increment, with its minimum. Add egress. On hourly providers, choose the machine count so each machine's work fills its paid hours.
5. **Pick** the cheapest candidate that meets the deadline within the limits. If two are within about 20%, take the per-second provider: a retry or a failed machine costs less there.
6. **Size the caps** from the estimate (see the flags below), then ask for approval.

AWS spot is not available yet; providers.md says what it needs.

## Run

On the coordinator:

```sh
homebased --json pool create afv --machine-class do-c16 --max 16 --deadline 5h \
  --spend-cap 18 --min-free-disk 80 --idle-limit 3m --env-file ~/.config/afv/pool.env
homebased --json ci run --class afv --job-id "$(uuidgen)" --git --kill-after 3h -- ./clean_scroll.sh PHerc-0172
homebased --json pool show afv
```

| Flag | How to set it |
| --- | --- |
| `--machine-class` | The class you chose, as named in the Worker config |
| `--max` | Most machines at once, 1 to 64. From the cost step; never above the account limit or the Worker's machines-per-run cap |
| `--deadline` | Expected wall time + boot grace + margin. At most 24h, and the Worker refuses one past its hours-per-run cap. No machine outlives its class's `max_hours` either |
| `--spend-cap` | US dollars, about 1.25x the estimate. On an hourly provider, at least `--max` x paid hours x price, since each hour bills when it starts; cloudlease can also overshoot by up to one hour per hourly machine. A value above the Worker's per-run cap ends the pool with `invalid_request` |
| `--slots` | Jobs per machine (default 1). Fit them to the machine's cores, memory, and disk |
| `--idle-limit` | How long a machine without work waits before it releases itself (default 10m). On a per-second provider idle time is paid, so use 2m to 3m once every job is submitted. On an hourly provider the rest of the started hour is paid either way, so keep the default and plan the jobs to fill the hours |
| `--stall-after` | Longer than the longest silence of a healthy job (default 30m). A job that writes no output for this long loses its machine about 3 minutes later. Make jobs print progress |
| `--boot-timeout` | Leave it unset. The default is one minute before the class's boot grace ends (7m on DigitalOcean, 19m on phoenixNAP). Never set it below the provider's real boot time (providers.md). A value past the grace changes nothing, because cloudlease deletes a machine that has not joined when the grace ends |
| `--min-free-disk` | The peak disk of one job, in GiB, within what the class has free after boot (default 1) |
| `--env-file` | `NAME=value` secrets, such as an upload token, in a 0600 file on the coordinator. Never put a secret on a job's command line or in the repository |

Jobs:

- Submit one `ci run --class <pool>` per unit of work, all at once. The coordinator asks for machines while jobs wait, up to `--max`. Do not write a dispatcher.
- A new machine is a fresh Ubuntu with only homebased on it. Run with `--git` from a pushed commit (or `--repo` with `--commit`), and install what the job needs in the job or in a setup script it calls.
- Set `--kill-after` to the longest job plus a margin. A job runs only on a machine that will outlive its `--kill-after`, so it must fit in the deadline minus the boot timeout, and in the class's `max_hours` minus boot. Otherwise the job is refused with `pool_deadline_too_close`.
- **A job must upload its outputs, and check the upload, before it exits.** A deleted machine takes its local files and the job's log with it. Make jobs safe to rerun: a job whose machine was deleted ends `JOB_FAILED` with `machine_released`, and you resubmit it.
- Wait with `homebased wait --next <job ids>` for runs under an hour. A longer run is long work: run it from a child thread that sleeps with `homebased wait --long <job ids>` (the `use-agents` skill).
- When every job has its final event, run `homebased --json pool release <pool>`. That saves the idle tail. Never release while jobs run: release deletes every machine at once.

## What cloudlease enforces

Do not build your own watchdog, timer, or cleanup loop. These hold even if the coordinator, the agent, or the machine dies:

- **Heartbeat.** A machine renews its lease every 30 seconds, but only while it is useful: a job wrote output within `--stall-after`, jobs wait for it, or it owes events. cloudlease deletes a machine that has not joined when its boot grace ends, and one that misses renewals for 3 minutes.
- **Idle release.** After `--idle-limit` without work, the machine releases itself.
- **Hard deadline.** At the earlier of the pool's deadline and the class's `max_hours`, cloudlease deletes the machine, also through a ceiling tag on the resource.
- **Sweep.** Every 5 minutes, a cron sweep deletes any resource with the Worker's tag that no live lease claims.
- **Caps.** Machines per run, the per-run spend cap, and an hourly spend cap across runs. A create that would pass a cap is refused, and a run that reaches its spend cap releases its machines and alerts Praveen.

Never hold, ask for, or read a provider token or the cloudlease client token. Never create a cloud machine outside cloudlease (no `doctl`, provider APIs, or consoles), and never keep one alive from outside its lease. Never send plain tasks to a pool machine with `task run --machine`: the coordinator does not end them when the machine goes.

## Confirm that nothing is left

After the run, `homebased --json pool show <pool>` must show the pool `released` and every machine `released`, which cloudlease marks only after the provider confirms every resource absent. If a machine stays `releasing` for more than about 10 minutes, or the pool does not reach `released`, tell Praveen. The operator checks in the runbook's "Watching it" section list every provider resource with the Worker's tag; you do not run them.

When a pool stops or ends with a code (`spend_cap_reached`, `run_ended`, `create_failed`, `deadline_too_far`, `unknown_class`, `invalid_request`), tell Praveen the code. These need a budget, stock, or Worker change, not a retry. [pools.md](../homebased/references/pools.md) explains each one.

## Where the details live

- [providers.md](references/providers.md): effective cost, provider facts, classes and caps, unconfirmed numbers, and the worked example
- homebased `docs/temporary-machines.md` and the `homebased` skill's [pools.md](../homebased/references/pools.md): pool lifecycle, flags, error codes
- `~/code/research/dev-infra/cloudlease/README.md`: the cloudlease design, live test results, and the temporary machines contract
- `~/code/research/dev-infra/batch-compute-providers/README.md`: provider research, the speed model, and the afv-clean reference run
- `~/code/private-workbench/ops/cloudlease/machines-deploy.md`: deploying the Worker, classes, caps, and operator checks
- `apps/machines/wrangler.jsonc` in `~/code/cloudlease`: the Worker's classes and caps
