# Cloud rendering

Choose the smallest deployment model that meets the task:

| Command | Use when | Infrastructure |
| --- | --- | --- |
| `hyperframes cloud` | Render on HeyGen without local Chrome or FFmpeg | Managed HeyGen account and authentication |
| `hyperframes lambda` | Run distributed renders in the user's AWS account | AWS credentials, SAM, Lambda, Step Functions, and S3 |
| `hyperframes cloudrun` | Run distributed renders in the user's GCP project | GCP project, Cloud Run, Cloud Build, and Cloud Storage |

Prefer local `render` for iteration. Use a cloud backend for long, large, 4K, or parallel workloads.

## HeyGen cloud

Authenticate, submit, and inspect renders:

```bash
npx hyperframes auth login
npx hyperframes cloud render ./my-project --wait --output out.mp4
npx hyperframes cloud list --json
npx hyperframes cloud get <render-id> --json
npx hyperframes cloud delete <render-id> --no-confirm --json
```

`cloud render` supports `--fps`, `--quality`, `--format`, `--resolution`, `--aspect-ratio`, `--composition`, variable overrides, callbacks, and idempotency keys. It waits and downloads by default; pass `--no-wait` for fire-and-forget submission. Use `auth status` to inspect credentials and `auth logout` to remove them.

## AWS Lambda

```bash
npx hyperframes lambda deploy
npx hyperframes lambda render ./my-project --output-resolution 4k --wait
npx hyperframes lambda progress <render-id>
npx hyperframes lambda destroy
```

Read [lambda.md](lambda.md) before provisioning, rendering batches, changing IAM policies, or destroying the stack.

## Google Cloud Run

```bash
npx hyperframes cloudrun deploy --project <gcp-project>
npx hyperframes cloudrun render ./my-project --output-resolution 4k --wait
npx hyperframes cloudrun progress <render-id>
npx hyperframes cloudrun destroy --project <gcp-project>
```

The CLI exposes `deploy`, `sites`, `render`, `render-batch`, `progress`, and `destroy`. Deployment and render flags evolve with the service; inspect `npx hyperframes cloudrun --help` immediately before use. Keep `--max-instances`, `--max-parallel-chunks`, and batch concurrency within the user's quota and cost constraints.
