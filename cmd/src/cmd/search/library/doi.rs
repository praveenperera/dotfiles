use eyre::{bail, Result};
use sha2::{Digest, Sha256};

/// Normalize a DOI: strip common prefixes, trim, lowercase, validate
pub fn normalize_doi(input: &str) -> Result<String> {
    let doi = input
        .trim()
        .strip_prefix("doi:")
        .or_else(|| input.trim().strip_prefix("DOI:"))
        .or_else(|| input.trim().strip_prefix("https://doi.org/"))
        .or_else(|| input.trim().strip_prefix("http://doi.org/"))
        .or_else(|| input.trim().strip_prefix("https://dx.doi.org/"))
        .or_else(|| input.trim().strip_prefix("http://dx.doi.org/"))
        .unwrap_or(input.trim())
        .to_lowercase();

    if !doi.starts_with("10.") || !doi.contains('/') {
        bail!("invalid DOI: must start with '10.' and contain '/' — got: {doi}");
    }

    Ok(doi)
}

/// Hash a canonical DOI to a safe filename
pub fn doi_to_filename(doi: &str) -> String {
    let hash = Sha256::digest(doi.as_bytes());
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_strips_url_prefix() {
        assert_eq!(
            normalize_doi("https://doi.org/10.1145/3442188.3445922").unwrap(),
            "10.1145/3442188.3445922"
        );
    }

    #[test]
    fn test_normalize_strips_doi_prefix() {
        assert_eq!(
            normalize_doi("doi:10.1145/3442188.3445922").unwrap(),
            "10.1145/3442188.3445922"
        );
    }

    #[test]
    fn test_normalize_strips_dx_prefix() {
        assert_eq!(
            normalize_doi("http://dx.doi.org/10.1145/3442188.3445922").unwrap(),
            "10.1145/3442188.3445922"
        );
    }

    #[test]
    fn test_normalize_lowercases() {
        assert_eq!(
            normalize_doi("10.1038/S41586-020-2308-7").unwrap(),
            "10.1038/s41586-020-2308-7"
        );
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        assert_eq!(
            normalize_doi("  10.1145/3442188.3445922  ").unwrap(),
            "10.1145/3442188.3445922"
        );
    }

    #[test]
    fn test_normalize_rejects_invalid() {
        assert!(normalize_doi("not-a-doi").is_err());
        assert!(normalize_doi("10.1234").is_err());
    }

    #[test]
    fn test_doi_to_filename_deterministic() {
        let a = doi_to_filename("10.1145/3442188.3445922");
        let b = doi_to_filename("10.1145/3442188.3445922");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }
}
