use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use clap::Args;
use eyre::{eyre, ContextCompat, Result, WrapErr};
use reqwest::{StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::cmd::billing::{
    write_stdout, BillingOutput, BillingPeriod, JsonMoney, LabelFilter, Money,
};

const API_BASE_URL: &str = "https://api.digitalocean.com";
const BILLING_TOKEN_ENV_VAR: &str = "DIGITALOCEAN_BILLING_TOKEN";
const LEGACY_TOKEN_ENV_VAR: &str = "DIGITALOCEAN_TOKEN";
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const PAGE_SIZE: usize = 200;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Arguments for the DigitalOcean billing report
#[derive(Debug, Clone, Args)]
pub struct BillingArgs {
    /// DigitalOcean API token with the `billing:read` scope
    #[arg(
        long = "api-token",
        visible_alias = "token",
        env = BILLING_TOKEN_ENV_VAR,
        hide_env_values = true
    )]
    pub api_token: Option<String>,

    /// Include products by exact name; repeat or separate values with commas
    #[arg(long = "products", visible_alias = "product", value_delimiter = ',')]
    pub products: Vec<String>,

    /// Include invoice items with a zero billed cost
    #[arg(long)]
    pub verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t)]
    pub output: BillingOutput,

    /// Override the DigitalOcean API base URL
    #[arg(long, hide = true, default_value = API_BASE_URL)]
    pub api_base_url: String,
}

#[derive(Clone)]
struct BillingClient {
    http: reqwest::Client,
    base_url: Url,
    token: ApiToken,
}

#[derive(Clone)]
struct ApiToken(String);

#[derive(Debug)]
struct BillingReport {
    period: BillingPeriod,
    generated_at: DateTime<Utc>,
    products: Vec<ProductBilling>,
}

#[derive(Debug)]
struct ProductBilling {
    name: String,
    items: Vec<InvoiceItem>,
}

#[derive(Debug)]
struct InvoiceItem {
    description: String,
    group_description: Option<String>,
    project_name: Option<String>,
    resource_id: Option<String>,
    resource_uuid: Option<String>,
    duration: Option<UsageDuration>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    cost: Money,
}

#[derive(Debug)]
struct ProductInvoiceItem {
    product: String,
    item: InvoiceItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct UsageDuration {
    value: String,
    unit: String,
}

#[derive(Debug, Deserialize)]
struct InvoiceListResponse {
    invoice_preview: Option<InvoicePreview>,
}

#[derive(Debug, Deserialize)]
struct InvoicePreview {
    invoice_period: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct InvoiceItemsResponse {
    #[serde(default)]
    invoice_items: Vec<WireInvoiceItem>,
    links: PageLinks,
    meta: PageMeta,
}

#[derive(Debug, Deserialize)]
struct PageLinks {
    pages: Option<ForwardLinks>,
}

#[derive(Debug, Deserialize)]
struct ForwardLinks {
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PageMeta {
    total: usize,
}

#[derive(Debug, Deserialize)]
struct WireInvoiceItem {
    amount: Option<String>,
    description: Option<String>,
    duration: Option<String>,
    duration_unit: Option<String>,
    end_time: Option<DateTime<Utc>>,
    group_description: Option<String>,
    product: Option<String>,
    project_name: Option<String>,
    resource_id: Option<String>,
    resource_uuid: Option<String>,
    start_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonBillingReport {
    schema: &'static str,
    version: u32,
    period: BillingPeriod,
    generated_at: DateTime<Utc>,
    total_cost: Option<JsonMoney>,
    products: Vec<JsonProductBilling>,
}

#[derive(Debug, Serialize)]
struct JsonProductBilling {
    name: String,
    total_cost: JsonMoney,
    items: Vec<JsonInvoiceItem>,
}

#[derive(Debug, Serialize)]
struct JsonInvoiceItem {
    description: String,
    group_description: Option<String>,
    project_name: Option<String>,
    resource_id: Option<String>,
    resource_uuid: Option<String>,
    duration: Option<UsageDuration>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    billed_cost: JsonMoney,
}

pub(super) async fn run(args: BillingArgs) -> Result<()> {
    let token = resolve_token(args.api_token)?;
    let filters = LabelFilter::new(args.products)?;
    let client = BillingClient::new(&args.api_base_url, token)?;
    let (preview, items) = client.get_current_invoice().await?;
    let report = BillingReport::new(preview, items, &filters, args.verbose)?;
    let rendered = match args.output {
        BillingOutput::Human => render_report(&report)?,
        BillingOutput::Json => render_json_report(&report)?,
    };

    write_stdout(&rendered)
}

fn resolve_token(value: Option<String>) -> Result<ApiToken> {
    let value = value
        .or_else(|| env::var(LEGACY_TOKEN_ENV_VAR).ok())
        .ok_or_else(|| {
            eyre!("set {BILLING_TOKEN_ENV_VAR} or {LEGACY_TOKEN_ENV_VAR}, or pass --api-token")
        })?;

    ApiToken::new(value)
}

impl ApiToken {
    fn new(value: String) -> Result<Self> {
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err(eyre!(
                "DigitalOcean API token must not be empty or contain whitespace"
            ));
        }

