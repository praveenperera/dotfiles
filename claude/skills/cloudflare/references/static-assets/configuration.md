# Configuration

## Minimal Config (Pure Static)

No Worker code needed - just point to your build directory:

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

## Full Configuration Options

```jsonc
{
  "name": "my-app",
  "main": "src/index.ts",
  "compatibility_date": "2025-04-01",
  "assets": {
    "directory": "./dist",
    "binding": "ASSETS",
    "not_found_handling": "single-page-application",
    "html_handling": "auto-trailing-slash",
    "run_worker_first": ["/api/*", "!/api/docs/*"]
  }
}
```

## Configuration Keys

### `directory` (required)

Path to static assets folder relative to wrangler.jsonc:

```jsonc
{ "assets": { "directory": "./dist" } }      // Vite, Astro
{ "assets": { "directory": "./build" } }     // Create React App
{ "assets": { "directory": "./public" } }    // Custom
{ "assets": { "directory": "./.next/static" } } // Next.js
```

### `binding` (optional)

Name to access assets in Worker code. Only needed if you have a Worker (`main`):

```jsonc
{ "assets": { "binding": "ASSETS" } }
```

```typescript
// Access via env.ASSETS.fetch()
const response = await env.ASSETS.fetch(request);
```

### `not_found_handling` (optional)

Behavior when requested asset doesn't exist:

| Value | Behavior | Use Case |
|-------|----------|----------|
| `"none"` (default) | 404 response | APIs, exact-match only |
| `"single-page-application"` | Serve `/index.html` | React, Vue, Angular SPAs |
| `"404-page"` | Serve `/404.html` | SSG sites (Astro, Hugo, 11ty) |

```jsonc
// SPA configuration
{
  "assets": {
    "directory": "./dist",
    "not_found_handling": "single-page-application"
  }
}
```

```jsonc
// SSG configuration
{
  "assets": {
    "directory": "./dist",
    "not_found_handling": "404-page"
  }
}
```

### `html_handling` (optional)

How to handle HTML file requests:

| Value | Behavior |
|-------|----------|
| `"auto-trailing-slash"` (default) | `/about` serves `/about/index.html`, redirects `/about/` → `/about` |
| `"force-trailing-slash"` | `/about` redirects to `/about/`, serves `/about/index.html` |
| `"drop-trailing-slash"` | `/about/` redirects to `/about`, serves `/about.html` |
| `"none"` | Exact path matching only |

### `run_worker_first` (optional)

Controls when Worker code runs vs direct asset serving:

```jsonc
// Boolean: all requests go to Worker first
{ "assets": { "run_worker_first": true } }

// Array: only matching patterns go to Worker
{
  "assets": {
    "run_worker_first": [
      "/api/*",           // API routes
      "/admin/*",         // Admin area
      "!/admin/assets/*"  // Except admin static assets
    ]
  }
}
```

**Pattern syntax:**
- `*` matches any characters except `/`
- `**` matches any characters including `/`
- `!` prefix excludes (negates) the pattern

**Default is `false`** - assets served directly without Worker invocation (opposite of Pages behavior).

## `.assetsignore` File

Exclude files from upload. Create `.assetsignore` in your assets directory:

```
# .assetsignore
*.map
*.ts
.DS_Store
node_modules/
*.test.js
README.md
```

Unlike Pages, this file is NOT auto-populated. You must create it manually.

## Deployment Commands

```bash
# Standard deploy
npx wrangler deploy

# Auto-config for frameworks (experimental)
npx wrangler deploy --x-autoconfig

# Deploy to specific environment
npx wrangler deploy --env production

# Dry run (see what would deploy)
npx wrangler deploy --dry-run
```

## Local Development

```bash
# Start dev server (port 8787)
npx wrangler dev

# With custom port
npx wrangler dev --port 3000

# With local persistence for bindings
npx wrangler dev --persist-to .wrangler/state
```

## Environment Variables

```jsonc
{
  "vars": {
    "API_URL": "https://api.example.com"
  }
}
```

For secrets:
```bash
npx wrangler secret put API_KEY
```

## TypeScript Types

```bash
npx wrangler types
```

Generates `worker-configuration.d.ts`:

```typescript
interface Env {
  ASSETS: Fetcher;
  API_KEY: string;
}
```
