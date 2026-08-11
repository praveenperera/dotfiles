---
name: slopstation
description: Deploy, publish, verify, update, or troubleshoot websites and Cloudflare Workers on slopstation.net subdomains. Use when a request names slopstation.net, asks for a host such as app.slopstation.net, needs a source copy under ~/code/slopstation, or needs a browser-free Wrangler deployment to the Slopstation Cloudflare zone. Supports static sites, Workers, Vite or React builds, and vinext or ChatGPT Sites output.
---

# Slopstation

Keep a source copy of each site in `~/code/slopstation`, then deploy it on an exact `slopstation.net` hostname with Wrangler. Keep the project configuration as the source of truth and use Cloudflare Custom Domains when the Worker is the site origin.

Check current Cloudflare documentation or the installed Wrangler help before relying on options that can change.

## Safety boundary

- Treat source copies, deployments, Worker changes, Custom Domain changes, and DNS changes as mutations
- Copy or deploy only when the user asks to deploy, publish, host, update, or archive the site
- Resolve the exact source, archive directory, Worker name, and hostname before mutation
- Require the hostname to equal `slopstation.net` or end in `.slopstation.net`
- Inspect an existing archive directory and preserve unrelated work before updating it
- Do not delete or replace an existing source file, DNS record, route, Worker, or Custom Domain unless the user authorizes that exact change
- Never copy or print API tokens, OAuth credentials, environment files, private keys, or other secrets
- Do not use Browser, Chrome, or Computer Use for the normal workflow

## Tool ownership

Use Wrangler for Worker builds, deployments, versions, and Custom Domains.

Use `cmd cf` only for operations shown by `cmd cf --help`. The known command surface supports account billing reports and canonical redirect rules. Its DNS mutation is limited to the `www` record used by the redirect workflow. It is not a general DNS or arbitrary-subdomain command.

For a Worker Custom Domain, do not create an A or CNAME record first. Wrangler registers the Custom Domain, and Cloudflare creates the required DNS record and certificate.

If a hostname has an incompatible DNS record or another Worker mapping, stop and report the exact conflict. Do not use `cmd cf` to replace it.

## Workflow

### 1. Inspect the project

Inspect these files when present:

- `package.json` and its build or deploy scripts
- lockfiles and the selected package manager
- `wrangler.json`, `wrangler.jsonc`, or `wrangler.toml`
- `.openai/hosting.json`
- framework configuration and generated build output

Determine whether the project is:

- a plain static site
- a Vite or React application
- a Worker with Static Assets
- a vinext or ChatGPT Sites application

Do not assume that a valid site build contains a root `index.html`. A vinext build normally uses:

- `dist/server/wrangler.json`
- `dist/server/index.js`
- `dist/client/`

### 2. Keep the Slopstation source copy

Store each site in a directory under `~/code/slopstation` named from its subdomain. For example:

```text
coldcard.slopstation.net -> ~/code/slopstation/coldcard/
```

For a multi-label subdomain, preserve all labels before `slopstation.net` and join them with hyphens. Use `root` for the apex site.

Copy the deployable source project, not only its generated output. Exclude:

- nested `.git` directories
- `node_modules` and package-manager caches
- generated build and test caches that the build can reproduce
- `.env` files, credentials, private keys, and secrets
- operating-system metadata

If the destination does not exist, create it and copy the source. If it exists, inspect its Git status and compare it with the source before updating it. Preserve unrelated or newer destination changes. Do not use a destructive sync option such as `--delete`.

Build and deploy from the Slopstation copy so that the archived source matches the deployed site. Do not commit or push the Slopstation repository unless the user asks.

### 3. Check authentication

Run:

```bash
npx wrangler whoami
```

Use the existing Wrangler OAuth session when it is available. In non-interactive environments, use a securely configured `CLOUDFLARE_API_TOKEN`. Do not ask the user to paste a token into chat.

If authentication is missing, report the required `wrangler login` or token setup. Do not switch to browser control.

### 4. Build and check the deployment

Run the project's normal install and build commands from its Slopstation copy. Then run an exact dry run with the same configuration, Worker name, and hostname intended for production.

For a project-level Wrangler configuration:

```bash
npx wrangler deploy \
  --name <worker-name> \
  --domain <hostname> \
  --dry-run
```

For generated vinext output:

```bash
npx wrangler deploy \
  --config dist/server/wrangler.json \
  --name <worker-name> \
  --domain <hostname> \
  --dry-run
```

Fix build or configuration errors before deployment. A dry run checks the build output, but it does not prove that the live hostname is free of conflicts.

### 5. Deploy the Custom Domain

Remove only `--dry-run` from the verified command. For vinext:

```bash
npx wrangler deploy \
  --config dist/server/wrangler.json \
  --name <worker-name> \
  --domain <hostname>
```

Do not add a separate DNS record. The `--domain` option attaches the exact hostname as a Worker Custom Domain and lets Cloudflare manage DNS and TLS.

Use a Worker route instead only when the site has a separate origin server. A static site or vinext Worker is the origin, so use a Custom Domain.

### 6. Verify without a browser

Check public DNS through independent resolvers:

```bash
dig @1.1.1.1 <hostname>
dig @8.8.8.8 <hostname>
```

Check HTTPS, redirects, response content, and one built asset when applicable:

```bash
curl --fail --silent --show-error --location \
  --dump-header /dev/stderr \
  https://<hostname>/
```

Verify expected page text in the response. Do not report success from DNS or HTTP status alone.

### 7. Report the result

Report:

- source project and Slopstation copy path
- copied or preserved files and all exclusions
- build command
- Worker name and deployed version when Wrangler provides it
- exact Custom Domain URL
- DNS results
- HTTPS status and expected-content result
- any unresolved conflict or propagation delay
- the exact redeploy command

Do not report a hostname as ready until HTTPS returns the expected site content.