        Ok(Self(value))
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl BillingClient {
    fn new(base_url: &str, token: ApiToken) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("cmd/", env!("CARGO_PKG_VERSION")))
            .build()
            .wrap_err("create DigitalOcean billing HTTP client")?;
        let mut base_url = Url::parse(base_url).wrap_err("parse DigitalOcean API base URL")?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }

        Ok(Self {
            http,
            base_url,
            token,
        })
    }

    async fn get_current_invoice(&self) -> Result<(InvoicePreview, Vec<WireInvoiceItem>)> {
        let preview: InvoiceListResponse = self
            .get_json(
                "v2/customers/my/invoices?per_page=1&page=1",
                "get invoice preview",
            )
            .await?;
        let preview = preview
            .invoice_preview
            .context("DigitalOcean returned no current invoice preview")?;
        let mut items = Vec::new();
        let mut page = 1_usize;
        let mut expected_total = None;

        loop {
            let response: InvoiceItemsResponse = self
                .get_json(
                    &format!("v2/customers/my/invoices/preview?per_page={PAGE_SIZE}&page={page}"),
                    "get invoice preview items",
                )
                .await?;
            if expected_total.is_some_and(|total| response.meta.total != total) {
                return Err(eyre!(
                    "DigitalOcean changed the invoice item count during pagination"
                ));
            }

            expected_total = Some(response.meta.total);
            let has_next_page = response.links.pages.and_then(|pages| pages.next).is_some();
            if response.invoice_items.is_empty() && items.len() < response.meta.total {
                return Err(eyre!(
                    "DigitalOcean returned an empty invoice page before all items"
                ));
            }

            items.extend(response.invoice_items);
            if items.len() > response.meta.total {
                return Err(eyre!(
                    "DigitalOcean returned more invoice items than its total"
                ));
            }

            if items.len() == response.meta.total {
                break;
            }

            if !has_next_page {
                return Err(eyre!(
                    "DigitalOcean ended invoice pagination before all items"
                ));
            }

            page = page
                .checked_add(1)
                .ok_or_else(|| eyre!("DigitalOcean invoice page number is too large"))?;
        }

        Ok((preview, items))
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str, context: &str) -> Result<T> {
        let url = self
            .base_url
            .join(path)
            .wrap_err_with(|| format!("build DigitalOcean URL to {context}"))?;
        let response = self
            .http
            .get(url)
            .bearer_auth(self.token.expose())
            .send()
            .await
            .wrap_err_with(|| format!("DigitalOcean request failed: {context}"))?;
        let status = response.status();
        let body = read_limited_body(response, context).await?;
        if !status.is_success() {
            return Err(api_error(status, &body, context));
        }

        serde_json::from_slice(&body)
            .wrap_err_with(|| format!("parse DigitalOcean response: {context}"))
    }
}

async fn read_limited_body(mut response: reqwest::Response, context: &str) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .wrap_err_with(|| format!("read DigitalOcean response: {context}"))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(eyre!(
                "DigitalOcean response is larger than {MAX_RESPONSE_BYTES} bytes: {context}"
            ));
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

fn api_error(status: StatusCode, body: &[u8], context: &str) -> eyre::Report {
    let message = serde_json::from_slice::<ApiError>(body)
        .ok()
        .and_then(|error| error.message)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| "the provider returned no error message".to_string());

