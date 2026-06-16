use std::ffi::OsString;

use clap::{Args, Parser, Subcommand};
use eyre::{eyre, Context, Result};
use reqwest::{Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use xshell::Shell;

use crate::runtime;

const API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";
const API_TOKEN_ENV_VAR: &str = "CMD_CLOUDFLARE_REDIRECT_API_TOKEN";
const REDIRECT_PHASE: &str = "http_request_dynamic_redirect";
const REDIRECT_RULESET_NAME: &str = "Redirect rules ruleset";

#[derive(Debug, Clone, Parser)]
pub struct Cloudflare {
    #[command(subcommand)]
    pub subcommand: CloudflareCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CloudflareCmd {
    /// Manage Single Redirect rules
    #[command(arg_required_else_help = true)]
    Redirect {
        #[command(subcommand)]
        subcommand: RedirectCmd,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum RedirectCmd {
    /// List Single Redirect rules for a zone
    List(#[command(flatten)] ListRedirectArgs),

    /// Redirect www.example.com to example.com
    #[command(name = "www-to-apex")]
    WwwToApex(#[command(flatten)] RedirectArgs),

    /// Redirect example.com to www.example.com
    #[command(name = "apex-to-www")]
    ApexToWww(#[command(flatten)] RedirectArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ListRedirectArgs {
    /// Zone apex, hostname, or URL, for example example.com or www.example.com/path
    pub zone: String,

    /// Cloudflare API token
    #[arg(long, env = API_TOKEN_ENV_VAR)]
    pub api_token: Option<String>,

    /// Cloudflare zone ID, skips zone lookup when provided
    #[arg(long)]
    pub zone_id: Option<String>,

    /// Override the Cloudflare API base URL
    #[arg(long, hide = true, default_value = API_BASE_URL)]
    pub api_base_url: String,
}

#[derive(Debug, Clone, Args)]
pub struct RedirectArgs {
    /// Zone apex, hostname, or URL, for example example.com or www.example.com/path
    pub zone: String,

    /// Cloudflare API token
    #[arg(long, env = API_TOKEN_ENV_VAR)]
    pub api_token: Option<String>,

    /// Cloudflare zone ID, skips zone lookup when provided
    #[arg(long)]
    pub zone_id: Option<String>,

    /// Redirect status code
    #[arg(long, default_value_t = 301)]
    pub status_code: u16,

    /// Disable query string preservation
    #[arg(
        long = "no-preserve-query-string",
        default_value_t = true,
        action = clap::ArgAction::SetFalse
    )]
    pub preserve_query_string: bool,

    /// Ensure www.<zone> exists as a proxied CNAME to the zone apex
    #[arg(long, alias = "ensure-dns")]
    pub ensure_www_dns: bool,

    /// Print the planned API changes without mutating Cloudflare
    #[arg(long)]
    pub dry_run: bool,

    /// Override the Cloudflare API base URL
    #[arg(long, hide = true, default_value = API_BASE_URL)]
    pub api_base_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedirectDirection {
    WwwToApex,
    ApexToWww,
}

#[derive(Debug, Clone)]
struct RedirectPlan {
    zone_name: String,
    source_host: String,
    target_host: String,
    rule: RedirectRuleRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleChange {
    CreateRuleset,
    CreateRule,
    UpdateRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DnsChange {
    Skipped,
    AlreadyProxied,
    CreateCname,
    ProxyExistingCname,
}

#[derive(Debug)]
struct ApplyResult {
    zone_id: String,
    zone_name: String,
    source_host: String,
    target_host: String,
    rule_change: RuleChange,
    dns_change: DnsChange,
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct CloudflareEnvelope<T> {
    success: bool,
    result: Option<T>,
    #[serde(default)]
    errors: Vec<CloudflareApiMessage>,
    #[serde(default)]
    messages: Vec<CloudflareApiMessage>,
}

#[derive(Debug, Deserialize)]
struct CloudflareErrorEnvelope {
    #[serde(default)]
    errors: Vec<CloudflareApiMessage>,
}

#[derive(Debug, Deserialize)]
struct CloudflareApiMessage {
    code: Option<u64>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct Zone {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Ruleset {
    id: String,
    #[serde(default)]
    rules: Vec<RulesetRule>,
}

#[derive(Debug, Deserialize)]
struct RulesetRule {
    id: Option<String>,
    #[serde(rename = "ref")]
    ref_id: Option<String>,
    expression: Option<String>,
    description: Option<String>,
    action: Option<String>,
    action_parameters: Option<RedirectActionParameters>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    proxied: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CreateRulesetRequest {
    name: String,
    kind: &'static str,
    phase: &'static str,
    rules: Vec<RedirectRuleRequest>,
}

#[derive(Debug, Clone, Serialize)]
struct RedirectRuleRequest {
    #[serde(rename = "ref")]
    ref_id: String,
    expression: String,
    description: String,
    action: &'static str,
    action_parameters: RedirectActionParameters,
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RedirectActionParameters {
    from_value: RedirectFromValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RedirectFromValue {
    target_url: RedirectTargetUrl,
    status_code: u16,
    preserve_query_string: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RedirectTargetUrl {
    expression: String,
}

#[derive(Debug, Serialize)]
struct CreateDnsRecordRequest {
    #[serde(rename = "type")]
    record_type: &'static str,
    name: String,
    content: String,
    ttl: u16,
    proxied: bool,
}

#[derive(Debug, Serialize)]
struct PatchDnsRecordRequest {
    proxied: bool,
}

struct CloudflareApi {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl CloudflareApi {
    fn new(base_url: String, token: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("cmd-cloudflare")
            .build()?;

        Ok(Self {
            client,
            base_url,
            token,
        })
    }

    async fn find_zone(&self, zone_name: &str) -> Result<Option<Zone>> {
        let path = format!(
            "zones?{}",
            query_string(&[("name", zone_name), ("per_page", "50")])
        );
        let request = self.request(Method::GET, &path);
        let zones: Vec<Zone> = self.send_json(request, "list zones").await?;

        Ok(zones.into_iter().find(|zone| zone.name == zone_name))
    }

    async fn find_zone_for_host(&self, host: &str) -> Result<Zone> {
        for candidate in zone_name_candidates(host) {
            if let Some(zone) = self.find_zone(&candidate).await? {
                return Ok(zone);
            }
        }

        Err(eyre!(
            "Cloudflare zone not found for {host}; pass --zone-id or use a hostname in an accessible zone"
        ))
    }

    async fn get_zone(&self, zone_id: &str) -> Result<Zone> {
        let path = format!("zones/{zone_id}");
        let request = self.request(Method::GET, &path);
        self.send_json(request, "get zone").await
    }

    async fn get_redirect_ruleset(&self, zone_id: &str) -> Result<Option<Ruleset>> {
        let path = format!("zones/{zone_id}/rulesets/phases/{REDIRECT_PHASE}/entrypoint");
        let request = self.request(Method::GET, &path);
        self.send_optional_json(request, "get redirect ruleset")
            .await
    }

    async fn create_redirect_ruleset(
        &self,
        zone_id: &str,
        rule: RedirectRuleRequest,
    ) -> Result<Ruleset> {
        let path = format!("zones/{zone_id}/rulesets");
        let payload = CreateRulesetRequest {
            name: REDIRECT_RULESET_NAME.to_string(),
            kind: "zone",
            phase: REDIRECT_PHASE,
            rules: vec![rule],
        };
        let request = self.request(Method::POST, &path).json(&payload);
        self.send_json(request, "create redirect ruleset").await
    }

    async fn create_redirect_rule(
        &self,
        zone_id: &str,
        ruleset_id: &str,
        rule: RedirectRuleRequest,
    ) -> Result<Ruleset> {
        let path = format!("zones/{zone_id}/rulesets/{ruleset_id}/rules");
        let request = self.request(Method::POST, &path).json(&rule);
        self.send_json(request, "create redirect rule").await
    }

    async fn update_redirect_rule(
        &self,
        zone_id: &str,
        ruleset_id: &str,
        rule_id: &str,
        rule: RedirectRuleRequest,
    ) -> Result<Ruleset> {
        let path = format!("zones/{zone_id}/rulesets/{ruleset_id}/rules/{rule_id}");
        let request = self.request(Method::PATCH, &path).json(&rule);
        self.send_json(request, "update redirect rule").await
    }

    async fn list_dns_records(&self, zone_id: &str, name: &str) -> Result<Vec<DnsRecord>> {
        let path = format!(
            "zones/{zone_id}/dns_records?{}",
            query_string(&[("name", name), ("per_page", "100")])
        );
        let request = self.request(Method::GET, &path);
        self.send_json(request, "list DNS records").await
    }

    async fn create_proxied_cname(
        &self,
        zone_id: &str,
        name: &str,
        content: &str,
    ) -> Result<DnsRecord> {
        let path = format!("zones/{zone_id}/dns_records");
        let payload = CreateDnsRecordRequest {
            record_type: "CNAME",
            name: name.to_string(),
            content: content.to_string(),
            ttl: 1,
            proxied: true,
        };
        let request = self.request(Method::POST, &path).json(&payload);
        self.send_json(request, "create DNS record").await
    }

    async fn proxy_dns_record(&self, zone_id: &str, record_id: &str) -> Result<DnsRecord> {
        let path = format!("zones/{zone_id}/dns_records/{record_id}");
        let payload = PatchDnsRecordRequest { proxied: true };
        let request = self.request(Method::PATCH, &path).json(&payload);
        self.send_json(request, "update DNS record").await
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, self.url(path))
            .bearer_auth(&self.token)
            .header("Content-Type", "application/json")
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn send_json<T>(&self, request: RequestBuilder, context: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = request
            .send()
            .await
            .wrap_err_with(|| format!("failed to send Cloudflare request: {context}"))?;

        parse_response(response, context).await
    }

    async fn send_optional_json<T>(
        &self,
        request: RequestBuilder,
        context: &str,
    ) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let response = request
            .send()
            .await
            .wrap_err_with(|| format!("failed to send Cloudflare request: {context}"))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        parse_response(response, context).await.map(Some)
    }
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Cloudflare::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(_sh: &Shell, flags: Cloudflare) -> Result<()> {
    runtime::block_on(run_async(flags))?
}

async fn run_async(flags: Cloudflare) -> Result<()> {
    match flags.subcommand {
        CloudflareCmd::Redirect { subcommand } => match subcommand {
            RedirectCmd::List(args) => {
                let result = list_redirects(args).await?;
                print_list_result(&result);
            }
            RedirectCmd::WwwToApex(args) => {
                let result = apply_redirect(RedirectDirection::WwwToApex, args).await?;
                print_apply_result(&result);
            }
            RedirectCmd::ApexToWww(args) => {
                let result = apply_redirect(RedirectDirection::ApexToWww, args).await?;
                print_apply_result(&result);
            }
        },
    }

    Ok(())
}

#[derive(Debug)]
struct ListResult {
    zone_id: String,
    zone_name: String,
    rules: Vec<RulesetRule>,
}

async fn list_redirects(args: ListRedirectArgs) -> Result<ListResult> {
    let host = normalize_hostname(&args.zone)?;
    let api = cloudflare_api(args.api_base_url, args.api_token)?;
    let zone = resolve_zone(&api, &host, args.zone_id).await?;
    let rules = api
        .get_redirect_ruleset(&zone.id)
        .await?
        .map(|ruleset| {
            ruleset
                .rules
                .into_iter()
                .filter(|rule| rule.action.as_deref() == Some("redirect"))
                .collect()
        })
        .unwrap_or_default();

    Ok(ListResult {
        zone_id: zone.id,
        zone_name: zone.name,
        rules,
    })
}

async fn apply_redirect(direction: RedirectDirection, args: RedirectArgs) -> Result<ApplyResult> {
    validate_status_code(args.status_code)?;

    let host = normalize_hostname(&args.zone)?;
    let api = cloudflare_api(args.api_base_url, args.api_token)?;
    let zone = resolve_zone(&api, &host, args.zone_id).await?;
    let plan = RedirectPlan::new(
        direction,
        &zone.name,
        args.status_code,
        args.preserve_query_string,
    )?;

    let dns_change = ensure_www_dns(
        &api,
        &zone.id,
        &plan.zone_name,
        args.ensure_www_dns,
        args.dry_run,
    )
    .await?;
    let rule_change = apply_redirect_rule(&api, &zone.id, &plan.rule, args.dry_run).await?;

    Ok(ApplyResult {
        zone_id: zone.id,
        zone_name: plan.zone_name,
        source_host: plan.source_host,
        target_host: plan.target_host,
        rule_change,
        dns_change,
        dry_run: args.dry_run,
    })
}

fn cloudflare_api(base_url: String, api_token: Option<String>) -> Result<CloudflareApi> {
    let token = api_token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| eyre!("set {API_TOKEN_ENV_VAR} or pass --api-token"))?;

    CloudflareApi::new(base_url, token)
}

async fn resolve_zone(api: &CloudflareApi, host: &str, zone_id: Option<String>) -> Result<Zone> {
    if let Some(zone_id) = zone_id.filter(|zone_id| !zone_id.trim().is_empty()) {
        return api.get_zone(&zone_id).await;
    }

    api.find_zone_for_host(host).await
}

async fn ensure_www_dns(
    api: &CloudflareApi,
    zone_id: &str,
    zone_name: &str,
    ensure_www_dns: bool,
    dry_run: bool,
) -> Result<DnsChange> {
    if !ensure_www_dns {
        return Ok(DnsChange::Skipped);
    }

    let www_host = format!("www.{zone_name}");
    let records = api.list_dns_records(zone_id, &www_host).await?;
    let change = planned_dns_change(&records, zone_name)?;

    if dry_run {
        return Ok(change);
    }

    match change {
        DnsChange::CreateCname => {
            api.create_proxied_cname(zone_id, &www_host, zone_name)
                .await?;
        }
        DnsChange::ProxyExistingCname => {
            let record = matching_cname(&records, zone_name)
                .ok_or_else(|| eyre!("expected existing CNAME for {www_host}"))?;
            api.proxy_dns_record(zone_id, &record.id).await?;
        }
        DnsChange::Skipped | DnsChange::AlreadyProxied => {}
    }

    Ok(change)
}

fn planned_dns_change(records: &[DnsRecord], zone_name: &str) -> Result<DnsChange> {
    if records.iter().any(|record| record.proxied == Some(true)) {
        return Ok(DnsChange::AlreadyProxied);
    }

    if records.is_empty() {
        return Ok(DnsChange::CreateCname);
    }

    if matching_cname(records, zone_name).is_some() {
        return Ok(DnsChange::ProxyExistingCname);
    }

    Err(eyre!(
        "DNS records already exist for www.{zone_name}, but none are a proxied CNAME to {zone_name}; refusing to replace them"
    ))
}

fn matching_cname<'a>(records: &'a [DnsRecord], zone_name: &str) -> Option<&'a DnsRecord> {
    records.iter().find(|record| {
        record.record_type == "CNAME"
            && normalize_dns_content(&record.content) == normalize_dns_content(zone_name)
    })
}

fn normalize_dns_content(value: &str) -> String {
    value.trim().trim_end_matches('.').to_ascii_lowercase()
}

async fn apply_redirect_rule(
    api: &CloudflareApi,
    zone_id: &str,
    rule: &RedirectRuleRequest,
    dry_run: bool,
) -> Result<RuleChange> {
    let Some(ruleset) = api.get_redirect_ruleset(zone_id).await? else {
        if !dry_run {
            api.create_redirect_ruleset(zone_id, rule.clone()).await?;
        }

        return Ok(RuleChange::CreateRuleset);
    };

    let existing_rule = find_existing_rule(&ruleset.rules, rule);
    let Some(rule_id) = existing_rule.and_then(|rule| rule.id.as_deref()) else {
        if !dry_run {
            api.create_redirect_rule(zone_id, &ruleset.id, rule.clone())
                .await?;
        }

        return Ok(RuleChange::CreateRule);
    };

    if !dry_run {
        api.update_redirect_rule(zone_id, &ruleset.id, rule_id, rule.clone())
            .await?;
    }

    Ok(RuleChange::UpdateRule)
}

fn find_existing_rule<'a>(
    rules: &'a [RulesetRule],
    desired: &RedirectRuleRequest,
) -> Option<&'a RulesetRule> {
    rules
        .iter()
        .find(|rule| rule.ref_id.as_deref() == Some(desired.ref_id.as_str()))
        .or_else(|| {
            rules.iter().find(|rule| {
                rule.action.as_deref() == Some("redirect")
                    && rule.expression.as_deref() == Some(desired.expression.as_str())
            })
        })
        .or_else(|| {
            rules.iter().find(|rule| {
                rule.action.as_deref() == Some("redirect")
                    && rule.description.as_deref() == Some(desired.description.as_str())
            })
        })
}

impl RedirectPlan {
    fn new(
        direction: RedirectDirection,
        zone: &str,
        status_code: u16,
        preserve_query_string: bool,
    ) -> Result<Self> {
        let zone_name = normalize_hostname(zone)?;
        let (source_host, target_host, ref_id, description) = match direction {
            RedirectDirection::WwwToApex => (
                format!("www.{zone_name}"),
                zone_name.clone(),
                "cmd_redirect_www_to_apex",
                "Redirect www to apex",
            ),
            RedirectDirection::ApexToWww => (
                zone_name.clone(),
                format!("www.{zone_name}"),
                "cmd_redirect_apex_to_www",
                "Redirect apex to www",
            ),
        };

        let rule = RedirectRuleRequest {
            ref_id: ref_id.to_string(),
            expression: format!("http.host eq \"{source_host}\""),
            description: description.to_string(),
            action: "redirect",
            action_parameters: RedirectActionParameters {
                from_value: RedirectFromValue {
                    target_url: RedirectTargetUrl {
                        expression: format!(
                            "concat(\"https://{target_host}\", http.request.uri.path)"
                        ),
                    },
                    status_code,
                    preserve_query_string,
                },
            },
            enabled: true,
        };

        Ok(Self {
            zone_name,
            source_host,
            target_host,
            rule,
        })
    }
}

fn normalize_hostname(host: &str) -> Result<String> {
    let host = host.trim();
    let host = if host.starts_with("https://") || host.starts_with("http://") {
        url::Url::parse(host)
            .wrap_err_with(|| format!("invalid URL: {host}"))?
            .host_str()
            .ok_or_else(|| eyre!("URL does not include a hostname: {host}"))?
            .to_string()
    } else {
        host.split('/')
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default()
            .to_string()
    };

    let zone = host.trim();
    let zone = zone
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();

    if zone.is_empty() {
        return Err(eyre!("hostname is empty"));
    }

    if zone.contains(['"', '&', '?', '#', '=']) || zone.contains(char::is_whitespace) {
        return Err(eyre!("invalid hostname: {zone}"));
    }

    Ok(zone)
}

fn zone_name_candidates(host: &str) -> Vec<String> {
    let labels = host.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return vec![host.to_string()];
    }

    (0..labels.len() - 1)
        .map(|index| labels[index..].join("."))
        .collect()
}

fn query_string(params: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish()
}

fn validate_status_code(status_code: u16) -> Result<()> {
    if matches!(status_code, 301 | 302 | 307 | 308) {
        return Ok(());
    }

    Err(eyre!(
        "redirect status code must be one of 301, 302, 307, or 308"
    ))
}

async fn parse_response<T>(response: reqwest::Response, context: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(eyre!(
            "Cloudflare {context} failed with HTTP {status}: {}",
            format_error_body(&body)
        ));
    }

    let envelope: CloudflareEnvelope<T> = serde_json::from_str(&body)
        .wrap_err_with(|| format!("failed to parse Cloudflare {context} response: {body}"))?;

    if !envelope.success {
        return Err(eyre!(
            "Cloudflare {context} failed: {}",
            format_api_messages(&envelope.errors)
        ));
    }

    for message in &envelope.messages {
        eprintln!("Cloudflare {context}: {}", message.message);
    }

    envelope
        .result
        .ok_or_else(|| eyre!("Cloudflare {context} response did not include a result"))
}

fn format_error_body(body: &str) -> String {
    match serde_json::from_str::<CloudflareErrorEnvelope>(body) {
        Ok(envelope) if !envelope.errors.is_empty() => format_api_messages(&envelope.errors),
        _ => body.to_string(),
    }
}

fn format_api_messages(messages: &[CloudflareApiMessage]) -> String {
    if messages.is_empty() {
        return "no error details".to_string();
    }

    messages
        .iter()
        .map(|message| match message.code {
            Some(code) => format!("{code}: {}", message.message),
            None => message.message.clone(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn print_apply_result(result: &ApplyResult) {
    let prefix = if result.dry_run {
        "Would apply"
    } else {
        "Applied"
    };

    println!(
        "{prefix} Cloudflare redirect: {} -> https://{}",
        result.source_host, result.target_host
    );
    println!("Zone: {} ({})", result.zone_name, result.zone_id);
    println!("Rule: {}", rule_change_label(result.rule_change));
    println!("DNS: {}", dns_change_label(result.dns_change));
}

fn print_list_result(result: &ListResult) {
    println!("Zone: {} ({})", result.zone_name, result.zone_id);

    if result.rules.is_empty() {
        println!("No Single Redirect rules found");
        return;
    }

    for (index, rule) in result.rules.iter().enumerate() {
        println!();
        println!("{}. {}", index + 1, rule_title(rule));
        println!("   ID: {}", rule.id.as_deref().unwrap_or("-"));
        println!("   Ref: {}", rule.ref_id.as_deref().unwrap_or("-"));
        println!(
            "   Enabled: {}",
            rule.enabled
                .map(|enabled| if enabled { "yes" } else { "no" })
                .unwrap_or("unknown")
        );
        println!("   When: {}", rule.expression.as_deref().unwrap_or("-"));

        if let Some(parameters) = &rule.action_parameters {
            println!("   Target: {}", parameters.from_value.target_url.expression);
            println!("   Status: {}", parameters.from_value.status_code);
            println!(
                "   Preserve query string: {}",
                if parameters.from_value.preserve_query_string {
                    "yes"
                } else {
                    "no"
                }
            );
        }
    }
}

fn rule_title(rule: &RulesetRule) -> &str {
    rule.description.as_deref().unwrap_or("Untitled redirect")
}

fn rule_change_label(change: RuleChange) -> &'static str {
    match change {
        RuleChange::CreateRuleset => "create redirect ruleset",
        RuleChange::CreateRule => "create redirect rule",
        RuleChange::UpdateRule => "update redirect rule",
    }
}

fn dns_change_label(change: DnsChange) -> &'static str {
    match change {
        DnsChange::Skipped => "skipped",
        DnsChange::AlreadyProxied => "www record already proxied",
        DnsChange::CreateCname => "create proxied www CNAME",
        DnsChange::ProxyExistingCname => "proxy existing www CNAME",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cloudflare_api, find_existing_rule, normalize_hostname, planned_dns_change, rule_title,
        validate_status_code, zone_name_candidates, DnsChange, DnsRecord, RedirectDirection,
        RedirectPlan, RulesetRule, API_TOKEN_ENV_VAR,
    };

    #[test]
    fn builds_www_to_apex_redirect_rule() {
        let plan = RedirectPlan::new(RedirectDirection::WwwToApex, "Example.COM", 301, true)
            .expect("valid plan");

        assert_eq!(plan.zone_name, "example.com");
        assert_eq!(plan.source_host, "www.example.com");
        assert_eq!(plan.target_host, "example.com");
        assert_eq!(plan.rule.ref_id, "cmd_redirect_www_to_apex");
        assert_eq!(plan.rule.expression, "http.host eq \"www.example.com\"");
        assert_eq!(
            plan.rule.action_parameters.from_value.target_url.expression,
            "concat(\"https://example.com\", http.request.uri.path)"
        );
        assert!(plan.rule.action_parameters.from_value.preserve_query_string);
    }

    #[test]
    fn builds_apex_to_www_redirect_rule() {
        let plan = RedirectPlan::new(RedirectDirection::ApexToWww, "example.com", 308, false)
            .expect("valid plan");

        assert_eq!(plan.source_host, "example.com");
        assert_eq!(plan.target_host, "www.example.com");
        assert_eq!(plan.rule.ref_id, "cmd_redirect_apex_to_www");
        assert_eq!(plan.rule.expression, "http.host eq \"example.com\"");
        assert_eq!(
            plan.rule.action_parameters.from_value.target_url.expression,
            "concat(\"https://www.example.com\", http.request.uri.path)"
        );
        assert_eq!(plan.rule.action_parameters.from_value.status_code, 308);
        assert!(!plan.rule.action_parameters.from_value.preserve_query_string);
    }

    #[test]
    fn rejects_invalid_redirect_status_code() {
        assert!(validate_status_code(303).is_err());
        assert!(validate_status_code(301).is_ok());
    }

    #[test]
    fn missing_api_token_names_command_specific_env_var() {
        let Err(err) = cloudflare_api("https://example.test".to_string(), None) else {
            panic!("expected missing token error");
        };

        assert!(err.to_string().contains(API_TOKEN_ENV_VAR));
    }

    #[test]
    fn normalizes_zone_names_from_urls() {
        assert_eq!(
            normalize_hostname("https://Example.COM/path").expect("valid zone"),
            "example.com"
        );
    }

    #[test]
    fn normalizes_hostnames_from_urls_and_ports() {
        assert_eq!(
            normalize_hostname("https://www.Example.COM:443/path?x=1").expect("valid host"),
            "www.example.com"
        );
    }

    #[test]
    fn creates_zone_lookup_candidates_from_longest_suffix() {
        assert_eq!(
            zone_name_candidates("www.foo.example.co.uk"),
            [
                "www.foo.example.co.uk",
                "foo.example.co.uk",
                "example.co.uk",
                "co.uk"
            ]
        );
    }

    #[test]
    fn plans_to_create_missing_www_dns_record() {
        let records = [];
        assert_eq!(
            planned_dns_change(&records, "example.com").expect("dns plan"),
            DnsChange::CreateCname
        );
    }

    #[test]
    fn plans_to_proxy_existing_www_cname() {
        let records = [DnsRecord {
            id: "record-id".to_string(),
            record_type: "CNAME".to_string(),
            content: "example.com.".to_string(),
            proxied: Some(false),
        }];

        assert_eq!(
            planned_dns_change(&records, "example.com").expect("dns plan"),
            DnsChange::ProxyExistingCname
        );
    }

    #[test]
    fn refuses_to_replace_unrelated_dns_record() {
        let records = [DnsRecord {
            id: "record-id".to_string(),
            record_type: "A".to_string(),
            content: "192.0.2.1".to_string(),
            proxied: Some(false),
        }];

        assert!(planned_dns_change(&records, "example.com").is_err());
    }

    #[test]
    fn finds_existing_rule_by_ref_or_expression() {
        let plan = RedirectPlan::new(RedirectDirection::WwwToApex, "example.com", 301, true)
            .expect("valid plan");
        let rules = [RulesetRule {
            id: Some("rule-id".to_string()),
            ref_id: None,
            expression: Some("http.host eq \"www.example.com\"".to_string()),
            description: None,
            action: Some("redirect".to_string()),
            action_parameters: None,
            enabled: None,
        }];

        assert_eq!(
            find_existing_rule(&rules, &plan.rule).and_then(|rule| rule.id.as_deref()),
            Some("rule-id")
        );
    }

    #[test]
    fn uses_fallback_title_for_unnamed_redirect_rule() {
        let rule = RulesetRule {
            id: None,
            ref_id: None,
            expression: None,
            description: None,
            action: Some("redirect".to_string()),
            action_parameters: None,
            enabled: None,
        };

        assert_eq!(rule_title(&rule), "Untitled redirect");
    }
}
