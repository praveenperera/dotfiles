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

    /// Show the Modal billing report
    Modal(#[command(flatten)] crate::cmd::modal::BillingArgs),
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

    /// Amount without its currency, for ordering within one currency
    pub(crate) fn amount(&self) -> Decimal {
        self.value
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
    Modal,
}

impl BillingProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Cloudflare => "Cloudflare",
            Self::DigitalOcean => "DigitalOcean",
            Self::Modal => "Modal",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProviderBillingSummary {
    period: Option<BillingPeriod>,
    /// Metered cost, before any credits the provider applies
    total_cost: Option<Money>,
    /// Cost after the provider applies its own credits and discounts
    net_cost: Option<Money>,
}

impl ProviderBillingSummary {
    pub(crate) fn new(period: Option<BillingPeriod>, total_cost: Option<Money>) -> Self {
        Self {
            period,
            total_cost,
            net_cost: None,
        }
    }

    /// Records what the provider actually bills once its credits apply
    ///
    /// Modal grants a monthly credit allowance, so its metered cost can be far
    /// above the invoiced amount. The overview shows both, and totals the
    /// invoiced amount, so the grand total stays the money actually owed.
    pub(crate) fn with_net_cost(mut self, net_cost: Money) -> Self {
        self.net_cost = Some(net_cost);
        self
    }

    /// What the provider charges, which is what the grand total sums
    fn billable_cost(&self) -> Option<&Money> {
        self.net_cost.as_ref().or(self.total_cost.as_ref())
    }

    /// The net cost, only when it tells the reader something the total does not
    fn distinct_net_cost(&self) -> Option<&Money> {
        self.net_cost
            .as_ref()
            .filter(|net| Some(*net) != self.total_cost.as_ref())
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

/// Provider reports in the fixed order the overview displays them
#[derive(Debug)]
struct BillingOverview {
    providers: Vec<(BillingProvider, ProviderOutcome)>,
    grand_total: GrandTotal,
}

impl BillingOverview {
    fn new(results: Vec<(BillingProvider, Result<ProviderBillingSummary>)>) -> Result<Self> {
        let providers = results
            .into_iter()
            .map(|(provider, result)| (provider, ProviderOutcome::from_result(result)))
            .collect::<Vec<_>>();
        let grand_total = calculate_grand_total(&providers)?;

        Ok(Self {
            providers,
            grand_total,
        })
    }

    fn providers(&self) -> impl Iterator<Item = (BillingProvider, &ProviderOutcome)> {
        self.providers
            .iter()
            .map(|(provider, outcome)| (*provider, outcome))
    }

    fn failure_count(&self) -> usize {
        self.providers()
            .filter(|(_, outcome)| matches!(outcome, ProviderOutcome::Error(_)))
            .count()
    }

    fn is_complete(&self) -> bool {
        self.failure_count() == 0
    }

    /// Whether credits make the grand total smaller than the listed costs
    fn has_credits(&self) -> bool {
        self.providers().any(|(_, outcome)| match outcome {
            ProviderOutcome::Ok(summary) => summary.distinct_net_cost().is_some(),
            ProviderOutcome::Error(_) => false,
        })
    }
}

fn calculate_grand_total(providers: &[(BillingProvider, ProviderOutcome)]) -> Result<GrandTotal> {
    if providers
        .iter()
        .any(|(_, outcome)| matches!(outcome, ProviderOutcome::Error(_)))
    {
        return Ok(GrandTotal::Incomplete);
    }

    let mut totals = providers.iter().filter_map(|(_, outcome)| match outcome {
        ProviderOutcome::Ok(summary) => summary.billable_cost(),
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
        /// Present only when the provider bills something other than its metered cost
        #[serde(skip_serializing_if = "Option::is_none")]
        net_cost: Option<JsonMoney>,
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
            .map(|(provider, outcome)| match outcome {
                ProviderOutcome::Ok(summary) => JsonProviderOutcome::Ok {
                    provider,
                    period: summary.period,
                    total_cost: summary.total_cost.as_ref().map(Money::json),
                    net_cost: summary.distinct_net_cost().map(Money::json),
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
        Some(BillingCmd::Modal(args)) => crate::cmd::modal::run_billing(args).await,
        None => run_overview(flags.output).await,
    }
}

async fn run_overview(output: BillingOutput) -> Result<()> {
    let (aws, cloudflare, digitalocean, modal) = tokio::join!(
        crate::cmd::aws::billing_summary(),
        crate::cmd::cloudflare::billing_summary(),
        crate::cmd::digitalocean::billing_summary(),
        crate::cmd::modal::billing_summary(),
    );
    let overview = BillingOverview::new(vec![
        (BillingProvider::Aws, aws),
        (BillingProvider::Cloudflare, cloudflare),
        (BillingProvider::DigitalOcean, digitalocean),
        (BillingProvider::Modal, modal),
    ])?;
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

                // not "billed cost": Modal reports usage before its credits apply
                if let Some(total) = &summary.total_cost {
                    writeln!(output, "  Total cost: {total}")?;
                } else {
                    writeln!(output, "  Total cost: no billable usage")?;
                }

                if let Some(net) = summary.distinct_net_cost() {
                    writeln!(output, "  Billed after credits: {net}")?;
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
        // name the credits, or the total reads as an arithmetic error against the rows
        GrandTotal::Available(total) if overview.has_credits() => {
            writeln!(output, "\nGrand total: {total} (after credits)")?
        }
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
        render_json_overview, render_overview, BillingOverview, BillingPeriod, BillingProvider,
        GrandTotal, LabelFilter, Money, ProviderBillingSummary,
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
        let overview = overview([
            Ok(summary(Some(period), Some(money("1.10", "USD")))),
            Ok(summary(Some(period), Some(money("2.20", "USD")))),
            Ok(summary(Some(period), None)),
            Ok(summary(Some(period), Some(money("0.40", "USD")))),
        ]);

        assert!(matches!(
            &overview.grand_total,
            GrandTotal::Available(total) if total.to_string() == "$3.70 USD"
        ));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: $3.70 USD"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_overview(&overview).unwrap()).unwrap();
        assert_eq!(json["schema"], "cmd.billing");
        assert_eq!(json["version"], 1);
        assert_eq!(json["complete"], true);
        assert_eq!(json["grand_total"]["value"], serde_json::json!(3.7));
        assert_eq!(json["providers"][0]["provider"], "aws");
        assert_eq!(json["providers"][1]["provider"], "cloudflare");
        assert_eq!(json["providers"][2]["provider"], "digitalocean");
        assert_eq!(json["providers"][3]["provider"], "modal");
    }

    #[test]
    fn overview_reports_all_errors_and_has_no_partial_grand_total() {
        let overview = overview([
            Ok(summary(None, Some(money("1.10", "USD")))),
            Err(eyre!("set the Cloudflare billing token")),
            Err(eyre!("DigitalOcean request failed")),
            Err(eyre!("install the Modal CLI")),
        ]);

        assert_eq!(overview.failure_count(), 3);
        assert!(matches!(overview.grand_total, GrandTotal::Incomplete));

        let rendered = render_overview(&overview).unwrap();
        assert!(rendered.contains("AWS\n  Period: unavailable\n  Total cost: $1.10 USD"));
        assert!(rendered.contains("Cloudflare\n  Error: set the Cloudflare billing token"));
        assert!(rendered.contains("DigitalOcean\n  Error: DigitalOcean request failed"));
        assert!(rendered.contains("Modal\n  Error: install the Modal CLI"));
        assert!(rendered.contains("Grand total: unavailable because the report is incomplete"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_overview(&overview).unwrap()).unwrap();
        assert_eq!(json["complete"], false);
        assert!(json["grand_total"].is_null());
        assert_eq!(json["providers"][1]["status"], "error");
        assert_eq!(json["providers"][2]["status"], "error");
        assert_eq!(json["providers"][3]["status"], "error");
    }

    #[test]
    fn overview_does_not_combine_mixed_currencies() {
        let overview = overview([
            Ok(summary(None, Some(money("1", "USD")))),
            Ok(summary(None, Some(money("2", "EUR")))),
            Ok(summary(None, None)),
            Ok(summary(None, None)),
        ]);

        assert!(overview.is_complete());
        assert!(matches!(overview.grand_total, GrandTotal::MixedCurrencies));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: unavailable because currencies differ"));
    }

    #[test]
    fn overview_reports_no_usage_when_all_totals_are_empty() {
        let overview = overview([
            Ok(summary(None, None)),
            Ok(summary(None, None)),
            Ok(summary(None, None)),
            Ok(summary(None, None)),
        ]);

        assert!(matches!(overview.grand_total, GrandTotal::NoUsage));
        assert!(render_overview(&overview)
            .unwrap()
            .contains("Grand total: no billable usage"));
    }

    #[test]
    fn overview_shows_a_net_cost_only_when_credits_change_it() {
        let credited = summary(None, Some(money("10.80", "USD"))).with_net_cost(money("0", "USD"));
        let uncredited =
            summary(None, Some(money("1.10", "USD"))).with_net_cost(money("1.10", "USD"));
        let overview = overview([
            Ok(uncredited),
            Ok(summary(None, None)),
            Ok(summary(None, None)),
            Ok(credited),
        ]);

        let rendered = render_overview(&overview).unwrap();
        assert!(rendered.contains("  Total cost: $10.80 USD\n  Billed after credits: $0.00 USD"));
        // the AWS row carries an equal net cost, which says nothing worth a line
        assert!(rendered.contains("AWS\n  Period: unavailable\n  Total cost: $1.10 USD\n\n"));
        assert_eq!(rendered.matches("Billed after credits").count(), 1);

        // credits cover the $10.80, so only the uncredited $1.10 is owed
        assert!(matches!(
            &overview.grand_total,
            GrandTotal::Available(total) if total.to_string() == "$1.10 USD"
        ));
        assert!(rendered.contains("Grand total: $1.10 USD (after credits)"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_overview(&overview).unwrap()).unwrap();
        assert_eq!(json["providers"][3]["net_cost"]["value"], 0);
        assert!(json["providers"][0].get("net_cost").is_none());
    }

    fn overview(results: [Result<ProviderBillingSummary, eyre::Report>; 4]) -> BillingOverview {
        let providers = [
            BillingProvider::Aws,
            BillingProvider::Cloudflare,
            BillingProvider::DigitalOcean,
            BillingProvider::Modal,
        ];

        BillingOverview::new(providers.into_iter().zip(results).collect()).unwrap()
    }

    fn summary(period: Option<BillingPeriod>, total_cost: Option<Money>) -> ProviderBillingSummary {
        ProviderBillingSummary::new(period, total_cost)
    }

    fn money(value: &str, currency: &str) -> Money {
        Money::new(value, currency, "test money").unwrap()
    }
}
