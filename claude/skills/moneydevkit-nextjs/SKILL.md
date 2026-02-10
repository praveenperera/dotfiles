---
name: moneydevkit-nextjs
description: MoneyDevKit (MDK) integration for Next.js App Router apps. Use this skill when the user wants to add Lightning Bitcoin payments to a Next.js project — setting up MDK, creating checkouts (fixed amount or product-based), building custom checkout UIs with QR codes, handling payment verification, or making server-side MDK calls. Triggers on mentions of MoneyDevKit, MDK, Lightning payments, bitcoin checkout, or sat/sats payments in the context of Next.js.
---

# MoneyDevKit for Next.js

## Overview

MoneyDevKit (MDK) is an SDK for embedding Lightning-powered Bitcoin payments in Next.js App Router apps. It provides:
- Client hooks (`useCheckout`, `useProducts`, `useCheckoutSuccess`) for creating and verifying checkouts
- A hosted `<Checkout>` component that renders a full payment page with QR code
- A server route handler that proxies MDK API calls
- A Next.js plugin (`withMdkCheckout`) that configures bundling

Two checkout types: **AMOUNT** (fixed/custom amounts for donations, tips) and **PRODUCTS** (product-based with fixed or pay-what-you-want pricing).

Two UI paths: **hosted checkout** (redirect to `<Checkout>` component) or **custom UI** (call MDK handlers directly, render your own QR code, poll for payment).

## Setup

### 1. Install

```bash
npm install @moneydevkit/nextjs
```

### 2. Environment Variables

Add to `.env.local`:

```env
MDK_ACCESS_TOKEN=your_api_key_here
MDK_MNEMONIC=your_mnemonic_here
```

