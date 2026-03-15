/// A paper stored in the local library
pub struct LocalPaper {
    #[allow(dead_code)]
    pub id: i64,
    pub doi: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub pdf_path: String,
    pub source: String,
    pub downloaded_at: String,
    pub file_size: Option<i64>,
    pub text_length: Option<i64>,
    pub full_text: Option<String>,
}

/// A search result from FTS5
pub struct SearchResult {
    pub doi: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub snippet: String,
    #[allow(dead_code)]
    pub rank: f64,
}
