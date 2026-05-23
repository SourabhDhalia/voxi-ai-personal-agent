//! Embedding store — RAG vector storage for semantic search.

use rusqlite::{params, Connection};
use serde_json::{json, Value};

const DEFAULT_EMBEDDING_MODEL: &str = "all-MiniLM-L6-v2";

pub struct EmbeddingStore {
    conn: Option<Connection>,
    knowledge_dbs: Vec<String>,
}

impl Default for EmbeddingStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingStore {
    pub fn new() -> Self {
        EmbeddingStore {
            conn: None,
            knowledge_dbs: vec![],
        }
    }

    pub fn initialize(&mut self, db_path: &str) -> bool {
        match super::sqlite::open_database(db_path) {
            Ok(conn) => {
                let _ = conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS embeddings (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        source TEXT NOT NULL,
                        chunk_text TEXT NOT NULL,
                        embedding BLOB,
                        embedding_dim INTEGER DEFAULT 0,
                        embedding_model TEXT DEFAULT '',
                        content_hash TEXT DEFAULT '',
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE INDEX IF NOT EXISTS idx_emb_source ON embeddings(source);",
                );
                let _ = Self::ensure_column(&conn, "embedding_dim", "INTEGER DEFAULT 0");
                let _ = Self::ensure_column(&conn, "embedding_model", "TEXT DEFAULT ''");
                let _ = Self::ensure_column(&conn, "content_hash", "TEXT DEFAULT ''");
                self.conn = Some(conn);
                true
            }
            Err(e) => {
                log::error!("EmbeddingStore: failed to open {}: {}", db_path, e);
                false
            }
        }
    }

    fn ensure_column(
        conn: &Connection,
        column_name: &str,
        column_def: &str,
    ) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(embeddings)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for column in columns {
            if column? == column_name {
                return Ok(());
            }
        }

