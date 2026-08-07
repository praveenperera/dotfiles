# Container Gotchas

## WebSockets Require `fetch()`

**Problem:** A WebSocket upgrade fails.

**Cause:** `containerFetch()` does not support WebSockets.

**Fix:** Forward the request with `fetch()`. For another port, use `switchPort()`.

```typescript
import { getContainer, switchPort } from "@cloudflare/containers";

const container = getContainer(env.WS_CONTAINER, sessionId);
return container.fetch(switchPort(request, 8081));
```

## Request Methods Start Containers Automatically

`fetch()` and `containerFetch()` start a stopped Container. Do not add an explicit start before
normal request forwarding.

Use `startAndWaitForPorts()` when pre-warming, running scheduled work, applying per-instance startup
options, or checking custom readiness. Use `start()` for work that does not expose ports or when
readiness is managed separately.

## `waitForPort()` Takes an Options Object

```typescript
await container.waitForPort({
  portToCheck: 9222,
  retries: 20,
  waitInterval: 500
});
```

Do not use the obsolete positional `waitForPort(port, { timeout })` form.

## `exec()` Does Not Start the Container

**Problem:** A command fails because no Container process is active.

**Fix:** Check `this.ctx.container.running` inside the Container class and call `start()` when needed.

```typescript
if (!this.ctx.container.running) {
  await this.start();
}

const process = await this.ctx.container.exec(["node", "--version"]);
```

`exec()` does not invoke a shell. Pass arguments as array items, or explicitly run a shell when its
features are required. Do not interpolate untrusted data into shell commands.

## Long Background Work Must Renew Activity

Incoming requests reset `sleepAfter`; background work does not. Call
`this.renewActivityTimeout()` while the work is active. Writing an arbitrary Durable Object storage
key is not the activity-renewal API.

## `onActivityExpired()` Must Stop or Renew

The default implementation calls `stop()`. If an override does not call `stop()` or `destroy()`, the
timer renews and the hook runs again later.

```typescript
override async onActivityExpired(): Promise<void> {
  if (await this.hasActiveJobs()) {
    this.renewActivityTimeout();
    return;
  }

  await this.stop();
}
```

## Do Not Override `alarm()`

The Container class uses the Durable Object alarm for its own scheduling. Use
`schedule(when, callback, payload?)` and a named callback method.

```typescript
await this.schedule(60, "checkHealth");
```

## `onStop()` Runs After Process Exit

Put process-level graceful shutdown in the image's `SIGTERM` handler. Cloudflare sends `SIGTERM`
and then sends `SIGKILL` after 15 minutes. Use `onStop({ exitCode, reason })` for Worker-side logging,
alerts, and recovery decisions after the process exits.

## Container State Uses `healthy`

`getState().status` can be `running`, `healthy`, `stopping`, `stopped`, or `stopped_with_code`.
`running` means startup is in progress; `healthy` means the Container accepts requests.

## Custom Instance Type Shape Changed

Use an object in `instance_type`, with `disk_mb`:

```jsonc
"instance_type": {
  "vcpu": 2,
  "memory_mib": 8192,
  "disk_mb": 16000
}
```

Do not use `instance_type_custom` or `disk_mib`.

## Capacity Errors

### Maximum Instances Reached

`max_instances` limits concurrently running production instances. Stopped instances do not count.

- Increase `max_instances` when account capacity permits
- Set an appropriate `sleepAfter`
- Stop job-specific Containers when work is complete
- Check for instance leaks

### No Container Instance Available

Account capacity or placement constraints can prevent a start.

- Check current account limits and live usage
- Check the instance type and placement constraints
- Reduce resource size or concurrent work
- Contact Cloudflare support when account capacity must increase

## Image and Deployment Constraints

- Build images for `linux/amd64`
- The image-size limit equals the selected instance disk size
- A first deployment can need several minutes before Container requests succeed
- Container instances update through rolling deployments while Worker code updates immediately
- Keep Worker and Container protocol changes compatible until the rollout completes
- Deleting an image can break rollback to an older Worker version

## Ephemeral Disk and Restarts

Container disk resets after the Container stops. Persist required state in Durable Object storage,
R2, or another durable store. A Container can restart in a different location. Treat the process as
replaceable and make work recoverable or idempotent.

## Operational Constraints

Containers are generally available, but these constraints still affect architecture:

- **No built-in autoscaling:** Select a fixed pool for `getRandom()` or address explicit instances
- **Random pool routing:** `getRandom()` is not latency-aware
- **Rolling deployments:** Container rollout timing differs from Worker deployment timing
- **Cold starts:** Startup is often 1-3 seconds but depends on image size and initialization
- **HTTP entry path:** End users cannot connect directly to a Container with arbitrary TCP or UDP

Test restart recovery, capacity behavior, and rolling deployment compatibility before production.

## Current Official References

- [Container Interface](https://developers.cloudflare.com/containers/container-class/)
- [Lifecycle](https://developers.cloudflare.com/containers/platform-details/architecture/)
- [Limits](https://developers.cloudflare.com/containers/platform-details/limits/)
- [Scaling and routing](https://developers.cloudflare.com/containers/platform-details/scaling-and-routing/)
- [Rollouts](https://developers.cloudflare.com/containers/platform-details/rollouts/)
