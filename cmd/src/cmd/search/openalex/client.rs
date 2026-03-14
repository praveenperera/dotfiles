use eyre::{eyre, Result};

use super::types::*;
use super::CommonFilters;
use crate::cmd::search::config;

const API_BASE: &str = "https://api.openalex.org";
const USER_AGENT: &str =
    "cmd-search/1.0 (github.com/praveenperera/dotfiles; mailto:praveen@praveenperera.com)";

pub struct OpenAlexClient {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl OpenAlexClient {
    pub fn new() -> Result<Self> {
        let api_key = config::get_api_key("oa-api-key", "OPENALEX_API_KEY");

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        Ok(Self { client, api_key })
    }

    fn auth_param(&self) -> String {
        match &self.api_key {
            Some(key) => format!("api_key={key}"),
            None => String::new(),
        }
    }

    fn join_params(&self, url: &str) -> String {
        let auth = self.auth_param();
        if auth.is_empty() {
            return url.to_string();
        }

        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}{auth}")
    }

    async fn get(&self, url: &str) -> Result<reqwest::Response> {
        let url = self.join_params(url);
        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let msg = if self.api_key.is_none() {
                "rate limited by OpenAlex — set OPENALEX_API_KEY for higher limits"
            } else {
                "rate limited by OpenAlex (authenticated)"
            };
            return Err(eyre!(msg));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(eyre!("OpenAlex API error {status}: {body}"));
        }

        Ok(response)
    }

    pub async fn search_works(&self, params: &WorkSearchParams) -> Result<ListResponse<Work>> {
        let search_param = if params.semantic {
            format!("search.semantic={}", urlencoded(&params.query))
        } else {
            format!("search={}", urlencoded(&params.query))
        };

        let mut url = format!(
            "{API_BASE}/works?{search_param}&per_page={}&page={}",
            params.filters.limit, params.filters.offset
        );

        let mut filter_parts: Vec<String> = Vec::new();

        if let Some(year) = &params.filters.year {
            if let Some(range) = parse_year_filter(year) {
                filter_parts.push(range);
            }
        }
        if let Some(min) = params.filters.min_citations {
            filter_parts.push(format!("cited_by_count:>{min}"));
        }
        if params.filters.open_access {
            filter_parts.push("is_oa:true".to_string());
        }
        if let Some(wt) = &params.work_type {
            filter_parts.push(format!("type:{wt}"));
        }
        if let Some(inst) = &params.institution {
            filter_parts.push(format!("authorships.institutions.id:{inst}"));
        }
        if let Some(src) = &params.source {
            filter_parts.push(format!("primary_location.source.id:{src}"));
        }
        if let Some(t) = &params.topic {
            filter_parts.push(format!("topics.id:{t}"));
        }
        if let Some(extra) = &params.extra_filter {
            filter_parts.push(extra.clone());
        }

        if !filter_parts.is_empty() {
            url.push_str(&format!("&filter={}", filter_parts.join(",")));
        }
        if let Some(sort) = &params.sort {
            url.push_str(&format!("&sort={sort}"));
        }

        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn work_detail(&self, id: &str) -> Result<Work> {
        let resolved = resolve_oa_id(id);
        let url = format!("{API_BASE}/works/{resolved}");
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn citations(&self, id: &str, filters: &CommonFilters) -> Result<ListResponse<Work>> {
        let oa_id = extract_oa_short_id(id);
        let url = format!(
            "{API_BASE}/works?filter=cited_by:{oa_id}&per_page={}&page={}",
            filters.limit, filters.offset
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn references(
        &self,
        id: &str,
        filters: &CommonFilters,
    ) -> Result<ListResponse<Work>> {
        let oa_id = extract_oa_short_id(id);
        let url = format!(
            "{API_BASE}/works?filter=cites:{oa_id}&per_page={}&page={}",
            filters.limit, filters.offset
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn author_search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<ListResponse<OAAuthorDetail>> {
        let url = format!(
            "{API_BASE}/authors?search={}&per_page={limit}",
            urlencoded(query)
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn author_detail(&self, id: &str) -> Result<OAAuthorDetail> {
        let resolved = resolve_oa_id(id);
        let url = format!("{API_BASE}/authors/{resolved}");
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn search_institutions(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<ListResponse<Institution>> {
        let url = format!(
            "{API_BASE}/institutions?search={}&per_page={limit}",
            urlencoded(query)
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn search_topics(&self, query: &str, limit: u32) -> Result<ListResponse<Topic>> {
        let url = format!(
            "{API_BASE}/topics?search={}&per_page={limit}",
            urlencoded(query)
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn group_by(
        &self,
        field: &str,
        filter: &Option<String>,
    ) -> Result<ListResponse<Work>> {
        let mut url = format!("{API_BASE}/works?group_by={field}");
        if let Some(f) = filter {
            url.push_str(&format!("&filter={f}"));
        }
        Ok(self.get(&url).await?.json().await?)
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('"', "%22")
        .replace('&', "%26")
}

/// Convert year filter like "2020", "2020-2024", "2020-" into OpenAlex filter syntax
fn parse_year_filter(year: &str) -> Option<String> {
    if let Some(range) = year.strip_suffix('-') {
        Some(format!("from_publication_date:{range}-01-01"))
    } else if year.contains('-') {
        let parts: Vec<&str> = year.splitn(2, '-').collect();
        Some(format!(
            "from_publication_date:{}-01-01,to_publication_date:{}-12-31",
            parts[0], parts[1]
        ))
    } else {
        Some(format!("publication_year:{year}"))
    }
}

/// Resolve user-provided ID to an OpenAlex-compatible form
fn resolve_oa_id(id: &str) -> String {
    if id.starts_with('W') || id.starts_with('A') || id.starts_with('I') || id.starts_with('T') {
        id.to_string()
    } else if id.starts_with("https://doi.org/") || id.starts_with("10.") {
        let doi = id.strip_prefix("https://doi.org/").unwrap_or(id);
        format!("https://doi.org/{doi}")
    } else {
        id.to_string()
    }
}

/// Extract short OpenAlex ID (W1234) from full URL or bare ID
fn extract_oa_short_id(id: &str) -> String {
    if let Some(short) = id.strip_prefix("https://openalex.org/") {
        short.to_string()
    } else {
        id.to_string()
    }
}
