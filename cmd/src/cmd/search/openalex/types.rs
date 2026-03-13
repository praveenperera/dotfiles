use serde::{Deserialize, Serialize};

use super::CommonFilters;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub meta: Option<Meta>,
    pub results: Option<Vec<T>>,
    pub group_by: Option<Vec<GroupByResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub count: Option<u64>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub id: Option<String>,
    pub doi: Option<String>,
    pub title: Option<String>,
    pub publication_year: Option<u32>,
    pub publication_date: Option<String>,
    pub cited_by_count: Option<u64>,
    #[serde(rename = "type")]
    pub work_type: Option<String>,
    pub is_oa: Option<bool>,
    pub authorships: Option<Vec<Authorship>>,
    pub primary_location: Option<Location>,
    pub open_access: Option<OpenAccess>,
    pub topics: Option<Vec<TopicTag>>,
    #[serde(rename = "abstract_inverted_index")]
    pub abstract_index: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorship {
    pub author: Option<OAAuthor>,
    pub institutions: Option<Vec<Institution>>,
    pub author_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAAuthor {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub orcid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAAuthorDetail {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub orcid: Option<String>,
    pub works_count: Option<u64>,
    pub cited_by_count: Option<u64>,
    pub summary_stats: Option<SummaryStats>,
    pub affiliations: Option<Vec<Affiliation>>,
    pub last_known_institutions: Option<Vec<Institution>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStats {
    pub h_index: Option<u32>,
    pub i10_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affiliation {
    pub institution: Option<Institution>,
    pub years: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institution {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub country_code: Option<String>,
    #[serde(rename = "type")]
    pub institution_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub source: Option<Source>,
    pub is_oa: Option<bool>,
    pub landing_page_url: Option<String>,
    pub pdf_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub issn_l: Option<String>,
    #[serde(rename = "type")]
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAccess {
    pub is_oa: Option<bool>,
    pub oa_status: Option<String>,
    pub oa_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicTag {
    pub id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupByResult {
    pub key: Option<String>,
    pub key_display_name: Option<String>,
    pub count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub works_count: Option<u64>,
    pub cited_by_count: Option<u64>,
    pub description: Option<String>,
}

pub struct WorkSearchParams {
    pub query: String,
    pub semantic: bool,
    pub filters: CommonFilters,
    pub sort: Option<String>,
    pub extra_filter: Option<String>,
    pub work_type: Option<String>,
    pub institution: Option<String>,
    pub source: Option<String>,
    pub topic: Option<String>,
}
