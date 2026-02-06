# Workers Static Assets

**The recommended way to deploy static websites on Cloudflare.** Workers with static assets replaces Pages for new static site deployments.

## Why Workers for Static Sites?

Cloudflare now recommends Workers static assets over Pages for new projects:

- **Completely free**: Pure static deployments (no `main` entry point) have no request limits or costs
- **Broader feature access**: Durable Objects, Cron Triggers, Queues, Email Workers
- **Better observability**: Full Workers analytics and logging
- **Unified platform**: Same tooling for static and dynamic workloads
- **Active development**: Workers receives new features; Pages is in maintenance mode

## Default Approach: Pure Static

**Start with pure static (no Worker code) - it's free and sufficient for most static sites. Add a Worker only when you need server-side logic.**

- **Pure static** (no `main` entry point): Static asset requests are **free**, unlimited, not counted as Worker invocations
- **With Worker code** (`main` + `run_worker_first`): Requests that invoke Worker code count against Workers pricing (free tier: 100k requests/day, then $0.30/million)

## When to Use

| Use Case | Approach |
|----------|----------|
| Pure static site (HTML/CSS/JS) | Workers static assets (no Worker code) |
| Single-page application (SPA) | Workers + `not_found_handling: "single-page-application"` |
| Static site generator (SSG) | Workers + `not_found_handling: "404-page"` |
| Full-stack app with static assets | Workers + `ASSETS` binding |
| Existing Pages project | Continue using Pages (migration optional) |

## Quick Start

### Pure Static Site (No Worker Code)

```bash
# Create project
mkdir my-site && cd my-site
mkdir dist
echo '<h1>Hello World</h1>' > dist/index.html
```

```jsonc
// wrangler.jsonc
{
  "name": "my-site",
  "compatibility_date": "2025-01-01",
  "assets": {
    "directory": "./dist"
  }
}
```

```bash
# Deploy
npx wrangler deploy
```

### SPA (React, Vue, etc)

```jsonc
// wrangler.jsonc
{
  "name": "my-spa",
  "compatibility_date": "2025-01-01",
  "assets": {
    "directory": "./dist",
    "not_found_handling": "single-page-application"
  }
}
```

### Full-Stack with API Routes

```jsonc
// wrangler.jsonc
{
  "name": "my-app",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",
  "assets": {
    "directory": "./dist",
    "binding": "ASSETS",
    "run_worker_first": ["/api/*"]
  }
}
```

```typescript
// src/index.ts
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname.startsWith("/api/")) {
      return new Response(JSON.stringify({ message: "API" }), {
        headers: { "Content-Type": "application/json" }
      });
    }

    return env.ASSETS.fetch(request);
  }
};
```

## Commands

```bash
# Local development
npx wrangler dev

# Deploy
npx wrangler deploy

# Auto-config mode (for frameworks)
npx wrangler deploy --x-autoconfig
```

## Resources

- [Static Assets Docs](https://developers.cloudflare.com/workers/static-assets/)
- [Migration from Pages](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/)
- [Configuration Reference](https://developers.cloudflare.com/workers/static-assets/configuration/)

## In This Reference

- [configuration.md](./configuration.md) - wrangler.jsonc options, deployment
- [api.md](./api.md) - ASSETS.fetch() binding API
- [patterns.md](./patterns.md) - SPA, SSG, hybrid, migration patterns
- [gotchas.md](./gotchas.md) - Common pitfalls, limitations
