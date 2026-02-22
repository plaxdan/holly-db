use crate::db::HollyDb;
use crate::error::Result;
use crate::nodes::Node;
use rusqlite::params;

/// A search result with optional relevance score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub node: Node,
    pub score: f64,
}

/// Options for search.
#[derive(Debug, Default)]
pub struct SearchOptions {
    pub node_type: Option<String>,
    pub repo: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub limit: Option<u32>,
}

impl HollyDb {
    /// Full-text search with 3-tier fallback:
    /// 1. Raw FTS5 query
    /// 2. Quoted phrase fallback
    /// 3. LIKE %query% fallback
    pub fn fts_search(&self, query: &str, opts: SearchOptions) -> Result<Vec<SearchResult>> {
        let limit = opts.limit.unwrap_or(20);

        // Tier 1: raw FTS5
        if let Ok(results) = self.fts_search_raw(query, &opts, limit) {
            if !results.is_empty() {
                return Ok(results);
            }
        }

        // Tier 2: quoted phrase
        let quoted = format!("\"{}\"", query.replace('"', "\"\""));
        if let Ok(results) = self.fts_search_raw(&quoted, &opts, limit) {
            if !results.is_empty() {
                return Ok(results);
            }
        }

        // Tier 3: LIKE fallback
        self.like_search(query, &opts, limit)
    }

