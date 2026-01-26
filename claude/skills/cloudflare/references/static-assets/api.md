# API Reference

## ASSETS Binding

The `ASSETS` binding provides programmatic access to static assets from Worker code.

### Setup

```jsonc
// wrangler.jsonc
{
  "main": "src/index.ts",
  "assets": {
    "directory": "./dist",
    "binding": "ASSETS"
  }
}
```

### TypeScript Types

```typescript
interface Env {
  ASSETS: Fetcher;
}
```

The `Fetcher` type is built into `@cloudflare/workers-types`.

## `ASSETS.fetch()`

Fetch an asset from the static assets directory.

### Signatures

```typescript
// Forward incoming request
env.ASSETS.fetch(request: Request): Promise<Response>

// Fetch by URL string
env.ASSETS.fetch(url: string): Promise<Response>

// Fetch by URL object
env.ASSETS.fetch(url: URL): Promise<Response>

// Fetch with request options
env.ASSETS.fetch(url: string | URL, init?: RequestInit): Promise<Response>
```

### Examples

**Forward request directly:**
```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return env.ASSETS.fetch(request);
  }
};
```

**Fetch specific asset:**
```typescript
// URL string (domain doesn't matter, only path is used)
const logo = await env.ASSETS.fetch("https://placeholder/logo.png");

// Relative-style path
const css = await env.ASSETS.fetch("/styles/main.css");
```

**Fetch with URL object:**
```typescript
const url = new URL(request.url);
url.pathname = "/fallback.html";
const fallback = await env.ASSETS.fetch(url);
```

**Fetch with modified request:**
```typescript
const url = new URL(request.url);
url.pathname = "/index.html";
const newRequest = new Request(url, request);
return env.ASSETS.fetch(newRequest);
```

## Response Handling

`ASSETS.fetch()` returns a standard `Response` object:

```typescript
const response = await env.ASSETS.fetch(request);

// Check status
if (response.status === 404) {
  return new Response("Not Found", { status: 404 });
}

// Read body
const text = await response.text();
const json = await response.json();
const buffer = await response.arrayBuffer();

// Clone for multiple reads
const cloned = response.clone();
```

### Modifying Response Headers

```typescript
const response = await env.ASSETS.fetch(request);
const modified = new Response(response.body, response);

modified.headers.set("Cache-Control", "public, max-age=31536000");
modified.headers.set("X-Custom-Header", "value");
modified.headers.delete("X-Powered-By");

return modified;
```

### Streaming Response

```typescript
const response = await env.ASSETS.fetch(request);
const { readable, writable } = new TransformStream();

response.body?.pipeTo(writable);

return new Response(readable, {
  headers: response.headers
});
```

## Error Handling

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    try {
      const response = await env.ASSETS.fetch(request);

      if (response.status === 404) {
        // Serve custom 404 page
        return env.ASSETS.fetch("/404.html");
      }

      return response;
    } catch (error) {
      return new Response("Internal Error", { status: 500 });
    }
  }
};
```

## Common Patterns

### Conditional Asset Serving

```typescript
const url = new URL(request.url);

if (url.pathname === "/") {
  // Serve different homepage based on condition
  const page = isLoggedIn ? "/dashboard.html" : "/index.html";
  return env.ASSETS.fetch(page);
}

return env.ASSETS.fetch(request);
```

### Asset with Custom MIME Type

```typescript
const response = await env.ASSETS.fetch("/data.json");
const modified = new Response(response.body, response);
modified.headers.set("Content-Type", "application/json; charset=utf-8");
return modified;
```

### Proxy to Assets with Path Rewrite

```typescript
const url = new URL(request.url);

// /v2/docs/intro → /docs/intro
if (url.pathname.startsWith("/v2/")) {
  url.pathname = url.pathname.replace("/v2", "");
  return env.ASSETS.fetch(url);
}

return env.ASSETS.fetch(request);
```
