# Braiins Hashpower Marketplace Research

## Scope

This note documents how Braiins Hashpower appears to work as of April 11, 2026, what components are needed to operate a similar two-sided hashpower marketplace, and what miners, buyers, and the marketplace operator need to run.

## Executive Summary

Braiins Hashpower appears to be a centralized hashrate market operated by Braiins, not a direct peer-to-peer protocol between buyer and miner.

The public product and terms show:

- buyers fund accounts with BTC and place live spot bids
- buyers provide a destination mining pool URL and worker identity
- Braiins validates destination pool compatibility before bid creation
- Braiins delivers hashrate in real time to the buyer's selected destination
- Braiins is always the contractual seller of the hashrate, even when supply comes from third-party providers

The critical implication is that Braiins sits in the middle as:

- supply aggregator
- traffic router
- settlement operator
- compliance gatekeeper
- buyer-facing marketplace

A proxy is almost certainly part of the delivery stack, but the proxy alone is not the market. The hard parts are the control plane, treasury, accounting, settlement, abuse prevention, and supply reliability.

## What Braiins Publicly Exposes

### Hashpower market behavior

The live site and terms expose the following model:

- live order book priced in `BTC/EHs/day`
- buyer bids with budget, price, optional speed limit, pool URL, and worker identity
- dedicated BTC deposit address per account
- owner token and read-only token access model
- Telegram-backed confirmation flow for sensitive actions
- pool compatibility checks before order placement
- real-time delivery and reporting

The public terms are especially important:

- Braiins says it is always the seller of the hash rate
- delivery starts when matching supply is available
- delivery may fluctuate in the short term
- total delivered hash rate over the full delivery period is guaranteed to meet or exceed the purchased amount
- pool rewards go directly to the buyer's selected pool account or solo payout address

This is not a direct buyer-to-miner contract. Braiins intermediates both legally and operationally.

### Pool compatibility

The public market page says the service works with most mining pools that support `extranonce2` size of at least 7 bytes.

That means a marketplace operator needs a compatibility layer that:

- validates the buyer destination before allocating hashrate
- checks protocol assumptions like extranonce sizing
- rejects pools that break routing or accounting assumptions

### Signup and auth flow

The public web app shows:

- signup requires email, Telegram chat ID, and account name
- Telegram chat ID is verified through Braiins' bot
- the account is issued an owner token and read-only token
- admin vs read-only permissions gate actions like bid creation and editing

This suggests a serious control plane rather than a lightweight frontend.

## What Braiins Proxy Does

Braiins Proxy is not the marketplace itself. It is the traffic and farm-operations layer between miners and upstream pools.

Public product pages and the source-available repo show that Braiins Proxy:

- accepts miner connections locally
- aggregates many miners behind one proxy
- reduces data transfer by about 95 percent
- forwards accepted work upstream
- supports multiple pools at once
- supports backup pools and endpoint failover
- supports automatic endpoint quality optimization
- exposes monitoring through Grafana, Prometheus, and an API
- supports GUI and TOML-based configuration

The public config templates show the core abstractions:

- `server`: downstream miner listener
- `target`: upstream pool destination
- `routing`: mapping from miner ingress to one or more destinations
- `goal.level`: fallback routing levels
- `hr_weight`: weighted hashrate split across targets

This is exactly the kind of primitive a hashrate marketplace allocator would need.

### What the proxy does not do

The proxy does not by itself provide:

- buyer onboarding
- seller onboarding
- market pricing or order matching
- BTC custody or seller payouts
- share-based settlement
- compliance workflows
- anti-fraud systems
- proof of delivery dashboards for both sides

## What Is Open and What Is Not

Braiins Farm Proxy code is visible on GitHub, but it is not open source in the normal licensing sense.

- the repo is public
- the code is source-available
- the license is proprietary
- it should not be treated as forkable infrastructure for a competing marketplace

Open alternatives do exist lower in the stack:

