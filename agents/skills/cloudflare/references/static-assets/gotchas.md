# Gotchas & Limitations

## `run_worker_first` Default is `false`

Unlike Pages (where Worker/Functions run first by default), Workers static assets serve assets directly without Worker invocation unless configured otherwise.

**Symptom:** Worker code never executes for asset requests.

**Fix:** Set `run_worker_first` to `true` or use patterns:

```jsonc
{
  "assets": {
    "run_worker_first": true
  }
}
```

Or selectively:

```jsonc
{
  "assets": {
    "run_worker_first": ["/api/*", "/auth/*"]
  }
}
```

## Navigation Request Optimization

For SPAs with `not_found_handling: "single-page-application"`, navigation requests (clicking links) can skip Worker invocation entirely for better performance.

**Requires:** `compatibility_date >= "2025-04-01"`

```jsonc
{
  "compatibility_date": "2025-04-01",
  "assets": {
    "not_found_handling": "single-page-application"
  }
}
```

Without this date, all requests go through the Worker (if `run_worker_first` is true).

## `.assetsignore` Not Auto-Populated

Unlike Pages, Workers does NOT automatically ignore files like `.git`, `node_modules`, etc.

**Create `.assetsignore` manually in your assets directory:**

```
# .assetsignore
.git/
node_modules/
*.map
*.ts
.DS_Store
.env
*.test.js
*.spec.js
__tests__/
```

**Symptom:** Large deploy sizes, slow uploads, source maps exposed.

## Workers Sites is Deprecated

Don't confuse with the old "Workers Sites" (using `@cloudflare/kv-asset-handler`). That's deprecated.

**Use Workers Static Assets instead:**

```jsonc
// OLD (deprecated)
{
  "site": {
    "bucket": "./dist"
  }
}

// NEW (use this)
{
  "assets": {
    "directory": "./dist"
  }
}
```

## Port Difference from Pages

- **Workers:** `wrangler dev` uses port **8787**
- **Pages:** `wrangler pages dev` uses port **8788**

Update your development scripts if migrating.

## Custom Domains Require Cloudflare DNS

To use a custom domain, your domain must use Cloudflare's nameservers.

1. Add domain to Cloudflare
2. Update nameservers at registrar
3. Add custom domain in Workers dashboard or via wrangler

```bash
npx wrangler deploy
# Then add domain in dashboard: Workers & Pages → your-worker → Settings → Domains
```

## No Automatic Redirects for Trailing Slashes

Unlike some static hosts, there's no automatic redirect. Configure via `html_handling`:

```jsonc
{
  "assets": {
    "html_handling": "force-trailing-slash"  // /about → /about/
  }
}
```

Or handle in Worker code if you need custom logic.

## Large File Uploads

Individual files are limited to 25 MiB. For larger files, use R2.

**Symptom:** Deploy fails with size error.

**Fix:** Store large assets in R2 and fetch in Worker:

```typescript
const largeFile = await env.R2_BUCKET.get("large-video.mp4");
return new Response(largeFile?.body);
```

## Asset Path Resolution

`ASSETS.fetch()` uses the path from the URL, not the full URL. The domain is ignored.

```typescript
// These are equivalent:
env.ASSETS.fetch("https://example.com/logo.png");
env.ASSETS.fetch("https://anything.local/logo.png");
env.ASSETS.fetch("/logo.png");
```

## 404 Handling with `run_worker_first: false`

If `run_worker_first` is `false` (default) and an asset doesn't exist, the response depends on `not_found_handling`:

- `"none"`: 404 response (no Worker invocation)
- `"single-page-application"`: Serves `/index.html`
- `"404-page"`: Serves `/404.html`

Your Worker code won't run for 404s unless `run_worker_first` includes the path or is `true`.

## Headers on Static Assets

Without a Worker, you can't add custom headers to static assets. If you need security headers, caching headers, or CORS:

```jsonc
{
  "main": "src/index.ts",
  "assets": {
    "binding": "ASSETS",
    "run_worker_first": true
  }
}
```

Then add headers in Worker code.

## Framework Build Output Directories

Common build output directories:

| Framework | Directory |
|-----------|-----------|
| Vite | `./dist` |
| Create React App | `./build` |
| Next.js (static export) | `./out` |
| Astro | `./dist` |
| SvelteKit (static) | `./build` |
| Nuxt (static) | `./dist` |
| Hugo | `./public` |
| 11ty | `./_site` |

## TypeScript: Missing `Fetcher` Type

If `ASSETS` binding type isn't recognized:

```bash
npm install -D @cloudflare/workers-types
```

```jsonc
// tsconfig.json
{
  "compilerOptions": {
    "types": ["@cloudflare/workers-types"]
  }
}
```

Or generate types:

```bash
npx wrangler types
```

## Environment Variables in Static Assets

Static assets are built at deploy time. Environment variables must be injected during build, not runtime.

**For Vite:**
```bash
VITE_API_URL=https://api.example.com npm run build
```

**For runtime config, use a Worker:**

```typescript
// src/index.ts
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/config.js") {
      return new Response(`window.CONFIG = { apiUrl: "${env.API_URL}" }`, {
        headers: { "Content-Type": "application/javascript" }
      });
    }

    return env.ASSETS.fetch(request);
  }
};
```
