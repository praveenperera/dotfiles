use std::collections::BTreeMap;
use std::fmt::Write as _;

use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, ValueEnum};
use eyre::{eyre, ContextCompat, Result};
use serde::{Deserialize, Serialize};

use crate::cmd::billing::{write_stdout, BillingOutput};

use super::{CloudflareApi, API_BASE_URL};

const ACCOUNT_ID_ENV_VAR: &str = "CLOUDFLARE_ACCOUNT_ID";
const BILLING_API_TOKEN_ENV_VAR: &str = "CMD_CLOUDFLARE_BILLING_API_TOKEN";

/// Arguments for the Cloudflare billing report
#[derive(Debug, Clone, Args)]
pub struct BillingArgs {
    /// Cloudflare account ID
    #[arg(long, env = ACCOUNT_ID_ENV_VAR, hide_env_values = true)]
    pub account_id: Option<String>,

    /// Cloudflare API token with Account Billing Read permission
    #[arg(long, env = BILLING_API_TOKEN_ENV_VAR, hide_env_values = true)]
    pub api_token: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t)]
    pub output: BillingOutput,

    /// Include products; repeat or separate values with commas
    #[arg(
        long = "products",
        visible_alias = "product",
        value_enum,
        value_delimiter = ','
    )]
    pub products: Vec<BillingProduct>,

    /// Include metrics with a zero billed cost
    #[arg(long)]
    pub verbose: bool,

    /// Override the Cloudflare API base URL
    #[arg(long, hide = true, default_value = API_BASE_URL)]
    pub api_base_url: String,
}

/// Cloudflare product that can be selected for a billing report
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BillingProduct {
    /// Cloudflare Workers
    Workers,
    /// Workers KV
    WorkersKv,
    /// Durable Objects (alias: do)
    #[value(alias = "do")]
    DurableObjects,
    /// R2 object storage
    R2,
    /// D1 databases
    D1,
    /// Cloudflare Queues
    Queues,
}

#[derive(Debug, Deserialize)]
pub(super) struct PaygoUsageRecord {
    #[serde(rename = "ServiceName")]
    service_name: String,
    #[serde(rename = "ServiceFamilyName")]
    service_family_name: Option<String>,
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
    products: Vec<ProductBilling>,
}

#[derive(Debug)]
struct ProductBilling {
    label: String,
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

#[derive(Debug, Serialize)]
struct JsonBillingReport {
    schema: &'static str,
    version: u32,
    period: Option<JsonBillingPeriod>,
    total_cost: Option<JsonMoney>,
    products: Vec<JsonProductBilling>,
}

#[derive(Debug, Serialize)]
struct JsonBillingPeriod {
    start: NaiveDate,
    usage_through: NaiveDate,
}

#[derive(Debug, Serialize)]
struct JsonProductBilling {
    name: String,
    total_cost: JsonMoney,
    metrics: Vec<JsonBillingMetric>,
}

#[derive(Debug, Serialize)]
struct JsonBillingMetric {
    name: String,
    usage: JsonQuantity,
    billable_usage: JsonQuantity,
    billed_cost: JsonMoney,
}

#[derive(Debug, Serialize)]
struct JsonQuantity {
    value: f64,
    unit: String,
}

#[derive(Debug, Serialize)]
struct JsonMoney {
    value: f64,
    currency: String,
}

pub(super) async fn run(args: BillingArgs) -> Result<()> {
    let account_id = required_arg(args.account_id, ACCOUNT_ID_ENV_VAR, "--account-id")?;
    let token = required_arg(args.api_token, BILLING_API_TOKEN_ENV_VAR, "--api-token")?;
    let api = CloudflareApi::new(args.api_base_url, token)?;
    let records = get_usage(&api, &account_id).await?;
    let report = BillingReport::new(
        account_id,
        records,
        &args.products,
        args.verbose,
        Utc::now().date_naive(),
    )?;
    let rendered = match args.output {
        BillingOutput::Human => render_report(&report)?,
        BillingOutput::Json => render_json_report(&report)?,
    };
    write_stdout(&rendered)
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
    fn new(
        account_id: String,
        records: Vec<PaygoUsageRecord>,
        selected_products: &[BillingProduct],
        verbose: bool,
        today: NaiveDate,
    ) -> Result<Self> {
        let period = BillingPeriod::current(&records, today)?;
        let mut products = BTreeMap::<String, BTreeMap<String, BillingMetric>>::new();

        for record in records
            .into_iter()
            .filter(|record| period.is_some_and(|period| period.contains(record)))
            .filter(|record| record.matches_any(selected_products))
        {
            let product_name = record.product_name().to_string();
            let metrics = products.entry(product_name).or_default();
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

        let products = products
            .into_iter()
            .filter_map(|(label, metrics)| {
                let metrics = metrics
                    .into_values()
                    .filter(|metric| verbose || metric.has_non_zero_cost())
                    .collect::<Vec<_>>();

                if metrics.is_empty() {
                    return None;
                }

                Some(ProductBilling { label, metrics })
            })
            .collect();

        Ok(Self {
            account_id,
            period,
            products,
        })
    }

    fn total_cost(&self) -> Result<Money> {
        let first = self
            .products
            .first()
            .context("cannot total an empty Cloudflare billing report")?;
        let mut total = Money::zero(first.total_cost()?.currency);

        for product in &self.products {
            total.merge(product.total_cost()?, "Cloudflare total")?;
        }

        Ok(total)
    }
}

impl BillingMetric {
    fn has_non_zero_cost(&self) -> bool {
        self.cost.value != 0.0
    }
}

impl ProductBilling {
    fn total_cost(&self) -> Result<Money> {
        let first = self
            .metrics
            .first()
            .context("cannot total a product without billing metrics")?;
        let mut total = Money::zero(first.cost.currency.clone());

        for metric in &self.metrics {
            total.merge(metric.cost.clone(), &self.label)?;
        }

        Ok(total)
    }
}

impl BillingProduct {
    fn service_family_name(self) -> &'static str {
        match self {
            Self::Workers => "Workers",
            Self::WorkersKv => "Workers KV",
            Self::DurableObjects => "Durable Objects",
            Self::R2 => "R2",
            Self::D1 => "D1",
            Self::Queues => "Queues",
        }
    }
}

impl PaygoUsageRecord {
    fn matches_any(&self, selected_products: &[BillingProduct]) -> bool {
        selected_products.is_empty()
            || self.service_family_name.as_deref().is_some_and(|family| {
                selected_products
                    .iter()
                    .any(|product| family.eq_ignore_ascii_case(product.service_family_name()))
            })
    }