        conn.execute_batch(&format!(
            "ALTER TABLE embeddings ADD COLUMN {} {}",
            column_name, column_def
        ))
    }

    fn content_hash(content: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut h);
        format!("{:016x}", h.finish())
    }

    fn encode_embedding_blob(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * std::mem::size_of::<f32>());
        for value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn decode_embedding_blob(blob: &[u8], expected_dim: usize) -> Option<Vec<f32>> {
        if blob.len() != expected_dim * std::mem::size_of::<f32>() {
            return None;
        }

        let mut embedding = Vec::with_capacity(expected_dim);
        for chunk in blob.chunks_exact(std::mem::size_of::<f32>()) {
            embedding.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Some(embedding)
    }

    pub fn register_knowledge_db(&mut self, path: &str) {
        self.knowledge_dbs.push(path.to_string());
    }

    pub fn get_pending_knowledge_count(&self) -> usize {
        self.knowledge_dbs.len()
    }

    pub fn detach_knowledge_dbs(&self) {
        // Detach any attached DBs to reclaim file cache
        if let Some(conn) = &self.conn {
            for (i, _) in self.knowledge_dbs.iter().enumerate() {
                let alias = format!("kb_{}", i);
                let _ = conn.execute_batch(&format!("DETACH DATABASE IF EXISTS {}", alias));
            }
        }
    }

    pub fn ingest(&self, source: &str, text: &str) -> Result<usize, String> {
        self.ingest_with_embeddings(source, text, |_| None)
    }

    pub fn ingest_with_embeddings<F>(
        &self,
        source: &str,
        text: &str,
        mut embed: F,
    ) -> Result<usize, String>
    where
        F: FnMut(&str) -> Option<Vec<f32>>,
    {
        let conn = self.conn.as_ref().ok_or("Not initialized")?;
        // Chunk text into ~500 char segments
        let chunks: Vec<&str> = text
            .as_bytes()
            .chunks(500)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        let mut count = 0;
        for chunk in &chunks {
            if chunk.trim().is_empty() {
                continue;
            }
            let embedding = embed(chunk);
            let (embedding_blob, embedding_dim, embedding_model) = match embedding {
                Some(values) if !values.is_empty() => (
                    Some(Self::encode_embedding_blob(&values)),
                    values.len() as i64,
                    DEFAULT_EMBEDDING_MODEL.to_string(),
                ),
                _ => (None, 0i64, String::new()),
            };
            let content_hash = Self::content_hash(chunk);
            conn.execute(
                "INSERT INTO embeddings
                    (source, chunk_text, embedding, embedding_dim, embedding_model, content_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    source,
                    chunk,
                    embedding_blob,
                    embedding_dim,
                    embedding_model,
                    content_hash
                ],
            )
            .map_err(|e| e.to_string())?;
            count += 1;
        }
        log::debug!(
            "EmbeddingStore: ingested {} chunks from '{}'",
            count,
            source
        );
        Ok(count)
    }

    pub fn search_by_embedding(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Vec<Value> {
        if query_embedding.is_empty() {
            return vec![];
        }

        let conn = match &self.conn {
            Some(c) => c,
            None => return vec![],
        };

        let mut attached_aliases = Vec::new();
        for (i, db_path) in self.knowledge_dbs.iter().take(10).enumerate() {
            let alias = format!("kb_{}", i);
            let safe_path = db_path.replace("'", "''");
            let attach_sql = format!("ATTACH DATABASE '{}' AS {}", safe_path, alias);
            if conn.execute_batch(&attach_sql).is_ok() {
                attached_aliases.push(alias);
            }
        }

        let mut sql_parts = vec![
            "SELECT source, chunk_text, embedding, embedding_dim FROM embeddings \
             WHERE embedding IS NOT NULL AND embedding_dim = ?1"
                .to_string(),
        ];
        for alias in &attached_aliases {
            sql_parts.push(format!(
                "SELECT source, chunk_text, embedding, embedding_dim FROM {}.embeddings \
                 WHERE embedding IS NOT NULL AND embedding_dim = ?1",
                alias
            ));
        }

        let full_sql = sql_parts.join(" UNION ALL ");
        let mut scored = Vec::new();
        if let Ok(mut stmt) = conn.prepare(&full_sql) {
            if let Ok(rows) = stmt.query_map(params![query_embedding.len() as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            }) {
                for row in rows.filter_map(|row| row.ok()) {
                    let (source, text, blob, dim) = row;
                    let Some(embedding) = Self::decode_embedding_blob(&blob, dim as usize) else {
                        continue;
                    };
                    let score: f32 = query_embedding
                        .iter()
                        .zip(embedding.iter())
                        .map(|(a, b)| a * b)
                        .sum();
                    if score >= threshold {
                        scored.push((score, source, text));
                    }
                }
            }
        }

        for alias in attached_aliases {
            let _ = conn.execute_batch(&format!("DETACH DATABASE {}", alias));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(top_k)
            .map(|(score, source, text)| {
                json!({
                    "source": source,
                    "text": text,
                    "score": score,
                })
            })
            .collect()
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<Value> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return vec![],
        };

        // Lazy-load knowledge databases via ATTACH
        let mut attached_aliases = Vec::new();
        for (i, db_path) in self.knowledge_dbs.iter().take(10).enumerate() {
            let alias = format!("kb_{}", i);
            let safe_path = db_path.replace("'", "''");
            let attach_sql = format!("ATTACH DATABASE '{}' AS {}", safe_path, alias);
            if let Err(e) = conn.execute_batch(&attach_sql) {
                log::warn!(
                    "EmbeddingStore: failed to attach knowledge DB {}: {}",
                    db_path,
                    e
                );
            } else {
                attached_aliases.push(alias);
            }
        }

        let pattern = format!("%{}%", query);
        let mut sql_parts =
            vec!["SELECT source, chunk_text FROM embeddings WHERE chunk_text LIKE ?1".to_string()];

        for alias in &attached_aliases {
            sql_parts.push(format!(
                "SELECT source, chunk_text FROM {}.embeddings WHERE chunk_text LIKE ?1",
                alias
            ));
        }

        let full_sql = format!("{} LIMIT ?2", sql_parts.join(" UNION ALL "));

        let results = if let Ok(mut stmt) = conn.prepare(&full_sql) {
            stmt.query_map(params![pattern, top_k as i64], |row| {
                Ok(json!({
                    "source": row.get::<_, String>(0).unwrap_or_default(),
                    "text": row.get::<_, String>(1).unwrap_or_default(),
                }))
            })
            .ok()
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
        } else {
            vec![]
        };

        // Immediately detach to reclaim memory and file handles
        for alias in attached_aliases {
            let _ = conn.execute_batch(&format!("DETACH DATABASE {}", alias));
        }

        results
    }

    pub fn close(&mut self) {
        self.conn = None;
    }
}
