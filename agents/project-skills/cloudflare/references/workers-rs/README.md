# Cloudflare Workers Rust SDK

Use official [`cloudflare/workers-rs`](https://github.com/cloudflare/workers-rs) to implement a
Cloudflare Worker in Rust and compile it to WebAssembly.

## Contents

- [Runtime Boundary](#runtime-boundary)
- [Retrieval Workflow](#retrieval-workflow)
- [Setup](#setup)
- [Durable Object, SQLite, and Alarms](#durable-object-sqlite-and-alarms)
- [Scheduled Cron Handler](#scheduled-cron-handler)
- [Bindings](#bindings)
- [Outbound Fetch](#outbound-fetch)
- [Testing](#testing)

## Runtime Boundary

Select workers-rs for an existing Rust Worker or an explicit Rust request. For greenfield work,
compare the required crate ecosystem and current workers-rs binding coverage with the JavaScript
ecosystem. Select TypeScript when a required dependency, unsupported binding, or the Cloudflare
Agents SDK makes it the better runtime.

workers-rs calls the Workers runtime through generated JavaScript bootstrap code, WebAssembly,
`wasm-bindgen`, and `worker-sys` FFI. This means the runtime API is implemented through Wasm FFI;
it does not mean the application needs handwritten TypeScript. Introduce a narrow JavaScript or
TypeScript adapter only for a verified unsupported boundary.

## Retrieval Workflow

Check current APIs before writing code:

1. Open [`worker` on docs.rs](https://docs.rs/worker/latest/worker/) for crate features, public
   types, and signatures.
2. Run `btx cloudflare/workers-rs` and inspect the returned checkout.
3. Inspect `templates/` for current Cargo and Wrangler setup.
4. Inspect `worker/src/` for binding implementations and `test/` for working runtime examples.
5. Match dependency versions to the current template or existing project instead of copying the
   versions in this reference.

## Setup

Start from the official template when the repository does not already contain a Rust Worker:

```bash
cargo generate cloudflare/workers-rs
npx wrangler dev
```

The generated project uses a `cdylib`, the `worker` and `worker-macros` crates, `worker-build`, and a
Wrangler `main` entry that points to generated JavaScript. Preserve the generated entry path because
it can change between workers-rs releases.

## Durable Object, SQLite, and Alarms

Use the `DurableObject` trait and `#[durable_object]` macro. SQLite Durable Object storage is
available from `State::storage().sql()`, and alarms are available from the same storage object.

```rust
use serde::Deserialize;
use std::time::Duration;
use worker::{
    console_log, durable_object, wasm_bindgen, DurableObject, Env, Request, Response, Result,
    SqlStorage, State,
};

/// A counter stored in one Durable Object SQLite database
#[durable_object]
pub struct Counter {
    state: State,
    sql: SqlStorage,
}

impl DurableObject for Counter {
    fn new(state: State, _env: Env) -> Self {
        let sql = state.storage().sql();
        sql.exec(
            "CREATE TABLE IF NOT EXISTS counters (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)",
            None,
        )
        .expect("create counters table");

        Self { state, sql }
    }

    async fn fetch(&self, _request: Request) -> Result<Response> {
        #[derive(Deserialize)]
        struct CountRow {
            value: i32,
        }

        let row: CountRow = self
            .sql
            .exec(
                "INSERT INTO counters (id, value) VALUES (1, 1) \
                 ON CONFLICT(id) DO UPDATE SET value = value + 1 \
                 RETURNING value",
                None,
            )?
            .one()?;

        self.state
            .storage()
            .set_alarm(Duration::from_secs(60))
            .await?;

        Response::ok(row.value.to_string())
    }

    async fn alarm(&self) -> Result<Response> {
        console_log!("Counter alarm ran");
        Response::empty()
    }
}
```

Configure the binding and create the class as SQLite-backed storage:

```toml
[durable_objects]
bindings = [{ name = "COUNTER", class_name = "Counter" }]

[[migrations]]
tag = "v1"
new_sqlite_classes = ["Counter"]
```

Access the object from the Worker entrypoint:

```rust
let namespace = env.durable_object("COUNTER")?;
let stub = namespace.get_by_name("global")?;
let response = stub.fetch_with_str("https://counter.internal/increment").await?;
```

Use current workers-rs source for supported Durable Object RPC and WebSocket details. Do not copy a
TypeScript runtime class only because Cloudflare product documentation shows TypeScript first.

## Scheduled Cron Handler

Wrangler owns the cron schedule. Rust owns the scheduled handler:

```toml
[triggers]
crons = ["*/5 * * * *"]
```

```rust
use worker::{console_error, event, Env, Result, ScheduleContext, ScheduledEvent};

#[event(scheduled)]
async fn scheduled(event: ScheduledEvent, env: Env, ctx: ScheduleContext) {
    ctx.wait_until(async move {
        if let Err(error) = record_scheduled_run(event, env).await {
            console_error!("Scheduled handler failed: {error}");
        }
    });
}

async fn record_scheduled_run(event: ScheduledEvent, env: Env) -> Result<()> {
    let store = env.kv("JOBS")?;
    store
        .put("last-run", event.schedule().to_string())?
        .execute()
        .await?;

    Ok(())
}
```

The event macro requires `ScheduledEvent`, `Env`, and `ScheduleContext`, and the scheduled handler
returns `()`.

## Bindings

Use typed accessors on `Env`. Enable the corresponding Cargo feature where the crate requires one,
such as `d1` for D1 bindings.

```rust
let mode = env.var("MODE")?;
let api_token = env.secret("API_TOKEN")?;
let cache = env.kv("CACHE")?;
let files = env.bucket("FILES")?;
let database = env.d1("DB")?;
let counters = env.durable_object("COUNTER")?;
let upstream = env.service("UPSTREAM")?;
```

Do not expose secret values in responses or logs. When a binding has no stable wrapper, inspect
`Env::get_binding` and current source before deciding whether it needs custom Wasm bindings or a
narrow JavaScript/TypeScript adapter.

## Outbound Fetch

Use `Fetch` for outbound HTTP requests. Use the `Fetcher` returned by `Env::service` for a service
binding.

```rust
use worker::{event, Context, Env, Fetch, Request, Response, Result};

#[event(fetch)]
async fn fetch(_request: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let upstream = Fetch::Url("https://example.com/health".parse()?)
        .send()
        .await?;

    Response::ok(format!("upstream status={}", upstream.status_code()))
}
```

## Testing

Keep pure domain tests and suitable unit tests in Rust. Runtime integration tests usually:

1. Build the Rust Worker to Wasm and generated JavaScript with `worker-build` or a Wrangler dry run.
2. Start Miniflare from JavaScript or TypeScript.
3. Configure `modules: true` and a `CompiledWasm` module rule.
4. Configure the same bindings, Durable Object classes, and SQLite storage mode used by the Worker.
5. Dispatch fetch, scheduled, alarm, or queue events and assert observable behavior.

This JavaScript or TypeScript test harness is a Node interface to Miniflare. It does not require the
Worker application or Durable Object class to be TypeScript.
