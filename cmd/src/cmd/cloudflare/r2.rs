use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, Write as _};

use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, Subcommand};
use eyre::{eyre, ContextCompat, Result};
use serde::Deserialize;

use super::{CloudflareApi, API_BASE_URL};

const ACCOUNT_ID_ENV_VAR: &str = "CLOUDFLARE_ACCOUNT_ID";
const BILLING_API_TOKEN_ENV_VAR: &str = "CMD_CLOUDFLARE_BILLING_API_TOKEN";

#[derive(Debug, Clone, Subcommand)]
pub enum R2Cmd {
    /// Show R2 usage and charges for the current billing period
    Billing(#[command(flatten)] BillingArgs),
}

#[derive(Debug, Clone, Args)]
pub struct BillingArgs {
    /// Cloudflare account ID
    #[arg(long, env = ACCOUNT_ID_ENV_VAR)]
    pub account_id: Option<String>,

    /// Cloudflare API token with Account Billing Read permission
    #[arg(long, env = BILLING_API_TOKEN_ENV_VAR)]
    pub api_token: Option<String>,

    /// Override the Cloudflare API base URL
    #[arg(long, hide = true, default_value = API_BASE_URL)]
    pub api_base_url: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PaygoUsageRecord {
    #[serde(rename = "ServiceName")]
    service_name: String,
    #[serde(rename = "ServiceFamilyName")]
    service_family_name: String,
    #[serde(rename = "BillingPeriodStart")]
    billing_period_start: DateTime<Utc>,
    #[serde(rename = "ChargePeriodEnd")]
    charge_period_end: DateTime<Utc>,
    #[serde(rename = "ConsumedQuantity")]
    consumed_quantity: f64,
    #[serde(rename = "ConsumedUnit")]
    consumed_unit: String,
    #[serde(rename = "PricingQuantity")]
    pricing_quantity: f64,
    #[serde(rename = "ContractedCost")]
    contracted_cost: f64,
    #[serde(rename = "BillingCurrency")]
    billing_currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BillingPeriod {
    start: NaiveDate,
    usage_through: NaiveDate,
}

#[derive(Debug)]
struct BillingReport {
    account_id: String,
    period: Option<BillingPeriod>,
    metrics: Vec<BillingMetric>,
}

#[derive(Debug)]
struct BillingMetric {
    label: String,
    consumed: Quantity,
    billable: Quantity,
    cost: Money,
}

#[derive(Debug, Clone)]
struct Quantity {
    value: f64,
    unit: String,
}

#[derive(Debug, Clone)]
struct Money {
    value: f64,
    currency: String,
}

pub(super) async fn run(command: R2Cmd) -> Result<()> {
    match command {
        R2Cmd::Billing(args) => run_billing(args).await,
    }
}

async fn run_billing(args: BillingArgs) -> Result<()> {
    let account_id = required_arg(args.account_id, ACCOUNT_ID_ENV_VAR, "--account-id")?;
    let token = required_arg(args.api_token, BILLING_API_TOKEN_ENV_VAR, "--api-token")?;
    let api = CloudflareApi::new(args.api_base_url, token)?;
    let records = get_usage(&api, &account_id).await?;
    let report = BillingReport::new(account_id, records, Utc::now().date_naive())?;
    let rendered = render_report(&report)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;

    Ok(())
}

async fn get_usage(api: &CloudflareApi, account_id: &str) -> Result<Vec<PaygoUsageRecord>> {
    api.get_json(
        &format!("accounts/{account_id}/paygo-usage"),
        "get account PayGo usage",
    )
    .await
}

fn required_arg(value: Option<String>, env_var: &str, flag: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| eyre!("set {env_var} or pass {flag}"))
}

impl BillingPeriod {
    fn current(records: &[PaygoUsageRecord], today: NaiveDate) -> Result<Option<Self>> {
        let Some(start) = records
            .iter()
            .map(PaygoUsageRecord::billing_period_start)
            .filter(|start| *start <= today)
            .max()
        else {
            return Ok(None);
        };
        let charge_period_end = records
            .iter()
            .filter(|record| record.billing_period_start() == start)
            .map(PaygoUsageRecord::charge_period_end)
            .max()
            .context("Cloudflare returned a billing period without usage dates")?;
        let usage_through = charge_period_end
            .pred_opt()
            .context("Cloudflare returned an invalid charge period")?;

        if usage_through < start {
            return Err(eyre!(
                "Cloudflare returned a charge period ending before the billing period started"
            ));
        }

        Ok(Some(Self {
            start,
            usage_through,
        }))
    }

