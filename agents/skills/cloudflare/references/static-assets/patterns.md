# Patterns

## Pure Static Site (No Worker Code)

Simplest deployment - no `main` entry point needed:

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

```
dist/
├── index.html
├── about.html
├── styles.css
└── script.js
```

## Single-Page Application (SPA)

Client-side routing with fallback to index.html:

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

All non-asset routes serve `/index.html`, allowing React Router, Vue Router, etc to handle routing.

## Static Site Generator (SSG)

Pre-rendered pages with custom 404:

```jsonc
// wrangler.jsonc
{
  "name": "my-blog",
  "compatibility_date": "2025-01-01",
  "assets": {
    "directory": "./dist",
    "not_found_handling": "404-page"
  }
}
```

Requires `/404.html` in your dist folder. Works with Astro, Hugo, 11ty, Jekyll.

## Hybrid: Static Assets + API Routes

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
      return handleApi(request, env);
    }

    // Fallback to static assets (shouldn't reach here due to run_worker_first)
    return env.ASSETS.fetch(request);
  }
};

async function handleApi(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url);

  if (url.pathname === "/api/health") {
    return Response.json({ status: "ok" });
  }

  if (url.pathname === "/api/data" && request.method === "GET") {
    return Response.json({ items: [] });
  }

  return new Response("Not Found", { status: 404 });
}
```

## Adding Headers/Caching to Assets

```typescript
// src/index.ts
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const response = await env.ASSETS.fetch(request);
    const modified = new Response(response.body, response);

    const url = new URL(request.url);

    // Immutable caching for hashed assets
    if (url.pathname.match(/\.[a-f0-9]{8}\.(js|css|woff2?)$/)) {
      modified.headers.set("Cache-Control", "public, max-age=31536000, immutable");
    }

    // Security headers
    modified.headers.set("X-Content-Type-Options", "nosniff");
    modified.headers.set("X-Frame-Options", "DENY");

    return modified;
  }
};
```

Config to run Worker for all requests:

```jsonc
{
  "assets": {
    "directory": "./dist",
    "binding": "ASSETS",
    "run_worker_first": true
  }
}
```

## OAuth Callback for SPAs

Handle OAuth redirects server-side, then redirect to SPA:

```typescript
// src/index.ts
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/auth/callback") {
      const code = url.searchParams.get("code");

      // Exchange code for token
      const tokenResponse = await fetch("https://oauth.provider.com/token", {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({
          code: code!,
          client_id: env.CLIENT_ID,
          client_secret: env.CLIENT_SECRET,
          redirect_uri: `${url.origin}/auth/callback`,
          grant_type: "authorization_code"
        })
      });

      const { access_token } = await tokenResponse.json();

      // Redirect to SPA with token in fragment (or set cookie)
      return Response.redirect(`${url.origin}/#token=${access_token}`, 302);
    }

    return env.ASSETS.fetch(request);
  }
};
```

## Migration from Pages

### Step 1: Create wrangler.jsonc

```jsonc
{
  "name": "my-pages-site",
  "compatibility_date": "2025-01-01",
  "assets": {
    "directory": "./dist",
    "not_found_handling": "single-page-application"
  }
}
```

### Step 2: Move Functions to Worker

If you have `/functions/api/users.ts`:

```typescript
// Old Pages Function
export const onRequestGet: PagesFunction = async (context) => {
  return Response.json({ users: [] });
};
```

Convert to Worker:

```typescript
// src/index.ts
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/users" && request.method === "GET") {
      return Response.json({ users: [] });
    }

    return env.ASSETS.fetch(request);
  }
};
```

### Step 3: Update Config

```jsonc
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

### Step 4: Deploy

```bash
npx wrangler deploy
```

### Key Differences from Pages

| Pages | Workers Static Assets |
|-------|----------------------|
| `wrangler pages deploy` | `wrangler deploy` |
| Port 8788 | Port 8787 |
| Functions in `/functions/` | Worker in `main` entry |
| Auto-populated `.gitignore` | Manual `.assetsignore` |
| `run_worker_first` default `true` | Default `false` |