- Stratum V2 low-level libraries in `stratum-mining/stratum`
- Stratum V2 application-level components in `stratum-mining/sv2-apps`

These include:

- SV1 to SV2 translator proxy
- SV2 pool
- Job Declarator Client
- Job Declarator Server
- Template Provider integration

Those projects are open source under Apache-2.0 or MIT.

## System Model for a Two-Sided Marketplace

The cleanest architecture is a centralized marketplace with three planes:

- seller ingress plane
- market control plane
- delivery and settlement plane

### Seller ingress plane

This is where miner supply enters the marketplace.

Responsibilities:

- authenticate the seller or farm
- accept miner traffic
- aggregate many miners into fewer upstream sessions
- monitor hashrate, rejects, stales, and uptime
- apply local failover if needed

Typical deployment:

- farm proxy on the seller site for hosted farms
- lightweight seller agent or hosted edge proxy for smaller miners

### Market control plane

This is the business logic and web application.

Responsibilities:

- user accounts and auth
- wallet balances and deposits
- order creation and edits
- supply inventory and availability
- order matching and allocation
- policy enforcement
- buyer and seller dashboards
- support tooling

### Delivery and settlement plane

This is the protocol and accounting core.

Responsibilities:

- route live seller hashrate to the correct buyer destination
- track accepted, stale, and rejected shares
- attribute work to seller and buyer
- calculate delivered hash rate over time
- settle buyer spend and seller earnings
- generate evidence for disputes

## What the Marketplace Operator Needs To Run

### 1. Public website and authenticated application

Minimum components:

- landing page
- signup and onboarding
- auth and token management
- buyer dashboard
- seller dashboard
- orderbook and market stats
- transaction history
- support and compliance flows

The Braiins web app strongly suggests the following backend endpoints or equivalents:

- config and feature flags
- signup and registrator flow
- Telegram verification
- account balance
- permissions checks
- market stats
- spot market settings
- orderbook feed
- bid CRUD
- pool validation
- confirmation workflow
- favorite pools
- withdrawal or refund workflow

### 2. Wallet and treasury infrastructure

You need:

- dedicated BTC deposit address generation
- chain monitoring and confirmation tracking
- account balance ledger
- bid fund reservation
- refunds where applicable
- seller payout ledger
- operational treasury and reconciliation tools

This is not optional. Without a proper internal ledger, the market becomes impossible to audit or reconcile.

### 3. Matching and allocation engine

At minimum, the engine needs to:

- accept buy bids
- know available seller capacity
- allocate live capacity to funded demand
- rebalance when bids change
- stop allocations when funds are exhausted
- keep routing decisions consistent with settlement

It can start simple:

- fixed-price or RFQ
- then a live spot order book later

### 4. Routing and proxy infrastructure

This is the live delivery fabric.

It must:

- accept miner sessions
- maintain upstream pool sessions
- switch routing when allocations change
- preserve protocol correctness
- support failover and endpoint quality optimization
- avoid accounting ambiguity during reroutes

### 5. Share accounting and settlement engine

This is one of the hardest components.

It needs to:

- attribute accepted work to the right seller and buyer
- handle rejects and stale shares fairly
- convert observed work into spend and payout
- keep a holdback reserve for fraud and post-facto reconciliation
- expose delivery proofs and history

### 6. Compliance, fraud, and policy systems

A real marketplace needs:

- sanctions checks
- KYC triggers
- transaction monitoring
- suspicious order and seller behavior detection
- account freezes and manual review
- payout holds

This is not incidental. Braiins' own terms explicitly mention KYC, sanctions screening, and freezes.

### 7. Operator tooling

You need internal tools for:

- support investigation
- routing inspection
- session and miner health
- manual overrides
- reconciliation
- seller risk review
- pool compatibility incidents

## What Sellers Need To Run

There are two realistic seller profiles:

- professional farms / hosts
- individual miners

### Professional farms / hosts

Recommended stack:

- ASIC miners
- local farm proxy
- monitoring stack
- stable networking and failover
- seller identity and payout configuration

This is the best first-market segment because:

- hashrate is concentrated
- uptime is better
- support burden is lower
- fraud risk is lower
- routing is more predictable

### Individual miners

Minimum possible setup:

- ASIC miner pointed at marketplace-controlled proxy endpoint

Recommended setup:

- seller agent or proxy under marketplace control
- authenticated registration
- miner inventory and observed performance reporting

Why individuals are harder:

- inconsistent connectivity
- misconfigured pool settings
- low and unstable hashrate
- higher Sybil and fraud risk
- much higher support burden per PH

If the product starts with retail miners, the operator must expect a large operational tax.

## What Buyers Need To Run

Buyers fall into three categories.

### 1. Simple pool buyer

This is the easiest case.

The buyer only needs:

- funded marketplace account
- owner token
- compatible pool URL
- worker identity or username

The marketplace delivers hashpower to that pool/account. The buyer does not need to run mining infrastructure.

### 2. Advanced buyer with own pool integration

This buyer wants direct pool-side control or custom accounting.

They may need:

- their own pool endpoint
- compatibility with the marketplace routing model
- robust stratum configuration
- payout and accounting infrastructure on their side

This is more demanding but still manageable if the marketplace supports a vetted list of destinations.

### 3. SV2-native or solo-style buyer

This is the most advanced case.

Using the open Stratum V2 stack as reference, the buyer may run:

- SV2 pool
- Job Declarator Client
- Job Declarator Server
- Template Provider
- Bitcoin Core with IPC

This gives much more control, including custom job declaration or solo-style mining flows, but it is far beyond what a normal buyer will operate.

## Protocol and Infrastructure Components

### Stratum V1

Most deployed ASIC miners still speak Stratum V1.

A marketplace that wants broad hardware compatibility will almost certainly need:

- SV1 miner support downstream
- translation or proxying upstream if the operator wants SV2 internally

### Stratum V2

SV2 gives cleaner long-term primitives:

- channels
- encryption
- better routing
- explicit role separation

Relevant open components:

- translator proxy for SV1 miners to SV2 upstream
- SV2 pool
- Job Declarator Client
- Job Declarator Server
- Template Provider integration

### Why a translator or proxy matters

The translator/proxy sits between legacy miners and modern pool infrastructure. In an open reference stack, it already supports:

- downstream SV1 miner connections
- upstream SV2 pool or JDC connections
- channel aggregation
- failover
- monitoring
- vardiff options

This makes it an excellent base for a marketplace delivery engine if you do not want to write a proxy from scratch.

## Detailed Component Breakdown

### Miner-side proxy or seller agent

Required capabilities:

- downstream miner listener
- upstream destination management
- authentication to marketplace
- local telemetry
- remote config updates
- failover logic
- worker inventory awareness

Optional but useful:

- firmware integrations
- per-worker attribution
- batch configuration
- signed heartbeats

### Global routing proxy

Required capabilities:

- allocate miners or channels to buyers
- preserve search-space correctness
- support upstream pool multiplexing
- manage failover between buyer destinations
- report real-time throughput and acceptance

### Pool validator

Required capabilities:

- syntax validation of stratum URL
- live connectivity check
- compatibility check
- authorization check for supplied worker identity where possible
- difficulty and operational warnings

The Braiins frontend clearly performs this before bid creation.

### Confirmation service

Sensitive actions should require step-up confirmation:

- bid creation
- bid edits
- withdrawals or refunds
- account recovery

Braiins appears to use Telegram for this. Email or TOTP could also work, but out-of-band confirmations are useful for high-value actions.

### Observability stack

Required metrics:

- miner connectivity
- hashrate by seller
- hashrate by buyer
- upstream pool health
- accepted/rejected/stale shares
- routing changes
- settlement deltas
- pending imbalances

Useful tools:

- Prometheus
- Grafana
- structured logs
- event trail for every routing and financial action

## Marketplace Economics and Settlement

You need to define what is actually being bought and sold.

Braiin's terms effectively model it as leased computing power over time, measured by total delivered hashrate over a delivery period.

A marketplace needs a defensible internal unit of account:

- accepted-share based settlement
- plus time-windowed delivered-hashrate reporting for UX

Important policy questions:

- how to charge for stale or rejected work
- what happens during pool outages
- whether speed-limited bids have different minimums
- whether sellers are paid on gross observed work or net accepted work
- how shortfalls are made whole

The live Braiins frontend suggests the market has:

- min and max bid amount
- min and max bid price
- min speed limit
- separate constraints for limited-speed bids
- minimum duration constraints for limited-speed bids
- tick size on price

Those market parameters are essential for keeping routing and settlement manageable.

## Recommended MVP

The lowest-risk MVP is:

- SHA-256 only
- centralized marketplace
- hosted farms first, not retail sellers
- simple buyer flow to existing pool accounts
- spot market only
- limited list of supported upstream pools
- seller-side proxy required

### MVP components

Buyer-facing:

- account creation
- deposits
- owner/read-only access
- order creation
- supported pool selection
- delivery dashboard

Seller-facing:

- onboarding
- proxy registration
- observed capacity and uptime
- payout ledger

Operator-facing:

- routing control plane
- settlement pipeline
- compliance workflows
- support tools

### What to defer

Do not build these in v1 unless they are core to the thesis:

- open retail seller network
- derivatives or long-duration contracts
- fully decentralized matching
- custom firmware management
- arbitrary pool compatibility
- buyer-run solo infrastructure

## Recommended Product Sequence

### Phase 1

- centralized spot market
- professional sellers only
- buyer destination = vetted external pools
- seller payout based on delivered accepted work

### Phase 2

- broader seller onboarding
- better risk scoring
- richer reporting
- more pool compatibility

### Phase 3

- optional SV2-native buyer integrations
- advanced routing products
- maybe reserved capacity or term contracts

## Main Risks

Technical risks:

- protocol incompatibilities
- broken pool integrations
- routing-induced accounting errors
- share attribution bugs
- latency and stale-rate spikes

Marketplace risks:

- unreliable seller supply
- buyer expectations around precise delivered rate
- liquidity fragmentation
- support burden from retail miners

Compliance risks:

- sanctions exposure
- source-of-funds issues
- frozen balances
- regulatory recharacterization of the product

## Bottom Line

Yes, you can build a two-sided marketplace that connects miners to buyers.

No, the main problem is not just writing a stratum proxy.

The minimum real product is:

- a miner ingress layer
- a market control plane
- a routing layer
- a settlement and treasury system
- a compliance and abuse system

The most practical first version is a centralized marketplace where professional sellers point miners at your proxy and buyers point purchased hashpower at their own pool accounts.

## Sources

- Braiins Hashpower market page
  - https://hashpower.braiins.com/
- Braiins Hashpower account page
  - https://hashpower.braiins.com/account.html
- Braiins Hashpower orders page
  - https://hashpower.braiins.com/orders.html
- Braiins Hashpower signup page
  - https://hashpower.braiins.com/signup.html
- Braiins Hashpower terms and conditions
  - https://hashpower.braiins.com/terms-and-conditions.html
- Braiins Proxy product page
  - https://braiins.com/braiins-proxy
- Braiins blog: Why Miners Need Farm Proxies
  - https://braiins.com/blog/why-miners-need-farm-proxies
- Braiins Farm Proxy repo
  - https://github.com/braiins/farm-proxy
- Stratum V2 mining protocol specification
  - https://stratumprotocol.org/specification/05-mining-protocol/
- Stratum V2 low-level libraries
  - https://github.com/stratum-mining/stratum
- Stratum V2 application stack
  - https://github.com/stratum-mining/sv2-apps
