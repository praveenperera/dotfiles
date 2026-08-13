use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::Utc;
use clap::Args;
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::process::Command;

use crate::cmd::billing::{
    write_stdout, BillingOutput, BillingPeriod, JsonMoney, LabelFilter, Money,
    ProviderBillingSummary,
};

/// Modal bills in US dollars and its API reports no currency code
const CURRENCY: &str = "USD";
const PROFILE_ENV_VAR: &str = "MODAL_PROFILE";
const CREDITS_ADJUSTMENT: &str = "credits";

/// Monthly credit grant, assumed because Modal never reports it
///
/// The billing API returns only the credits already drawn against the grant,
/// never its size or the balance left. Without this constant there is no way to
/// show how much of the month's allowance remains. Change it if the plan does.
const MONTHLY_CREDIT_ALLOWANCE: &str = "30";

/// Arguments for the Modal billing report
#[derive(Debug, Clone, Args)]
pub struct BillingArgs {
    /// Modal profile that selects the workspace
    #[arg(long, env = PROFILE_ENV_VAR)]
    pub profile: Option<String>,

    /// Include cost buckets by exact name; repeat or separate values with commas
    #[arg(long = "buckets", visible_alias = "bucket", value_delimiter = ',')]
    pub buckets: Vec<String>,

    /// Show the per-app cost breakdown
    #[arg(long)]
    pub verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t)]
    pub output: BillingOutput,
}

#[derive(Debug)]
struct ModalBillingClient(PathBuf);

#[derive(Debug)]
struct BillingReport {
    period: BillingPeriod,
    /// Raw usage before credits and reservations; the figure the overview totals
    metered_cost: Money,
    /// Usage after credits and reservations
    billed_cost: Money,
    credits: CreditAllowance,
    buckets: Vec<CostBucket>,
    adjustments: Vec<CostBucket>,
    /// Populated only in verbose mode, where Modal is asked for the detailed report
    apps: Vec<AppBilling>,
}

#[derive(Debug)]
struct CostBucket {
    name: String,
    cost: Money,
}

/// The month's credit grant, how much of it usage has drawn, and what is left
#[derive(Debug)]
struct CreditAllowance {
    allowance: Money,
    used: Money,
    remaining: Money,
}

#[derive(Debug)]
struct AppBilling {
    object_id: String,
    description: String,
    environment: String,
    cost: Money,
}

#[derive(Debug, Deserialize)]
struct WireSummary {
    metered_cost: String,
    billed_cost: String,
    /// Open map: Modal adds adjustment kinds without warning
    #[serde(default)]
    adjustments: BTreeMap<String, String>,
    #[serde(default)]
    metered_cost_breakdown: BTreeMap<String, String>,
}

/// One row per billed object per interval; the report collapses the intervals,
/// so `interval_start` is deliberately not deserialized
#[derive(Debug, Deserialize)]
struct WireReportRow {
    object_id: String,
    description: Option<String>,
    environment: Option<String>,
    cost: String,
}

#[derive(Debug, Serialize)]
struct JsonBillingReport {
    schema: &'static str,
    version: u32,
    period: BillingPeriod,
    metered_cost: JsonMoney,
    billed_cost: JsonMoney,
    credits: JsonCreditAllowance,
    buckets: Vec<JsonCostBucket>,
    adjustments: Vec<JsonCostBucket>,
    apps: Vec<JsonAppBilling>,
}

#[derive(Debug, Serialize)]
struct JsonCostBucket {
    name: String,
    cost: JsonMoney,
}

#[derive(Debug, Serialize)]
struct JsonCreditAllowance {
    /// Assumed, not reported by Modal
    allowance: JsonMoney,
    assumed: bool,
    used: JsonMoney,
    remaining: JsonMoney,
}

#[derive(Debug, Serialize)]
struct JsonAppBilling {
    object_id: String,
    description: String,
    environment: String,
    cost: JsonMoney,
}

pub(super) async fn run(args: BillingArgs) -> Result<()> {
    let output = args.output;
    let report = load_report(args).await?;
    let rendered = match output {
        BillingOutput::Human => render_report(&report)?,
        BillingOutput::Json => render_json_report(&report)?,
    };

    write_stdout(&rendered)
}