    fn contains(self, record: &PaygoUsageRecord) -> bool {
        record.billing_period_start() == self.start
    }
}

impl BillingReport {
    fn new(account_id: String, records: Vec<PaygoUsageRecord>, today: NaiveDate) -> Result<Self> {
        let period = BillingPeriod::current(&records, today)?;
        let mut metrics = BTreeMap::<String, BillingMetric>::new();

        for record in records
            .into_iter()
            .filter(PaygoUsageRecord::is_r2)
            .filter(|record| period.is_some_and(|period| period.contains(record)))
        {
            let metric = metrics
                .entry(record.service_name.clone())
                .or_insert_with(|| BillingMetric {
                    label: record.service_name.clone(),
                    consumed: Quantity::zero(record.consumed_unit.clone()),
                    billable: Quantity::zero(record.consumed_unit.clone()),
                    cost: Money::zero(record.billing_currency.clone()),
                });

            metric.consumed.add(
                record.consumed_quantity,
                &record.consumed_unit,
                &metric.label,
            )?;
            metric.billable.add(
                record.pricing_quantity,
                &record.consumed_unit,
                &metric.label,
            )?;
            metric.cost.add(
                record.contracted_cost,
                &record.billing_currency,
                &metric.label,
            )?;
        }

        Ok(Self {
            account_id,
            period,
            metrics: metrics.into_values().collect(),
        })
    }

    fn total_cost(&self) -> Result<Money> {
        let first = self
            .metrics
            .first()
            .context("cannot total an empty R2 billing report")?;
        let mut total = Money::zero(first.cost.currency.clone());

        for metric in &self.metrics {
            total.merge(metric.cost.clone(), "R2 total")?;
        }

        Ok(total)
    }
}

impl PaygoUsageRecord {
    fn is_r2(&self) -> bool {
        self.service_family_name.eq_ignore_ascii_case("R2")
    }

    fn billing_period_start(&self) -> NaiveDate {
        self.billing_period_start.date_naive()
    }

    fn charge_period_end(&self) -> NaiveDate {
        self.charge_period_end.date_naive()
    }
}

impl Quantity {
    fn zero(unit: String) -> Self {
        Self { value: 0.0, unit }
    }

    fn add(&mut self, value: f64, unit: &str, metric: &str) -> Result<()> {
        ensure_finite(value, metric)?;
        if self.unit != unit {
            return Err(eyre!(
                "Cloudflare returned mixed consumed units for {metric}: {} and {unit}",
                self.unit
            ));
        }

        self.value += value;
        Ok(())
    }
}

impl Money {
    fn zero(currency: String) -> Self {
        Self {
            value: 0.0,
            currency,
        }
    }

    fn add(&mut self, value: f64, currency: &str, metric: &str) -> Result<()> {
        ensure_finite(value, metric)?;
        if self.currency != currency {
            return Err(eyre!(
                "Cloudflare returned mixed billing currencies for {metric}: {} and {currency}",
                self.currency
            ));
        }

        self.value += value;
        Ok(())
    }

    fn merge(&mut self, other: Self, metric: &str) -> Result<()> {
        self.add(other.value, &other.currency, metric)
    }
}

fn ensure_finite(value: f64, metric: &str) -> Result<()> {
    if value.is_finite() {
        return Ok(());
    }

    Err(eyre!("Cloudflare returned a non-finite value for {metric}"))
}

fn render_report(report: &BillingReport) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "R2 billing usage")?;
    writeln!(output, "Account: {}", report.account_id)?;

    match report.period {
        Some(period) => writeln!(
            output,
            "Period: {} through {}",
            period.start, period.usage_through
        )?,
        None => writeln!(output, "Period: unavailable")?,
    }

    if report.metrics.is_empty() {
        writeln!(output, "\nNo R2 billable usage found")?;
        return Ok(output);
    }

    writeln!(
        output,
        "Total billed cost: {}",
        format_money(&report.total_cost()?)
    )?;

    for metric in &report.metrics {
        writeln!(output, "\n{}", metric.label)?;
        writeln!(output, "  Usage: {}", format_quantity(&metric.consumed))?;
        writeln!(output, "  Billable: {}", format_quantity(&metric.billable))?;
        writeln!(output, "  Billed cost: {}", format_money(&metric.cost))?;
    }

    Ok(output)
}

fn format_quantity(quantity: &Quantity) -> String {
    let value = format_number(quantity.value);
    if quantity.unit.is_empty() {
        return value;
    }

    format!("{value} {}", quantity.unit)
}