    fn fts_search_raw(
        &self,
        fts_query: &str,
        opts: &SearchOptions,
        limit: u32,
    ) -> Result<Vec<SearchResult>> {
        let mut sql = "
            SELECT n.id, n.node_type, n.title, n.content, n.tags, n.repo, n.status,
                   n.source, n.agent, n.user, n.llm, n.created_at, n.updated_at,
                   bm25(knowledge_fts) as score
            FROM knowledge_fts
            JOIN knowledge_nodes n ON knowledge_fts.rowid = n.rowid
            WHERE knowledge_fts MATCH ?1"
            .to_string();

        let mut extra_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fts_query.to_string())];
        let mut idx = 2usize;

        if let Some(ref t) = opts.node_type {
            sql.push_str(&format!(" AND n.node_type=?{idx}"));
            extra_params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref r) = opts.repo {
            sql.push_str(&format!(" AND n.repo=?{idx}"));
            extra_params.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref s) = opts.status {
            sql.push_str(&format!(" AND n.status=?{idx}"));
            extra_params.push(Box::new(s.clone()));
            idx += 1;
        }
        if let Some(ref src) = opts.source {
            sql.push_str(&format!(" AND n.source=?{idx}"));
            extra_params.push(Box::new(src.clone()));
            idx += 1;
        }

        sql.push_str(&format!(" ORDER BY score LIMIT ?{idx}"));
        extra_params.push(Box::new(limit));

        let refs: Vec<&dyn rusqlite::ToSql> = extra_params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;

        let results = stmt
            .query_map(refs.as_slice(), |row| {
                let score: f64 = row.get(13)?;
                let content_str: String = row.get(3)?;
                let tags_str: String = row.get(4)?;
                let content = serde_json::from_str(&content_str).unwrap_or_default();
                let tags = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(SearchResult {
                    node: Node {
                        id: row.get(0)?,
                        node_type: row.get(1)?,
                        title: row.get(2)?,
                        content,
                        tags,
                        repo: row.get(5)?,
                        status: row.get(6)?,
                        source: row.get(7)?,
                        agent: row.get(8)?,
                        user: row.get(9)?,
                        llm: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                    },
                    score: -score, // bm25 returns negative values; negate for ascending relevance
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn like_search(
        &self,
        query: &str,
        opts: &SearchOptions,
        limit: u32,
    ) -> Result<Vec<SearchResult>> {
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let mut sql = "
            SELECT id, node_type, title, content, tags, repo, status, source,
                   agent, user, llm, created_at, updated_at
            FROM knowledge_nodes
            WHERE (title LIKE ?1 ESCAPE '\\' OR content LIKE ?1 ESCAPE '\\')"
            .to_string();

        let mut extra_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(pattern)];
        let mut idx = 2usize;

        if let Some(ref t) = opts.node_type {
            sql.push_str(&format!(" AND node_type=?{idx}"));
            extra_params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref r) = opts.repo {
            sql.push_str(&format!(" AND repo=?{idx}"));
            extra_params.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref s) = opts.status {
            sql.push_str(&format!(" AND status=?{idx}"));
            extra_params.push(Box::new(s.clone()));
            idx += 1;
        }

        sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT ?{idx}"));
        extra_params.push(Box::new(limit));

        let refs: Vec<&dyn rusqlite::ToSql> = extra_params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;

        let results = stmt
            .query_map(refs.as_slice(), |row| {
                let content_str: String = row.get(3)?;
                let tags_str: String = row.get(4)?;
                let content = serde_json::from_str(&content_str).unwrap_or_default();
                let tags = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(SearchResult {
                    node: Node {
                        id: row.get(0)?,
                        node_type: row.get(1)?,
                        title: row.get(2)?,
                        content,
                        tags,
                        repo: row.get(5)?,
                        status: row.get(6)?,
                        source: row.get(7)?,
                        agent: row.get(8)?,
                        user: row.get(9)?,
                        llm: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                    },
                    score: 0.5, // Uniform score for LIKE results
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Vector similarity search (requires sqlite-vec).
    /// Returns nodes sorted by cosine similarity to the query embedding.
    pub fn vec_search(
        &self,
        embedding: &[f32],
        limit: u32,
        exclude_id: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        if !self.vec_available {
            return Ok(Vec::new());
        }

        let bytes = floats_to_bytes(embedding);
        let limit_i = limit as i64;

        let sql = if exclude_id.is_some() {
            "SELECT kv.id, kv.distance
             FROM knowledge_vec kv
             WHERE kv.embedding MATCH ?1 AND k=?2 AND kv.id != ?3
             ORDER BY kv.distance"
        } else {
            "SELECT kv.id, kv.distance
             FROM knowledge_vec kv
             WHERE kv.embedding MATCH ?1 AND k=?2
             ORDER BY kv.distance"
        };

        let mut stmt = self.conn.prepare(sql)?;

        let id_results: Vec<(String, f64)> = if let Some(excl) = exclude_id {
            stmt.query_map(params![bytes, limit_i, excl], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![bytes, limit_i], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let mut results = Vec::new();
        for (id, distance) in id_results {
            if let Ok(node) = self.get_node(&id) {
                let score = 1.0 / (1.0 + distance);
                results.push(SearchResult { node, score });
            }
        }
        Ok(results)
    }

    /// Upsert a node's embedding into the vector table.
    pub fn vec_upsert(&self, id: &str, embedding: &[f32]) -> Result<()> {
        if !self.vec_available {
            return Ok(());
        }
        let bytes = floats_to_bytes(embedding);
        self.conn.execute(
            "INSERT OR REPLACE INTO knowledge_vec(id, embedding) VALUES (?1, ?2)",
            params![id, bytes],
        )?;
        Ok(())
    }

    /// Hybrid search: combine FTS and vector results.
    /// If embeddings are provided, merges both ranked lists.
    pub fn hybrid_search(
        &self,
        query: &str,
        embedding: Option<&[f32]>,
        opts: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let limit = opts.limit.unwrap_or(20);

        let fts_results = self.fts_search(query, SearchOptions {
            node_type: opts.node_type.clone(),
            repo: opts.repo.clone(),
            status: opts.status.clone(),
            source: opts.source.clone(),
            limit: Some(limit * 2),
        })?;

        let vec_results = if let Some(emb) = embedding {
            self.vec_search(emb, limit * 2, None)?
        } else {
            Vec::new()
        };

        if vec_results.is_empty() {
            return Ok(fts_results.into_iter().take(limit as usize).collect());
        }

        // Reciprocal rank fusion
        use std::collections::HashMap;
        let mut scores: HashMap<String, f64> = HashMap::new();

        for (rank, r) in fts_results.iter().enumerate() {
            let rrf = 1.0 / (60.0 + (rank + 1) as f64);
            *scores.entry(r.node.id.clone()).or_default() += rrf;
        }
        for (rank, r) in vec_results.iter().enumerate() {
            let rrf = 1.0 / (60.0 + (rank + 1) as f64);
            *scores.entry(r.node.id.clone()).or_default() += rrf;
        }

        // Collect unique nodes
        let mut seen = std::collections::HashSet::new();
        let mut merged: Vec<SearchResult> = Vec::new();

        for r in fts_results.iter().chain(vec_results.iter()) {
            if seen.insert(r.node.id.clone()) {
                let score = *scores.get(&r.node.id).unwrap_or(&r.score);
                merged.push(SearchResult {
                    node: r.node.clone(),
                    score,
                });
            }
        }

        merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        merged.truncate(limit as usize);
        Ok(merged)
    }

    /// Find nodes semantically similar to a given node, with optional type filter.
    pub fn find_similar(&self, node_id: &str, limit: u32, node_type: Option<&str>) -> Result<Vec<SearchResult>> {
        if !self.vec_available {
            return Ok(Vec::new());
        }

        // Get the stored embedding
        let bytes: Option<Vec<u8>> = self.conn.query_row(
            "SELECT embedding FROM knowledge_vec WHERE id=?1",
            params![node_id],
            |row| row.get(0),
        ).ok();

        let Some(bytes) = bytes else {
            return Ok(Vec::new());
        };

        let embedding = bytes_to_floats(&bytes);
        // Fetch extra results so we can filter by type
        let fetch_limit = if node_type.is_some() { limit * 3 } else { limit };
        let results = self.vec_search(&embedding, fetch_limit, Some(node_id))?;

        if let Some(nt) = node_type {
            Ok(results.into_iter().filter(|r| r.node.node_type == nt).take(limit as usize).collect())
        } else {
            Ok(results.into_iter().take(limit as usize).collect())
        }
    }
}

fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HollyDb;
    use crate::nodes::CreateNodeInput;

    fn test_db() -> HollyDb {
        HollyDb::open_in_memory().unwrap()
    }

    fn seed(db: &HollyDb) {
        for (t, title) in &[
            ("decision", "Use SQLite for storage"),
            ("constraint", "Java 17 required for build"),
            ("idea", "Add dark mode support"),
            ("error", "Null pointer exception in login"),
        ] {
            db.create_node(CreateNodeInput {
                node_type: t.to_string(),
                title: title.to_string(),
                ..Default::default()
            })
            .unwrap();
        }
    }

    #[test]
    fn test_fts_basic() {
        let db = test_db();
        seed(&db);

        let results = db
            .fts_search("SQLite", SearchOptions::default())
            .unwrap();
        assert!(!results.is_empty());
        assert!(results[0].node.title.contains("SQLite"));
    }

    #[test]
    fn test_fts_fallback_special_chars() {
        let db = test_db();
        seed(&db);

        // Query with special FTS5 chars should fall back gracefully
        let results = db
            .fts_search("Java AND:invalid", SearchOptions::default())
            .unwrap();
        // Should not panic; may return 0 or some results
        let _ = results;
    }

    #[test]
    fn test_fts_type_filter() {
        let db = test_db();
        seed(&db);

        let results = db
            .fts_search(
                "mode",
                SearchOptions {
                    node_type: Some("idea".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node.node_type, "idea");
    }

    #[test]
    fn test_like_fallback() {
        let db = test_db();
        seed(&db);

        // Query that FTS won't match (very short or unusual)
        let results = db
            .fts_search("dark", SearchOptions::default())
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hybrid_without_embeddings() {
        let db = test_db();
        seed(&db);

        let results = db
            .hybrid_search("SQLite", None, SearchOptions::default())
            .unwrap();
        assert!(!results.is_empty());
    }
}
