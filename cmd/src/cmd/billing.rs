//! Billing commands, shared report types, and formatting

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::str::FromStr;

use chrono::{Datelike, NaiveDate};
use clap::{Args, Subcommand, ValueEnum};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::runtime;

/// Arguments for the combined billing command
#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Billing {
    /// Output format for the combined overview
    #[arg(long, value_enum, default_value_t)]
    pub output: BillingOutput,

    /// Run one provider's existing billing report
    #[command(subcommand)]
    pub subcommand: Option<BillingCmd>,
}

/// Providers available through the combined billing command
#[derive(Debug, Clone, Subcommand)]
pub enum BillingCmd {
    /// Show the AWS billing report
    Aws(#[command(flatten)] crate::cmd::aws::BillingArgs),

    /// Show the Cloudflare billing report
    #[command(visible_alias = "cf")]
    Cloudflare(#[command(flatten)] crate::cmd::cloudflare::BillingArgs),

    /// Show the DigitalOcean billing report
    #[command(
        name = "digitalocean",
        visible_aliases = ["do", "digital-ocean"]
    )]
    DigitalOcean(#[command(flatten)] crate::cmd::digitalocean::BillingArgs),
}

/// Output format for a cloud billing report
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum BillingOutput {
    /// Human-readable text
    #[default]
    Human,
    /// Stable machine-readable JSON
    Json,
}

/// Inclusive dates covered by a billing report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct BillingPeriod {
    pub(crate) start: NaiveDate,
    pub(crate) usage_through: NaiveDate,
}

impl BillingPeriod {
    pub(crate) fn current_month(today: NaiveDate) -> Result<Self> {
        let start = today
            .with_day(1)
            .ok_or_else(|| eyre!("calculate the first day of the current month"))?;

        Ok(Self {
            start,
            usage_through: today,
        })
    }

