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
    ProviderBillingSummary,
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
    /// DigitalOcean API token with the `billing:read` scope; defaults to the `doctl auth login` token
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

    /// Show all invoice item details, including items with a zero billed cost
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
    let output = args.output;
    let verbose = args.verbose;
    let report = load_report(args).await?;
    let rendered = match output {
        BillingOutput::Human => render_report(&report, verbose)?,
        BillingOutput::Json => render_json_report(&report)?,
    };

    write_stdout(&rendered)
}

pub(super) async fn summary() -> Result<ProviderBillingSummary> {
    let report = load_report(BillingArgs {
        api_token: env::var(BILLING_TOKEN_ENV_VAR).ok(),
        products: Vec::new(),
        verbose: false,
        output: BillingOutput::Human,
        api_base_url: API_BASE_URL.to_string(),
    })
    .await?;
    let total_cost = report.total_cost()?;

    Ok(ProviderBillingSummary::new(Some(report.period), total_cost))
}

async fn load_report(args: BillingArgs) -> Result<BillingReport> {
    load_report_with(args, env::var(LEGACY_TOKEN_ENV_VAR).ok(), doctl_auth_token).await
}

async fn load_report_with(
    args: BillingArgs,
    legacy_token: Option<String>,
    doctl_token: impl Fn() -> Option<String>,
) -> Result<BillingReport> {
    let filters = LabelFilter::new(args.products)?;
    let (source, token) = resolve_token(args.api_token, legacy_token, &doctl_token)?;
    let (preview, items) = match fetch_invoice(&args.api_base_url, token).await {
        Err(err) if source.is_configured() && is_token_rejection(&err) => {
            retry_with_doctl(&args.api_base_url, source, err, &doctl_token).await?
        }
        result => result?,
    };

    BillingReport::new(preview, items, &filters, args.verbose)
}

async fn fetch_invoice(
    api_base_url: &str,
    token: ApiToken,
) -> Result<(InvoicePreview, Vec<WireInvoiceItem>)> {
    BillingClient::new(api_base_url, token)?
        .get_current_invoice()
        .await
}

// a configured token can go stale while the doctl login stays fresh, so a rejected
// token falls back to doctl instead of failing the whole report
async fn retry_with_doctl(
    api_base_url: &str,
    source: TokenSource,
    rejection: eyre::Report,
    doctl_token: impl Fn() -> Option<String>,
) -> Result<(InvoicePreview, Vec<WireInvoiceItem>)> {
    let Some(token) = doctl_token().and_then(non_empty_token) else {
        return Err(rejection);
    };

    eprintln!(
        "warning: DigitalOcean rejected the token from {}, using the doctl login instead: {rejection}",
        source.label()
    );
    fetch_invoice(api_base_url, ApiToken::new(token)?).await
}

fn is_token_rejection(err: &eyre::Report) -> bool {
    err.downcast_ref::<TokenRejected>().is_some()
}

/// Where the DigitalOcean API token came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenSource {
    /// `--api-token` or `DIGITALOCEAN_BILLING_TOKEN`, which clap merges
    Explicit,
    Legacy,
    Doctl,
}

impl TokenSource {
    fn is_configured(self) -> bool {
        self != Self::Doctl
    }

    fn label(self) -> String {
        match self {
            Self::Explicit => format!("--api-token or {BILLING_TOKEN_ENV_VAR}"),
            Self::Legacy => LEGACY_TOKEN_ENV_VAR.to_string(),
            Self::Doctl => "`doctl auth token`".to_string(),
        }
    }
}

// configured tokens win so a dedicated billing token still overrides the doctl login,
// and doctl only runs when nothing is configured
fn resolve_token(
    explicit_token: Option<String>,
    legacy_token: Option<String>,
    doctl_token: impl Fn() -> Option<String>,
) -> Result<(TokenSource, ApiToken)> {
    let (source, value) = explicit_token
        .and_then(non_empty_token)
        .map(|token| (TokenSource::Explicit, token))
        .or_else(|| legacy_token.and_then(non_empty_token).map(|token| (TokenSource::Legacy, token)))
        .or_else(|| doctl_token().and_then(non_empty_token).map(|token| (TokenSource::Doctl, token)))
        .ok_or_else(|| {
            eyre!(
                "set {BILLING_TOKEN_ENV_VAR} or {LEGACY_TOKEN_ENV_VAR}, pass --api-token, or run `doctl auth login`"
            )
        })?;

    Ok((source, ApiToken::new(value)?))
}

