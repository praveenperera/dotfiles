use eyre::{eyre, Result};
use std::path::Path;
use turso::{Builder, Connection, Value};

use super::types::{LocalPaper, SearchResult};

pub struct PaperDb {
    conn: Connection,
}

impl PaperDb {
    pub async fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Builder::new_local(db_path.to_str().ok_or_else(|| eyre!("invalid db path"))?)
            .experimental_index_method(true)
            .build()
            .await
            .map_err(|e| eyre!("failed to open database: {e}"))?;

        let conn = db.connect().map_err(|e| eyre!("failed to connect: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS papers (
                id INTEGER PRIMARY KEY,
                doi TEXT UNIQUE NOT NULL,
                title TEXT,
                authors TEXT,
                year INTEGER,
                pdf_path TEXT NOT NULL,
                source TEXT NOT NULL,
                downloaded_at TEXT NOT NULL,
                file_size INTEGER,
                text_length INTEGER,
                full_text TEXT
            );

            CREATE INDEX IF NOT EXISTS papers_fts ON papers USING fts (title, authors, full_text);",
        )
        .await
        .map_err(|e| eyre!("failed to init schema: {e}"))?;

        Ok(Self { conn })
    }

    pub async fn find_by_doi(&self, doi: &str) -> Result<Option<LocalPaper>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, doi, title, authors, year, pdf_path, source, downloaded_at, file_size, text_length, full_text FROM papers WHERE doi = ?1")
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query([Value::from(doi)])
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? else {
            return Ok(None);
        };

        Ok(Some(row_to_paper(&row)?))
    }

    pub async fn insert(&self, paper: &LocalPaper) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO papers (doi, title, authors, year, pdf_path, source, downloaded_at, file_size, text_length, full_text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                [
                    Value::from(paper.doi.as_str()),
                    opt_text(&paper.title),
                    opt_text(&paper.authors),
                    paper.year.map(Value::from).unwrap_or(Value::Null),
                    Value::from(paper.pdf_path.as_str()),
                    Value::from(paper.source.as_str()),
                    Value::from(paper.downloaded_at.as_str()),
                    paper.file_size.map(Value::from).unwrap_or(Value::Null),
                    paper.text_length.map(Value::from).unwrap_or(Value::Null),
                    opt_text(&paper.full_text),
                ],
            )
            .await
            .map_err(|e| eyre!("insert error: {e}"))?;

        Ok(())
    }

    pub async fn list_all(&self) -> Result<Vec<LocalPaper>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, doi, title, authors, year, pdf_path, source, downloaded_at, file_size, text_length, full_text FROM papers ORDER BY downloaded_at DESC")
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query(())
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let mut papers = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? {
            papers.push(row_to_paper(&row)?);
        }

        Ok(papers)
    }

    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT doi, title, authors, year,
                        fts_highlight(title, '»', '«', ?1) as highlighted,
                        fts_score(title, authors, full_text, ?1) as score
                 FROM papers
                 WHERE fts_match(title, authors, full_text, ?1)
                 ORDER BY score DESC
                 LIMIT ?2",
            )
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query([Value::from(query), Value::from(limit as i64)])
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? {
            results.push(SearchResult {
                doi: get_text(&row, 0)?,
                title: get_opt_text(&row, 1),
                authors: get_opt_text(&row, 2),
                year: get_opt_int(&row, 3),
                snippet: get_opt_text(&row, 4).unwrap_or_default(),
                rank: row.get::<f64>(5).unwrap_or(0.0),
            });
        }

        Ok(results)
    }

    pub async fn remove(&self, doi: &str) -> Result<Option<LocalPaper>> {
        let Some(paper) = self.find_by_doi(doi).await? else {
            return Ok(None);
        };

        // turso FTS indexes auto-update on DELETE
        self.conn
            .execute("DELETE FROM papers WHERE doi = ?1", [Value::from(doi)])
            .await
            .map_err(|e| eyre!("delete error: {e}"))?;

        Ok(Some(paper))
    }
}

fn opt_text(s: &Option<String>) -> Value {
    match s {
        Some(v) => Value::from(v.as_str()),
        None => Value::Null,
    }
}

fn get_text(row: &turso::Row, idx: usize) -> Result<String> {
    row.get::<String>(idx)
        .map_err(|e| eyre!("column {idx} error: {e}"))
}

fn get_opt_text(row: &turso::Row, idx: usize) -> Option<String> {
    match row.get_value(idx) {
        Ok(Value::Text(s)) => Some(s),
        _ => None,
    }
}

fn get_opt_int(row: &turso::Row, idx: usize) -> Option<i64> {
    match row.get_value(idx) {
        Ok(Value::Integer(n)) => Some(n),
        _ => None,
    }
}

fn row_to_paper(row: &turso::Row) -> Result<LocalPaper> {
    Ok(LocalPaper {
        id: row
            .get::<i64>(0)
            .map_err(|e| eyre!("id column error: {e}"))?,
        doi: get_text(row, 1)?,
        title: get_opt_text(row, 2),
        authors: get_opt_text(row, 3),
        year: get_opt_int(row, 4),
        pdf_path: get_text(row, 5)?,
        source: get_text(row, 6)?,
        downloaded_at: get_text(row, 7)?,
        file_size: get_opt_int(row, 8),
        text_length: get_opt_int(row, 9),
        full_text: get_opt_text(row, 10),
    })
}
