# Providers, classes, and effective cost

Facts checked on 2026-11-24 unless a row says otherwise. Each one names its source. A number that nobody has measured on our machines is listed under [Unconfirmed](#unconfirmed); update that list, and the tables, when a measurement comes in.

## Effective cost

List price is not cost. What a run costs depends on how each machine's time is billed:

```text
work per machine   = total work / speed / machines
billed per machine = boot (if billed) + work per machine + idle tail,
                     rounded up to the billing increment, at least the minimum
run cost           = machines x billed per machine x price
                     + extras (IP blocks, volumes) + GB uploaded x egress price
```

- **Total work** is in DO `c-16`-hours, the unit of the speed model in the batch-compute-providers README. Measure it from one timed unit of the real job when you can.
- **Speed** is the work one machine does per hour relative to a `c-16`. Every bare-metal speed below is an estimate from benchmarks.
- **Boot** is from the create call until the machine joins the pool. It is paid where billing starts at create.
- **Idle tail** is the time after a machine's last job until it is gone: `--idle-limit`, plus the delete: about a second (under a second on Latitude.sh [10]), 31 s to gone on AWS [11], and about 2.5 minutes on phoenixNAP before the server leaves the list [9]. Count it before rounding. On an hourly provider, a machine whose work ends within `--idle-limit` of an hour boundary pays one more hour, so keep the idle limit short there too. Releasing the pool the moment the last job ends removes the tail only for the machines that finished last.
- **Machine count on an hourly provider.** A machine with 25 minutes of work pays a full hour. Choose the fewest machines that meet the deadline, about total machine-hours divided by the whole hours each machine may run, and submit all jobs at once so each machine takes the next job without a gap. Spread over many machines, hourly rounding takes most of the saving: in the worked example, 16 phoenixNAP machines cost 1.8 times as much as 3.

## Providers

| | DigitalOcean | phoenixNAP Bare Metal Cloud | Latitude.sh |
| --- | --- | --- | --- |
| In the Worker | Yes, deployed (Phase 2E pilot) | Yes, deployed and live-tested in Phase 4c: `pnap-s2-large` and `pnap-s2-small`, under test caps [9][12] | Yes, deployed and live-tested in Phase 4d: `lat-m4-medium`. The caps deployed since Phase 4f (48 cents an hour) are below its $1.25 hour, so it cannot create a machine until they are raised [10][11][12] |
| Billing | Per second, minimum 60 s or $0.01 [1] | Every started hour in full, minimum 1 hour, charged when the hour starts. The hours run from `provisionedOn`, about 12 minutes after the create. Billing stops at deprovision by the docs; the usage record cannot show it [2][9] | Every started hour in full: a server that lived 3 minutes was charged one hour, $1.25. The line started 52 s after the create. Whether the second hour starts 60 minutes after that line's start is unconfirmed; cloudlease counts each started hour from its own `created_at`, the safe side [10] |
| Boot billed | Yes: from create; cloudlease counts spend from `created_at` [1] | Mostly no: the charge starts at `provisionedOn`, 11m40s after the create on `s2.c2.large`, so provisioning is free. Only the last 1.5 minutes or so, from `provisionedOn` to joined, is billed [9] | On the instant path it cannot be told: the server was `on` within 3 s and the billing line started 52 s after the create, both inside the first billed hour. Unconfirmed outside `deploys_instantly` [10] |
| Boot to joined | 2m12s to 2m17s, measured on `s-1vcpu-1gb` in `nyc3` [4] | 13m08s, measured on `s2.c2.large` in SEA: 11m40s of provisioning, the OS up 71 s later after the cloud-init reboot, joined 17 s after that [9]. Docs: 1 to 15 minutes by OS [2] | 6.0 s, measured on `m4.metal.medium` in CHI with `ubuntu_24_04_x64_lts`: an instant deploy hands over a server that is already running [10]. Docs: about 10 minutes outside `deploys_instantly`. Not measured [3] |
| Boot grace | 8 min; default `--boot-timeout` 7m [5] | 20 min, kept after Phase 4c [9]; default `--boot-timeout` 19m [5] | 15 min, kept after Phase 4d [10]; default `--boot-timeout` 14m [3][5] |
| Account limit | At least 17 droplets: the reference run used 17 [6] | 1 server (`bmc.servers.max_count`); Praveen has asked phoenixNAP to raise it [2][9] | 2 bare-metal servers per team (`limits.bare_metal`); $100 an hour of spend for a new account. The Worker shares the account's only project [3][10] |
| Egress | 7,000 GiB pooled, then $0.01/GiB [6] | 15 TB a month per location included [6][9] | 20 TB per server, then $0.01/GB [3] |
| Extras | Volumes bill hourly, attached or not; prefer local disk [1] | /31 IP block, $0.007 an hour ($0.0035 per IP), from `provisionedOn`; it was charged $0.01 for the test hour and is released at once on delete. The first server in a location also creates a location-default private network that stays, untagged, with no usage line [2][9] | None: the test hour had no bandwidth or IP line [10] |

Classes and prices:

| Provider | Type | $/h | CPU, RAM, disk | Speed vs `c-16` | Stock on 2026-11-24 |
| --- | --- | --- | --- | --- | --- |
| DigitalOcean | `c-16` | 0.50 [6] | 16 vCPU (8 cores + SMT), 32 GB, 200 GiB SSD | 1.0 (the unit) | Not checked |
| phoenixNAP | `s3.c3.medium` | 0.47 [2] | i9-14900K, 128 GB, 2 x 2 TB NVMe | About 4.0, estimate [6] | None in any location all day [2][9] |
| phoenixNAP | `s2.c2.large` | 0.40, 0.407 with its IP block [2][10] | E-2388G 8 cores (16 threads), 128 GB, 2 x 1 TB NVMe [9] | About 1.6, estimate from single-thread PassMark [9] | 1 in NLD, 0 in SEA (3:55 PM CT), then SEA had one at 4:16 PM CT; moves within hours [7][9] |
| phoenixNAP | `s2.c2.small` | 0.31, 0.317 with its IP block [2][10] | E-2388G 8 cores, 64 GB, one NVMe | Unconfirmed | 2 in SEA [7] |
| Latitude.sh | `m4.metal.medium` | 1.25 (US) [3][10] | EPYC 9124 16 cores, 128 GB, 2 x 480 GB + 2 x 1.9 TB NVMe | About 2.8, estimate [6] | 1 each in DAL, CHI, MIA2, SJC2 [3]; by 5:09 PM CT only MIA2 and SJC2 in the US. Moves within minutes [10] |
| Latitude.sh | `f4.metal.medium` | 1.52 (US) [3] | EPYC 4564P 16 cores, 128 GB, same disks | About 4.4, estimate [6] | None in the US or EU [3] |

Latitude.sh's pricing page shows the monthly price divided by 730; the API's hourly price is about twice that. Use the API price [3].

Stock is a live fact. A stock-out on phoenixNAP or Latitude.sh fails the create and bills nothing, and a class can list several locations to try in order. Fall-through worked live on Latitude.sh (DAL to CHI) [10] and on AWS (zone 1f to 1b) [11]; on phoenixNAP it is not exercised [9]. The pool stops after three failed creates in a row (`create_failed`) [8].

## Classes and caps

The Worker's classes and caps are in `apps/machines/wrangler.jsonc` on cloudlease master. A pool may name only a class listed there under `apps.homebased.classes`. On master 7ad650c, the same config Phase 4f deployed as Worker version `ff36628e` [11][12], they are the caps of the live provider tests, too small for a real run:

| Setting | Value |
| --- | --- |
| Classes | `do-s1-1g` (1 vCPU, 1 GB, $0.00893/h), `pnap-s2-large` ($0.407/h with its IP block, SEA then NLD), `pnap-s2-small` ($0.317/h, SEA then NLD), `lat-m4-medium` ($1.25/h, DAL, CHI then SJC2), `aws-c8a-4xl` (`c8a.4xlarge` then `c7a.4xlarge` in us-east-1 zones 1f, 1b, 1d, 50 GiB root, Spot max $0.45, $0.48/h); each `max_hours` 1 |
| `max_machines_per_run` | 1 |
| `max_run_hours` | 1 |
| `run_spend_cap_usd` | 0.48 |
| `HOURLY_SPEND_CAP_CENTS` | 48, across every run: one `aws-c8a-4xl` or `pnap-s2-large` hour, not a `lat-m4-medium` one |
| `heartbeat_ttl_secs` | 180 |

No `do-c16` class exists yet. The deployed version can differ from master: the runbook and the cloudlease README record each deploy. For a real run, the request to Praveen also names the Worker change it needs: the class, `max_machines_per_run` of at least `--max`, `max_run_hours` and the class's `max_hours` of at least the deadline, `run_spend_cap_usd` of at least `--spend-cap`, and `HOURLY_SPEND_CAP_CENTS` of at least `--max` x the hourly price, in cents.

A homebased pool machine needs the job's disk free after boot (`--min-free-disk`). On phoenixNAP `s2.c2.large` the root filesystem is LVM on one NVMe, 916 GB, enough for one scroll, and the second 1 TB drive is unused [9]. Whether a job can use it, and how Latitude.sh lays out its four drives, is unconfirmed. On `aws-c8a-4xl` the root is 48 GB, from the class's 50 GiB volume [11].

## AWS spot

AWS spot suits runs that upload little: about a third of DigitalOcean's cost per unit of work, but egress costs $0.09/GB after the account's free 100 GB a month, which is shared by the whole AWS account [6]. It beats the included-bandwidth hosts below about 1 GB uploaded per `c-16`-hour of work, and loses to them above about 3 GB [6]. Inbound is free, and S3 reads in us-east-1 (the Vesuvius open-data bucket) are free in-region [6]. Prices: `c7a.4xlarge` $0.329/h, `c8a.4xlarge` $0.366/h, plus about $0.021/h of EBS and IPv4, per second with a 60 s minimum [6]. In the Phase 4f test, `c8a.4xlarge` cost $0.3721/h in zone 1b, unchanged from 2:00 PM CT through the test; 1f, the cheapest at $0.29, had no capacity for either create [11].

Measured in Phase 4f, two `c8a.4xlarge` machines in us-east-1b [11]:

- **In the Worker.** Class `aws-c8a-4xl`, deployed under the test caps in [Classes and caps](#classes-and-caps).
- **Boot to joined.** 18.5 s and 19.0 s from the create. Boot grace is 8 minutes.
- **Release to gone.** 31 s; billing stops at `shutting-down`.
- **Billing.** About $0.035 for both, computed from instance times (197 s and 129 s) at the Spot price. AWS's own billing lines had not appeared yet. The Worker counted $0.058 from create to `gone_at` at the class price, the safe side.
- **Interruption.** An instance terminated outside the Worker ended its lease `interrupted` 75 s later, at the Worker's next whole-minute check after EC2 reported `terminated`. Its job ended `JOB_FAILED` with `machine_released` (`reason: interrupted`, `output_lost: true`); homebased neither retried it nor created another machine, so resubmit it. The Worker does not use AWS's 2-minute interruption warning.
- **Spot quota.** "All Standard Spot Instance Requests" (`L-34B43A08`) is 256 vCPUs in us-east-1: 16 machines of 16 vCPUs, with no headroom. The README asks for 320 or more, so a replacement can launch while an old instance is still `shutting-down`.

Before a real pool can use it: higher caps (see [Classes and caps](#classes-and-caps)), a root volume that fits the job (afv-clean needs 150 GiB), the higher Spot quota, and Praveen's approval and a new deploy [11].

Add spot interruption to the cost of any AWS estimate [6]:

- **Rates.** The Spot Advisor's monthly frequency of interruption in us-east-1 is 5 to 10% for `c7a` and 10 to 15% for `c8a`. The chance during one run of about an hour is far lower, a few percent per machine; the README estimates it and has not measured it.
- **Cost of a loss.** AWS charges nothing for a spot machine it interrupts in its first hour. After that, a loss costs the hours that machine had run, about $0.40 to $1.20, plus the delay of rerunning its job.
- **Mitigation.** Launch through an instant fleet with several types (`c7a`, `c8a`, `m7a`, `c7i`), as rembuild does.

## Worked example: the afv-clean reference run

The run cleans 16 scrolls, one job each: 27.0 `c-16`-hours of work in all, about 120 GB uploaded to Hugging Face, and up to about 80 GB of disk per job [6]. The goal is to finish in about 3 hours, as the reference run on 16 DigitalOcean droplets did.

| Option | Machines | Billed per machine | (a) Little upload, compute only | (b) This run, 120 GB uploaded | Wall time | Can run today |
| --- | --- | --- | --- | --- | --- | --- |
| `code` | 1 | $0 | $0 | $0 | About 22 h, or 11 h without GPU jobs [6] | Yes |
| DigitalOcean `c-16`, `--idle-limit 3m` | 16 | 2.5 min boot + 1.69 h work + 3 min idle = 1.78 h; 28.5 h x $0.50 | **$14.24** | **$14.24** | About 3 h | Needs a `do-c16` class and higher caps; `c-16` stock unchecked |
| Same, default `--idle-limit` (10m) | 16 | 1.90 h; 30.3 h x $0.50 | $15.17 | $15.17 | About 3 h | As above |
| phoenixNAP `s3.c3.medium`, one job each | 16 | 0.42 h work + boot, rounded to 1 h; 16 h x $0.47 | $7.52 | $7.52 | About 1 h | No: quota 1, no stock |
| phoenixNAP `s3.c3.medium`, `--max 3 --slots 2`, `--idle-limit 3m` | 3 | 2.25 h work + up to 0.35 h boot + 3 min idle, rounded to 3 h; 9 h x $0.47 | **$4.23**; worst case one more hour each, $5.64 | **$4.23**; worst case $5.64 | About 2.6 h | No: quota 1, no stock |
| Latitude.sh `m4.metal.medium` | 2 (team limit) | 4.82 h work, rounded to 5 h; 10 h x $1.25 | $12.50 | $12.50 | About 5 h | Needs higher caps; the class is deployed [10][11] |
| AWS spot `c8a.4xlarge`, us-east-1 | 16, one job each | 0.58 h work (1.69 h ÷ speed 2.9) + 19 s boot + 3 min idle + 31 s to gone, per second; 9.3 h of work x $0.393 (the measured $0.3721 Spot price plus a 150 GiB volume and IPv4) = $3.65, plus about $0.39 for boot, idle and release [11] | $4.05 [11] | $14.85; $5.85 if the account's free 100 GB is unused that month [11] | About 0.6 h | Needs higher caps and a 150 GiB class; the Spot quota fits 16 with no headroom [11] |
| AWS spot `c7a.4xlarge`, us-east-1 | 16, one job each | 0.77 h work (1.69 h ÷ speed 2.2), per second; 12.3 h x $0.350 | $4.30 | $15.10; $6.10 if the free 100 GB is unused | About 0.8 h plus boot | As above |

The `c8a` row is Phase 4f's estimate [11]: the README's 9.3 machine-hours at speed 2.9, priced at the Spot price measured in zone 1b, plus the measured boot, a 3-minute idle tail and the release, about $4.05; about $3.45 if every machine landed in 1f at $0.29. The `c7a` row uses the batch-compute-providers README's "Cost of the run" table [6]: machine-hours at speed 2.2, priced as spot plus EBS and IPv4, without boot or the idle tail. Both leave out interruption (below). The flat-bandwidth rows cost the same in both scenarios, because each provider includes far more than 120 GB: 7,000 GiB pooled on DigitalOcean, 15 TB per location on phoenixNAP, 20 TB per server on Latitude.sh. The phoenixNAP IP block adds a few cents. Inbound is free everywhere, so the 540 GB downloaded from Hugging Face costs nothing.

Interruption rates and the cost of a loss are under [AWS spot](#aws-spot). Each AWS machine here works 0.6 to 0.8 h, so most of its life falls in the first hour, which AWS does not charge when it interrupts (derived from the speeds above and a boot of about 19 s [11]).

What this run should pick:

- If it can wait a day, use `code`.
- If it must finish in about 3 hours today, use DigitalOcean `c-16` with 16 machines, about $14.24. Ask for `--spend-cap 18` and the Worker changes above. With AWS spot, it is the only option not blocked by a known account limit or stock-out, and AWS costs more here once the upload is counted ($14.85 [11]). DigitalOcean's own stock was not checked.
- Once phoenixNAP raises the quota to 3 or more and `s3.c3.medium` is in stock, use 3 of them with 2 slots each, about $4.23. Do not use 16: every machine pays a full hour for 25 minutes of work.
- A run with the same work but little upload would be cheapest on AWS spot, about $4.05 on `c8a` or $4.30 plus boot on `c7a`, plus interruption, once the Worker's caps allow it. This run uploads about 4.4 GB per `c-16`-hour (120 GB over 27 hours), which puts it on the flat-bandwidth side.

The per-machine work is an average. Scrolls range from 15 to 47 GB, so the largest job sets the wall time and its `--kill-after`.

## Unconfirmed

Update this list, and the tables above, when a measurement lands.

| Number | Where it will come from |
| --- | --- |
| phoenixNAP: whether a deprovision just before an hour boundary avoids the next hour. Phase 4c saw the hour charged up front at `provisionedOn`, so when billing stops is not visible; the docs say at deprovision | A server that crosses an hour |
| phoenixNAP: boot to joined for `s3.c3.medium` (13m08s on `s2.c2.large` [9]) | The first `s3.c3.medium` create |
| phoenixNAP: speed of `s2.c2.large` and `s3.c3.medium` on our work (both are estimates) | A timed scroll on each |
| phoenixNAP: whether jobs can use the second NVMe, unused on `s2.c2.large` [9]; free disk after boot on `s3.c3.medium` | The first pool that needs it |
| Latitude.sh: whether the second hour starts 60 minutes after the billing line's start | A server that crosses an hour |
| Latitude.sh: boot to joined, and whether boot is billed, outside `deploys_instantly` (6.0 s on the instant path [10]) | A create on a plan, site or OS that does not deploy instantly |
| Latitude.sh: free disk after boot, and whether jobs can use the extra NVMe drives | The first pool on it |
| Latitude.sh: speed of `m4` and `f4` on our work | A timed scroll |
| DigitalOcean: boot to joined for `c-16` (measured only on `s-1vcpu-1gb`) | The first `c-16` pool |
| All: download speed from Hugging Face | The first timed scroll on each provider |
| AWS: its own billing lines for Phase 4f; the $0.035 is computed from instance times [11] | Cost Explorer, usually within a day |
| AWS: boot on `c7a.4xlarge`, the fall-through type (about 19 s on `c8a.4xlarge` [11]) | The first `c7a` machine |
| AWS: a real Spot interruption; Phase 4f tested only a terminate from outside the Worker [11] | AWS FIS, or the first real interruption |
| AWS: chance that a machine is interrupted during a run of about an hour (the README estimates a few percent) | The first spot runs |
| Whether the Worker was redeployed after Phase 4f (`ff36628e`) | The runbook's deploy log and the cloudlease README |
| How a pool machine can fetch a private repository for `--git` | Nobody has tested it; ask before the first pool on a private repository |

## Sources

1. `~/code/research/dev-infra/cloudlease/README.md`, Phase 2C result, "Spend caps" (DigitalOcean billing docs, pages updated 2026-11-17)
2. `~/code/research/dev-infra/batch-compute-providers/README.md`, "phoenixNAP" (billing KB, specs, live reads)
3. Same README, "Latitude.sh", and the Phase 4d progress notes; boot grace from the 4d design
4. `~/code/private-workbench/ops/cloudlease/machines-deploy.md`, "Pool notes learned in the pilot" (Phase 2E)
5. homebased `docs/temporary-machines.md`, `--boot-timeout`, released in homebased v0.37.0
6. `~/code/research/dev-infra/batch-compute-providers/README.md`, "Recommendation" (break-even rule), "Comparison", "Reference run" (including "Cost of the run"), "Our own machines", and "Risks" (spot interruption)
7. Phase 4c progress notes (`phase4c-progress.md` in the cloudlease run directory): stock check at 3:55 PM CT on 2026-11-24, and the test deploy with phoenixNAP enabled
8. cloudlease README, Phase 4b result, "Locations"; homebased `docs/temporary-machines.md`, "Lifecycle"
9. `~/code/research/dev-infra/cloudlease/README.md`, "Phase 4c result (2026-11-24)": the first live phoenixNAP test, one `s2.c2.large` in SEA
10. Same README, "Phase 4d result (2026-11-24)": the first live Latitude.sh test, one `m4.metal.medium` in CHI
11. Same README, "Phase 4f result (2026-11-24)": the first live AWS Spot test, two `c8a.4xlarge` in us-east-1b; the AWS boot grace from "Phase 4e result"
12. `apps/machines/wrangler.jsonc` on cloudlease master 7ad650c, unchanged since 9f3ac11, the commit Phase 4f deployed
