# Cloudflare Containers Skill Reference

**APPLIES TO: Cloudflare Containers ONLY - NOT general Cloudflare Workers**

Use when deploying container images on the Workers platform, configuring container-enabled Durable
Objects, managing container lifecycles, or selecting stateful and stateless routing patterns.

## Availability

Cloudflare Containers and the Sandbox SDK have been
[generally available since April 13, 2026](https://developers.cloudflare.com/changelog/post/2026-04-13-containers-sandbox-ga/).
Containers are available on the Workers Paid plan. Check the current changelog and documentation
before relying on limits, pricing, or platform capabilities.

## Core Concepts

**Container-backed Durable Object:** Each named Container has a persistent Durable Object identity.
Use `getContainer(binding, name)` for a stable identity or `getRandom(binding, count)` for random
routing across a fixed pool.

**Automatic startup:** `fetch()` and `containerFetch()` start a stopped Container. Call lifecycle
methods directly only for pre-warming, scheduled work, batch jobs, or explicit lifecycle control.

**Image deployment:** Cloudflare distributes images across its network and pre-fetches them at
selected locations. Container deployments use rolling updates.

**Lifecycle:** Cold starts are often 1-3 seconds, but image size and startup work affect this time.
Containers stop after their `sleepAfter` timeout. Built-in autoscaling is not available.

**Persistent identity, ephemeral disk:** Durable Object storage persists across Container restarts,
but the Container disk resets after a stop. Use Durable Object storage, R2, or another durable store
for required data.

**Additional processes:** Use `this.ctx.container.exec()` to run commands inside an active
Container. `exec()` does not start a stopped Container.

**Outbound controls:** Allow, deny, or intercept Container HTTP traffic with class properties and
Worker handlers. Export `ContainerProxy` when outbound interception is configured.

## Quick Start

```typescript
import { Container, getContainer } from "@cloudflare/containers";

export class MyContainer extends Container {
  defaultPort = 8080;
  sleepAfter = "30m";
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return getContainer(env.MY_CONTAINER, "instance-1").fetch(request);
  }
};
```

## Reading Order

| Task | Files |
|------|-------|
| Setup new Container project | README → configuration.md |
| Implement Container logic | README → api.md → patterns.md |
| Choose a routing pattern | patterns.md |
| Debug issues | gotchas.md |
| Prepare for production | gotchas.md → patterns.md |

## Routing Decision Tree

**How should requests reach Containers?**

- **Same user or session → same Container:** Use `getContainer(binding, sessionId)`
- **Stateless fixed pool:** Use `await getRandom(binding, instanceCount)`
- **One job → one Container:** Use `getContainer(binding, jobId)` and explicit lifecycle control
- **One shared identity:** Use `getContainer(binding)` or a stable name such as `"singleton"`

## When to Use Containers vs Workers

**Use Containers when:**
- The application requires a full Linux environment, filesystem, or system packages
- The workload uses an existing container image or a non-Workers runtime
- The workload needs more CPU, memory, or disk than a Worker isolate provides
- A user, session, or job needs a dedicated compute instance
- The application must run CLI tools or additional processes with `exec()`

**Use Workers when:**
- The workload is a stateless HTTP handler
- Very low cold-start latency is required
- Built-in horizontal scaling is required
- The application fits the Workers runtime and dependency model

## In This Reference

- **[configuration.md](configuration.md)** - Wrangler configuration, instance types, placement, rollout options, properties, and account limits
- **[api.md](api.md)** - Current Container class lifecycle, request, process, state, scheduling, and routing APIs
- **[patterns.md](patterns.md)** - Named and random routing, WebSockets, long-running work, multi-port services, Workflows, and Queues
- **[gotchas.md](gotchas.md)** - Startup, WebSocket, lifecycle, capacity, disk, deployment, and operational constraints

## See Also

- [Durable Objects](../durable-objects/) - Containers extend Durable Objects
- [Workflows](../workflows/) - Orchestrate Container operations
- [Queues](../queues/) - Trigger Containers from queue messages
- [Cloudflare Containers documentation](https://developers.cloudflare.com/containers/)
- [Containers changelog](https://developers.cloudflare.com/changelog/product/containers/)
