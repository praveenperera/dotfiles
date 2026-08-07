# Container Class API

Check the current
[Container Interface](https://developers.cloudflare.com/containers/container-class/) before using
an exact signature. Install a current `@cloudflare/containers` release and generate current Worker
types for the project.

## Container Definition

```typescript
import { Container, type StopParams } from "@cloudflare/containers";

export class MyContainer extends Container {
  defaultPort = 8080;
  requiredPorts = [8080, 9222];
  sleepAfter = "30m";
  enableInternet = false;
  pingEndpoint = "localhost/ready";
  envVars = { NODE_ENV: "production" };
  entrypoint = ["npm", "run", "start"];

  override onStart(): void | Promise<void> {}

  override onStop({ exitCode, reason }: StopParams): void | Promise<void> {
    console.log("Container stopped", { exitCode, reason });
  }

  override onError(error: unknown): unknown {
    throw error;
  }

  override async onActivityExpired(): Promise<void> {
    await this.stop();
  }
}
```

`onStop()` runs after the process exits. A Container process receives `SIGTERM` before Cloudflare
sends `SIGKILL` 15 minutes later, so process-level graceful shutdown must handle `SIGTERM` inside
the image.

If `onActivityExpired()` is overridden, call `stop()` or `destroy()` when the Container must sleep.
If neither method is called, the activity timer renews and the hook runs again at the next expiry.

## Routing Helpers

Use the helpers exported by `@cloudflare/containers`:

```typescript
import { getContainer, getRandom } from "@cloudflare/containers";

const session = getContainer(env.MY_CONTAINER, "user-123");
const singleton = getContainer(env.MY_CONTAINER);
const random = await getRandom(env.MY_CONTAINER, 5);
```

- `getContainer(binding, name?)` returns a stable named instance
- `getRandom(binding, instances?)` selects one instance from a fixed pool and defaults to three
- `getRandom()` is random routing, not latency-aware load balancing or autoscaling

## Request Methods

### `fetch()`

`fetch(request)` starts the Container when needed and forwards HTTP or WebSocket requests to
`defaultPort`. Prefer it when forwarding an incoming request.

```typescript
return getContainer(env.MY_CONTAINER, sessionId).fetch(request);
```

When overriding `fetch()` in a Container subclass, call `this.containerFetch()` to avoid recursive
calls:

```typescript
override async fetch(request: Request): Promise<Response> {
  if (new URL(request.url).pathname === "/health") {
    return new Response("ok");
  }

  return this.containerFetch(request);
}
```

### `containerFetch()`

`containerFetch()` sends HTTP directly to the Container process and starts it when needed. It does
not support WebSockets. It accepts an optional target port:

```typescript
return this.containerFetch(
  "http://localhost/internal/metrics",
  { headers: request.headers },
  9090
);
```

### `switchPort()`

Use the exported `switchPort(request, port)` helper when `fetch()` must target a different port,
including for WebSockets:

```typescript
import { getContainer, switchPort } from "@cloudflare/containers";

return getContainer(env.MY_CONTAINER).fetch(switchPort(request, 9090));
```

## Explicit Startup

Most request handlers do not need explicit startup because `fetch()` and `containerFetch()` start
the Container. Use these methods for pre-warming, scheduled work, batch jobs, or custom readiness.

### `startAndWaitForPorts()`

```typescript
const container = getContainer(env.MY_CONTAINER, "tenant-42");

await container.startAndWaitForPorts({
  ports: [8080, 9222],
  startOptions: {
    envVars: { TENANT_ID: "tenant-42" }
  },
  cancellationOptions: {
    portReadyTimeoutMS: 30_000
  }
});
```

Port resolution is explicit `ports`, then `requiredPorts`, then `defaultPort`. The default instance
acquisition timeout is 8 seconds and the default port-ready timeout is 20 seconds.

### `start()` and `waitForPort()`

Use `start()` for a Container that does not expose a port or when readiness is managed separately:

```typescript
await container.start({
  entrypoint: ["node", "scripts/nightly-report.js"],
  envVars: { REPORT_DATE: new Date().toISOString() },
  enableInternet: false
});

await container.waitForPort({
  portToCheck: 9222,
  retries: 20,
  waitInterval: 500
});
```

## Process Execution

`exec()` starts another process inside a running Container. It does not start a stopped Container.
Pass the executable and arguments as separate array items; shell expansion is not implicit.

```typescript
async readVersion() {
  if (!this.ctx.container.running) {
    await this.start();
  }

  const process = await this.ctx.container.exec(["node", "--version"]);
  const output = await process.output();

  return {
    exitCode: output.exitCode,
    stdout: new TextDecoder().decode(output.stdout)
  };
}
```

Use a shell explicitly only when pipes, redirects, globbing, or variable expansion are required.
Do not interpolate untrusted values into a shell command.

## Outbound Traffic Controls

Use `enableInternet`, `allowedHosts`, and `deniedHosts` for static policy. Use `outboundByHost` or
`outbound` when trusted Worker code must inspect, block, or translate HTTP requests from the
Container. Export `ContainerProxy` when interception is configured.

```typescript
import {
  Container,
  ContainerProxy,
  getContainer
} from "@cloudflare/containers";

export class RestrictedContainer extends Container {
  defaultPort = 8080;
  enableInternet = false;
  allowedHosts = ["api.example.com"];

  static override outboundByHost = {
    "bindings.internal": async (_request, env: Env) => {
      const value = await env.CONFIG.get("current");
      return new Response(value ?? "", { status: value ? 200 : 404 });
    }
  };
}

export { ContainerProxy };

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return getContainer(env.RESTRICTED_CONTAINER).fetch(request);
  }
};
```

Use `enableInternet = false` as the default for untrusted workloads. Check the current outbound
traffic documentation for HTTPS interception, dynamic policy methods, and binding access.

## State and Activity

```typescript
const state = await this.getState();
// status: "running" | "healthy" | "stopping" | "stopped" | "stopped_with_code"

if (state.status === "stopped_with_code") {
  console.error("Container exited", state.exitCode);
}
```

`running` means startup is in progress. `healthy` means the Container passed its health check and
accepts requests. Use `this.ctx.container.running` only for an internal synchronous running check.

Call `this.renewActivityTimeout()` from background work that must reset `sleepAfter`:

```typescript
for (const jobId of jobIds) {
  this.renewActivityTimeout();
  await this.containerFetch(`http://localhost/jobs/${jobId}`, { method: "POST" });
}
```

## Scheduling

Use `schedule(when, callback, payload?)`. A numeric `when` is a delay in seconds; a `Date` is an
absolute time. Do not override `alarm()` because the Container class owns the Durable Object alarm.

```typescript
override async onStart(): Promise<void> {
  await this.schedule(60, "healthReport");
}

async healthReport(): Promise<void> {
  console.log("Container status", await this.getState());
  await this.schedule(60, "healthReport");
}
```

## Current Official References

- [Container Interface](https://developers.cloudflare.com/containers/container-class/)
- [Execute commands](https://developers.cloudflare.com/containers/execute-commands/)
- [Outbound traffic](https://developers.cloudflare.com/containers/platform-details/outbound-traffic/)
- [Scaling and routing](https://developers.cloudflare.com/containers/platform-details/scaling-and-routing/)
