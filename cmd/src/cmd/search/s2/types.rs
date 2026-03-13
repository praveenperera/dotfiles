use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSearchResponse {
    pub total: Option<u64>,
    pub offset: Option<u32>,
    pub next: Option<u32>,
    pub data: Option<Vec<Paper>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    pub paper_id: Option<String>,
    pub title: Option<String>,
    pub year: Option<u32>,
    pub authors: Option<Vec<Author>>,
    pub venue: Option<String>,
    pub citation_count: Option<u64>,
    pub external_ids: Option<ExternalIds>,
    pub r#abstract: Option<String>,
    pub open_access_pdf: Option<OpenAccessPdf>,
    pub publication_types: Option<Vec<String>>,
    pub fields_of_study: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIds {
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    #[serde(rename = "ArXiv")]
    pub arxiv: Option<String>,
    #[serde(rename = "PubMed")]
    pub pubmed: Option<String>,
    #[serde(rename = "CorpusId")]
    pub corpus_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAccessPdf {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub author_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorDetail {
    pub author_id: Option<String>,
    pub name: Option<String>,
    pub h_index: Option<u32>,
    pub paper_count: Option<u32>,
    pub citation_count: Option<u64>,
    pub affiliations: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorSearchResponse {
    pub total: Option<u64>,
    pub data: Option<Vec<AuthorDetail>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationResult {
    pub contexts: Option<Vec<String>>,
    pub intents: Option<Vec<String>>,
    pub is_influential: Option<bool>,
    pub citing_paper: Option<Paper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceResult {
    pub cited_paper: Option<Paper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationResponse {
    pub offset: Option<u32>,
    pub next: Option<u32>,
    pub data: Option<Vec<CitationResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceResponse {
    pub offset: Option<u32>,
    pub next: Option<u32>,
    pub data: Option<Vec<ReferenceResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationResponse {
    pub recommended_papers: Option<Vec<Paper>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub snippet_text: Option<String>,
    pub paper: Option<Paper>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetResponse {
    pub data: Option<Vec<Snippet>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResponse {
    pub data: Option<Vec<Paper>>,
}