    fn product_name(&self) -> &str {
        self.service_family_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Uncategorized")
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
    writeln!(output, "Cloudflare billing usage")?;
    writeln!(output, "Account: {}", report.account_id)?;

    match report.period {
        Some(period) => writeln!(
            output,
            "Period: {} through {}",
            period.start, period.usage_through
        )?,
        None => writeln!(output, "Period: unavailable")?,
    }

    if report.products.is_empty() {
        writeln!(output, "\nNo billable usage found")?;
        return Ok(output);
    }

    writeln!(
        output,
        "Total billed cost: {}",
        format_money(&report.total_cost()?)
    )?;

    for product in &report.products {
        writeln!(output, "\n{}", product.label)?;
        writeln!(
            output,
            "  Total billed cost: {}",
            format_money(&product.total_cost()?)
        )?;

        for metric in &product.metrics {
            writeln!(output, "\n  {}", metric.label)?;
            writeln!(output, "    Usage: {}", format_quantity(&metric.consumed))?;
            writeln!(
                output,
                "    Billable: {}",
                format_quantity(&metric.billable)
            )?;
            writeln!(output, "    Billed cost: {}", format_money(&metric.cost))?;
        }
    }

    Ok(output)
}

fn render_json_report(report: &BillingReport) -> Result<String> {
    let report = JsonBillingReport::new(report)?;
    let mut output = serde_json::to_string_pretty(&report)?;
    output.push('\n');

    Ok(output)
}

impl JsonBillingReport {
    fn new(report: &BillingReport) -> Result<Self> {
        let period = report.period.map(|period| JsonBillingPeriod {
            start: period.start,
            usage_through: period.usage_through,
        });
        let total_cost = report
            .products
            .first()
            .map(|_| report.total_cost())
            .transpose()?
            .map(|money| JsonMoney {
                value: money.value,
                currency: money.currency,
            });
        let products = report
            .products
            .iter()
            .map(JsonProductBilling::try_from)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            schema: "cmd.cloudflare.billing",
            version: 1,
            period,
            total_cost,
            products,
        })
    }
}

impl TryFrom<&ProductBilling> for JsonProductBilling {
    type Error = eyre::Report;

    fn try_from(product: &ProductBilling) -> Result<Self> {
        let total_cost = product.total_cost()?;

        Ok(Self {
            name: product.label.clone(),
            total_cost: JsonMoney {
                value: total_cost.value,
                currency: total_cost.currency,
            },
            metrics: product
                .metrics
                .iter()
                .map(JsonBillingMetric::from)
                .collect(),
        })
    }
}

