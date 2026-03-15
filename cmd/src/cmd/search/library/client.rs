use eyre::{eyre, Result};
use std::path::Path;

use crate::cmd::search::config;
use crate::cmd::search::openalex::client::OpenAlexClient;
use crate::cmd::search::s2::client::S2Client;

const USER_AGENT: &str = "cmd-search/1.0 (github.com/praveenperera/dotfiles)";

/// Download a PDF for the given DOI, trying OA sources first then Sci-Hub
pub async fn download_pdf(doi: &str, dest: &Path) -> Result<String> {
    // try S2 open access URL
    if let Some(source) = try_s2_oa(doi, dest).await {
        return Ok(source);
    }

    // try OpenAlex pdf_url
    if let Some(source) = try_openalex_pdf(doi, dest).await {
        return Ok(source);
    }

    // try Sci-Hub
    if let Some(source) = try_scihub(doi, dest).await {
        return Ok(source);
    }

    Err(eyre!(
        "could not download PDF for {doi} — no open access URL found and Sci-Hub failed"
    ))
}

async fn try_s2_oa(doi: &str, dest: &Path) -> Option<String> {
    let client = S2Client::new().ok()?;
    let paper = client.paper_detail(&format!("DOI:{doi}")).await.ok()?;

    let url = paper.open_access_pdf?.url?;
    if download_url(&url, dest).await.is_ok() {
        return Some("oa-s2".to_string());
    }

    None
}

async fn try_openalex_pdf(doi: &str, dest: &Path) -> Option<String> {
    let client = OpenAlexClient::new().ok()?;
    let work = client
        .work_detail(&format!("https://doi.org/{doi}"))
        .await
        .ok()?;

    let pdf_url = work.primary_location?.pdf_url?;
    if download_url(&pdf_url, dest).await.is_ok() {
        return Some("oa-openalex".to_string());
    }

    None
}

async fn try_scihub(doi: &str, dest: &Path) -> Option<String> {
    let base_url = config::get_scihub_url()?;
    let page_url = format!("{base_url}/{doi}");

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .ok()?;

    let html = client.get(&page_url).send().await.ok()?.text().await.ok()?;
    let pdf_url = parse_scihub_pdf_url(&html, &base_url)?;

    // validate URL scheme
    if !pdf_url.starts_with("http://") && !pdf_url.starts_with("https://") {
        return None;
    }

    if download_url(&pdf_url, dest).await.is_ok() {
        return Some("scihub".to_string());
    }

    None
}

fn parse_scihub_pdf_url(html: &str, base_url: &str) -> Option<String> {
    let doc = scraper::Html::parse_document(html);

    let selectors = [
        ("meta[name='citation_pdf_url']", "content"),
        ("iframe#pdf", "src"),
        ("embed[src]", "src"),
        (r#"object[type="application/pdf"]"#, "data"),
        (r#"a[href$=".pdf"]"#, "href"),
    ];

    for (sel, attr) in selectors {
        let selector = scraper::Selector::parse(sel).ok()?;
        if let Some(el) = doc.select(&selector).next() {
            if let Some(val) = el.value().attr(attr) {
                // strip fragment (e.g. #navpanes=0)
                let val = val.split('#').next().unwrap_or(val);
                return Some(normalize_scihub_url(val, base_url));
            }
        }
    }

    None
}

fn normalize_scihub_url(url: &str, base_url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("{base_url}{url}")
    } else {
        url.to_string()
    }
}

async fn download_url(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        return Err(eyre!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await?;

    // validate PDF magic bytes
    if bytes.len() < 4 || &bytes[..4] != b"%PDF" {
        return Err(eyre!("downloaded file is not a valid PDF"));
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // write to temp file first then rename for atomicity
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, dest)?;

    Ok(())
}
