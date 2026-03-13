use eyre::{eyre, Result};

use super::types::*;
use super::CommonFilters;

const API_BASE: &str = "https://api.semanticscholar.org/graph/v1";
const RECS_BASE: &str = "https://api.semanticscholar.org/recommendations/v1";
const USER_AGENT: &str = "cmd-search/1.0 (github.com/praveenperera/dotfiles)";

pub const PAPER_FIELDS: &str = "paperId,title,year,authors,venue,citationCount,externalIds,abstract,openAccessPdf,publicationTypes,fieldsOfStudy";
const CITATION_FIELDS: &str = "contexts,intents,isInfluential,citingPaper.paperId,citingPaper.title,citingPaper.year,citingPaper.authors,citingPaper.venue,citingPaper.citationCount";
const REFERENCE_FIELDS: &str = "citedPaper.paperId,citedPaper.title,citedPaper.year,citedPaper.authors,citedPaper.venue,citedPaper.citationCount";

pub struct S2Client {
    client: reqwest::Client,
}

impl S2Client {
    pub fn new() -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Ok(key) = std::env::var("SEMANTIC_SCHOLAR_API_KEY") {
            headers.insert("x-api-key", key.parse()?);
        }

        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()?;

        Ok(Self { client })
    }

    async fn get(&self, url: &str) -> Result<reqwest::Response> {
        let response = self.client.get(url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let msg = if std::env::var("SEMANTIC_SCHOLAR_API_KEY").is_err() {
                "rate limited by Semantic Scholar — set SEMANTIC_SCHOLAR_API_KEY for higher limits"
            } else {
                "rate limited by Semantic Scholar (authenticated)"
            };
            return Err(eyre!(msg));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(eyre!("S2 API error {status}: {body}"));
        }

        Ok(response)
    }

    pub async fn search_papers(
        &self,
        query: &str,
        filters: &CommonFilters,
        venue: &Option<String>,
        pub_type: &Option<String>,
    ) -> Result<PaperSearchResponse> {
        let mut url = format!(
            "{API_BASE}/paper/search?query={}&fields={PAPER_FIELDS}&limit={}&offset={}",
            urlencoded(query),
            filters.limit,
            filters.offset
        );

        if let Some(year) = &filters.year {
            url.push_str(&format!("&year={year}"));
        }
        if let Some(field) = &filters.field {
            url.push_str(&format!("&fieldsOfStudy={}", urlencoded(field)));
        }
        if filters.open_access {
            url.push_str("&openAccessPdf");
        }
        if let Some(venue) = venue {
            url.push_str(&format!("&venue={}", urlencoded(venue)));
        }
        if let Some(pub_type) = pub_type {
            url.push_str(&format!("&publicationTypes={pub_type}"));
        }
        if let Some(min) = filters.min_citations {
            url.push_str(&format!("&minCitationCount={min}"));
        }

        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn paper_detail(&self, id: &str) -> Result<Paper> {
        let url = format!("{API_BASE}/paper/{id}?fields={PAPER_FIELDS}");
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn citations(&self, id: &str, filters: &CommonFilters) -> Result<CitationResponse> {
        let url = format!(
            "{API_BASE}/paper/{id}/citations?fields={CITATION_FIELDS}&limit={}&offset={}",
            filters.limit, filters.offset
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn references(&self, id: &str, filters: &CommonFilters) -> Result<ReferenceResponse> {
        let url = format!(
            "{API_BASE}/paper/{id}/references?fields={REFERENCE_FIELDS}&limit={}&offset={}",
            filters.limit, filters.offset
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn author_search(&self, query: &str) -> Result<AuthorSearchResponse> {
        let url = format!(
            "{API_BASE}/author/search?query={}&fields=name,hIndex,paperCount,citationCount,affiliations",
            urlencoded(query)
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn author_detail(&self, id: &str) -> Result<AuthorDetail> {
        let url = format!(
            "{API_BASE}/author/{id}?fields=name,hIndex,paperCount,citationCount,affiliations"
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn recommend(
        &self,
        id: &str,
        pool: &str,
        limit: u32,
    ) -> Result<RecommendationResponse> {
        let url = format!(
            "{RECS_BASE}/papers/forpaper/{id}?fields={PAPER_FIELDS}&limit={limit}&from={pool}"
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn snippets(&self, query: &str, limit: u32) -> Result<SnippetResponse> {
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/snippet/search?query={}&limit={limit}",
            urlencoded(query)
        );
        Ok(self.get(&url).await?.json().await?)
    }

    pub async fn title_match(&self, title: &str) -> Result<Paper> {
        let url = format!(
            "{API_BASE}/paper/search/match?query={}&fields={PAPER_FIELDS}",
            urlencoded(title)
        );
        let resp: MatchResponse = self.get(&url).await?.json().await?;
        resp.data
            .and_then(|d| d.into_iter().next())
            .ok_or_else(|| eyre!("no matching paper found"))
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('"', "%22")
        .replace('&', "%26")
}
