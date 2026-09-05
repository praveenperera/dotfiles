# rb troubleshooting

Read this reference when a build fails readiness checks, cannot obtain a lane,
or behaves differently from the expected lease lifecycle.

## Read lane state

Each build carries labels that `rb status` shows under
`lifecycle.lanes[].lease`: `owner` (`RB_BUILD_OWNER` or `user@host`),
`command` with build-arg values redacted, `startedAt`, `lastHeartbeatAt`,
`heartbeatDueAt`, and `expiresAt`. Set `RB_BUILD_OWNER` in CI or agent runs so
the lane holder is identifiable.

If every lane is held by another build, `rb build` fails with
`409 project_capacity_exhausted` and prints each occupied lane, its owner,
command, running time, and last heartbeat. Queue instead of failing:

```bash
rb build --wait -- -t example/app:latest --push .
rb build --wait --wait-timeout 1h -- -t example/app:latest --push .
```

The default `--wait-timeout` is `30m`. Do not kill a queued `rb build --wait`
to free a lane; the lane belongs to the build shown in the report.

For a lane marked `active` without a local `rb build`, check
`rb status` `lease.heartbeatDueAt`. If it is in the past, the control plane
expires the lease and `rb terminate` or `rb build --wait` can proceed. If it is
in the future, the build is live elsewhere.

## Common failures

| Symptom | Action |
| --- | --- |
| no control-plane URL or token | authenticate with `RB_TOKEN`; see [project-management.md](project-management.md) |
| Docker, Buildx, or SSH fails in doctor | fix the local install and re-run `rb doctor` |
| project missing | run `rb build` for the default policy, or use `rb project init --name <name>` for custom policy |
| name conflict | choose a new name with `--project` or `RB_PROJECT`; the existing name belongs to another SSH key or policy |
| SSH or host-key issue | use `rb project init --name <name> --rotate-key` only when the key is bad and rotation was requested |
| builder provisioning timeout | re-run the build and check `rb status` |
| `409 project_capacity_exhausted` | read the printed owner and command, then use `--wait` or ask the owner |
| cheaper idle operation | use `rb stop`; it keeps the cache |

If rb fails for tooling or authentication, run `rb doctor`, report the error,
and use local Docker only after the user accepts that fallback.
