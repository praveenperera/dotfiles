# Setup and hosted checkout

## Contents

- Compatibility and installation
- Route and Next.js configuration
- Hosted checkout page
- Amount and product checkout creation
- Success presentation and lifecycle verification

Use the hosted path for standard amount or product checkout.

## Inspect versions and exports

Read these before editing:

```bash
node -p "require('./package.json').dependencies?.['@moneydevkit/nextjs'] ?? require('./package.json').devDependencies?.['@moneydevkit/nextjs']"
node -p "require('./node_modules/@moneydevkit/nextjs/package.json').version"
node -p "require('./node_modules/@moneydevkit/nextjs/package.json').peerDependencies"
```

Also inspect the package's `exports`, `dist/index.d.ts`, `dist/server/index.d.ts`, and
`dist/server/route.d.ts`. Use the project's package manager. Do not upgrade Next, React, or MDK
without establishing that the user requested or accepts the migration.

## Install and configure

Install the package using the existing lockfile's manager. Obtain credentials through the
MoneyDevKit account flow or the documented `npx @moneydevkit/create@latest` flow. Keep secrets
in the project's server environment, commonly `.env.local` for local Next.js development:

```env
MDK_ACCESS_TOKEN=...
MDK_MNEMONIC=...
```

Never commit values. Add names to an example env file only if that is the repo convention.

Expose the documented route:

```ts
// app/api/mdk/route.ts
export { POST } from '@moneydevkit/nextjs/server/route'
```

The current package also exports `GET` for signed URL actions used by helpers such as
`createCheckoutUrl`. Export it only when the installed declarations and chosen flow require it:

```ts
export { GET, POST } from '@moneydevkit/nextjs/server/route'
```

Wrap the existing Next config without discarding other plugins or options:

```ts
import withMdkCheckout from '@moneydevkit/nextjs/next-plugin'

const nextConfig = {
  // preserve existing configuration
}

export default withMdkCheckout(nextConfig)
```

## Render hosted checkout

Use the App Router dynamic page shape required by the project's Next version. The current docs
show asynchronous params:

```tsx
'use client'

import { Checkout } from '@moneydevkit/nextjs'
import { use } from 'react'

export default function CheckoutPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params)
  return <Checkout id={id} />
}
```

Prefer the exported `CheckoutProps` if wrapping the component. Do not re-declare its props.

## Create a checkout

Use `useCheckout()` in a Client Component. Current inputs form a domain union:

- `AMOUNT`: requires `amount` and `currency: 'USD' | 'SAT'`; title/description are allowed
- `PRODUCTS`: requires a dashboard product ID; product pricing determines the amount

USD amounts use base units (cents); SAT amounts use satoshis. Preserve that distinction in the
application's own money type instead of passing an unlabelled number between layers.

```tsx
'use client'

import { useCheckout } from '@moneydevkit/nextjs'

export function BuyButton() {
  const { createCheckout, isLoading } = useCheckout()

  async function buy() {
    const result = await createCheckout({
      type: 'AMOUNT',
      title: 'Example purchase',
      amount: 500,
      currency: 'USD',
      successUrl: '/checkout/success',
      metadata: { orderId: 'application-order-id' },
    })

    if (result.error) {
      // render a user-safe message and retain machine-readable error context for diagnostics
      return
    }

    window.location.assign(result.data.checkoutUrl)
  }

  return <button disabled={isLoading} onClick={buy}>Buy</button>
}
```

Generate and persist the order ID server-side where fulfillment depends on it. Client metadata
helps correlate records but is not authorization or proof of payment.

For products, use the installed `useProducts()` return type rather than a hand-written Product
interface. CUSTOM/pay-what-you-want behavior belongs to the hosted component unless the exact
lower-level confirmation contract has been verified for a custom UI.

## Success presentation

`useCheckoutSuccess()` can present "verifying", "paid", and "not confirmed" UI after redirect.
It does not authorize fulfillment. Never grant access, ship, or mutate authoritative order state
only from this client hook.

## Lifecycle verification

Run the repository's formatter/linter/typecheck/build commands. Then exercise, using supported
test/sandbox facilities:

- checkout creation error and success
- hosted route rendering
- pending invoice and expiry
- successful return URL
- refresh/back navigation
- duplicate success/fulfillment attempt
- missing server credentials without leaking values
