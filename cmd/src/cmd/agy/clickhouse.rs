//! Inserting NDJSON rows over ClickHouse's HTTP interface

use std::time::Duration;

use eyre::{bail, Result, WrapErr};
use reqwest::Client;
use url::Url;

use crate::runtime::current_thread_runtime;

pub struct ClickHouse {
    // reqwest pools connections on the runtime that opened them, so every insert uses this one
    runtime: tokio::runtime::Runtime,
    client: Client,
    endpoint: String,
    user: String,
    password: String,
}

impl ClickHouse {
    pub fn new(endpoint: &str, user: &str, password: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            runtime: current_thread_runtime()?,
            client,
            endpoint: endpoint.trim_end_matches('/').to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        })
    }

    /// Inserts rows into `chat.<table>`, waiting until ClickHouse has written them
    pub fn insert(&self, table: &str, rows: &[u8]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        // ClickHouse drops an insert whose rows match one of the last 10,000, which would make an
        // upload after deleting rows silently do nothing; this never retries an insert, and the
        // tables' ReplacingMergeTree collapses rows a repeated run sends again
        let query = format!("INSERT INTO chat.{table} FORMAT JSONEachRow");
        let url = Url::parse_with_params(
            &format!("{}/", self.endpoint),
            [
                ("query", query.as_str()),
                ("insert_deduplicate", "0"),
                ("wait_end_of_query", "1"),
            ],
        )?;

        let request = self
            .client
            .post(url)
            .header("X-ClickHouse-User", &self.user)
            .header("X-ClickHouse-Key", &self.password)
            .body(rows.to_vec());

        let (status, text) = self
            .runtime
            .block_on(async {
                let response = request.send().await?;
                let status = response.status();
                Ok::<_, reqwest::Error>((status, response.text().await.unwrap_or_default()))
            })
            .wrap_err_with(|| format!("inserting into chat.{table}"))?;

        if !status.is_success() || text.contains("DB::Exception") {
            bail!(
                "ClickHouse refused rows for chat.{table} ({status}): {}",
                text.trim()
            );
        }

        Ok(())
    }
}