    pub(crate) fn end_exclusive(self) -> Result<NaiveDate> {
        self.usage_through
            .succ_opt()
            .ok_or_else(|| eyre!("calculate the end of the current billing period"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Money {
    value: Decimal,
    currency: String,
}

impl Money {
    pub(crate) fn new(amount: &str, currency: &str, context: &str) -> Result<Self> {
        let value = parse_decimal(amount)
            .wrap_err_with(|| format!("{context} has an invalid monetary amount"))?;

        Self::from_decimal(value, currency, context)
    }

    pub(crate) fn from_decimal(value: Decimal, currency: &str, context: &str) -> Result<Self> {
        let currency = currency.trim();
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(eyre!("{context} has an invalid currency code"));
        }

        Ok(Self {
            value: value.normalize(),
            currency: currency.to_string(),
        })
    }

    pub(crate) fn zero(currency: impl Into<String>) -> Self {
        Self {
            value: Decimal::ZERO,
            currency: currency.into(),
        }
    }

    pub(crate) fn add(&mut self, other: &Self, context: &str) -> Result<()> {
        if self.currency != other.currency {
            return Err(eyre!(
                "{context} uses mixed billing currencies: {} and {}",
                self.currency,
                other.currency
            ));
        }
        self.value = self
            .value
            .checked_add(other.value)
            .ok_or_else(|| eyre!("{context} monetary total is too large"))?
            .normalize();

        Ok(())
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    pub(crate) fn currency(&self) -> &str {
        &self.currency
    }

    pub(crate) fn json(&self) -> JsonMoney {
        JsonMoney {
            value: self.value,
            currency: self.currency.clone(),
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.currency == "USD" {
            return write!(formatter, "${:.2} USD", self.value);
        }

        write!(formatter, "{:.2} {}", self.value, self.currency)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct JsonMoney {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    value: Decimal,
    currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum BillingProvider {
    Aws,
    Cloudflare,
    DigitalOcean,
}

impl BillingProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Cloudflare => "Cloudflare",
            Self::DigitalOcean => "DigitalOcean",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProviderBillingSummary {
    period: Option<BillingPeriod>,
    total_cost: Option<Money>,
}

impl ProviderBillingSummary {
    pub(crate) fn new(period: Option<BillingPeriod>, total_cost: Option<Money>) -> Self {
        Self { period, total_cost }
    }
}

#[derive(Debug)]
enum ProviderOutcome {
    Ok(ProviderBillingSummary),
    Error(String),
}

impl ProviderOutcome {
    fn from_result(result: Result<ProviderBillingSummary>) -> Self {
        match result {
            Ok(summary) => Self::Ok(summary),
            Err(error) => Self::Error(format!("{error:#}")),
        }
    }
}

#[derive(Debug)]
enum GrandTotal {
    Available(Money),
    NoUsage,
    MixedCurrencies,
    Incomplete,
}

#[derive(Debug)]
struct BillingOverview {
    aws: ProviderOutcome,
    cloudflare: ProviderOutcome,
    digitalocean: ProviderOutcome,
    grand_total: GrandTotal,
}

impl BillingOverview {
    fn new(
        aws: Result<ProviderBillingSummary>,
        cloudflare: Result<ProviderBillingSummary>,
        digitalocean: Result<ProviderBillingSummary>,
    ) -> Result<Self> {
        let aws = ProviderOutcome::from_result(aws);
        let cloudflare = ProviderOutcome::from_result(cloudflare);
        let digitalocean = ProviderOutcome::from_result(digitalocean);
        let grand_total = calculate_grand_total([&aws, &cloudflare, &digitalocean])?;

        Ok(Self {
            aws,
            cloudflare,
            digitalocean,
            grand_total,
        })
    }

    fn providers(&self) -> [(BillingProvider, &ProviderOutcome); 3] {
        [
            (BillingProvider::Aws, &self.aws),
            (BillingProvider::Cloudflare, &self.cloudflare),
            (BillingProvider::DigitalOcean, &self.digitalocean),
        ]
    }

    fn failure_count(&self) -> usize {
        self.providers()
            .into_iter()
            .filter(|(_, outcome)| matches!(outcome, ProviderOutcome::Error(_)))
            .count()
    }

    fn is_complete(&self) -> bool {
        self.failure_count() == 0
    }
}

fn calculate_grand_total(outcomes: [&ProviderOutcome; 3]) -> Result<GrandTotal> {
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, ProviderOutcome::Error(_)))
    {
        return Ok(GrandTotal::Incomplete);
    }

    let mut totals = outcomes.iter().filter_map(|outcome| match outcome {
        ProviderOutcome::Ok(summary) => summary.total_cost.as_ref(),
        ProviderOutcome::Error(_) => None,
    });
    let Some(first) = totals.next() else {
        return Ok(GrandTotal::NoUsage);
    };
    if totals
        .clone()
        .any(|total| total.currency() != first.currency())
    {
        return Ok(GrandTotal::MixedCurrencies);
    }

    let mut grand_total = Money::zero(first.currency());
    grand_total.add(first, "billing overview grand total")?;
    for total in totals {
        grand_total.add(total, "billing overview grand total")?;
    }

    Ok(GrandTotal::Available(grand_total))
}

#[derive(Debug, Serialize)]
struct JsonBillingOverview {
    schema: &'static str,
    version: u32,
    complete: bool,
    grand_total: Option<JsonMoney>,
    providers: Vec<JsonProviderOutcome>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum JsonProviderOutcome {
    Ok {
        provider: BillingProvider,
        period: Option<BillingPeriod>,
        total_cost: Option<JsonMoney>,
    },
    Error {
        provider: BillingProvider,
        error: String,
    },
}

impl From<&BillingOverview> for JsonBillingOverview {
    fn from(overview: &BillingOverview) -> Self {
        let providers = overview
            .providers()
            .into_iter()
            .map(|(provider, outcome)| match outcome {
                ProviderOutcome::Ok(summary) => JsonProviderOutcome::Ok {
                    provider,
                    period: summary.period,
                    total_cost: summary.total_cost.as_ref().map(Money::json),
                },
                ProviderOutcome::Error(error) => JsonProviderOutcome::Error {
                    provider,
                    error: error.clone(),
                },
            })
            .collect();
        let grand_total = match &overview.grand_total {
            GrandTotal::Available(total) => Some(total.json()),
            GrandTotal::NoUsage | GrandTotal::MixedCurrencies | GrandTotal::Incomplete => None,
        };

        Self {
            schema: "cmd.billing",
            version: 1,
            complete: overview.is_complete(),
            grand_total,
            providers,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LabelFilter(Vec<String>);

impl LabelFilter {
    pub(crate) fn new(values: Vec<String>) -> Result<Self> {
        let mut labels = BTreeSet::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() {
                return Err(eyre!("billing filters must not be empty"));
            }

            labels.insert(value.to_lowercase());
        }

        Ok(Self(labels.into_iter().collect()))
    }

    pub(crate) fn matches(&self, label: &str) -> bool {
        self.0.is_empty() || self.0.binary_search(&label.to_lowercase()).is_ok()
    }
}

pub(crate) fn write_stdout(rendered: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;

    Ok(())
}

/// Runs the combined billing command
pub fn run_with_flags(flags: Billing) -> Result<()> {
    runtime::block_on(run_async(flags))?
}

async fn run_async(flags: Billing) -> Result<()> {
    match flags.subcommand {
        Some(BillingCmd::Aws(args)) => crate::cmd::aws::run_billing(args).await,
        Some(BillingCmd::Cloudflare(args)) => crate::cmd::cloudflare::run_billing(args).await,
        Some(BillingCmd::DigitalOcean(args)) => crate::cmd::digitalocean::run_billing(args).await,
        None => run_overview(flags.output).await,
    }
}

async fn run_overview(output: BillingOutput) -> Result<()> {
    let (aws, cloudflare, digitalocean) = tokio::join!(
        crate::cmd::aws::billing_summary(),
        crate::cmd::cloudflare::billing_summary(),
        crate::cmd::digitalocean::billing_summary(),
    );
    let overview = BillingOverview::new(aws, cloudflare, digitalocean)?;
    let rendered = match output {
        BillingOutput::Human => render_overview(&overview)?,
        BillingOutput::Json => render_json_overview(&overview)?,
    };

    write_stdout(&rendered)?;

    let failure_count = overview.failure_count();
    if failure_count == 0 {
        return Ok(());
    }

    Err(eyre!(
        "{failure_count} billing provider{} failed",
        if failure_count == 1 { "" } else { "s" }
    ))
}

fn render_overview(overview: &BillingOverview) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "Billing overview")?;

    for (provider, outcome) in overview.providers() {
        writeln!(output, "\n{}", provider.label())?;
        match outcome {
            ProviderOutcome::Ok(summary) => {
                if let Some(period) = summary.period {
                    writeln!(
                        output,
                        "  Period: {} through {}",
                        period.start, period.usage_through
                    )?;
                } else {
                    writeln!(output, "  Period: unavailable")?;
                }

                if let Some(total) = &summary.total_cost {
                    writeln!(output, "  Total billed cost: {total}")?;
                } else {
                    writeln!(output, "  Total billed cost: no billable usage")?;
                }
            }
            ProviderOutcome::Error(error) => {
                for (index, line) in error.lines().enumerate() {
                    let prefix = if index == 0 { "  Error: " } else { "         " };
                    writeln!(output, "{prefix}{line}")?;
                }
            }
        }
    }

    match &overview.grand_total {
        GrandTotal::Available(total) => writeln!(output, "\nGrand total: {total}")?,
        GrandTotal::NoUsage => writeln!(output, "\nGrand total: no billable usage")?,
        GrandTotal::MixedCurrencies => writeln!(
            output,
            "\nGrand total: unavailable because currencies differ"
        )?,
        GrandTotal::Incomplete => writeln!(
            output,
            "\nGrand total: unavailable because the report is incomplete"
        )?,
    }

    Ok(output)
}

fn render_json_overview(overview: &BillingOverview) -> Result<String> {
    let mut output = serde_json::to_string_pretty(&JsonBillingOverview::from(overview))?;
    output.push('\n');

    Ok(output)
}

fn parse_decimal(value: &str) -> Result<Decimal, rust_decimal::Error> {
    if value.trim() != value || value.is_empty() {
        return Decimal::from_str("");
    }

    Decimal::from_str(value).or_else(|_| Decimal::from_scientific(value))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use eyre::eyre;

    use super::{
        render_json_overview, render_overview, BillingOverview, BillingPeriod, GrandTotal,
        LabelFilter, Money, ProviderBillingSummary,
    };

    #[test]
    fn current_month_includes_today() {
        let period =
            BillingPeriod::current_month(NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()).unwrap();

        assert_eq!(period.start, NaiveDate::from_ymd_opt(2026, 8, 1).unwrap());
        assert_eq!(
            period.end_exclusive().unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()
        );
    }

    #[test]
    fn money_addition_is_exact() {
        let mut total = Money::new("0.1", "USD", "first value").unwrap();
        total
            .add(&Money::new("0.2", "USD", "second value").unwrap(), "total")
            .unwrap();

        assert_eq!(total.to_string(), "$0.30 USD");
        assert_eq!(
            serde_json::to_string(&total.json()).unwrap(),
            r#"{"value":0.3,"currency":"USD"}"#
        );
    }

    #[test]
    fn label_filters_are_trimmed_exact_and_case_insensitive() {
        let filter =
            LabelFilter::new(vec![" Droplets ".to_string(), "DROPLETS".to_string()]).unwrap();

        assert!(filter.matches("droplets"));
        assert!(!filter.matches("GPU Droplets"));
    }

    #[test]
    fn overview_sums_complete_single_currency_reports() {
        let period = BillingPeriod {
            start: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            usage_through: NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
        };
        let overview = BillingOverview::new(
            Ok(summary(Some(period), Some(money("1.10", "USD")))),
            Ok(summary(Some(period), Some(money("2.20", "USD")))),
            Ok(summary(Some(period), None)),
        )
        .unwrap();

        assert!(matches!(
            &overview.grand_total,
            GrandTotal::Available(total) if total.to_string() == "$3.30 USD"
        ));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: $3.30 USD"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_overview(&overview).unwrap()).unwrap();
        assert_eq!(json["schema"], "cmd.billing");
        assert_eq!(json["version"], 1);
        assert_eq!(json["complete"], true);
        assert_eq!(json["grand_total"]["value"], serde_json::json!(3.3));
        assert_eq!(json["providers"][0]["provider"], "aws");
        assert_eq!(json["providers"][1]["provider"], "cloudflare");
        assert_eq!(json["providers"][2]["provider"], "digitalocean");
    }

    #[test]
    fn overview_reports_all_errors_and_has_no_partial_grand_total() {
        let overview = BillingOverview::new(
            Ok(summary(None, Some(money("1.10", "USD")))),
            Err(eyre!("set the Cloudflare billing token")),
            Err(eyre!("DigitalOcean request failed")),
        )
        .unwrap();

        assert_eq!(overview.failure_count(), 2);
        assert!(matches!(overview.grand_total, GrandTotal::Incomplete));

        let rendered = render_overview(&overview).unwrap();
        assert!(rendered.contains("AWS\n  Period: unavailable\n  Total billed cost: $1.10 USD"));
        assert!(rendered.contains("Cloudflare\n  Error: set the Cloudflare billing token"));
        assert!(rendered.contains("DigitalOcean\n  Error: DigitalOcean request failed"));
        assert!(rendered.contains("Grand total: unavailable because the report is incomplete"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_overview(&overview).unwrap()).unwrap();
        assert_eq!(json["complete"], false);
        assert!(json["grand_total"].is_null());
        assert_eq!(json["providers"][1]["status"], "error");
        assert_eq!(json["providers"][2]["status"], "error");
    }

    #[test]
    fn overview_does_not_combine_mixed_currencies() {
        let overview = BillingOverview::new(
            Ok(summary(None, Some(money("1", "USD")))),
            Ok(summary(None, Some(money("2", "EUR")))),
            Ok(summary(None, None)),
        )
        .unwrap();

        assert!(overview.is_complete());
        assert!(matches!(overview.grand_total, GrandTotal::MixedCurrencies));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: unavailable because currencies differ"));
    }

    #[test]
    fn overview_reports_no_usage_when_all_totals_are_empty() {
        let overview = BillingOverview::new(
            Ok(summary(None, None)),
            Ok(summary(None, None)),
            Ok(summary(None, None)),
        )
        .unwrap();

        assert!(matches!(overview.grand_total, GrandTotal::NoUsage));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: no billable usage"));
    }

    fn summary(period: Option<BillingPeriod>, total_cost: Option<Money>) -> ProviderBillingSummary {
        ProviderBillingSummary::new(period, total_cost)
    }

    fn money(value: &str, currency: &str) -> Money {
        Money::new(value, currency, "test money").unwrap()
    }
}
