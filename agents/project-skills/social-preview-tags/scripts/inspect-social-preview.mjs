#!/usr/bin/env node

const userAgent =
  "Mozilla/5.0 (compatible; SocialPreviewAudit/1.0; +https://openai.com)";

const target = process.argv[2];

if (!target) {
  console.error("Usage: inspect-social-preview.mjs <https-url>");
  process.exit(2);
}

let targetUrl;
try {
  targetUrl = new URL(target);
} catch {
  console.error(`Invalid URL: ${target}`);
  process.exit(2);
}

const fetchOptions = {
  redirect: "follow",
  headers: { "user-agent": userAgent, accept: "text/html,*/*;q=0.8" },
};

const pageResponse = await fetch(targetUrl, fetchOptions);
const html = await pageResponse.text();
const pageContentType = pageResponse.headers.get("content-type") ?? "";
const tags = extractMetaTags(html);
const canonical = extractCanonical(html, pageResponse.url);
const imageUrl =
  first(tags, "property", "og:image") ??
  first(tags, "name", "twitter:image") ??
  first(tags, "property", "og:image:url");

const report = {
  page: {
    requestedUrl: targetUrl.href,
    finalUrl: pageResponse.url,
    status: pageResponse.status,
    contentType: pageContentType,
    canonical,
  },
  tags: summarizeTags(tags),
  checks: [],
};

check(
  report,
  pageResponse.ok,
  `page status is ${pageResponse.status}`,
  "page must return 2xx to crawlers",
);
check(
  report,
  pageContentType.includes("text/html"),
  `page content-type is ${pageContentType || "missing"}`,
  "page should return text/html",
);

for (const required of ["og:title", "og:type", "og:image", "og:url"]) {
  check(
    report,
    Boolean(first(tags, "property", required)),
    `${required} present`,
    `${required} missing`,
  );
}

for (const recommended of [
  "og:description",
  "og:image:secure_url",
  "og:image:type",
  "og:image:width",
  "og:image:height",
  "og:image:alt",
]) {
  check(
    report,
    Boolean(first(tags, "property", recommended)),
    `${recommended} present`,
    `${recommended} missing`,
  );
}

check(
  report,
  first(tags, "name", "twitter:card") === "summary_large_image" ||
    first(tags, "name", "twitter:card") === "summary",
  "twitter:card present",
  "twitter:card missing or unsupported",
);

for (const recommended of [
  "twitter:title",
  "twitter:description",
  "twitter:image",
  "twitter:image:alt",
]) {
  check(
    report,
    Boolean(first(tags, "name", recommended)),
    `${recommended} present`,
    `${recommended} missing`,
  );
}

if (imageUrl) {
  let resolvedImageUrl;
  try {
    resolvedImageUrl = new URL(imageUrl, pageResponse.url);
    report.image = await inspectImage(resolvedImageUrl);
    check(
      report,
      resolvedImageUrl.protocol === "https:",
      `image URL uses ${resolvedImageUrl.protocol}`,
      "image URL should use https",
    );
    check(
      report,
      report.image.status >= 200 && report.image.status < 300,
      `image status is ${report.image.status}`,
      "image must return 2xx",
    );
    check(
      report,
      report.image.contentType.startsWith("image/"),
      `image content-type is ${report.image.contentType || "missing"}`,
      "image content-type should be image/*",
    );
    check(
      report,
      report.image.bytes <= 5 * 1024 * 1024,
      `image size is ${report.image.bytes} bytes`,
      "image should usually be under 5 MB",
    );
    if (report.image.width && report.image.height) {
      check(
        report,
        report.image.width >= 300 && report.image.height >= 157,
        `image dimensions are ${report.image.width}x${report.image.height}`,
        "image is below common large-card minimum dimensions",
      );
    }
  } catch (error) {
    report.checks.push({
      ok: false,
      message: `failed to inspect image: ${error.message}`,
    });
  }
} else {
  report.checks.push({ ok: false, message: "no og:image or twitter:image found" });
}

report.robots = await inspectRobots(targetUrl);

console.log(JSON.stringify(report, null, 2));

function extractMetaTags(html) {
  const tags = [];
  const metaRegex = /<meta\s+[^>]*>/gi;
  for (const match of html.matchAll(metaRegex)) {
    const attrs = extractAttributes(match[0]);
    if (attrs.property || attrs.name) {
      tags.push(attrs);
    }
  }
  return tags;
}

function extractCanonical(html, baseUrl) {
  const linkRegex = /<link\s+[^>]*>/gi;
  for (const match of html.matchAll(linkRegex)) {
    const attrs = extractAttributes(match[0]);
    if (attrs.rel?.toLowerCase() === "canonical" && attrs.href) {
      return new URL(attrs.href, baseUrl).href;
    }
  }
  return null;
}

