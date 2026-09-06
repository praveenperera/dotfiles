# rb registries and managed cache

Read this reference when an image or BuildKit cache will be pushed to a
registry, or when the automatic managed cache is configured.

## Choose a registry

Choose from the image's visibility, not the source repository's visibility:

| Image use | Preferred registry | Reason |
| --- | --- | --- |
| Private application image | Cloudflare Managed Registry | short-lived, scoped credentials keep the remote builder and deployment host outside the GitHub source credential boundary |
| Public application image | Docker Hub | anonymous digest pulls need no credential and Docker Hub is the public image registry |
| Private-image BuildKit cache | Cloudflare Managed Registry | the application image and cache share one temporary build credential |
| Public-image BuildKit cache | private Docker Hub repository | the public application repository stays separate from the private build cache |

## Managed Cloudflare cache

For a private Cloudflare image, enable the automatic managed cache in the
nearest `.rb.toml`:

```toml
project = "my-app"

[registry_cache]
provider = "cloudflare"
account_id = "<OPTIONAL_ACCOUNT_ID>"
retention_days = 7
credential_minutes = 120
```

Omit `account_id` when Wrangler already has one selected. Wrangler must be
installed and logged in only when this mode is enabled. After a lane is ready,
`rb` issues a temporary push-and-pull credential, uses an isolated temporary
Docker configuration, imports the newest retained cache tag for each lane, and
exports the current lane to
`<project>-buildcache:rb-v1-<lane>-<UTC-date>`. It prunes expired rb-owned tags
after a successful build and lease release. A prune error warns, and the next
successful build retries it. The isolated configuration keeps Docker CLI
plug-in discovery paths but does not copy existing registry authentication.

Use `rb build --no-managed-cache -- ...` to bypass the configured cache for one
build. Manual `--cache-from` and `--cache-to` options are additive to the
project Volume cache and managed cache.

Pass managed-cache edges before the `--` separator:

```bash
rb build \
  --cache-from type=registry,ref=… \
  --cache-to type=registry,ref=… \
  -- -t example/app:latest --push .
```

## Push credentials

Push private Cloudflare images to:

```text
registry.cloudflare.com/<ACCOUNT_ID>/<IMAGE>:<TAG>
```

Then build normally:

```bash
rb build -- --platform linux/amd64 --push \
  --tag registry.cloudflare.com/<ACCOUNT_ID>/<IMAGE>:<TAG> .
```

Wrangler is the temporary credential issuer; it is not required for the image
transfer. Give a deployment host a separate short-lived pull-only credential.
Never send the build credential to a deployment host.

Push public images to Docker Hub:

```text
docker.io/<NAMESPACE>/<IMAGE>:<TAG>
```

Then build normally:

```bash
rb build -- --platform linux/amd64 --push \
  --tag docker.io/<NAMESPACE>/<IMAGE>:<TAG> .
```

Use a separate private Docker Hub repository for the public-image BuildKit
cache, and configure consumers to pull the verified digest without
authentication.