    eyre!("DigitalOcean {context} failed ({status}): {message}")
}

impl BillingReport {
    fn new(
        preview: InvoicePreview,
        items: Vec<WireInvoiceItem>,
        filters: &LabelFilter,
        verbose: bool,
    ) -> Result<Self> {
        let invoice_period = required_text(preview.invoice_period, "invoice_period")?;
        let start = NaiveDate::parse_from_str(&format!("{invoice_period}-01"), "%Y-%m-%d")
            .wrap_err("DigitalOcean returned an invalid invoice_period")?;
        let generated_at = preview
            .updated_at
            .context("DigitalOcean invoice preview has no updated_at")?;
        let period = BillingPeriod {
            start,
            usage_through: generated_at.date_naive(),
        };
        if period.usage_through < period.start {
            return Err(eyre!(
                "DigitalOcean invoice preview was generated before its billing period"
            ));
        }

        let mut products = BTreeMap::<String, Vec<InvoiceItem>>::new();

        for item in items {
            let item = ProductInvoiceItem::try_from(item)?;
            if !filters.matches(&item.product) || (!verbose && item.item.cost.is_zero()) {
                continue;
            }

            products.entry(item.product).or_default().push(item.item);
        }

        let products = products
            .into_iter()
            .map(|(name, mut items)| {
                items.sort_by(|left, right| {
                    left.description
                        .cmp(&right.description)
                        .then(left.resource_id.cmp(&right.resource_id))
                        .then(left.resource_uuid.cmp(&right.resource_uuid))
                        .then(left.start_time.cmp(&right.start_time))
                });

                ProductBilling { name, items }
            })
            .collect();

        Ok(Self {
            period,
            generated_at,
            products,
        })
    }

    fn total_cost(&self) -> Result<Option<Money>> {
        let Some(first) = self.products.first() else {
            return Ok(None);
        };
        let mut total = Money::zero(first.total_cost()?.currency());

        for product in &self.products {
            total.add(&product.total_cost()?, "DigitalOcean billing total")?;
        }

        Ok(Some(total))
    }
}

impl ProductBilling {
    fn total_cost(&self) -> Result<Money> {
        let first = self
            .items
            .first()
            .context("cannot total a DigitalOcean product without invoice items")?;
        let mut total = Money::zero(first.cost.currency());

        for item in &self.items {
            total.add(&item.cost, &format!("DigitalOcean product {}", self.name))?;
        }

        Ok(total)
    }
}

impl TryFrom<WireInvoiceItem> for ProductInvoiceItem {
    type Error = eyre::Report;

    fn try_from(item: WireInvoiceItem) -> Result<Self> {
        let product = optional_text(item.product).unwrap_or_else(|| "Uncategorized".to_string());
        let description =
            optional_text(item.description).unwrap_or_else(|| "Unlabeled item".to_string());
        let amount = required_text(item.amount, "invoice item amount")?;
        let duration = match (
            optional_text(item.duration),
            optional_text(item.duration_unit),
        ) {
            (None, None) => None,
            (Some(value), Some(unit)) => Some(UsageDuration { value, unit }),
            _ => {
                return Err(eyre!(
                    "DigitalOcean invoice item has an incomplete duration"
                ));
            }
        };

        if item
            .start_time
            .zip(item.end_time)
            .is_some_and(|(start, end)| end < start)
        {
            return Err(eyre!("DigitalOcean invoice item ends before it starts"));
        }

        let cost = Money::new(&amount, "USD", "DigitalOcean invoice item")?;

        Ok(Self {
            product,
            item: InvoiceItem {
                description,
                group_description: optional_text(item.group_description),
                project_name: optional_text(item.project_name),
                resource_id: optional_text(item.resource_id),
                resource_uuid: optional_text(item.resource_uuid),
                duration,
                start_time: item.start_time,
                end_time: item.end_time,
                cost,
            },
        })
    }
}

fn required_text(value: Option<String>, field: &str) -> Result<String> {
    optional_text(value).ok_or_else(|| eyre!("DigitalOcean billing response has no {field}"))
}

fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn render_report(report: &BillingReport) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "DigitalOcean billing usage")?;
    writeln!(
        output,
        "Period: {} through {} (UTC)",
        report.period.start, report.period.usage_through
    )?;
    writeln!(output, "Generated at: {}", report.generated_at.to_rfc3339())?;

    let Some(total) = report.total_cost()? else {
        writeln!(output, "\nNo billable usage found")?;
        return Ok(output);
    };
    writeln!(output, "Total billed cost: {total}")?;

    for product in &report.products {
        writeln!(output, "\n{}", product.name)?;
        writeln!(output, "  Total billed cost: {}", product.total_cost()?)?;

        for item in &product.items {
            writeln!(output, "\n  {}", item.description)?;
            if let Some(group) = &item.group_description {
                writeln!(output, "    Group: {group}")?;
            }

            if let Some(project) = &item.project_name {
                writeln!(output, "    Project: {project}")?;
            }

            if let Some(resource_id) = &item.resource_id {
                writeln!(output, "    Resource ID: {resource_id}")?;
            }

            if let Some(resource_uuid) = &item.resource_uuid {
                writeln!(output, "    Resource UUID: {resource_uuid}")?;
            }

            if let Some(duration) = &item.duration {
                writeln!(output, "    Duration: {} {}", duration.value, duration.unit)?;
            }

            if item.start_time.is_some() || item.end_time.is_some() {
                writeln!(
                    output,
                    "    Usage: {} through {}",
                    item.start_time
                        .map_or_else(|| "unknown".to_string(), |value| value.to_rfc3339()),
                    item.end_time
                        .map_or_else(|| "unknown".to_string(), |value| value.to_rfc3339())
                )?;
            }

            writeln!(output, "    Billed cost: {}", item.cost)?;
        }
    }

    Ok(output)
}

