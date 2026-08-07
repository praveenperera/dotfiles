# Container Patterns

## Stable Instance per User or Session

Use an authenticated identifier or another stable application key. Do not accept an untrusted key
that lets one tenant address another tenant's Container.

```typescript
import { Container, getContainer } from "@cloudflare/containers";

export class SessionBackend extends Container {
  defaultPort = 3000;
  sleepAfter = "30m";
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const sessionId = await authenticatedSessionId(request);
    return getContainer(env.SESSION_BACKEND, sessionId).fetch(request);
  }
};
```

Use for user sessions, game rooms, per-tenant tools, and other individually addressable workloads.

## Fixed Stateless Pool

```typescript
import { getRandom } from "@cloudflare/containers";

const INSTANCE_COUNT = 5;

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const container = await getRandom(env.STATELESS_API, INSTANCE_COUNT);
    return container.fetch(request);
  }
};
```

`getRandom()` selects randomly from a fixed pool. It is not latency-aware and does not change the
pool size from demand. Use it only when each instance can serve any request.

## Shared Singleton Identity

```typescript
import { getContainer } from "@cloudflare/containers";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return getContainer(env.SHARED_SERVICE).fetch(request);
  }
};
```

This creates one logical identity, not a permanently running process or a fixed global location.
Cloudflare can restart the Container in another location.

## WebSocket Forwarding

Use `fetch()`, not `containerFetch()`, for a WebSocket upgrade:

```typescript
import { getContainer } from "@cloudflare/containers";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
      return new Response("WebSocket upgrade required", { status: 426 });
    }

    const sessionId = await authenticatedSessionId(request);
    return getContainer(env.WS_BACKEND, sessionId).fetch(request);
  }
};
```

Use `switchPort(request, port)` before `fetch()` when the WebSocket service is not on `defaultPort`.

## Long-Running Background Work

Incoming requests reset `sleepAfter` automatically. Background work does not. Renew the activity
timeout while work is active:

```typescript
export class LongRunningContainer extends Container {
  sleepAfter = "5m";

  async processLongJob(data: unknown): Promise<void> {
    const interval = setInterval(() => this.renewActivityTimeout(), 60_000);

    try {
      await this.doLongWork(data);
    } finally {
      clearInterval(interval);
    }
  }
}
```

This keeps the Container active. It does not make the job durable. Use Workflows when work must
resume after failures.

## Idle Shutdown Policy

The default `onActivityExpired()` calls `stop()`. Override it only when the application has another
reliable source of activity state:

```typescript
override async onActivityExpired(): Promise<void> {
  if (await this.hasActiveJobs()) {
    this.renewActivityTimeout();
    return;
  }

  await this.stop();
}
```

## Multiple Ports

For HTTP, pass a port to `containerFetch()`. For WebSockets, transform the request with the exported
`switchPort()` helper and keep using `fetch()`.

```typescript
import { getContainer, switchPort } from "@cloudflare/containers";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const container = getContainer(env.MULTI_PORT);
    const path = new URL(request.url).pathname;

    if (path.startsWith("/metrics")) {
      return container.fetch(switchPort(request, 9090));
    }

    return container.fetch(request);
  }
};
```

Inside a Container subclass, use `this.containerFetch(request, 9090)` for HTTP-only traffic.

## Workflow Integration

Use a stable job ID so retries reach the same logical Container. Keep step results serializable.

```typescript
import { getContainer } from "@cloudflare/containers";
import { WorkflowEntrypoint } from "cloudflare:workers";

export class ProcessingWorkflow extends WorkflowEntrypoint<Env, JobParams> {
  override async run(event, step): Promise<unknown> {
    const container = getContainer(this.env.PROCESSOR, event.payload.jobId);

    return step.do("process", async () => {
      const response = await container.fetch("https://container/process", {
        method: "POST",
        body: JSON.stringify(event.payload.data)
      });

      if (!response.ok) {
        throw new Error(`Container returned ${response.status}`);
      }

      return response.json();
    });
  }
}
```

## Queue Consumer Integration

```typescript
import { getContainer } from "@cloudflare/containers";

export default {
  async queue(batch: MessageBatch<JobMessage>, env: Env): Promise<void> {
    for (const message of batch.messages) {
      try {
        const container = getContainer(env.PROCESSOR, message.body.jobId);
        const response = await container.fetch("https://container/process", {
          method: "POST",
          body: JSON.stringify(message.body)
        });

        if (!response.ok) {
          message.retry();
          continue;
        }

        message.ack();
      } catch (error) {
        console.error("Queue processing error", error);
        message.retry();
      }
    }
  }
};
```

Make the Container operation idempotent because Queue delivery and Workflow steps can retry.
