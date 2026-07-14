# Custom checkout UI

Build a custom UI only when hosted checkout cannot meet the product requirement.

## Contents

- Supported integration boundary
- Typed checkout state machine
- Checkout and polling lifecycle
- QR and amount safety
- Version-gated internal adapters

## Establish the supported boundary

Inspect the installed package first. The current public Next.js root exports `Checkout`,
`useCheckout`, `useCheckoutSuccess`, `useProducts`, `useCustomer`, `MdkError`, and `Result` plus
selected props/customer types. It does not expose public low-level `getCheckout` or
`confirmCheckout` hooks, even though internal client code uses those operations.

Choose one boundary explicitly:

1. keep the hosted `<Checkout>` and customize only supported props/theme
2. use a documented public API added by the installed version
3. create a small application-owned server adapter after verifying the exact installed route
   contract
4. stop and ask before depending on undocumented internals

Do not paste a guessed `/api/mdk` fetch helper from an older package version.

## Use a typed state machine

Keep impossible UI states unrepresentable:

```ts
type CheckoutUiState =
  | { kind: 'idle' }
  | { kind: 'creating' }
  | { kind: 'needs-confirmation'; checkoutId: string }
  | {
      kind: 'awaiting-payment'
      checkoutId: string
      invoice: string
      expiresAt: Date
    }
  | { kind: 'paid'; checkoutId: string }
  | { kind: 'expired'; checkoutId: string }
  | { kind: 'failed'; retryable: boolean; message: string }
```

Map SDK states centrally. In current schemas, `UNCONFIRMED`, `CONFIRMED`, and
`PENDING_PAYMENT` are non-paid states; `PAYMENT_RECEIVED` is paid; `EXPIRED` is terminal. Verify
installed declarations and preserve unknown values as errors/unknowns.

## Checkout flow

1. Create the application order/payment attempt on the server.
2. Create an amount or product checkout using a verified package API.
3. If customer/product CUSTOM pricing requires confirmation, call only the installed version's
   verified confirmation surface.
4. Render the returned BOLT invoice and expiry.
5. Poll a verified checkout-read surface while the status is pollable.
6. Stop polling on authoritative paid, expiry, terminal failure, unmount, or cancellation.
7. Let server fulfillment handle the durable business transition.

Prefer exported/inferred types:

```ts
import type { MdkError, Result } from '@moneydevkit/nextjs'
```

If an entity type is not exported, infer it from an exported function when practical or validate
the boundary with a schema owned by the application. Do not create a broad hand-written
`MdkCheckout` interface that can silently drift.

## QR and amount safety

Render the exact invoice string returned by the verified API. Do not parse it to decide payment
completion. Display currency with an explicit domain type such as `{ currency: 'USD'; cents:
number } | { currency: 'SAT'; sats: number }` and convert only at UI boundaries.

`amountSatsReceived` is not a paid flag. A positive value can be partial/intermediate; never
transition the application state to paid from `> 0`.

## Polling behavior

Allow only one request in flight. Use cancellation on unmount, pause or slow polling when the
document is hidden, and apply bounded backoff/jitter after transient errors. Reset the error
counter after a successful read.

Classify outcomes:

- pollable: known non-paid checkout status before expiry
- paid: authoritative paid status from the server-read checkout
- expired: SDK `EXPIRED` or authoritative expiry; stop and offer a new checkout
- transient: network failure, timeout, or explicitly `retryable: true`
- terminal: invalid input, authentication/configuration, not found, or explicitly non-retryable
- unknown: surface an error and stop or retry conservatively; never mark paid

Do not use `catch {}` around polling. Record safe diagnostic context and show a recoverable UI
when retries are exhausted.

## Version-gated internal adapter

If no public server checkout-read API exists and the user accepts an internal dependency:

1. inspect `dist/server/route.js` and its declarations in the installed package
2. verify the accepted handler name/body and response envelope
3. verify whether a server-to-server secret-header override exists and which environment value
   it compares—never infer this from old examples
4. keep the adapter in a server-only module and never expose the secret to the browser
5. validate returned data at the boundary
6. add a focused test using the exact installed package
7. document the pinned compatibility dependency in code at the adapter boundary

Prefer replacing this adapter when the package gains a public export.
