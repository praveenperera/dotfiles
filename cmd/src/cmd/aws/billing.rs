use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::Utc;
use clap::Args;
use eyre::{eyre, ContextCompat, Result, WrapErr};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::cmd::billing::{
    write_stdout, BillingOutput, BillingPeriod, JsonMoney, LabelFilter, Money,
};

const COST_METRIC: &str = "UnblendedCost";

/// Arguments for the AWS billing report
#[derive(Debug, Clone, Args)]
pub struct BillingArgs {
    /// AWS CLI profile; defaults to the standard AWS credential chain
    #[arg(long, env = "AWS_PROFILE")]
    pub profile: Option<String>,

    /// Include services by exact name; repeat or separate values with commas
    #[arg(long = "services", visible_alias = "service", value_delimiter = ',')]
    pub services: Vec<String>,

    /// Include services with a zero billed cost
    #[arg(long)]
    pub verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t)]
    pub output: BillingOutput,
}

#[derive(Debug)]
struct AwsBillingClient(PathBuf);

#[derive(Debug)]
struct BillingReport {
    period: BillingPeriod,
    estimated: bool,
    services: Vec<ServiceBilling>,
}

#[derive(Debug)]
struct ServiceBilling {
    name: String,
    cost: Money,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CostResponse {
    next_page_token: Option<String>,
    #[serde(default)]
    group_definitions: Vec<GroupDefinition>,
    results_by_time: Vec<CostResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GroupDefinition {
    r#type: String,
    key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CostResult {
    time_period: WirePeriod,
    #[serde(default)]
    groups: Vec<CostGroup>,
    estimated: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
struct WirePeriod {
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CostGroup {
    keys: Vec<String>,
    metrics: BTreeMap<String, CostMetric>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CostMetric {
    amount: String,
    unit: String,
}

#[derive(Debug, Serialize)]
struct JsonBillingReport {
    schema: &'static str,
    version: u32,
    period: BillingPeriod,
    estimated: bool,
    total_cost: Option<JsonMoney>,
    services: Vec<JsonServiceBilling>,
}

#[derive(Debug, Serialize)]
struct JsonServiceBilling {
    name: String,
    billed_cost: JsonMoney,
}

pub(super) async fn run(args: BillingArgs) -> Result<()> {
    let filters = LabelFilter::new(args.services)?;
    let period = BillingPeriod::current_month(Utc::now().date_naive())?;
    let pages = AwsBillingClient::new()
        .get_cost_and_usage(period, args.profile.as_deref())
        .await?;
    let report = BillingReport::new(pages, period, &filters, args.verbose)?;
    let rendered = match args.output {
        BillingOutput::Human => render_report(&report)?,
        BillingOutput::Json => render_json_report(&report)?,
    };

    write_stdout(&rendered)
}

impl AwsBillingClient {
    fn new() -> Self {
        Self(PathBuf::from("aws"))
    }

    async fn get_cost_and_usage(
        &self,
        period: BillingPeriod,
        profile: Option<&str>,
    ) -> Result<Vec<CostResponse>> {
        let time_period = format!("Start={},End={}", period.start, period.end_exclusive()?);
        let mut pages = Vec::new();
        let mut next_page_token = None;
        let mut seen_tokens = BTreeSet::new();

        loop {
            let mut command = Command::new(&self.0);
            command.args([
                "ce",
                "get-cost-and-usage",
                "--time-period",
                &time_period,
                "--granularity",
                "MONTHLY",
                "--metrics",
                COST_METRIC,
                "--group-by",
                "Type=DIMENSION,Key=SERVICE",
                "--region",
                "us-east-1",
                "--output",
                "json",
                "--no-cli-pager",
            ]);
            if let Some(profile) = profile {
                command.args(["--profile", profile]);
            }

            if let Some(token) = next_page_token.as_deref() {
                command.args(["--next-page-token", token]);
            }

            let output = command.output().await.wrap_err(
                "run AWS CLI; install and configure `aws`, or select a valid --profile",
            )?;
            if !output.status.success() {
                let message = String::from_utf8_lossy(&output.stderr);
                let message = message.trim();

                return Err(eyre!(
                    "AWS Cost Explorer request failed: {}",
                    if message.is_empty() {
                        output.status.to_string()
                    } else {
                        message.to_string()
                    }
                ));
            }

            let page: CostResponse = serde_json::from_slice(&output.stdout)
                .wrap_err("parse AWS Cost Explorer response")?;
            next_page_token = page
                .next_page_token
                .as_deref()
                .filter(|token| !token.is_empty())
                .map(str::to_string);
            if next_page_token
                .as_ref()
                .is_some_and(|token| !seen_tokens.insert(token.clone()))
            {
                return Err(eyre!("AWS Cost Explorer repeated a pagination token"));
            }

            pages.push(page);

            if next_page_token.is_none() {
                break;
            }
        }

        Ok(pages)
    }
}

impl BillingReport {
    fn new(
        pages: Vec<CostResponse>,
        period: BillingPeriod,
        filters: &LabelFilter,
        verbose: bool,
    ) -> Result<Self> {
        if pages.is_empty() {
            return Err(eyre!("AWS Cost Explorer returned no response pages"));
        }

        let expected_period = WirePeriod {
            start: period.start,
            end: period.end_exclusive()?,
        };
        let mut estimated = false;
        let mut services = BTreeMap::<String, Money>::new();

        for page in pages {
            validate_group_definition(&page.group_definitions)?;
            if page.results_by_time.len() != 1 {
                return Err(eyre!(
                    "AWS Cost Explorer returned {} monthly results; expected one",
                    page.results_by_time.len()
                ));
            }

            let result = page
                .results_by_time
                .into_iter()
                .next()
                .context("AWS Cost Explorer returned no monthly result")?;
            if result.time_period != expected_period {
                return Err(eyre!(
                    "AWS Cost Explorer returned a different period than requested"
                ));
            }

            estimated |= result.estimated;

            for mut group in result.groups {
                if group.keys.len() != 1 {
                    return Err(eyre!(
                        "AWS Cost Explorer returned a service group with {} keys; expected one",
                        group.keys.len()
                    ));
                }

                let name = group.keys.remove(0);
                let name = name.trim();
                if name.is_empty() {
                    return Err(eyre!("AWS Cost Explorer returned an empty service name"));
                }

                let name = name.to_string();
                let metric = group.metrics.remove(COST_METRIC).ok_or_else(|| {
                    eyre!("AWS Cost Explorer service {name} has no {COST_METRIC} metric")
                })?;
                let cost = Money::new(
                    &metric.amount,
                    &metric.unit,
                    &format!("AWS Cost Explorer service {name}"),
                )?;
                if let Some(existing) = services.get_mut(&name) {
                    existing.add(&cost, &format!("AWS service {name}"))?;
                } else {
                    services.insert(name, cost);
                }
            }
        }

        let services = services
            .into_iter()
            .filter(|(name, cost)| filters.matches(name) && (verbose || !cost.is_zero()))
            .map(|(name, cost)| ServiceBilling { name, cost })
            .collect();

        Ok(Self {
            period,
            estimated,
            services,
        })
    }

    fn total_cost(&self) -> Result<Option<Money>> {
        let Some(first) = self.services.first() else {
            return Ok(None);
        };
        let mut total = Money::zero(first.cost.currency());

        for service in &self.services {
            total.add(&service.cost, "AWS billing total")?;
        }

        Ok(Some(total))
    }
}

fn validate_group_definition(definitions: &[GroupDefinition]) -> Result<()> {
    let valid = definitions.len() == 1
        && definitions[0].r#type == "DIMENSION"
        && definitions[0].key == "SERVICE";
    if !valid {
        return Err(eyre!(
            "AWS Cost Explorer returned an unexpected group definition"
        ));
    }

    Ok(())
}

fn render_report(report: &BillingReport) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "AWS billing usage")?;
    writeln!(
        output,
        "Period: {} through {} (UTC)",
        report.period.start, report.period.usage_through
    )?;
    writeln!(
        output,
        "Estimated: {}",
        if report.estimated { "yes" } else { "no" }
    )?;

    let Some(total) = report.total_cost()? else {
        writeln!(output, "\nNo billable usage found")?;
        return Ok(output);
    };
    writeln!(output, "Total billed cost: {total}")?;

    for service in &report.services {
        writeln!(output, "\n{}", service.name)?;
        writeln!(output, "  Billed cost: {}", service.cost)?;
    }

    Ok(output)
}

fn render_json_report(report: &BillingReport) -> Result<String> {
    let report = JsonBillingReport {
        schema: "cmd.aws.billing",
        version: 1,
        period: report.period,
        estimated: report.estimated,
        total_cost: report.total_cost()?.map(|money| money.json()),
        services: report
            .services
            .iter()
            .map(|service| JsonServiceBilling {
                name: service.name.clone(),
                billed_cost: service.cost.json(),
            })
            .collect(),
    };
    let mut output = serde_json::to_string_pretty(&report)?;
    output.push('\n');

    Ok(output)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::{render_json_report, render_report, BillingReport, CostResponse};
    use crate::cmd::billing::{BillingPeriod, LabelFilter};

    #[test]
    fn groups_pages_filters_exact_names_and_hides_zero_costs() {
        let report = BillingReport::new(
            vec![
                response(
                    Some("next"),
                    true,
                    &[
                        ("Amazon Simple Storage Service", "0.10"),
                        ("AWS Key Management Service", "0"),
                    ],
                ),
                response(
                    None,
                    false,
                    &[
                        ("Amazon Simple Storage Service", "0.20"),
                        ("EC2 - Other", "-0.01"),
                    ],
                ),
            ],
            period(),
            &LabelFilter::new(vec![
                "amazon simple storage service".to_string(),
                "EC2 - Other".to_string(),
                "AWS Key Management Service".to_string(),
            ])
            .unwrap(),
            false,
        )
        .unwrap();

        assert!(report.estimated);
        assert_eq!(report.services.len(), 2);
        assert_eq!(report.services[0].name, "Amazon Simple Storage Service");
        assert_eq!(report.services[0].cost.to_string(), "$0.30 USD");
        assert_eq!(report.services[1].name, "EC2 - Other");
    }

    #[test]
    fn verbose_includes_zero_cost_services() {
        let report = BillingReport::new(
            vec![response(
                None,
                false,
                &[("AWS Key Management Service", "0")],
            )],
            period(),
            &LabelFilter::default(),
            true,
        )
        .unwrap();

        assert_eq!(report.services.len(), 1);
    }

    #[test]
    fn renders_stable_human_and_json_reports() {
        let report = BillingReport::new(
            vec![response(
                None,
                true,
                &[("Amazon Simple Storage Service", "12.340")],
            )],
            period(),
            &LabelFilter::default(),
            false,
        )
        .unwrap();

        assert_eq!(
            render_report(&report).unwrap(),
            concat!(
                "AWS billing usage\n",
                "Period: 2026-08-01 through 2026-08-10 (UTC)\n",
                "Estimated: yes\n",
                "Total billed cost: $12.34 USD\n",
                "\nAmazon Simple Storage Service\n",
                "  Billed cost: $12.34 USD\n",
            )
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&render_json_report(&report).unwrap())
                .unwrap(),
            json!({
                "schema": "cmd.aws.billing",
                "version": 1,
                "period": {"start": "2026-08-01", "usage_through": "2026-08-10"},
                "estimated": true,
                "total_cost": {"value": 12.34, "currency": "USD"},
                "services": [{
                    "name": "Amazon Simple Storage Service",
                    "billed_cost": {"value": 12.34, "currency": "USD"}
                }]
            })
        );
    }

    #[test]
    fn rejects_a_different_period() {
        let mut response = response(None, false, &[]);
        response.results_by_time[0].time_period.start =
            NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();

        assert!(
            BillingReport::new(vec![response], period(), &LabelFilter::default(), false,).is_err()
        );
    }

    #[test]
    fn rejects_mixed_currencies_when_it_totals_services() {
        let mut response = response(
            None,
            false,
            &[("Amazon Simple Storage Service", "1"), ("EC2 - Other", "2")],
        );
        response.results_by_time[0].groups[1]
            .metrics
            .get_mut("UnblendedCost")
            .unwrap()
            .unit = "EUR".to_string();
        let report =
            BillingReport::new(vec![response], period(), &LabelFilter::default(), false).unwrap();

        assert!(report.total_cost().is_err());
    }

    fn period() -> BillingPeriod {
        BillingPeriod {
            start: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            usage_through: NaiveDate::from_ymd_opt(2026, 8, 10).unwrap(),
        }
    }

    fn response(
        next_page_token: Option<&str>,
        estimated: bool,
        services: &[(&str, &str)],
    ) -> CostResponse {
        serde_json::from_value(json!({
            "NextPageToken": next_page_token,
            "GroupDefinitions": [{"Type": "DIMENSION", "Key": "SERVICE"}],
            "ResultsByTime": [{
                "TimePeriod": {"Start": "2026-08-01", "End": "2026-08-11"},
                "Groups": services.iter().map(|(name, amount)| json!({
                    "Keys": [name],
                    "Metrics": {"UnblendedCost": {"Amount": amount, "Unit": "USD"}}
                })).collect::<Vec<_>>(),
                "Estimated": estimated
            }]
        }))
        .unwrap()
    }
}
