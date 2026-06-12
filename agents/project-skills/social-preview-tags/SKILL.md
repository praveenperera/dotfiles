---
name: social-preview-tags
description: Audit, debug, and implement social link preview metadata for websites. Use when Open Graph images, Twitter/X cards, Signal/iMessage/Slack/Discord previews, `og:image`, `twitter:image`, crawler access, robots.txt, canonical URLs, or social preview caching are involved.
---

# Social Preview Tags

Use this skill to diagnose link preview failures and make grounded metadata fixes for web apps and static sites.

## Workflow

1. Inspect the actual project code before assuming framework defaults.
2. If a deployed URL exists, inspect the live HTML and headers as crawlers see them.
3. For platform-specific claims, verify current primary docs or official support pages when available; preview rules and validators change.
4. Run the helper script when a public URL is available:

```bash
node /Users/praveen/code/dotfiles/agents/project-skills/social-preview-tags/scripts/inspect-social-preview.mjs https://example.com/page
```

5. Classify findings separately:
   - broken: missing baseline tags, relative image URLs, non-200 page/image, non-image content type, unsupported image format, blocked crawler, oversized image
   - suspicious: canonical URL mismatch, redirects, very large images, incomplete structured image metadata, dimensions outside common card ratios
   - cache-related: tags and image fetch are correct, but a platform still shows stale or missing previews
6. Make the narrowest code change that fixes the observed issue.
7. Rebuild and inspect generated HTML, not only source components.

## Metadata Baseline

For broad previews, prefer static tags in the initial `<head>` HTML:

```html
<link rel="canonical" href="https://example.com/page" />
<meta property="og:title" content="Page title" />
<meta property="og:description" content="Short description" />
<meta property="og:type" content="website" />
<meta property="og:url" content="https://example.com/page" />
<meta property="og:image" content="https://example.com/og-image.png" />
<meta property="og:image:secure_url" content="https://example.com/og-image.png" />
<meta property="og:image:type" content="image/png" />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta property="og:image:alt" content="Image description" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="Page title" />
<meta name="twitter:description" content="Short description" />
<meta name="twitter:image" content="https://example.com/og-image.png" />
<meta name="twitter:image:alt" content="Image description" />
```

Use absolute HTTPS URLs for page and image metadata. Do not rely on client-rendered metadata unless the target crawler is known to execute that JavaScript.

## Calibration

Treat a working preview as evidence that the basics matter most:

- static raw HTML metadata
- `og:title`, `og:description`, `og:type`, `og:url`, `og:image`, `og:image:secure_url`, `og:image:type`, image dimensions, and `og:image:alt`
- `twitter:card=summary_large_image`, `twitter:title`, `twitter:description`, `twitter:image`, and `twitter:image:alt`
- an absolute HTTPS image URL that returns `200` with an image content type
- image dimensions near the common large-card ratio and a reasonably small file size

## Debug Checklist

- Page returns `200` and `content-type: text/html` to a crawler-like user agent.
- Social tags are present in the raw HTML response, not injected after hydration.
- `og:image` and `twitter:image` resolve to absolute HTTPS URLs.
- Image URL returns `200`, an image content type, and no login or hotlink protection response.
- Image format is one commonly accepted by social crawlers: PNG, JPEG, WebP, or static GIF.
- Image is large enough for the requested card type and below platform file-size limits; for large summary cards, `1200x630` is a safe default.
- `robots.txt` does not block the relevant crawler or image path.
- Canonical URL, `og:url`, and the shared URL do not fight each other through unexpected redirects.
- Existing social platforms may cache stale tags or images; distinguish cache from markup defects.

## Answering Root-Cause Questions

Be explicit about confidence. Say "this is likely the root cause" only when the current evidence shows a crawler-visible failure. If the tags and image fetch are correct but a platform still does not render the preview, investigate caching, redirects, and platform-specific crawler behavior before changing unrelated code.

When citing sources, prefer:

- Open Graph protocol for required and structured `og:*` fields
- X/Twitter developer docs or official X support/forum posts for Twitter card behavior
- Signal support for Signal preview behavior
- Apple developer docs for Messages previews

If current official docs are unavailable or incomplete, state that and label third-party validator guidance as secondary evidence.