// `doctl auth login` stores a short-lived OAuth token that doctl renews itself, so
// asking doctl for it avoids keeping a long-lived personal access token around
fn doctl_auth_token() -> Option<String> {
    let output = std::process::Command::new("doctl")
        .args(["auth", "token"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

fn non_empty_token(token: String) -> Option<String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// DigitalOcean refused the token itself (401 or 403), as opposed to failing the request
#[derive(Debug)]
struct TokenRejected {
    status: StatusCode,
    context: String,
    message: String,
}

impl std::fmt::Display for TokenRejected {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            status,
            context,
            message,
        } = self;
        write!(
            formatter,
            "DigitalOcean {context} failed ({status}): {message}"
        )
    }
}

impl std::error::Error for TokenRejected {}

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
        let preview_request = self.get_json::<InvoiceListResponse>(
            "v2/customers/my/invoices?per_page=1&page=1",
            "get invoice preview",
        );
        let first_items_request = self.get_invoice_items_page(1);
        let (preview, first_items) = tokio::join!(preview_request, first_items_request);
        let preview = preview?
            .invoice_preview
            .context("DigitalOcean returned no current invoice preview")?;

        let mut items = Vec::new();
        let mut page = 1_usize;
        let mut response = first_items?;
        let expected_total = response.meta.total;

        loop {
            if response.meta.total != expected_total {
                return Err(eyre!(
                    "DigitalOcean changed the invoice item count during pagination"
                ));
            }

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
            response = self.get_invoice_items_page(page).await?;
        }

        Ok((preview, items))
    }

    async fn get_invoice_items_page(&self, page: usize) -> Result<InvoiceItemsResponse> {
        self.get_json(
            &format!("v2/customers/my/invoices/preview?per_page={PAGE_SIZE}&page={page}"),
            "get invoice preview items",
        )
        .await
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

    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return eyre::Report::new(TokenRejected {
            status,
            context: context.to_string(),
            message,
        });
    }

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

fn render_report(report: &BillingReport, verbose: bool) -> Result<String> {
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

    if !verbose {
        writeln!(output, "\nProducts")?;
        for product in &report.products {
            let item_count = product.items.len();
            let item_label = if item_count == 1 { "item" } else { "items" };
            writeln!(
                output,
                "  {}: {} ({item_count} {item_label})",
                product.name,
                product.total_cost()?
            )?;
        }

        return Ok(output);
    }

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
        load_report_with, render_json_report, render_report, resolve_token, ApiToken, BillingArgs,
        BillingClient, BillingOutput, BillingReport, TokenSource, BILLING_TOKEN_ENV_VAR,
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

        assert!(render_report(&report, true)
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
    fn default_report_shows_only_product_totals() {
        let preview = serde_json::from_value(json!({
            "invoice_period": "2026-08",
            "updated_at": "2026-08-10T12:00:00Z"
        }))
        .unwrap();
        let items = vec![
            serde_json::from_value(item("Droplets", "builder", "1.25")).unwrap(),
            serde_json::from_value(item("Droplets", "runner", "0.75")).unwrap(),
        ];
        let report = BillingReport::new(preview, items, &LabelFilter::default(), false).unwrap();

        let output = render_report(&report, false).unwrap();

        assert!(output.contains("Products\n  Droplets: $2.00 USD (2 items)"));
        assert!(!output.contains("builder"));
        assert!(!output.contains("Resource ID"));
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
    fn configured_tokens_win_over_doctl() {
        let no_doctl = || -> Option<String> { panic!("doctl must not run when a token is set") };
        let doctl = || Some("doctl-token\n".to_string());

        let (explicit, _) =
            resolve_token(Some("billing".into()), Some("legacy".into()), no_doctl).unwrap();
        let (blank_explicit, _) =
            resolve_token(Some("  ".into()), Some("legacy".into()), no_doctl).unwrap();
        let (only_doctl, token) = resolve_token(None, None, doctl).unwrap();
        let none = resolve_token(None, None, || Some(String::new()));

        assert_eq!(explicit, TokenSource::Explicit);
        assert_eq!(blank_explicit, TokenSource::Legacy);
        assert_eq!(only_doctl, TokenSource::Doctl);
        assert_eq!(token.expose(), "doctl-token");
        assert!(none.is_err());
    }

    #[tokio::test]
    async fn rejected_configured_token_falls_back_to_doctl() {
        let server = MockServer::start().await;
        Mock::given(header("authorization", "Bearer stale"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(json!({"message": "Unable to authenticate you"})),
            )
            .mount(&server)
            .await;
        mount_single_page_invoice(&server, "Bearer fresh").await;

        let report = load_report_with(billing_args(&server, "stale"), None, || {
            Some("fresh".to_string())
        })
        .await
        .unwrap();

        let products: Vec<_> = report
            .products
            .iter()
            .map(|product| product.name.as_str())
            .collect();
        assert_eq!(products, ["Droplets"]);
    }

    #[tokio::test]
    async fn non_auth_failures_do_not_fall_back_to_doctl() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({"message": "down"})))
            .mount(&server)
            .await;
        let doctl_calls = std::cell::Cell::new(0);

        let error = load_report_with(billing_args(&server, "configured"), None, || {
            doctl_calls.set(doctl_calls.get() + 1);
            Some("fresh".to_string())
        })
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("500"));
        assert_eq!(doctl_calls.get(), 0);
    }

    async fn mount_single_page_invoice(server: &MockServer, authorization: &str) {
        Mock::given(path("/v2/customers/my/invoices"))
            .and(header("authorization", authorization))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "invoice_preview": {
                    "invoice_period": "2026-08",
                    "updated_at": "2026-08-10T12:00:00Z"
                }
            })))
            .mount(server)
            .await;
        Mock::given(path("/v2/customers/my/invoices/preview"))
            .and(header("authorization", authorization))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                vec![item("Droplets", "builder", "1.25")],
                1,
                false,
            )))
            .mount(server)
            .await;
    }

    fn billing_args(server: &MockServer, token: &str) -> BillingArgs {
        BillingArgs {
            api_token: Some(token.to_string()),
            products: Vec::new(),
            verbose: false,
            output: BillingOutput::Human,
            api_base_url: server.uri(),
        }
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
