# Cloudflare Dashboard Redirects

Use this workflow when a user asks to make one hostname canonical in Cloudflare, especially `www` to apex or apex to `www`, without Worker JavaScript.

Prefer Terraform or the local `cmd cloudflare redirect` API workflow when credentials and scopes are available. Use the dashboard with Computer Use when the available token cannot edit DNS or Rulesets, when the user explicitly asks to use the logged-in Cloudflare dashboard, or when existing Chrome session state is required.

## Tool Choice

- Try `cmd cf redirect` first when the local command is available and `CMD_CLOUDFLARE_REDIRECT_API_TOKEN` is set. Use `cmd cloudflare redirect` interchangeably in docs or scripts when the long form is clearer.
- Use Computer Use with Chrome for Cloudflare dashboard actions that depend on the user's logged-in browser session.
- Use Browser only for verification or local web inspection when a browser view is useful; do not use Browser for Cloudflare dashboard auth unless that is where the user is logged in.
- Use shell checks such as `curl -sS -I` and DNS lookups for verification. If DNS/network commands fail due to sandboxing, rerun with escalated permissions.
- Do not use Worker JavaScript for simple host canonicalization unless the user explicitly asks for it.

## Local Cmd API Workflow

Use `cmd cloudflare redirect` before the dashboard when this dotfiles command is available and `CMD_CLOUDFLARE_REDIRECT_API_TOKEN` is set. The token should have zone-scoped `DNS:Edit`, `Single Redirect:Edit`, and `Zone:Read`; pass `--zone-id` when zone lookup is not allowed.

Inspect existing redirects before changing them:

```bash
cmd cf redirect list example.com
```

Preview the change first:

```bash
cmd cf redirect www-to-apex example.com --ensure-www-dns --dry-run
cmd cf redirect apex-to-www example.com --dry-run
```

Apply the change:

```bash
cmd cf redirect www-to-apex example.com --ensure-www-dns
cmd cf redirect apex-to-www example.com
```

Notes:

- The positional argument can be a zone apex, hostname, or URL. The command resolves the Cloudflare zone by trying hostname suffixes from longest to shortest.
- `--ensure-www-dns` creates or proxies `www.<zone>` as a proxied CNAME to the zone apex. Use it for `www-to-apex` when `www` does not already resolve through Cloudflare.
- The command creates or updates the zone-level Single Redirect entrypoint ruleset without replacing unrelated redirect rules.
- Keep `--api-token` for one-off overrides; do not reuse a broad `CLOUDFLARE_API_TOKEN` unless the user explicitly asks for general Cloudflare credentials.

## Preflight

1. Read local project config before changing Cloudflare:
   - `wrangler.toml` for `name`, `routes`, `custom_domain`, and static assets
   - Terraform files for existing `cloudflare_ruleset` or DNS records
   - site code/config that lists companion domains, if the task mentions related sites
2. Check live behavior:
   - `curl -sS -I https://www.example.com/`
   - `curl -sS -I 'https://www.example.com/path?x=1'`
   - `curl -sS -I https://example.com/`
3. Confirm whether the source hostname resolves. A Redirect Rule cannot run if Cloudflare never receives the request.

## Dashboard Workflow: WWW To Apex

1. Open Chrome with Computer Use and navigate to:
   `https://dash.cloudflare.com/<account-id>/<zone>/rules/overview`
2. Open `Rules` -> `Overview` -> `Templates`.
3. Choose `Redirect from WWW to root`.
4. Configure the rule:
   - Rule name: `Redirect www to apex`
   - Match type: `Wildcard pattern`
   - Request URL: `http*://www.example.com/*`
   - Target URL: `https://example.com/${2}`
   - Status code: `301 - Permanent Redirect`
   - Preserve query string: enabled
5. Deploy the rule.
6. If Cloudflare warns that the rule may not apply because DNS may not be proxied, inspect DNS before proceeding. It is acceptable to choose "Ignore and deploy rule anyway" only when there is already a proxied Worker/custom-domain/DNS record for the source hostname.

## Dashboard Workflow: Apex To WWW

Use the same flow but invert the hosts:

- Request URL: `http*://example.com/*`
- Target URL: `https://www.example.com/${2}`
- Rule name: `Redirect apex to www`

Ensure `www.example.com` resolves and is proxied before deploying.

## Missing WWW DNS

If `www.example.com` does not resolve but there is already a redirect rule for it:

1. Open `DNS` -> `Records`.
2. Add a proxied record:
   - Type: `CNAME`
   - Name: `www`
   - Target: `example.com`
   - Proxy status: `Proxied`
   - TTL: `Auto`
3. Save the record.
4. Verify DNS propagation with a public resolver if the local resolver has cached NXDOMAIN.

Do not add DNS records for unrelated domains. If the user says "companion sites," find the authoritative list in local project code/config before deciding which zones to touch.

## Verification

Verify both the redirect and non-redirect host:

```bash
curl -sS -I https://www.example.com/
curl -sS -I 'https://www.example.com/some/path?x=1'
curl -sS -I https://example.com/
```

Expected results for `www` to apex:

- `www` returns `301`
- `Location` points to the apex
- path and query string are preserved
- apex remains `200` or the site's expected non-redirect response

If local DNS still says NXDOMAIN after adding a Cloudflare record, check public DNS and force `curl` through a returned Cloudflare IP:

```bash
dig +short @1.1.1.1 www.example.com A
curl -sS -I --resolve www.example.com:443:<cloudflare-ip> https://www.example.com/
```

## Reporting

In the final response, state:

- which zones were changed
- exact redirect direction and status code
- whether path/query preservation was verified
- any sites skipped because the host did not resolve or was not in the local companion list

Do not claim a redirect is fixed until live verification succeeds or DNS propagation is clearly the only remaining delay and public DNS already resolves.
