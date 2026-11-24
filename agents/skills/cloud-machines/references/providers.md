# Providers, classes, and effective cost

Facts checked on 2026-11-24 unless a row says otherwise. Each one names its source. A number that nobody has measured on our machines is listed under [Unconfirmed](#unconfirmed); update that list, not the tables, when a measurement comes in.

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
- **Idle tail** is the time after a machine's last job until it is gone: `--idle-limit`, plus about a second for the delete. Count it before rounding. On an hourly provider, a machine whose work ends within `--idle-limit` of an hour boundary pays one more hour, so keep the idle limit short there too. Releasing the pool the moment the last job ends removes the tail only for the machines that finished last.
- **Machine count on an hourly provider.** A machine with 25 minutes of work pays a full hour. Choose the fewest machines that meet the deadline, about total machine-hours divided by the whole hours each machine may run, and submit all jobs at once so each machine takes the next job without a gap. Spread over many machines, hourly rounding takes most of the saving: in the worked example, 16 phoenixNAP machines cost 1.8 times as much as 3.

## Providers

| | DigitalOcean | phoenixNAP Bare Metal Cloud | Latitude.sh |
| --- | --- | --- | --- |
| In the Worker | Yes, deployed (Phase 2E pilot) | Classes on master; a Phase 4c test deploy enabled them with test caps, unconfirmed as the current deploy [7] | Wired on cloudlease master (8bac9ac), class not enabled |
| Billing | Per second, minimum 60 s or $0.01 [1] | Every started hour in full, minimum 1 hour, until deprovision [2] | Hourly; rounding and minimum unconfirmed. cloudlease bills whole started hours until a paid test shows otherwise [3] |
| Boot billed | Yes: from create; cloudlease counts spend from `created_at` [1] | Billing starts "at deploy"; create or power-on is unconfirmed [2] | Unconfirmed |
| Boot to joined | 2m12s to 2m17s, measured on `s-1vcpu-1gb` in `nyc3` [4] | Docs: 1 to 15 minutes by OS, plus a reboot for cloud-init. Not measured [2] | Docs: under 10 s for `ubuntu_24_04_x64_lts` where it deploys instantly, about 10 minutes otherwise. Not measured [3] |
| Boot grace | 8 min; default `--boot-timeout` 7m [5] | 20 min; default `--boot-timeout` 19m [5] | 15 min; default `--boot-timeout` 14m [3][5] |
| Account limit | At least 17 droplets: the reference run used 17 [6] | 1 server (`bmc.servers.max_count`) [2] | 2 bare-metal servers per team; $100 an hour of spend for a new account [3] |
| Egress | 7,000 GiB pooled, then $0.01/GiB [6] | 15 TB per location included [6] | 20 TB per server, then $0.01/GB [3] |
| Extras | Volumes bill hourly, attached or not; prefer local disk [1] | IP block, $0.0035 per IP per hour until deprovision [2] | None needed |

Classes and prices:

| Provider | Type | $/h | CPU, RAM, disk | Speed vs `c-16` | Stock on 2026-11-24 |
| --- | --- | --- | --- | --- | --- |
| DigitalOcean | `c-16` | 0.50 [6] | 16 vCPU (8 cores + SMT), 32 GB, 200 GiB SSD | 1.0 (the unit) | Not checked |
| phoenixNAP | `s3.c3.medium` | 0.47 [2] | i9-14900K, 128 GB, 2 x 2 TB NVMe | About 4.0, estimate [6] | None in any location [2] |
| phoenixNAP | `s2.c2.large` | 0.40 [2] | E-2388G 8 cores, 128 GB | Unconfirmed | 1 in NLD, 0 in SEA (3:55 PM CT); moves within hours [7] |
| phoenixNAP | `s2.c2.small` | 0.31 [2] | E-2388G 8 cores, 64 GB, one NVMe | Unconfirmed | 2 in SEA [7] |
| Latitude.sh | `m4.metal.medium` | 1.25 (US) [3] | EPYC 9124 16 cores, 128 GB, 2 x 480 GB + 2 x 1.9 TB NVMe | About 2.8, estimate [6] | 1 each in DAL, CHI, MIA2, SJC2 [3] |
| Latitude.sh | `f4.metal.medium` | 1.52 (US) [3] | EPYC 4564P 16 cores, 128 GB, same disks | About 4.4, estimate [6] | None in the US or EU [3] |

Latitude.sh's pricing page shows the monthly price divided by 730; the API's hourly price is about twice that. Use the API price [3].

Stock is a live fact. A stock-out on phoenixNAP or Latitude.sh fails the create and bills nothing, and a class can list several locations to try in order. The pool stops after three failed creates in a row (`create_failed`) [8].

## Classes and caps

The Worker's classes and caps are in `apps/machines/wrangler.jsonc` on cloudlease master. A pool may name only a class listed there under `apps.homebased.classes`. On master 8bac9ac they are the caps of the first live tests, too small for a real run:

| Setting | Value |
| --- | --- |
| Classes | `do-s1-1g` (1 vCPU, 1 GB, $0.00893/h), `pnap-s2-large` ($0.40/h, SEA then NLD), `pnap-s2-small` ($0.31/h, SEA then NLD); each `max_hours` 1 |
| `max_machines_per_run` | 1 |
| `max_run_hours` | 1 |
| `run_spend_cap_usd` | 0.40 |
| `HOURLY_SPEND_CAP_CENTS` | 40, across every run |
| `heartbeat_ttl_secs` | 180 |

No `do-c16` class exists yet. The deployed version can differ from master: the runbook and the cloudlease README record each deploy. For a real run, the request to Praveen also names the Worker change it needs: the class, `max_machines_per_run` of at least `--max`, `max_run_hours` and the class's `max_hours` of at least the deadline, `run_spend_cap_usd` of at least `--spend-cap`, and `HOURLY_SPEND_CAP_CENTS` of at least `--max` x the hourly price, in cents.

A homebased pool machine needs the job's disk free after boot (`--min-free-disk`). On bare metal, the extra NVMe drives may not be mounted at boot. Whether a job can use them is unconfirmed.

## AWS spot: not available yet

AWS spot suits runs that upload little: about a third of DigitalOcean's cost per unit of work, but egress costs $0.09/GB after the account's free 100 GB a month, which is shared by the whole AWS account [6]. It beats the included-bandwidth hosts below about 1 GB uploaded per `c-16`-hour of work, and loses to them above about 3 GB [6]. Inbound is free, and S3 reads in us-east-1 (the Vesuvius open-data bucket) are free in-region [6]. Prices: `c7a.4xlarge` $0.329/h, `c8a.4xlarge` $0.366/h, plus about $0.021/h of EBS and IPv4, per second with a 60 s minimum [6].

Before a pool can use it:

- An AWS variant in the machines Worker. rembuild's AWS adapter now lives in `cloudlease-provider` (`crates/cloudlease-provider/src/aws.rs`), but the machines Worker does not run it yet [9].
- Spot interruption handling: an interrupted machine must end its lease, so its job ends with `machine_released` and is resubmitted.
- The us-east-1 Service Quota "All Standard Spot Instance Requests" of at least `--max` x vCPUs, such as 320 for 20 machines of 16 vCPUs. Unchecked [6].
- Praveen's approval for the Worker change and its AWS credentials.

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
| Latitude.sh `m4.metal.medium` | 2 (team limit) | 4.82 h work, rounded to 5 h; 10 h x $1.25 | $12.50 | $12.50 | About 5 h | Needs the class enabled and caps |
| AWS spot `c8a.4xlarge`, us-east-1 | 16, one job each | 0.58 h work (1.69 h ÷ speed 2.9), per second; 9.3 h x $0.387 | $3.61 | $14.41; $5.41 if the account's free 100 GB is unused that month | About 0.6 h plus boot | No: no AWS adapter in the machines Worker yet |
| AWS spot `c7a.4xlarge`, us-east-1 | 16, one job each | 0.77 h work (1.69 h ÷ speed 2.2), per second; 12.3 h x $0.350 | $4.30 | $15.10; $6.10 if the free 100 GB is unused | About 0.8 h plus boot | As above |

The AWS rows use the figures in the batch-compute-providers README's "Cost of the run" table [6]: machine-hours at speed 2.9 and 2.2, priced as spot plus EBS and IPv4. They leave out boot and the idle tail, which are unmeasured on AWS, and they leave out interruption (below). The flat-bandwidth rows cost the same in both scenarios, because each provider includes far more than 120 GB: 7,000 GiB pooled on DigitalOcean, 15 TB per location on phoenixNAP, 20 TB per server on Latitude.sh. The phoenixNAP IP block adds a few cents. Inbound is free everywhere, so the 540 GB downloaded from Hugging Face costs nothing.

Interruption rates and the cost of a loss are under [AWS spot](#aws-spot-not-available-yet). Each AWS machine here works 0.6 to 0.8 h, so most of its life falls in the first hour, which AWS does not charge when it interrupts (derived from the speeds above; boot is unmeasured).

What this run should pick:

- If it can wait a day, use `code`.
- If it must finish in about 3 hours today, use DigitalOcean `c-16` with 16 machines, about $14.24. Ask for `--spend-cap 18` and the Worker changes above. It is the only option not blocked by a known account limit or stock-out; its own stock was not checked.
- Once phoenixNAP raises the quota to 3 or more and `s3.c3.medium` is in stock, use 3 of them with 2 slots each, about $4.23. Do not use 16: every machine pays a full hour for 25 minutes of work.
- A run with the same work but little upload would be cheapest on AWS spot, about $3.61 to $4.30 plus boot and interruption, once cloudlease can run it. This run uploads about 4.4 GB per `c-16`-hour (120 GB over 27 hours), which puts it on the flat-bandwidth side.

The per-machine work is an average. Scrolls range from 15 to 47 GB, so the largest job sets the wall time and its `--kill-after`.

## Unconfirmed

Update this list, and the tables above, when a measurement lands.

| Number | Where it will come from |
| --- | --- |
| phoenixNAP: whether billing starts at create or at power-on, and whether a deprovision just before an hour boundary avoids the next hour | Phase 4c live test (`phase4c-progress.md` in the cloudlease run directory), then the phoenixNAP section of the batch-compute-providers README |
| phoenixNAP: boot to joined for `s2.c2.large` and `s3.c3.medium` | Phase 4c live test |
| phoenixNAP: speed of `s2.c2.large` and `s3.c3.medium` on our work | A timed scroll on each |
| phoenixNAP: free disk after boot, and whether jobs can use the second NVMe | Phase 4c live test, or the first pool on it |
| Latitude.sh: partial-hour rounding and minimum charge; whether boot is billed; boot to joined | Phase 4d live test (`phase4d-progress.md`), then the Latitude.sh section of the batch-compute-providers README |
| Latitude.sh: speed of `m4` and `f4` on our work | A timed scroll |
| DigitalOcean: boot to joined for `c-16` (measured only on `s-1vcpu-1gb`) | The first `c-16` pool |
| All: download speed from Hugging Face | The first timed scroll on each provider |
| AWS: spot quota in us-east-1 | Praveen, from Service Quotas |
| AWS: boot to joined, and so the boot and idle tail and wall time that the worked example leaves out | The first spot pool, or one timed launch |
| AWS: chance that a machine is interrupted during a run of about an hour (the README estimates a few percent) | The first spot runs |
| Which Worker version is deployed and its caps | The runbook's deploy log and the cloudlease README |
| How a pool machine can fetch a private repository for `--git` | Nobody has tested it; ask before the first pool on a private repository |

## Sources

1. `~/code/research/dev-infra/cloudlease/README.md`, Phase 2C result, "Spend caps" (DigitalOcean billing docs, pages updated 2026-11-17)
2. `~/code/research/dev-infra/batch-compute-providers/README.md`, "phoenixNAP" (billing KB, specs, live reads)
3. Same README, "Latitude.sh", and the Phase 4d progress notes; boot grace from the 4d design
4. `~/code/private-workbench/ops/cloudlease/machines-deploy.md`, "Pool notes learned in the pilot" (Phase 2E)
5. homebased `docs/temporary-machines.md`, `--boot-timeout`, on branch `temporary-machines`
6. `~/code/research/dev-infra/batch-compute-providers/README.md`, "Recommendation" (break-even rule), "Comparison", "Reference run" (including "Cost of the run"), "Our own machines", and "Risks" (spot interruption)
7. Phase 4c progress notes (`phase4c-progress.md` in the cloudlease run directory): stock check at 3:55 PM CT on 2026-11-24, and the test deploy with phoenixNAP enabled
8. cloudlease README, Phase 4b result, "Locations"; homebased `docs/temporary-machines.md`, "Lifecycle"
9. cloudlease README, Phase 3 result; `crates/cloudlease-provider/src/aws.rs` on cloudlease master (the adapter moved there from rembuild); the machines Worker's classes in `apps/machines/wrangler.jsonc` list no AWS class
