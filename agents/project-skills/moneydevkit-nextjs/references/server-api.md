# Server API

Use only exports present in the installed `@moneydevkit/nextjs/server` declarations.

## Contents

- Current exported surface
- Server-only safety rules
- Signed checkout URLs and payouts
- Paid routes and checkout verification boundaries
- Error handling

## Current upstream surface

Current upstream exposes these specialized server helpers:

| Export | Purpose |
| --- | --- |
| `createCheckoutUrl` | create a signed checkout URL for server redirects |
| `programmaticPayout` | dispatch a payout and receive an acceptance, not its final outcome |
| `waitForPayoutResult` | poll an accepted payout to a terminal result |
| `getBalance` | read spendable merchant balance |
| `pay402`, `Pay402Error` | consume an L402/HTTP 402 resource |
| `withPayment` | protect a route with Lightning payment |
| `withDeferredSettlement` | delay credential consumption until service delivery succeeds |

Current server-entrypoint types include `CreateCheckoutUrlOptions`,
`ProgrammaticPayoutOptions`, `WaitForPayoutResultOptions`, `Pay402Options`,
`Pay402ErrorCode`, `WithPaymentConfig`, and `SettleResult`. Generic `MdkError` and `Result` are
exported from `@moneydevkit/nextjs`, not its `/server` entrypoint. Prefer these exports over local
lookalike interfaces and re-check installed declarations because the surface can differ by
version.

## Server-only rules

- Import from `@moneydevkit/nextjs/server` only in Server Actions, route handlers, jobs, or other
  server-only modules.
- Validate application authorization before payouts or paid side effects.
- Keep credentials out of client bundles, logs, thrown public messages, and telemetry fields.
- Treat `Result` as a discriminated success/error value; do not read `data` before checking
  `error`.
- Use `error.retryable === true` as an explicit retry signal. Missing `retryable` is not true.

## Signed checkout URLs

`createCheckoutUrl` is the supported server-side redirect helper when exported by the installed
version. The corresponding MDK route may require both `GET` and `POST` exports. Confirm the
installed `server/route` declarations and preserve the generated signature/query parameters.
Do not construct the signed URL manually.

## Programmatic payouts

Payouts are irreversible external effects. Require application-level authorization, limits,
destination validation, and an audit record. Use a stable idempotency key from the application's
domain record, such as a withdrawal ID. Never generate a fresh key on every retry.

`programmaticPayout` returning success means the node accepted the request. The final Lightning
outcome is asynchronous. Persist the returned `paymentId` with the stable idempotency key in a
`requested` state, then resolve it with `waitForPayoutResult` or an equivalent durable worker.
`REQUESTED` remains pending; only `SUCCESS` and `FAILED` are terminal.

```ts
'use server'

import {
  programmaticPayout,
  waitForPayoutResult,
} from '@moneydevkit/nextjs/server'

export async function dispatchWithdrawal(withdrawalId: string, destination: string) {
  // load and authorize the withdrawal before calling MDK
  const result = await programmaticPayout({
    amountSats: 10_000,
    destination,
    idempotencyKey: withdrawalId,
  })

  if (result.error) {
    return {
      kind: result.error.retryable === true ? 'retryable_error' : 'terminal_error',
      code: result.error.code,
    } as const
  }

  // persist withdrawalId, paymentId, and status=requested before returning
  return { kind: 'requested', paymentId: result.data.paymentId } as const
}

export async function resolveWithdrawal(paymentId: string) {
  const result = await waitForPayoutResult({ paymentId })

  if (result.error) {
    return {
      kind: result.error.retryable === true ? 'retryable_error' : 'terminal_error',
      code: result.error.code,
    } as const
  }

  switch (result.data.status) {
    case 'REQUESTED':
      return { kind: 'pending' } as const
    case 'SUCCESS':
      return { kind: 'succeeded' } as const
    case 'FAILED':
      return { kind: 'failed', reason: result.data.failureReason } as const
  }
}
```

The snippet returns safe domain outcomes; the application must persist each transition and
schedule another resolution attempt for `pending` or retryable errors. Do not mark a withdrawal
complete from the dispatch acknowledgement.

Do not run real payouts during ordinary tests. Mock the exported boundary or use a vendor-supported
test facility.

## Paid routes

Use `withPayment` when consuming the credential before application handling is acceptable. Use
`withDeferredSettlement` when work can fail and the payer must be able to retry the same paid
request. In deferred mode, call `settle()` only after successful service delivery and handle a
failed settlement explicitly.

Keep dynamic pricing deterministic for the same protected resource. Authorization credentials
must not be reusable for a differently priced or different endpoint.

## No generic server checkout client

Do not assume the server module exports `getCheckout`, `confirmCheckout`, or a generic
`mdkPost`. Current upstream does not. If fulfillment needs server checkout verification, prefer
a documented webhook or public API available to the installed version. A direct call to the
exported route with a synthetic `Request` is version-sensitive; load the custom-checkout
reference directly from `SKILL.md`, follow its gated procedure, and obtain user agreement before
depending on internals.

## Error handling

Map SDK errors to application-domain outcomes:

- retryable service/network condition: retry with the same idempotency key and bounded backoff
- authentication/configuration/input condition: stop and alert an operator or caller
- accepted payout still pending: use the exported wait helper if present; do not initiate another
  payout
- unknown failure: retain the domain operation as unresolved, not failed-and-safe-to-repeat

Public responses should contain a stable application error code and safe message, not raw
credentials, destinations, stack traces, or the complete SDK response.