fn format_money(money: &Money) -> String {
    if money.currency == "USD" {
        return format!("${:.2} USD", money.value);
    }

    format!("{:.2} {}", money.value, money.currency)
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }

    format!("{value:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{get_usage, render_report, BillingReport, PaygoUsageRecord};
    use crate::cmd::cloudflare::CloudflareApi;

    #[tokio::test]
    async fn fetches_paygo_usage_with_billing_read_permission() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/accounts/account-id/paygo-usage"))
            .and(header("authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "errors": [],
                "messages": [],
                "result": [{
                    "BillingCurrency": "USD",
                    "BillingPeriodStart": "2026-07-16T00:00:00Z",
                    "ChargePeriodEnd": "2026-07-28T00:00:00Z",
                    "ChargePeriodStart": "2026-07-27T00:00:00Z",
                    "ConsumedQuantity": 12000000,
                    "ConsumedUnit": "requests",
                    "ContractedCost": 0.72,
                    "CumulatedContractedCost": 0.72,
                    "CumulatedPricingQuantity": 2000000,
                    "PricingQuantity": 2000000,
                    "ServiceFamilyName": "R2",
                    "ServiceName": "R2 Standard Class B Operations",
                    "SubscriptionId": "subscription-id"
                }]
            })))
            .mount(&server)
            .await;
        let api = CloudflareApi::new(server.uri(), "token".to_string()).unwrap();

        let records = get_usage(&api, "account-id").await.unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].service_name, "R2 Standard Class B Operations");
        assert_eq!(records[0].contracted_cost, 0.72);
    }

    #[test]
    fn limits_report_to_r2_records_in_the_current_billing_period() {
        let records = vec![
            record(
                "R2 Standard Class A Operations",
                "R2",
                750_000.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "Workers Standard Requests",
                "Workers",
                10_000.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 29),
            ),
            record(
                "R2 Standard Class A Operations",
                "R2",
                250_000.0,
                0.0,
                0.0,
                date(2026, 6, 16),
                date(2026, 7, 16),
            ),
        ];

        let report =
            BillingReport::new("account-id".to_string(), records, date(2026, 7, 28)).unwrap();

        assert_eq!(report.metrics.len(), 1);
        assert_eq!(report.metrics[0].consumed.value, 750_000.0);
        assert_eq!(report.period.unwrap().usage_through, date(2026, 7, 28));
    }

    #[test]
    fn sums_daily_usage_billable_quantity_and_cost() {
        let records = vec![
            record(
                "R2 Standard Class B Operations",
                "R2",
                7_000_000.0,
                1_000_000.0,
                0.32,
                date(2026, 7, 16),
                date(2026, 7, 27),
            ),
            record(
                "R2 Standard Class B Operations",
                "R2",
                5_000_000.0,
                1_000_000.0,
                0.40,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
        ];
        let report =
            BillingReport::new("account-id".to_string(), records, date(2026, 7, 28)).unwrap();

        let rendered = render_report(&report).unwrap();

        assert!(rendered.contains("Period: 2026-07-16 through 2026-07-27"));
        assert!(rendered.contains("Total billed cost: $0.72 USD"));
        assert!(rendered.contains("Usage: 12000000 requests"));
        assert!(rendered.contains("Billable: 2000000 requests"));
        assert!(rendered.contains("Billed cost: $0.72 USD"));
    }

    #[test]
    fn omits_empty_units_from_quantities() {
        let report = BillingReport::new(
            "account-id".to_string(),
            vec![record(
                "R2 Storage Class A Operations",
                "R2",
                4.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            )
            .with_unit("")],
            date(2026, 7, 28),
        )
        .unwrap();

        let rendered = render_report(&report).unwrap();

        assert!(rendered.contains("Usage: 4\n"));
        assert!(rendered.contains("Billable: 0\n"));
    }

    fn date(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn record(
        service_name: &str,
        service_family_name: &str,
        consumed_quantity: f64,
        pricing_quantity: f64,
        contracted_cost: f64,
        billing_period_start: chrono::NaiveDate,
        charge_period_end: chrono::NaiveDate,
    ) -> PaygoUsageRecord {
        PaygoUsageRecord {
            service_name: service_name.to_string(),
            service_family_name: service_family_name.to_string(),
            billing_period_start: timestamp(billing_period_start),
            charge_period_end: timestamp(charge_period_end),
            consumed_quantity,
            consumed_unit: "requests".to_string(),
            pricing_quantity,
            contracted_cost,
            billing_currency: "USD".to_string(),
        }
    }

    fn timestamp(date: chrono::NaiveDate) -> chrono::DateTime<Utc> {
        Utc.from_utc_datetime(
            &date
                .and_hms_opt(0, 0, 0)
                .expect("test date should have a valid midnight"),
        )
    }

    trait UsageRecordExt {
        fn with_unit(self, unit: &str) -> Self;
    }

    impl UsageRecordExt for PaygoUsageRecord {
        fn with_unit(mut self, unit: &str) -> Self {
            self.consumed_unit = unit.to_string();
            self
        }
    }
}