impl From<&BillingMetric> for JsonBillingMetric {
    fn from(metric: &BillingMetric) -> Self {
        Self {
            name: metric.label.clone(),
            usage: JsonQuantity {
                value: metric.consumed.value,
                unit: metric.consumed.unit.clone(),
            },
            billable_usage: JsonQuantity {
                value: metric.billable.value,
                unit: metric.billable.unit.clone(),
            },
            billed_cost: JsonMoney {
                value: metric.cost.value,
                currency: metric.cost.currency.clone(),
            },
        }
    }
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
    use std::sync::Mutex;

    use chrono::{TimeZone, Utc};
    use clap::Args;
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{
        get_usage, render_json_report, render_report, BillingArgs, BillingOutput, BillingProduct,
        BillingReport, PaygoUsageRecord, BILLING_API_TOKEN_ENV_VAR,
    };
    use crate::cmd::cloudflare::CloudflareApi;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn human_output_is_the_default() {
        assert_eq!(BillingOutput::default(), BillingOutput::Human);
    }

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
    fn includes_all_products_in_the_current_billing_period() {
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
                "Future Service Requests",
                "Future Service",
                20_000.0,
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

        let report = BillingReport::new(
            "account-id".to_string(),
            records,
            &[],
            true,
            date(2026, 7, 28),
        )
        .unwrap();

        assert_eq!(
            report
                .products
                .iter()
                .map(|product| product.label.as_str())
                .collect::<Vec<_>>(),
            ["Future Service", "R2", "Workers"]
        );
        assert_eq!(report.products[1].metrics[0].consumed.value, 750_000.0);
        assert_eq!(report.period.unwrap().usage_through, date(2026, 7, 28));
    }