pub(super) async fn summary() -> Result<ProviderBillingSummary> {
    let report = load_report(BillingArgs {
        profile: env::var(PROFILE_ENV_VAR).ok(),
        buckets: Vec::new(),
        verbose: false,
        output: BillingOutput::Human,
    })
    .await?;
    let period = Some(report.period);
    if report.metered_cost.is_zero() {
        return Ok(ProviderBillingSummary::new(period, None));
    }

    Ok(
        ProviderBillingSummary::new(period, Some(report.metered_cost))
            .with_net_cost(report.billed_cost),
    )
}

async fn load_report(args: BillingArgs) -> Result<BillingReport> {
    let filters = LabelFilter::new(args.buckets)?;
    let period = BillingPeriod::current_month(Utc::now().date_naive())?;
    let client = ModalBillingClient::new();
    let profile = args.profile.as_deref();
    let summary = client.get_summary(period, profile).await?;
    let rows = match args.verbose {
        true => client.get_report(period, profile).await?,
        // the overview only needs the totals, so it stays at one subprocess
        false => Vec::new(),
    };

    BillingReport::new(summary, rows, period, &filters, args.verbose)
}

impl ModalBillingClient {
    fn new() -> Self {
        Self(PathBuf::from("modal"))
    }

    async fn get_summary(
        &self,
        period: BillingPeriod,
        profile: Option<&str>,
    ) -> Result<WireSummary> {
        let month = period.start.format("%Y-%m").to_string();

        self.get_json(
            &["billing", "summary", "--for", &month, "--json"],
            profile,
            "get the billing summary",
        )
        .await
    }

    async fn get_report(
        &self,
        period: BillingPeriod,
        profile: Option<&str>,
    ) -> Result<Vec<WireReportRow>> {
        let start = period.start.to_string();
        let end = period.end_exclusive()?.to_string();

        self.get_json(
            &[
                "billing", "report", "--start", &start, "--end", &end, "--json",
            ],
            profile,
            "get the billing report",
        )
        .await
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        args: &[&str],
        profile: Option<&str>,
        context: &str,
    ) -> Result<T> {
        let mut command = Command::new(&self.0);
        command.args(args);
        if let Some(profile) = profile {
            command.env(PROFILE_ENV_VAR, profile);
        }

        let output = command.output().await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                return eyre!("install the Modal CLI, or put `modal` on PATH, to {context}");
            }

            eyre!("run the Modal CLI to {context}: {error}")
        })?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            let message = message.trim();
            let message = match message.is_empty() {
                true => "the Modal CLI reported no error message",
                false => message,
            };

            return Err(eyre!("Modal failed to {context}: {message}"));
        }

        serde_json::from_slice(&output.stdout)
            .wrap_err_with(|| format!("parse the Modal response to {context}"))
    }
}

impl BillingReport {
    fn new(
        summary: WireSummary,
        rows: Vec<WireReportRow>,
        period: BillingPeriod,
        filters: &LabelFilter,
        verbose: bool,
    ) -> Result<Self> {
        let metered_cost = Money::new(&summary.metered_cost, CURRENCY, "Modal metered cost")?;
        let billed_cost = Money::new(&summary.billed_cost, CURRENCY, "Modal billed cost")?;
        // read credits before the adjustment list is filtered, which drops zeroes
        let credits = CreditAllowance::new(summary.adjustments.get(CREDITS_ADJUSTMENT))?;
        let buckets = cost_buckets(
            summary.metered_cost_breakdown,
            "Modal cost bucket",
            filters,
            verbose,
        )?;
        let adjustments = cost_buckets(
            summary.adjustments,
            "Modal adjustment",
            &LabelFilter::default(),
            verbose,
        )?;

        Ok(Self {
            period,
            metered_cost,
            billed_cost,
            credits,
            buckets,
            adjustments,
            apps: app_billing(rows)?,
        })
    }

    fn has_usage(&self) -> bool {
        !self.metered_cost.is_zero() || !self.billed_cost.is_zero()
    }
}