fn render_json_report(report: &BillingReport) -> Result<String> {
    let products = report
        .products
        .iter()
        .map(|product| {
            Ok(JsonProductBilling {
                name: product.name.clone(),
                total_cost: product.total_cost()?.json(),
                items: product.items.iter().map(JsonInvoiceItem::from).collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let report = JsonBillingReport {
        schema: "cmd.digitalocean.billing",
        version: 1,
        period: report.period,
        generated_at: report.generated_at,
        total_cost: report.total_cost()?.map(|money| money.json()),
        products,
    };
    let mut output = serde_json::to_string_pretty(&report)?;
    output.push('\n');

    Ok(output)
}

impl From<&InvoiceItem> for JsonInvoiceItem {
    fn from(item: &InvoiceItem) -> Self {
        Self {
            description: item.description.clone(),
            group_description: item.group_description.clone(),
            project_name: item.project_name.clone(),
            resource_id: item.resource_id.clone(),
            resource_uuid: item.resource_uuid.clone(),
            duration: item.duration.clone(),
            start_time: item.start_time,
            end_time: item.end_time,
            billed_cost: item.cost.json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Args;
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{
        render_json_report, render_report, ApiToken, BillingArgs, BillingClient, BillingReport,
        BILLING_TOKEN_ENV_VAR,
    };
    use crate::cmd::billing::LabelFilter;

    #[tokio::test]
    async fn fetches_all_invoice_preview_pages_with_a_billing_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/customers/my/invoices"))
            .and(query_param("per_page", "1"))
            .and(query_param("page", "1"))
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "invoice_preview": {
                    "invoice_period": "2026-08",
                    "updated_at": "2026-08-10T12:00:00Z"
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/customers/my/invoices/preview"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                vec![item("Droplets", "builder", "1.25")],
                2,
                true,
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/customers/my/invoices/preview"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                vec![item("Spaces", "cache", "0.50")],
                2,
                false,
            )))
            .mount(&server)
            .await;

        let client =
            BillingClient::new(&server.uri(), ApiToken::new("secret".to_string()).unwrap())
                .unwrap();
        let (_, items) = client.get_current_invoice().await.unwrap();

        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn reports_provider_errors_without_the_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/customers/my/invoices"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(json!({"message": "token is not authorized"})),
            )
            .mount(&server)
            .await;
        let client =
            BillingClient::new(&server.uri(), ApiToken::new("secret".to_string()).unwrap())
                .unwrap();

        let error = client.get_current_invoice().await.unwrap_err().to_string();

        assert!(error.contains("401 Unauthorized"));
        assert!(error.contains("token is not authorized"));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn filters_products_and_hides_zero_items_by_default() {
        let preview = serde_json::from_value(json!({
            "invoice_period": "2026-08",
            "updated_at": "2026-08-10T12:00:00Z"
        }))
        .unwrap();
        let items = vec![
            serde_json::from_value(item("Droplets", "zero", "0")).unwrap(),
            serde_json::from_value(item("Droplets", "builder", "1.25")).unwrap(),
            serde_json::from_value(item("Spaces", "cache", "2.50")).unwrap(),
        ];
        let report = BillingReport::new(
            preview,
            items,
            &LabelFilter::new(vec!["droplets".to_string()]).unwrap(),
            false,
        )
        .unwrap();

        assert_eq!(report.products.len(), 1);
        assert_eq!(report.products[0].items.len(), 1);
        assert_eq!(report.products[0].items[0].description, "builder");
    }

    #[test]
    fn verbose_includes_zero_items_and_output_is_stable() {
        let preview = serde_json::from_value(json!({
            "invoice_period": "2026-08",
            "updated_at": "2026-08-10T12:00:00Z"
        }))
        .unwrap();
        let items = vec![serde_json::from_value(item("Droplets", "builder", "0")).unwrap()];
        let report = BillingReport::new(preview, items, &LabelFilter::default(), true).unwrap();

        assert!(render_report(&report)
            .unwrap()
            .contains("Billed cost: $0.00 USD"));
        let json: serde_json::Value =
            serde_json::from_str(&render_json_report(&report).unwrap()).unwrap();
        assert_eq!(json["schema"], "cmd.digitalocean.billing");
        assert_eq!(json["total_cost"]["value"], 0);
        assert_eq!(json["products"][0]["items"][0]["description"], "builder");
        assert!(json.get("api_token").is_none());
    }

    #[test]
    fn keeps_negative_cost_items_in_the_default_report() {
        let preview = serde_json::from_value(json!({
            "invoice_period": "2026-08",
            "updated_at": "2026-08-10T12:00:00Z"
        }))
        .unwrap();
        let items = vec![serde_json::from_value(item("Credits", "credit", "-0.25")).unwrap()];

        let report = BillingReport::new(preview, items, &LabelFilter::default(), false).unwrap();

        assert_eq!(report.products.len(), 1);
        assert_eq!(
            report.total_cost().unwrap().unwrap().to_string(),
            "$-0.25 USD"
        );
    }

    #[test]
    fn rejects_invoice_items_without_an_amount() {
        let preview = serde_json::from_value(json!({
            "invoice_period": "2026-08",
            "updated_at": "2026-08-10T12:00:00Z"
        }))
        .unwrap();
        let items = vec![serde_json::from_value(json!({
            "product": "Droplets",
            "description": "builder"
        }))
        .unwrap()];

        assert!(BillingReport::new(preview, items, &LabelFilter::default(), false).is_err());
    }

    #[test]
    fn billing_help_never_displays_the_api_token() {
        let previous = std::env::var_os(BILLING_TOKEN_ENV_VAR);
        std::env::set_var(BILLING_TOKEN_ENV_VAR, "help-secret-token");
        let mut command = BillingArgs::augment_args(clap::Command::new("billing"));

        let help = command.render_long_help().to_string();

        match previous {
            Some(value) => std::env::set_var(BILLING_TOKEN_ENV_VAR, value),
            None => std::env::remove_var(BILLING_TOKEN_ENV_VAR),
        }
        assert!(help.contains(BILLING_TOKEN_ENV_VAR));
        assert!(!help.contains("help-secret-token"));
    }

    fn item(product: &str, description: &str, amount: &str) -> serde_json::Value {
        json!({
            "product": product,
            "description": description,
            "amount": amount,
            "duration": "10",
            "duration_unit": "Hours",
            "start_time": "2026-08-01T00:00:00Z",
            "end_time": "2026-08-10T00:00:00Z"
        })
    }

    fn page(
        invoice_items: Vec<serde_json::Value>,
        total: usize,
        has_next: bool,
    ) -> serde_json::Value {
        json!({
            "invoice_items": invoice_items,
            "links": {"pages": {"next": has_next.then_some("next")}},
            "meta": {"total": total}
        })
    }
}
