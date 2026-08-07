# Container Configuration

Check the current
[Wrangler configuration reference](https://developers.cloudflare.com/workers/wrangler/configuration/#containers)
and [limits page](https://developers.cloudflare.com/containers/platform-details/limits/) before
using exact fields or numeric limits.

## Basic Wrangler Configuration

```jsonc
{
  "name": "my-worker",
  "main": "src/index.ts",
  "compatibility_date": "2026-07-28",
  "containers": [
    {
      "class_name": "MyContainer",
      "image": "./Dockerfile",
      "instance_type": "standard-1",
      "max_instances": 10
    }
  ],
  "durable_objects": {
    "bindings": [
      {
        "name": "MY_CONTAINER",
        "class_name": "MyContainer"
      }
    ]
  },
  "migrations": [
    {
      "tag": "v1",
      "new_sqlite_classes": ["MyContainer"]
    }
  ]
}
```

- `class_name` must match the Container class export and Durable Object binding
- `image` can be a local Dockerfile path or a supported registry image reference
- `max_instances` limits concurrently running production instances and defaults to 20
- Container Durable Objects use a SQLite migration with `new_sqlite_classes`
- Use a current compatibility date for new deployments

Current supported external registries include Docker Hub, Amazon ECR, and Google Artifact Registry.
Check image-management documentation for authentication and exact reference formats.

## Instance Types

| Type | vCPU | Memory | Disk |
|------|------|--------|------|
| `lite` | 1/16 | 256 MiB | 2 GB |
| `basic` | 1/4 | 1 GiB | 4 GB |
| `standard-1` | 1/2 | 4 GiB | 8 GB |
| `standard-2` | 1 | 6 GiB | 12 GB |
| `standard-3` | 2 | 8 GiB | 16 GB |
| `standard-4` | 4 | 12 GiB | 20 GB |

The default is `lite`. The legacy `dev` and `standard` names remain aliases for `lite` and
`standard-1`, but do not use the legacy names in new configuration.

### Custom Instance Type

Set a custom object in `instance_type`. Do not use the obsolete `instance_type_custom` or
`disk_mib` fields.

```jsonc
{
  "containers": [
    {
      "class_name": "MyContainer",
      "image": "./Dockerfile",
      "instance_type": {
        "vcpu": 2,
        "memory_mib": 8192,
        "disk_mb": 16000
      }
    }
  ]
}
```

Current custom-type constraints:

- 1-4 vCPU
- Up to 12 GiB memory
- Up to 20 GB disk
- At least 3 GiB memory per vCPU
- At most 2 GB disk per 1 GiB memory

Use a predefined type for less than one vCPU.

## Account Limits

The following limits were current on July 3, 2026. Retrieve the current limits before planning
capacity.

| Resource | Limit |
|----------|-------|
| Concurrent memory | 6 TiB |
| Concurrent vCPU | 1,500 |
| Concurrent disk | 30 TB |
| Image size | Same as the selected instance disk space |
| Total image storage | 50 GB per account |

Deleting an image can break rollback to a Worker version that still refers to that image.

## Container Class Properties

```typescript
import { Container } from "@cloudflare/containers";

export class MyContainer extends Container {
  defaultPort = 8080;
  requiredPorts = [8080, 9222];
  sleepAfter = "30m";
  enableInternet = false;
  pingEndpoint = "localhost/ready";
  envVars = {
    NODE_ENV: "production",
    LOG_LEVEL: "info"
  };
  entrypoint = ["npm", "run", "start"];
}
```

- **`defaultPort`**: Target for `fetch()` and `containerFetch()` when no port is specified
- **`requiredPorts`**: Ports that `startAndWaitForPorts()` waits for when no explicit ports are given
- **`sleepAfter`**: Idle duration as seconds or a string such as `"30s"`, `"5m"`, or `"1h"`; defaults to `"10m"`
- **`enableInternet`**: Controls outbound HTTP access; defaults to `true`
- **`pingEndpoint`**: Host and path used for startup health checks; defaults to `"ping"`
- **`envVars`**: Environment variables applied on each start
- **`entrypoint`**: Optional replacement for the image entrypoint

Use `startAndWaitForPorts({ startOptions: ... })` for per-instance environment, entrypoint, or
internet-access overrides.

## Runtime Environment Variables

| Variable | Description |
|----------|-------------|
| `CLOUDFLARE_APPLICATION_ID` | Container application ID |
| `CLOUDFLARE_COUNTRY_A2` | Two-letter country code for the Container location |
| `CLOUDFLARE_LOCATION` | Cloudflare location name |
| `CLOUDFLARE_REGION` | Cloudflare region |
| `CLOUDFLARE_DURABLE_OBJECT_ID` | Associated Durable Object instance ID |

Do not overwrite runtime-provided names with user-defined `envVars`.

## Placement Constraints

Use `constraints.regions` for geographic placement or `constraints.jurisdiction` for a compliance
boundary. Retrieve the current valid values before configuration.

```jsonc
{
  "containers": [
    {
      "class_name": "MyContainer",
      "image": "./Dockerfile",
      "constraints": {
        "regions": ["ENAM", "WNAM"],
        "jurisdiction": "fedramp"
      }
    }
  ]
}
```

## Rolling Deployments

Container instances update with a rolling deployment while Worker code updates immediately. Keep
Worker and Container changes compatible until the rollout completes.

```jsonc
{
  "containers": [
    {
      "class_name": "MyContainer",
      "image": "./Dockerfile",
      "rollout_active_grace_period": 300,
      "rollout_step_percentage": [10, 100]
    }
  ]
}
```

- `rollout_active_grace_period` delays updates to active instances and defaults to `0`
- `rollout_step_percentage` defaults to `[10, 100]`
- `wrangler deploy --containers-rollout=immediate` uses one 100% rollout step but does not bypass the active grace period

## TOML Form

```toml
name = "my-worker"
main = "src/index.ts"
compatibility_date = "2026-07-28"

[[containers]]
class_name = "MyContainer"
image = "./Dockerfile"
max_instances = 10

[containers.instance_type]
vcpu = 2
memory_mib = 8_192
disk_mb = 16_000

[[durable_objects.bindings]]
name = "MY_CONTAINER"
class_name = "MyContainer"

[[migrations]]
tag = "v1"
new_sqlite_classes = ["MyContainer"]
```

## Current Official References

- [Wrangler Container configuration](https://developers.cloudflare.com/workers/wrangler/configuration/#containers)
- [Limits and instance types](https://developers.cloudflare.com/containers/platform-details/limits/)
- [Placement](https://developers.cloudflare.com/containers/platform-details/placement/)
- [Rollouts](https://developers.cloudflare.com/containers/platform-details/rollouts/)
- [Environment variables](https://developers.cloudflare.com/containers/platform-details/environment-variables/)