impl CreditAllowance {
    fn new(applied: Option<&String>) -> Result<Self> {
        let used = match applied {
            // Modal signs applied credits negative; report them as a positive draw
            Some(amount) => negate(&Money::new(amount, CURRENCY, "Modal credits")?),
            None => Money::zero(CURRENCY),
        };
        let allowance = Money::new(MONTHLY_CREDIT_ALLOWANCE, CURRENCY, "Modal credit allowance")?;
        // extra one-off credits can push the draw past the monthly grant
        let remaining = (allowance.amount() - used.amount()).max(Decimal::ZERO);

        Ok(Self {
            allowance,
            used,
            remaining: Money::from_decimal(remaining, CURRENCY, "Modal credits remaining")?,
        })
    }
}

fn negate(money: &Money) -> Money {
    Money::from_decimal(-money.amount(), CURRENCY, "Modal credits")
        .expect("negating a Money keeps its valid currency")
}

/// Turns one of Modal's open cost maps into a stable, filtered list
///
/// Zero entries are noise in the default report, but negative entries are not:
/// a credit is exactly what the reader wants to see.
fn cost_buckets(
    wire: BTreeMap<String, String>,
    context: &str,
    filters: &LabelFilter,
    verbose: bool,
) -> Result<Vec<CostBucket>> {
    let mut buckets = Vec::new();

    for (name, amount) in wire {
        if !filters.matches(&name) {
            continue;
        }

        let cost = Money::new(&amount, CURRENCY, context)?;
        if !verbose && cost.is_zero() {
            continue;
        }

        buckets.push(CostBucket { name, cost });
    }

    Ok(buckets)
}

/// Collapses the daily report rows into one entry per billed object
fn app_billing(rows: Vec<WireReportRow>) -> Result<Vec<AppBilling>> {
    let mut apps: BTreeMap<(String, String, String), Money> = BTreeMap::new();

    for row in rows {
        let cost = Money::new(&row.cost, CURRENCY, "Modal report row")?;
        let key = (
            row.object_id,
            row.description.unwrap_or_else(|| "Unlabeled".to_string()),
            row.environment.unwrap_or_else(|| "unknown".to_string()),
        );
        apps.entry(key)
            .or_insert_with(|| Money::zero(CURRENCY))
            .add(&cost, "Modal app total")?;
    }

    let mut apps = apps
        .into_iter()
        .map(|((object_id, description, environment), cost)| AppBilling {
            object_id,
            description,
            environment,
            cost,
        })
        .collect::<Vec<_>>();
    // biggest spenders first, then by id so equal costs keep a stable order
    apps.sort_by(|left, right| {
        right
            .cost
            .amount()
            .cmp(&left.cost.amount())
            .then(left.object_id.cmp(&right.object_id))
    });

    Ok(apps)
}

fn render_report(report: &BillingReport) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "Modal billing usage")?;
    writeln!(
        output,
        "Period: {} through {} (UTC)",
        report.period.start, report.period.usage_through
    )?;

    if !report.has_usage() {
        writeln!(output, "\nNo billable usage found")?;
        return Ok(output);
    }

    writeln!(output, "Metered cost: {}", report.metered_cost)?;
    writeln!(output, "Billed cost: {}", report.billed_cost)?;

    writeln!(
        output,
        "\nCredits (assumes a {} monthly grant)",
        report.credits.allowance
    )?;
    writeln!(output, "  Used: {}", report.credits.used)?;
    writeln!(output, "  Remaining: {}", report.credits.remaining)?;

    render_buckets(&mut output, "Cost buckets", &report.buckets)?;
    render_buckets(&mut output, "Adjustments", &report.adjustments)?;

    if !report.apps.is_empty() {
        writeln!(output, "\nApps")?;
        for app in &report.apps {
            writeln!(output, "\n  {}", app.description)?;
            writeln!(output, "    Object ID: {}", app.object_id)?;
            writeln!(output, "    Environment: {}", app.environment)?;
            writeln!(output, "    Metered cost: {}", app.cost)?;
        }
    }

    Ok(output)
}

fn render_buckets(output: &mut String, title: &str, buckets: &[CostBucket]) -> Result<()> {
    if buckets.is_empty() {
        return Ok(());
    }

    writeln!(output, "\n{title}")?;
    for bucket in buckets {
        writeln!(output, "  {}: {}", bucket.name, bucket.cost)?;
    }

    Ok(())
}