Get credentials at [moneydevkit.com](https://moneydevkit.com) or run `npx @moneydevkit/create`.

### 3. API Route

Create `app/api/mdk/route.js`:

```js
export { GET, POST } from '@moneydevkit/nextjs/server/route'
```

This single route handles all MDK client requests (create_checkout, get_checkout, list_products, etc).

### 4. Next.js Plugin

Wrap your config with `withMdkCheckout` in `next.config.mjs`:

```js
import withMdkCheckout from '@moneydevkit/nextjs/next-plugin'

/** @type {import('next').NextConfig} */
const nextConfig = {
  // your existing config
}

export default withMdkCheckout(nextConfig)
```

## Simple Checkout (AMOUNT Type)

Use `useCheckout` to create a checkout and redirect to the hosted checkout page.

### Client Component

```tsx
'use client'

import { useCheckout } from '@moneydevkit/nextjs'
import { useState } from 'react'

export default function DonateButton() {
  const { createCheckout, isLoading } = useCheckout()
  const [error, setError] = useState<string | null>(null)

  const handleDonate = async () => {
    setError(null)

    const result = await createCheckout({
      type: 'AMOUNT',
      title: 'Donate',
      description: 'Support our project',
      amount: 500,       // 500 cents = $5.00
      currency: 'USD',   // or 'SAT' for satoshis
      successUrl: '/checkout/success',
      metadata: {
        donor: 'anonymous',
      },
    })

    if (result.error) {
      setError(result.error.message)
      return
    }

    window.location.href = result.data.checkoutUrl
  }

  return (
    <>
      {error && <p className="text-red-600">{error}</p>}
      <button onClick={handleDonate} disabled={isLoading}>
        {isLoading ? 'Creating...' : 'Donate $5'}
      </button>
    </>
  )
}
```

### Hosted Checkout Page

Create `app/checkout/[id]/page.tsx`:

```tsx
'use client'

import { use } from 'react'
import { Checkout } from '@moneydevkit/nextjs'

export default function CheckoutPage({
  params,
}: {
  params: Promise<{ id: string }>
}) {
  const { id } = use(params)
  return <Checkout id={id} />
}
```

The `<Checkout>` component renders the full payment UI with QR code, countdown timer, and payment detection. It redirects to `successUrl` when paid.

### Server-Side Redirect (Alternative)

For server-initiated checkouts (e.g., from an API route), use `createCheckoutUrl`:

```ts
import { createCheckoutUrl } from '@moneydevkit/nextjs/server'
import { redirect } from 'next/navigation'

export async function GET() {
  const url = createCheckoutUrl({
    type: 'AMOUNT',
    title: 'Donate',
    description: 'Support our project',
    amount: 500,
    currency: 'USD',
    successUrl: '/checkout/success',
  })

  redirect(url)
}
```

## Product Checkout (PRODUCTS Type)

For selling products defined in your MDK dashboard.

### Listing Products

```tsx
'use client'

import { useCheckout, useProducts } from '@moneydevkit/nextjs'

export default function ProductList() {
  const { products } = useProducts()
  const { createCheckout, isLoading } = useCheckout()

  const handleBuy = async (productId: string) => {
    const result = await createCheckout({
      type: 'PRODUCTS',
      product: productId,
      successUrl: '/checkout/success',
    })

    if (result.error) return
    window.location.href = result.data.checkoutUrl
  }

  return (
    <div>
      {products?.map((product) => (
        <button
          key={product.id}
          onClick={() => handleBuy(product.id)}
          disabled={isLoading}
        >
          Buy {product.name} - ${(product.price?.priceAmount ?? 0) / 100}
        </button>
      ))}
    </div>
  )
}
```

### Pay What You Want (CUSTOM Pricing)

Products with CUSTOM pricing let users choose their own amount. The hosted `<Checkout>` component automatically shows an amount input for these products:

```tsx
const result = await createCheckout({
  type: 'PRODUCTS',
  product: customPriceProductId,
  successUrl: '/checkout/success',
})
```

For custom UIs with CUSTOM pricing, you need to use `confirm_checkout` to set the amount — see the Custom Checkout UI section below.

### Product/Price Types

```ts
type Product = {
  id: string
  name: string
  description: string | null
  prices: Array<{
    id: string
    currency: 'USD' | 'SAT'
    amountType: 'FIXED' | 'CUSTOM'
    priceAmount: number | null  // null for CUSTOM prices
  }>
}
```

Price amounts are in base units: cents for USD, satoshis for SAT.

## Custom Checkout UI

Instead of redirecting to the hosted `<Checkout>` component, you can build your own payment UI. This involves:
1. Calling `create_checkout` to get an invoice
2. Displaying a QR code with the Lightning invoice
3. Polling `get_checkout` for payment status
4. Handling expiry with a countdown timer

### CSRF Token Helper

Client-side calls to `/api/mdk` require a CSRF token:

```ts
function getCsrfToken(): string {
  const existing = document.cookie
    .split(';')
    .find((c) => c.trim().startsWith('mdk_csrf='))
    ?.split('=')[1]

  if (existing) return existing

  const token = crypto.randomUUID()
  document.cookie = `mdk_csrf=${token}; path=/; SameSite=Lax`
  return token
}
```

### Client-Side MDK Helper

```ts
async function postMdk<T>(
  handler: string,
  payload: Record<string, unknown>,
): Promise<T> {
  const res = await fetch('/api/mdk', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-moneydevkit-csrf-token': getCsrfToken(),
    },
    body: JSON.stringify({ handler, ...payload }),
  })

  if (!res.ok) throw new Error(`MDK request failed: ${res.status}`)
  return (await res.json()) as T
}
```

### Creating a Checkout (AMOUNT)

```ts
type MdkCreatedCheckout = {
  id: string
  currency: string
  invoiceAmountSats: number | null
  invoice: {
    invoice: string
    amountSats: number | null
    fiatAmount: number
    expiresAt: string
  }
}

const { data } = await postMdk<{ data: MdkCreatedCheckout }>(
  'create_checkout',
  {
    params: {
      type: 'AMOUNT',
      title: 'Donate',
      description: 'Support our project',
      amount: 500,
      currency: 'USD',
      successUrl: '/checkout/success',
    },
  },
)

// data.id — checkout ID for polling
// data.invoice.invoice — Lightning invoice string (for QR code)
// data.invoice.amountSats — amount in sats
// data.invoice.fiatAmount — amount in fiat base units (cents)
// data.invoice.expiresAt — ISO 8601 expiry timestamp
```

### Creating a Checkout (PRODUCTS with CUSTOM Pricing)

For products with CUSTOM pricing, you need a two-step flow: `create_checkout` then `confirm_checkout` to set the user-chosen amount:

```ts
// step 1: create the checkout
const createResponse = await postMdk<{ data: MdkCheckout }>(
  'create_checkout',
  {
    params: {
      type: 'PRODUCTS',
      product: productId,
      successUrl: '/success',
      metadata: { featureProductId: productId },
    },
  },
)

const checkoutId = createResponse.data.id

// step 2: confirm with the user-chosen amount
const confirmResponse = await postMdk<{ data: MdkCheckout }>(
  'confirm_checkout',
  {
    confirm: {
      checkoutId,
      products: [{
        productId: productId,
        priceAmount: amountSats,  // user-chosen amount in sats
      }],
    },
  },
)

// confirmResponse.data.invoice now contains the Lightning invoice
```

### Displaying a QR Code

Use `qrcode.react` to render the Lightning invoice:

```tsx
import { QRCodeSVG } from 'qrcode.react'

<QRCodeSVG
  value={checkout.invoice}
  size={240}
  bgColor="#ffffff"
  fgColor="#000000"
  level="Q"
/>
```

### Polling for Payment

Poll `get_checkout` every 2 seconds to detect payment:

```ts
type MdkCheckoutStatus = {
  status: string
  invoice?: {
    amountSatsReceived?: number | null
  } | null
}

useEffect(() => {
  if (step !== 'invoice' || !checkout) return

  const interval = setInterval(async () => {
    try {
      const { data } = await postMdk<{ data: MdkCheckoutStatus }>(
        'get_checkout',
        { checkoutId: checkout.id },
      )

      if (
        data.status === 'PAYMENT_RECEIVED' ||
        data.status === 'CONFIRMED' ||
        (data.invoice?.amountSatsReceived ?? 0) > 0
      ) {
        setStep('paid')
      } else if (data.status === 'EXPIRED') {
        setStep('expired')
      }
    } catch {
      // ignore polling errors
    }
  }, 2000)

  return () => clearInterval(interval)
}, [step, checkout])
```

### Countdown Timer

Track invoice expiry with a countdown:

```ts
useEffect(() => {
  if (step !== 'invoice' || !checkout) return

  const update = () => {
    const diff = new Date(checkout.expiresAt).getTime() - Date.now()
    if (diff <= 0) {
      setTimeRemaining('Expired')
      setStep('expired')
      return
    }
    const m = Math.floor(diff / 60000)
    const s = Math.floor((diff % 60000) / 1000)
    setTimeRemaining(`${m}:${s.toString().padStart(2, '0')}`)
  }

  update()
  const interval = setInterval(update, 1000)
  return () => clearInterval(interval)
}, [step, checkout])
```

### Custom UI Step Machine

A typical custom checkout UI uses these steps:

```ts
type Step = 'pick' | 'loading' | 'invoice' | 'paid' | 'expired'
```

- **pick** — user selects amount/product
- **loading** — creating the checkout
- **invoice** — showing QR code, polling for payment, countdown timer
- **paid** — payment confirmed
- **expired** — invoice expired, offer retry

## Server-Side MDK Calls

For server-side operations (API routes, server actions), call `mdkPost` directly instead of going through the client hooks.

### The callMdk Wrapper

```ts
import { POST as mdkPost } from '@moneydevkit/nextjs/server/route'

async function callMdk<T>(
  payload: Record<string, unknown>,
): Promise<T> {
  const accessToken = process.env.MDK_ACCESS_TOKEN

  if (!accessToken) {
    throw new Error('MDK_ACCESS_TOKEN is not configured')
  }

  // the URL doesn't matter — mdkPost reads the body, not the URL
  const request = new Request('https://internal.mdk/api', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-moneydevkit-webhook-secret': accessToken,
    },
    body: JSON.stringify(payload),
  })

  const response = await mdkPost(request)
  const json = await response.json().catch(() => null)

  if (!response.ok) {
    const error =
      typeof json === 'object' && json !== null && typeof json.error === 'string'
        ? json.error
        : `MDK request failed (${response.status})`
    throw new Error(error)
  }

  return json as T
}
```

The key insight: `mdkPost` is the same route handler exported from `@moneydevkit/nextjs/server/route`. You construct a synthetic `Request` and call it directly. The `x-moneydevkit-webhook-secret` header with the access token bypasses CSRF checks, since this is server-to-server.

### Available Handlers

#### create_checkout

```ts
const response = await callMdk<{ data: MdkCheckout }>({
  handler: 'create_checkout',
  params: {
    type: 'AMOUNT',           // or 'PRODUCTS'
    title: 'My Checkout',
    description: 'A purchase',
    amount: 1000,              // required for AMOUNT type (base units)
    currency: 'USD',           // or 'SAT'
    product: 'product-id',     // required for PRODUCTS type
    successUrl: '/success',
    metadata: { key: 'value' },
  },
})
```

#### confirm_checkout

Used with PRODUCTS that have CUSTOM pricing to set the user-chosen amount:

```ts
const response = await callMdk<{ data: MdkCheckout }>({
  handler: 'confirm_checkout',
  confirm: {
    checkoutId: 'checkout-id',
    products: [{
      productId: 'product-id',
      priceAmount: 5000,       // user-chosen amount in base currency units
    }],
  },
})
```

#### get_checkout

```ts
const response = await callMdk<{ data: MdkCheckout }>({
  handler: 'get_checkout',
  checkoutId: 'checkout-id',
})
```

#### list_products

```ts
const response = await callMdk<{ data: { products: Product[] } }>({
  handler: 'list_products',
})
```

## Customer Data

### Pre-filling Customer Info

```ts
const result = await createCheckout({
  type: 'AMOUNT',
  title: 'Premium Plan',
  description: 'Monthly subscription',
  amount: 1000,
  currency: 'USD',
  successUrl: '/checkout/success',
  customer: {
    name: 'John Doe',
    email: 'john@example.com',
  },
  requireCustomerData: ['name', 'email', 'company'],
})
```

### How It Works

- If all `requireCustomerData` fields are already in `customer`, the form is skipped
- If some required fields are missing, a form collects only those fields
- Email is required to create a customer record
- Field names are flexible: `tax_id`, `tax-id`, `taxId`, or `Tax ID` all normalize to `taxId`
- Custom fields (beyond name, email, externalId) are stored in customer metadata

### Returning Customers

Customers are matched by `email` or `externalId`. Existing data is preserved, and only missing required fields are requested.

### externalId for Authenticated Users

Link checkouts to your app's user accounts:

```ts
const result = await createCheckout({
  type: 'AMOUNT',
  title: 'Premium Plan',
  description: 'Subscription',
  amount: 1000,
  currency: 'USD',
  successUrl: '/checkout/success',
  customer: {
    externalId: user.id,
    name: user.name,
    email: user.email,
  },
  requireCustomerData: ['name', 'email'],
})
```

When `externalId` is provided, the system assumes the user is authenticated. If the customer already exists (matched by externalId), their stored data is used and only missing fields are requested.

## Payment Verification

### Hosted Checkout: useCheckoutSuccess

On your success page, use the `useCheckoutSuccess` hook:

```tsx
'use client'

import { useCheckoutSuccess } from '@moneydevkit/nextjs'

export default function SuccessPage() {
  const { isCheckoutPaidLoading, isCheckoutPaid, metadata } = useCheckoutSuccess()

  if (isCheckoutPaidLoading || isCheckoutPaid === null) {
    return <p>Verifying payment...</p>
  }

  if (!isCheckoutPaid) {
    return <p>Payment has not been confirmed.</p>
  }

  return <p>Payment confirmed!</p>
}
```

The `metadata` object contains whatever you passed in the `metadata` field when creating the checkout.

### Custom UI: Poll get_checkout

For custom UIs, poll `get_checkout` and check the status:

```ts
const { data } = await postMdk<{ data: MdkCheckoutStatus }>(
  'get_checkout',
  { checkoutId: checkout.id },
)

const isPaid =
  data.status === 'PAYMENT_RECEIVED' ||
  data.status === 'CONFIRMED' ||
  (data.invoice?.amountSatsReceived ?? 0) > 0
```

### Server-Side Verification

For recording payments server-side (e.g., updating a database after payment):

```ts
const response = await callMdk<{ data: MdkCheckout }>({
  handler: 'get_checkout',
  checkoutId,
})

const checkout = response.data

const isPaid =
  checkout.status === 'PAYMENT_RECEIVED' ||
  checkout.status === 'CONFIRMED'
```

## Key Types

### MdkCheckout

The full checkout object returned by MDK:

```ts
type MdkCheckout = {
  id: string
  status: string
  productId?: string | null
  product?: { id: string } | null
  products?: Array<{ id: string }> | null
  userMetadata?: Record<string, unknown> | null
  invoice?: {
    invoice: string           // Lightning invoice string
    expiresAt: string         // ISO 8601
    amountSats: number | null
    amountSatsReceived: number | null
    fiatAmount: number | null // base units (cents for USD)
  } | null
  invoiceAmountSats?: number | null
  providedAmount?: number | null
  totalAmount?: number | null
  currency?: string
}
```

### Checkout Statuses

- `PENDING` — checkout created, awaiting payment
- `PAYMENT_RECEIVED` — Lightning payment detected
- `CONFIRMED` — payment fully confirmed
- `EXPIRED` — invoice expired without payment

### Currency Units

- **USD**: amounts are in cents (divide by 100 for dollars)
- **SAT**: amounts are in satoshis
