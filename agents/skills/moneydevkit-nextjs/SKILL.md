---
name: moneydevkit-nextjs
description: Integrate MoneyDevKit's `@moneydevkit/nextjs` package into Next.js App Router applications for hosted or custom Lightning checkout, amount and product payments, server-only MDK operations, and payment verification. Use when a Next.js project mentions MoneyDevKit/MDK, Lightning checkout, Bitcoin sats, product checkout, payment polling, or server-side fulfillment. Do not use for non-Next.js MDK integrations or generic Bitcoin wallet code.
---

# MoneyDevKit for Next.js

Treat the installed package as the API authority. MoneyDevKit evolves quickly; inspect the
project's package version, installed exports/types, and [official Next.js docs](https://docs.moneydevkit.com/nextjs)
before writing code. Do not invent handlers, statuses, headers, or response shapes.

## Choose the integration path

| User need | Path | Read |
| --- | --- | --- |
| standard checkout page and fastest supported integration | hosted checkout | [setup-and-hosted-checkout.md](references/setup-and-hosted-checkout.md) |
| branded QR/invoice UI or explicit checkout state machine | custom checkout | [custom-checkout.md](references/custom-checkout.md) |
| Server Actions, route handlers, payouts, balances, or paid APIs | server API | [server-api.md](references/server-api.md) |
| order fulfillment, polling, success page, expiry, retries | verification | [payment-verification.md](references/payment-verification.md) |

Prefer hosted checkout unless the requested UX requires control the exported `<Checkout>`
component cannot provide. A custom UI depends on lower-level checkout operations that may not
be public exports in the installed version.

## Inspect before editing

1. Read `package.json`, the lockfile, and the app's Next/React versions.
2. If installed, inspect `node_modules/@moneydevkit/nextjs/package.json` and its `.d.ts` files.
3. Compare the installed version's README/types with the official docs.
4. Identify existing `app/api/mdk/route.*`, `app/checkout/[id]/page.*`, success route, env
   handling, `next.config.*`, order model, and fulfillment path.
5. Choose hosted, custom, server-only, or a combination before changing code.

Use the existing package manager and conventions. If package source and docs disagree, code to
the installed declarations and mention the version mismatch.

## Core setup

The supported App Router shape is:

```text
client useCheckout() -> /api/mdk POST -> checkout ID
                                      -> /checkout/[id] renders <Checkout>
MoneyDevKit webhook -> /api/mdk POST -> merchant Lightning node
success URL -> presentation verification; server -> authoritative fulfillment
```

Keep `MDK_ACCESS_TOKEN` and `MDK_MNEMONIC` server-only. Never prefix them with `NEXT_PUBLIC_`,
send them to a Client Component, log them, or include them in an error response. Wire the MDK
route and `withMdkCheckout` plugin exactly as documented for the installed version.

## Model checkout states explicitly

Do not scatter string checks across components. Represent UI state as a discriminated union and
centralize mapping from the SDK's checkout status. Current package schemas distinguish:

- `UNCONFIRMED`: customer input or confirmation is still required
- `CONFIRMED`: checkout is confirmed, but payment is not complete
- `PENDING_PAYMENT`: invoice exists and payment is pending
- `PAYMENT_RECEIVED`: payment-complete state
- `EXPIRED`: terminal invoice expiry

Verify this union in installed types. Unknown statuses must remain unknown/error states, never
silently become paid.

## Non-negotiable payment invariants

- Treat browser state, redirects, query parameters, and success pages as untrusted.
- Fulfill only after a server-authoritative payment check.
- Make fulfillment idempotent with a unique checkout/order key and transactional state change
  or outbox; retries must not double-grant or double-ship.
- Never treat `invoice.amountSatsReceived > 0` as full payment. It can represent a partial or
  intermediate receipt; use the authoritative paid status and expected checkout identity.
- Keep checkout creation separate from fulfillment. A created invoice is not revenue.
- Distinguish retryable transport/service failures, terminal validation/auth failures, and
  invoice expiry. Do not retry every error forever or swallow polling errors.

## Public APIs before internals

Prefer package exports and exported types. Current upstream exposes client hooks/components and
specific server helpers, but not a public general-purpose `getCheckout` server helper. Do not
copy private route wire shapes into application code without verifying the exact installed
package.

A synthetic `Request` sent directly to the exported MDK route, using a secret header to bypass
CSRF, is an internal compatibility technique—not a default API. Recommend it only after
confirming the installed implementation accepts the exact handler/body/header combination,
isolating it behind a server-only adapter, and adding a targeted test. Otherwise use a
documented/exported server API or ask the user to choose a supported webhook/API strategy.

## Implementation workflow

1. Add or reconcile package setup, environment validation, MDK route, and Next config.
2. Implement checkout creation with the installed `useCheckout` input type.
3. Add the hosted page or a typed custom state machine.
4. Persist the application's order/payment attempt before relying on a checkout ID.
5. Implement server-authoritative, idempotent fulfillment.
6. Surface actionable client errors and bounded retry behavior.
7. Verify build/typecheck and exercise create, pending, paid, expired, retry, and duplicate
   fulfillment paths.

Use sandbox/test facilities only when supported by the installed package and account. Never
trigger a real payout or payment as part of routine verification.
