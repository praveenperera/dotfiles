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

            CREATE INDEX IF NOT EXISTS papers_fts ON papers USING fts (title, authors, full_text);

            CREATE TABLE IF NOT EXISTS paper_tags (
                paper_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (paper_id, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_paper_tags_tag ON paper_tags(tag);",
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

    pub async fn update_full_text(&self, doi: &str, text: &str, text_length: i64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE papers SET full_text = ?1, text_length = ?2 WHERE doi = ?3",
                [
                    Value::from(text),
                    Value::from(text_length),
                    Value::from(doi),
                ],
            )
            .await
            .map_err(|e| eyre!("update error: {e}"))?;

        Ok(())
    }

    pub async fn remove(&self, doi: &str) -> Result<Option<LocalPaper>> {
        let Some(paper) = self.find_by_doi(doi).await? else {
            return Ok(None);
        };

        self.conn
            .execute(
                "DELETE FROM paper_tags WHERE paper_id = (SELECT id FROM papers WHERE doi = ?1)",
                [Value::from(doi)],
            )
            .await
            .map_err(|e| eyre!("delete tags error: {e}"))?;

        // turso FTS indexes auto-update on DELETE
        self.conn
            .execute("DELETE FROM papers WHERE doi = ?1", [Value::from(doi)])
            .await
            .map_err(|e| eyre!("delete error: {e}"))?;

        Ok(Some(paper))
    }

    pub async fn add_tags(&self, doi: &str, tags: &[String]) -> Result<()> {
        for tag in tags {
            let tag = tag.trim().to_lowercase();
            if tag.is_empty() {
                continue;
            }

            self.conn
                .execute(
                    "INSERT OR IGNORE INTO paper_tags (paper_id, tag)
                     SELECT id, ?2 FROM papers WHERE doi = ?1",
                    [Value::from(doi), Value::from(tag.as_str())],
                )
                .await
                .map_err(|e| eyre!("add tag error: {e}"))?;
        }
        Ok(())
    }

    pub async fn remove_tags(&self, doi: &str, tags: &[String]) -> Result<()> {
        for tag in tags {
            let tag = tag.trim().to_lowercase();

            self.conn
                .execute(
                    "DELETE FROM paper_tags
                     WHERE paper_id = (SELECT id FROM papers WHERE doi = ?1) AND tag = ?2",
                    [Value::from(doi), Value::from(tag.as_str())],
                )
                .await
                .map_err(|e| eyre!("remove tag error: {e}"))?;
        }
        Ok(())
    }

    pub async fn get_tags(&self, doi: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT tag FROM paper_tags
                 WHERE paper_id = (SELECT id FROM papers WHERE doi = ?1)
                 ORDER BY tag",
            )
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query([Value::from(doi)])
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? {
            tags.push(get_text(&row, 0)?);
        }

        Ok(tags)
    }

    pub async fn list_all_tags(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag, COUNT(*) as count FROM paper_tags GROUP BY tag ORDER BY tag")
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query(())
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? {
            let tag = get_text(&row, 0)?;
            let count = row.get::<i64>(1).map_err(|e| eyre!("count error: {e}"))?;
            tags.push((tag, count));
        }

        Ok(tags)
    }

    pub async fn list_by_tag(&self, tag: &str) -> Result<Vec<LocalPaper>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.doi, p.title, p.authors, p.year, p.pdf_path, p.source,
                        p.downloaded_at, p.file_size, p.text_length, p.full_text
                 FROM papers p
                 JOIN paper_tags pt ON p.id = pt.paper_id
                 WHERE pt.tag = ?1
                 ORDER BY p.downloaded_at DESC",
            )
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query([Value::from(tag)])
            .await
            .map_err(|e| eyre!("query error: {e}"))?;

        let mut papers = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| eyre!("row error: {e}"))? {
            papers.push(row_to_paper(&row)?);
        }

        Ok(papers)
    }

    pub async fn search_by_tag(
        &self,
        query: &str,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<SearchResult>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.doi, p.title, p.authors, p.year,
                        fts_highlight(p.title, '»', '«', ?1) as highlighted,
                        fts_score(p.title, p.authors, p.full_text, ?1) as score
                 FROM papers p
                 JOIN paper_tags pt ON p.id = pt.paper_id
                 WHERE pt.tag = ?3 AND fts_match(p.title, p.authors, p.full_text, ?1)
                 ORDER BY score DESC
                 LIMIT ?2",
            )
            .await
            .map_err(|e| eyre!("prepare error: {e}"))?;

        let mut rows = stmt
            .query([
                Value::from(query),
                Value::from(limit as i64),
                Value::from(tag),
            ])
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