## A/B Testing with Assets

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/") {
      // 50/50 split based on cookie or random
      const cookie = request.headers.get("Cookie") || "";
      const variant = cookie.includes("variant=b") ? "b" :
                      Math.random() < 0.5 ? "a" : "b";

      const page = variant === "b" ? "/index-b.html" : "/index.html";
      const response = await env.ASSETS.fetch(page);

      // Set variant cookie if not present
      if (!cookie.includes("variant=")) {
        const modified = new Response(response.body, response);
        modified.headers.set("Set-Cookie", `variant=${variant}; Path=/; Max-Age=86400`);
        return modified;
      }

      return response;
    }

    return env.ASSETS.fetch(request);
  }
};
```

## Redirects with Terraform

Use Cloudflare Bulk Redirects via Terraform for managing redirects without Worker code:

```hcl
# Create a redirect list
resource "cloudflare_list" "redirects" {
  account_id  = var.cloudflare_account_id
  name        = "my_site_redirects"
  kind        = "redirect"
  description = "Redirects for my-site.com"

  item {
    value {
      redirect {
        source_url            = "my-site.com/old-page"
        target_url            = "https://my-site.com/new-page"
        status_code           = 301
        include_subdomains    = "disabled"
        subpath_matching      = "disabled"
        preserve_query_string = "enabled"
        preserve_path_suffix  = "disabled"
      }
    }
  }

  item {
    value {
      redirect {
        source_url            = "my-site.com/blog/*"
        target_url            = "https://my-site.com/articles/"
        status_code           = 302
        subpath_matching      = "enabled"
        preserve_path_suffix  = "enabled"
      }
    }
  }
}

# Enable the redirect list as a bulk redirect rule
resource "cloudflare_ruleset" "redirects" {
  account_id  = var.cloudflare_account_id
  name        = "My Site Redirects"
  kind        = "root"
  phase       = "http_request_redirect"

  rules {
    action = "redirect"
    action_parameters {
      from_list {
        name = cloudflare_list.redirects.name
        key  = "http.request.full_uri"
      }
    }
    expression  = "http.request.full_uri in $${cloudflare_list.redirects.name}"
    description = "Apply bulk redirects"
    enabled     = true
  }
}
```

For domain-level redirects (e.g., `www` to apex):

```hcl
resource "cloudflare_record" "www_redirect" {
  zone_id = var.cloudflare_zone_id
  name    = "www"
  type    = "CNAME"
  content = "my-site.com"
  proxied = true
}

resource "cloudflare_ruleset" "www_to_apex" {
  zone_id = var.cloudflare_zone_id
  name    = "Redirect www to apex"
  kind    = "zone"
  phase   = "http_request_dynamic_redirect"

  rules {
    action = "redirect"
    action_parameters {
      from_value {
        status_code = 301
        target_url {
          expression = "concat(\"https://my-site.com\", http.request.uri.path)"
        }
      }
    }
    expression  = "(http.host eq \"www.my-site.com\")"
    description = "Redirect www to apex domain"
    enabled     = true
  }
}
```

## Serving Pre-compressed Assets

If your build outputs `.br` or `.gz` files:

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const acceptEncoding = request.headers.get("Accept-Encoding") || "";
    const url = new URL(request.url);

    // Try Brotli first
    if (acceptEncoding.includes("br")) {
      const brResponse = await env.ASSETS.fetch(`${url.pathname}.br`);
      if (brResponse.ok) {
        const modified = new Response(brResponse.body, brResponse);
        modified.headers.set("Content-Encoding", "br");
        return modified;
      }
    }

    // Try gzip
    if (acceptEncoding.includes("gzip")) {
      const gzResponse = await env.ASSETS.fetch(`${url.pathname}.gz`);
      if (gzResponse.ok) {
        const modified = new Response(gzResponse.body, gzResponse);
        modified.headers.set("Content-Encoding", "gzip");
        return modified;
      }
    }

    return env.ASSETS.fetch(request);
  }
};
```