function extractAttributes(tag) {
  const attrs = {};
  const attrRegex = /([:@\w-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g;
  for (const match of tag.matchAll(attrRegex)) {
    attrs[match[1].toLowerCase()] = decodeEntities(
      match[2] ?? match[3] ?? match[4] ?? "",
    );
  }
  return attrs;
}

function decodeEntities(value) {
  return value
    .replaceAll("&amp;", "&")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">");
}

function first(tags, attr, key) {
  return tags.find((tag) => tag[attr]?.toLowerCase() === key.toLowerCase())
    ?.content;
}

function summarizeTags(tags) {
  const wanted = new Set([
    "og:title",
    "og:description",
    "og:type",
    "og:url",
    "og:image",
    "og:image:secure_url",
    "og:image:type",
    "og:image:width",
    "og:image:height",
    "og:image:alt",
    "twitter:card",
    "twitter:title",
    "twitter:description",
    "twitter:image",
    "twitter:image:alt",
  ]);
  return tags
    .filter((tag) => wanted.has((tag.property ?? tag.name ?? "").toLowerCase()))
    .map((tag) => ({
      key: tag.property ?? tag.name,
      content: tag.content ?? "",
    }));
}

function check(report, ok, okMessage, failMessage) {
  report.checks.push({ ok, message: ok ? okMessage : failMessage });
}

async function inspectImage(url) {
  const response = await fetch(url, {
    redirect: "follow",
    headers: { "user-agent": userAgent, accept: "image/*,*/*;q=0.8" },
  });
  const buffer = Buffer.from(await response.arrayBuffer());
  return {
    url: url.href,
    finalUrl: response.url,
    status: response.status,
    contentType: response.headers.get("content-type") ?? "",
    bytes: buffer.length,
    ...imageDimensions(buffer),
  };
}

function imageDimensions(buffer) {
  if (buffer.length >= 24 && buffer.toString("ascii", 1, 4) === "PNG") {
    return {
      format: "png",
      width: buffer.readUInt32BE(16),
      height: buffer.readUInt32BE(20),
    };
  }
  if (buffer.length >= 10 && buffer[0] === 0xff && buffer[1] === 0xd8) {
    return jpegDimensions(buffer);
  }
  if (buffer.length >= 30 && buffer.toString("ascii", 0, 4) === "RIFF") {
    return webpDimensions(buffer);
  }
  if (buffer.length >= 10 && buffer.toString("ascii", 0, 3) === "GIF") {
    return {
      format: "gif",
      width: buffer.readUInt16LE(6),
      height: buffer.readUInt16LE(8),
    };
  }
  return {};
}

function jpegDimensions(buffer) {
  let offset = 2;
  while (offset < buffer.length) {
    if (buffer[offset] !== 0xff) return {};
    const marker = buffer[offset + 1];
    const length = buffer.readUInt16BE(offset + 2);
    if (marker >= 0xc0 && marker <= 0xc3) {
      return {
        format: "jpeg",
        width: buffer.readUInt16BE(offset + 7),
        height: buffer.readUInt16BE(offset + 5),
      };
    }
    offset += 2 + length;
  }
  return {};
}

function webpDimensions(buffer) {
  const chunk = buffer.toString("ascii", 12, 16);
  if (chunk === "VP8X" && buffer.length >= 30) {
    return {
      format: "webp",
      width: 1 + buffer.readUIntLE(24, 3),
      height: 1 + buffer.readUIntLE(27, 3),
    };
  }
  if (chunk === "VP8 " && buffer.length >= 30) {
    return {
      format: "webp",
      width: buffer.readUInt16LE(26) & 0x3fff,
      height: buffer.readUInt16LE(28) & 0x3fff,
    };
  }
  if (chunk === "VP8L" && buffer.length >= 25) {
    const bits = buffer.readUInt32LE(21);
    return {
      format: "webp",
      width: (bits & 0x3fff) + 1,
      height: ((bits >> 14) & 0x3fff) + 1,
    };
  }
  return { format: "webp" };
}

async function inspectRobots(url) {
  const robotsUrl = new URL("/robots.txt", url.origin);
  try {
    const response = await fetch(robotsUrl, fetchOptions);
    return {
      url: robotsUrl.href,
      status: response.status,
      note: response.ok ? "inspect manually for crawler-specific disallows" : "not found or unavailable",
    };
  } catch (error) {
    return { url: robotsUrl.href, error: error.message };
  }
}
