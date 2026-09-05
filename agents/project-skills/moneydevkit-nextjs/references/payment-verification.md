# Payment verification and fulfillment

Payment presentation and fulfillment are separate systems. The browser can display status; only
the server may transition the order into a fulfilled state.

## Contents

- Checkout status semantics
- Authoritative verification
- Idempotent fulfillment model
- Success pages, polling, and expiry
- Verification scenarios

## Status semantics

Verify the installed checkout status union. Current schemas use:

| Status | Meaning for fulfillment |
| --- | --- |
| `UNCONFIRMED` | not paid; customer/price confirmation may remain |
| `CONFIRMED` | not paid; checkout confirmed before or while invoice is minted |
| `PENDING_PAYMENT` | not paid; invoice awaits settlement |
| `PAYMENT_RECEIVED` | paid state; still verify checkout/order identity server-side |
| `EXPIRED` | terminal for that invoice; create a new attempt if the user retries |

Never treat `CONFIRMED`, redirect arrival, success query parameters, or
`amountSatsReceived > 0` as payment completion.

## Authoritative verification

Use a documented server-side webhook or checkout-read API supported by the installed package.
If the installed Next.js package offers only client verification, do not quietly promote that
hook to a fulfillment mechanism. Either integrate a supported authoritative surface or ask the
user to accept a pinned, tested internal adapter as described in the custom checkout reference.

Verify at least:

- checkout ID matches the stored payment attempt
- authoritative status is the paid status
- order is still eligible for fulfillment
- currency/product/expected amount correspond to the stored order where exposed
- environment/mode matches the order (test versus live)

The provider status is the payment truth. Comparing received and expected amounts can be an
additional invariant, but a positive received amount alone is never sufficient.

## Idempotent domain model

Persist an order and one or more payment attempts. Give every provider checkout a uniqueness
constraint. Represent fulfillment as a durable transition, not a callback side effect:

```text
Order: pending -> paid -> fulfilling -> fulfilled
PaymentAttempt: created -> awaiting_payment -> paid | expired | failed
FulfillmentJob: unique(order_id), pending -> running -> complete | retryable_failure
```

On authoritative paid notification/read, run a transaction that:

1. locks or conditionally updates the payment attempt by checkout ID
2. records the paid transition only if it was not already recorded
3. transitions the corresponding order to paid if eligible
4. inserts one uniquely keyed fulfillment job/outbox entry
5. commits before external delivery

The worker then performs delivery with its own stable idempotency key and marks completion. A
duplicate webhook, poll, redirect, or job retry becomes a no-op or resumes the same operation.

For a purely transactional entitlement, the entitlement grant and paid transition can occur in
one database transaction with a uniqueness constraint. For shipping, email, third-party APIs,
or other external effects, use an outbox/job boundary.

## Success pages

`useCheckoutSuccess()` is appropriate for reader-visible progress:

- loading/null: "Verifying payment"
- false: "Payment has not been confirmed"
- true: "Payment received" while fulfillment status is loaded from the application server

Do not put authorization, premium data, or a fulfillment mutation behind client-only
`isCheckoutPaid`. A user can revisit or forge browser state.

## Polling and expiry

Poll only known non-terminal states and allow one request at a time. Stop on paid, expired,
terminal error, cancellation, or retry-budget exhaustion. Use the invoice/server expiry rather
than a browser countdown as authority; the countdown is presentation only and clock skew exists.

Classify errors instead of swallowing them:

- transient: fetch/network timeout, temporary 5xx, or SDK `retryable: true`; retry with bounded
  exponential backoff and jitter
- terminal: invalid request, invalid credentials/configuration, forbidden, or explicit
  non-retryable error; stop and surface an actionable safe message
- expiry: stop polling that invoice and offer a new checkout/payment attempt
- unknown: report and preserve unresolved state; do not assume failure or payment

Log checkout/order correlation IDs and safe error codes, never credentials or full secret-bearing
payloads.

## Verification scenarios

Protect these behaviors with tests when they affect user-visible payment/fulfillment behavior:

- paid status creates exactly one fulfillment job
- duplicate paid events/checks do not duplicate fulfillment
- partial `amountSatsReceived` does not pay the order
- `CONFIRMED` and `PENDING_PAYMENT` do not pay the order
- expiry stops polling and leaves the order unfulfilled
- transient errors retry within bounds; terminal errors do not
- unknown status never maps to paid
- client success without server verification does not fulfill
- live/test environment mismatch is rejected

After unit/integration tests, run the repository's typecheck/build and exercise the supported
sandbox lifecycle end to end.