    #[test]
    fn filters_to_the_selected_products() {
        let records = vec![
            record(
                "R2 Requests",
                "R2",
                1.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "Workers Requests",
                "Workers",
                2.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "Durable Objects Requests",
                "Durable Objects",
                3.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
        ];

        let report = BillingReport::new(
            "account-id".to_string(),
            records,
            &[
                BillingProduct::Workers,
                BillingProduct::R2,
                BillingProduct::Workers,
            ],
            true,
            date(2026, 7, 28),
        )
        .unwrap();

        assert_eq!(
            report
                .products
                .iter()
                .map(|product| product.label.as_str())
                .collect::<Vec<_>>(),
            ["R2", "Workers"]
        );
    }

    #[test]
    fn removes_metrics_with_zero_billed_cost() {
        let records = vec![
            record(
                "D1 Rows Read",
                "D1",
                0.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "R2 Requests",
                "R2",
                1.0,
                0.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "Workers Requests",
                "Workers",
                0.0,
                2.0,
                0.0,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
            record(
                "Queues Operations",
                "Queues",
                0.0,
                0.0,
                0.10,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
        ];

        let report = BillingReport::new(
            "account-id".to_string(),
            records,
            &[],
            false,
            date(2026, 7, 28),
        )
        .unwrap();

        assert_eq!(
            report
                .products
                .iter()
                .map(|product| product.label.as_str())
                .collect::<Vec<_>>(),
            ["Queues"]
        );
    }

    #[test]
    fn sums_daily_usage_and_product_costs() {
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
            record(
                "Workers Standard Requests",
                "Workers",
                20_000_000.0,
                10_000_000.0,
                0.50,
                date(2026, 7, 16),
                date(2026, 7, 28),
            ),
        ];
        let report = BillingReport::new(
            "account-id".to_string(),
            records,
            &[],
            false,
            date(2026, 7, 28),
        )
        .unwrap();

        let rendered = render_report(&report).unwrap();

        assert!(rendered.contains("Period: 2026-07-16 through 2026-07-27"));
        assert!(rendered.contains("Total billed cost: $1.22 USD"));
        assert!(rendered.contains("R2\n  Total billed cost: $0.72 USD"));
        assert!(rendered.contains("Workers\n  Total billed cost: $0.50 USD"));
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
            &[],
            true,
            date(2026, 7, 28),
        )
        .unwrap();

        let rendered = render_report(&report).unwrap();

        assert!(rendered.contains("Usage: 4\n"));
        assert!(rendered.contains("Billable: 0\n"));
    }

    #[test]
    fn billing_help_never_displays_the_api_token() {
        let _lock = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os(BILLING_API_TOKEN_ENV_VAR);
        std::env::set_var(BILLING_API_TOKEN_ENV_VAR, "help-secret-token");
        let mut command = BillingArgs::augment_args(clap::Command::new("billing"));

        let help = command.render_long_help().to_string();

        match previous {
            Some(value) => std::env::set_var(BILLING_API_TOKEN_ENV_VAR, value),
            None => std::env::remove_var(BILLING_API_TOKEN_ENV_VAR),
        }
        assert!(help.contains(BILLING_API_TOKEN_ENV_VAR));
        assert!(!help.contains("help-secret-token"));
    }

    #[test]
    fn renders_stable_sanitized_json_in_product_and_metric_order() {
        let report = BillingReport::new(
            "sensitive-account-id".to_string(),
            vec![
                record(
                    "Workers Standard Requests",
                    "Workers",
                    20_000_000.0,
                    10_000_000.0,
                    0.50,
                    date(2026, 7, 16),
                    date(2026, 7, 28),
                ),
                record(
                    "R2 Standard Class B Operations",
                    "R2",
                    12_000_000.0,
                    2_000_000.0,
                    0.72,
                    date(2026, 7, 16),
                    date(2026, 7, 28),
                ),
                record(
                    "R2 Standard Class A Operations",
                    "R2",
                    3_000_000.0,
                    2_000_000.0,
                    0.25,
                    date(2026, 7, 16),
                    date(2026, 7, 28),
                ),
            ],
            &[],
            false,
            date(2026, 7, 28),
        )
        .unwrap();

        let rendered = render_json_report(&report).unwrap();

        assert_eq!(
            rendered,
            concat!(
                "{\n",
                "  \"schema\": \"cmd.cloudflare.billing\",\n",
                "  \"version\": 1,\n",
                "  \"period\": {\n",
                "    \"start\": \"2026-07-16\",\n",
                "    \"usage_through\": \"2026-07-27\"\n",
                "  },\n",
                "  \"total_cost\": {\n",
                "    \"value\": 1.47,\n",
                "    \"currency\": \"USD\"\n",
                "  },\n",
                "  \"products\": [\n",
                "    {\n",
                "      \"name\": \"R2\",\n",
                "      \"total_cost\": {\n",
                "        \"value\": 0.97,\n",
                "        \"currency\": \"USD\"\n",
                "      },\n",
                "      \"metrics\": [\n",
                "        {\n",
                "          \"name\": \"R2 Standard Class A Operations\",\n",
                "          \"usage\": {\n",
                "            \"value\": 3000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billable_usage\": {\n",
                "            \"value\": 2000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billed_cost\": {\n",
                "            \"value\": 0.25,\n",
                "            \"currency\": \"USD\"\n",
                "          }\n",
                "        },\n",
                "        {\n",
                "          \"name\": \"R2 Standard Class B Operations\",\n",
                "          \"usage\": {\n",
                "            \"value\": 12000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billable_usage\": {\n",
                "            \"value\": 2000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billed_cost\": {\n",
                "            \"value\": 0.72,\n",
                "            \"currency\": \"USD\"\n",
                "          }\n",
                "        }\n",
                "      ]\n",
                "    },\n",
                "    {\n",
                "      \"name\": \"Workers\",\n",
                "      \"total_cost\": {\n",
                "        \"value\": 0.5,\n",
                "        \"currency\": \"USD\"\n",
                "      },\n",
                "      \"metrics\": [\n",
                "        {\n",
                "          \"name\": \"Workers Standard Requests\",\n",
                "          \"usage\": {\n",
                "            \"value\": 20000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billable_usage\": {\n",
                "            \"value\": 10000000.0,\n",
                "            \"unit\": \"requests\"\n",
                "          },\n",
                "          \"billed_cost\": {\n",
                "            \"value\": 0.5,\n",
                "            \"currency\": \"USD\"\n",
                "          }\n",
                "        }\n",
                "      ]\n",
                "    }\n",
                "  ]\n",
                "}\n",
            )
        );
        assert!(!rendered.contains("sensitive-account-id"));
        assert!(!rendered.contains("api_token"));
    }

    #[test]
    fn renders_empty_json_with_stable_nullable_fields() {
        let report = BillingReport::new(
            "sensitive-account-id".to_string(),
            vec![],
            &[],
            false,
            date(2026, 7, 28),
        )
        .unwrap();

        let rendered = render_json_report(&report).unwrap();

        assert_eq!(
            rendered,
            concat!(
                "{\n",
                "  \"schema\": \"cmd.cloudflare.billing\",\n",
                "  \"version\": 1,\n",
                "  \"period\": null,\n",
                "  \"total_cost\": null,\n",
                "  \"products\": []\n",
                "}\n",
            )
        );
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
            service_family_name: Some(service_family_name.to_string()),
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