fn render_json_report(report: &BillingReport) -> Result<String> {
    let report = JsonBillingReport {
        schema: "cmd.modal.billing",
        version: 1,
        period: report.period,
        metered_cost: report.metered_cost.json(),
        billed_cost: report.billed_cost.json(),
        credits: JsonCreditAllowance::from(&report.credits),
        buckets: report.buckets.iter().map(JsonCostBucket::from).collect(),
        adjustments: report
            .adjustments
            .iter()
            .map(JsonCostBucket::from)
            .collect(),
        apps: report.apps.iter().map(JsonAppBilling::from).collect(),
    };
    let mut output = serde_json::to_string_pretty(&report)?;
    output.push('\n');

    Ok(output)
}

impl From<&CostBucket> for JsonCostBucket {
    fn from(bucket: &CostBucket) -> Self {
        Self {
            name: bucket.name.clone(),
            cost: bucket.cost.json(),
        }
    }
}

impl From<&CreditAllowance> for JsonCreditAllowance {
    fn from(credits: &CreditAllowance) -> Self {
        Self {
            allowance: credits.allowance.json(),
            assumed: true,
            used: credits.used.json(),
            remaining: credits.remaining.json(),
        }
    }
}

impl From<&AppBilling> for JsonAppBilling {
    fn from(app: &AppBilling) -> Self {
        Self {
            object_id: app.object_id.clone(),
            description: app.description.clone(),
            environment: app.environment.clone(),
            cost: app.cost.json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::{render_json_report, render_report, BillingReport, WireReportRow, WireSummary};
    use crate::cmd::billing::{BillingPeriod, LabelFilter};

    #[test]
    fn parses_scientific_notation_amounts() {
        let report = report(
            summary(json!({
                "metered_cost": "10.80178417",
                "billed_cost": "0E-8",
                "adjustments": {"reservation_adjustment": "-0E-8"},
                "metered_cost_breakdown": {"llm_tokens": "0E-8"}
            })),
            Vec::new(),
            &LabelFilter::default(),
            true,
        );

        assert_eq!(report.metered_cost.to_string(), "$10.80 USD");
        assert_eq!(report.billed_cost.to_string(), "$0.00 USD");
        assert!(report.adjustments[0].cost.is_zero());
        assert!(report.buckets[0].cost.is_zero());
    }

    #[test]
    fn sums_daily_report_rows_per_app() {
        let rows = vec![
            row("ap-one", "execdeck", "0.21596278"),
            row("ap-one", "execdeck", "0.12297928"),
            row("ap-two", "runner", "0.00014806"),
        ];

        let report = report(default_summary(), rows, &LabelFilter::default(), true);

        assert_eq!(report.apps.len(), 2);
        assert_eq!(report.apps[0].object_id, "ap-one");
        assert_eq!(report.apps[0].cost.to_string(), "$0.33 USD");
        assert_eq!(report.apps[1].object_id, "ap-two");
    }

    #[test]
    fn hides_zero_buckets_by_default_and_keeps_credits() {
        let report = report(
            default_summary(),
            Vec::new(),
            &LabelFilter::default(),
            false,
        );

        let bucket_names: Vec<_> = report.buckets.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(bucket_names, ["deployed_apps", "volumes"]);

        let adjustments: Vec<_> = report.adjustments.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(adjustments, ["credits"]);
        assert_eq!(report.adjustments[0].cost.to_string(), "$-10.79 USD");
    }

    #[test]
    fn filters_buckets_by_exact_name_without_changing_the_totals() {
        let filters = LabelFilter::new(vec!["Volumes".to_string()]).unwrap();

        let report = report(default_summary(), Vec::new(), &filters, false);

        assert_eq!(report.buckets.len(), 1);
        assert_eq!(report.buckets[0].name, "volumes");
        assert_eq!(report.metered_cost.to_string(), "$6.87 USD");
    }

    #[test]
    fn renders_stable_human_and_json_reports() {
        let rows = vec![row("ap-one", "execdeck", "0.33894206")];
        let report = report(default_summary(), rows, &LabelFilter::default(), false);

        let rendered = render_report(&report).unwrap();
        assert!(rendered.contains("Period: 2026-08-01 through 2026-08-13 (UTC)"));
        assert!(rendered.contains("Metered cost: $6.87 USD"));
        assert!(rendered.contains("Billed cost: $0.00 USD"));
        assert!(rendered.contains("Cost buckets\n  deployed_apps: $6.86 USD\n  volumes: $0.01 USD"));
        assert!(rendered.contains("Adjustments\n  credits: $-10.79 USD"));
        assert!(rendered.contains(
            "Credits (assumes a $30.00 USD monthly grant)\n  Used: $10.79 USD\n  Remaining: $19.21 USD"
        ));
        assert!(rendered.contains("Apps\n\n  execdeck"));

        let json: serde_json::Value =
            serde_json::from_str(&render_json_report(&report).unwrap()).unwrap();
        assert_eq!(json["schema"], "cmd.modal.billing");
        assert_eq!(json["version"], 1);
        assert_eq!(json["metered_cost"]["currency"], "USD");
        assert_eq!(json["buckets"][0]["name"], "deployed_apps");
        assert_eq!(json["apps"][0]["object_id"], "ap-one");
        assert_eq!(json["credits"]["assumed"], true);
        assert_eq!(
            json["credits"]["remaining"]["value"],
            serde_json::json!(19.21)
        );
    }

    #[test]
    fn derives_remaining_credit_from_the_assumed_monthly_grant() {
        let report = report(
            default_summary(),
            Vec::new(),
            &LabelFilter::default(),
            false,
        );

        assert_eq!(report.credits.allowance.to_string(), "$30.00 USD");
        assert_eq!(report.credits.used.to_string(), "$10.79 USD");
        assert_eq!(report.credits.remaining.to_string(), "$19.21 USD");
    }

    #[test]
    fn credit_draw_beyond_the_grant_leaves_nothing_remaining() {
        let summary = summary(json!({
            "metered_cost": "31.67",
            "billed_cost": "1.67",
            "adjustments": {"credits": "-40.00000000"}
        }));

        let report = report(summary, Vec::new(), &LabelFilter::default(), false);

        assert_eq!(report.credits.used.to_string(), "$40.00 USD");
        assert!(report.credits.remaining.is_zero());
    }

    #[test]
    fn a_month_without_credits_keeps_the_whole_grant() {
        let summary = summary(json!({"metered_cost": "1.00", "billed_cost": "1.00"}));

        let report = report(summary, Vec::new(), &LabelFilter::default(), false);

        assert!(report.credits.used.is_zero());
        assert_eq!(report.credits.remaining.to_string(), "$30.00 USD");
    }

    #[test]
    fn reports_no_billable_usage_when_every_total_is_zero() {
        let report = report(
            summary(json!({"metered_cost": "0E-8", "billed_cost": "0E-8"})),
            Vec::new(),
            &LabelFilter::default(),
            false,
        );

        assert!(render_report(&report)
            .unwrap()
            .contains("No billable usage found"));
    }

    fn report(
        summary: WireSummary,
        rows: Vec<WireReportRow>,
        filters: &LabelFilter,
        verbose: bool,
    ) -> BillingReport {
        let period = BillingPeriod {
            start: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            usage_through: NaiveDate::from_ymd_opt(2026, 8, 13).unwrap(),
        };

        BillingReport::new(summary, rows, period, filters, verbose).unwrap()
    }

    fn summary(value: serde_json::Value) -> WireSummary {
        serde_json::from_value(value).unwrap()
    }

    fn default_summary() -> WireSummary {
        summary(json!({
            "metered_cost": "6.87928490",
            "billed_cost": "0E-8",
            "adjustments": {"credits": "-10.79000000", "plan_cost": "0E-8"},
            "metered_cost_breakdown": {
                "deployed_apps": "6.86750073",
                "llm_tokens": "0E-8",
                "volumes": "0.01178417"
            }
        }))
    }

    fn row(object_id: &str, description: &str, cost: &str) -> WireReportRow {
        serde_json::from_value(json!({
            "object_id": object_id,
            "description": description,
            "environment": "main",
            "interval_start": "2026-08-05T00:00:00",
            "cost": cost
        }))
        .unwrap()
    }
}
