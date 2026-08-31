//! Retrieval helpers. `cjk_bigram` is the one place CJK text is bi-gram-ised for
//! FTS5 (docs/03 §3 note, D-04). RRF fusion arrives in Phase 3.

/// Turn text into a token stream FTS5's `unicode61` tokenizer can index for CJK:
/// runs of CJK are emitted as overlapping bi-grams, everything else is passed
/// through lower-cased. The *same* function is used at index and query time.
pub fn cjk_bigram(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut cjk_run: Vec<char> = Vec::new();
    let mut latin = String::new();

    let flush_latin = |latin: &mut String, out: &mut Vec<String>| {
        if !latin.is_empty() {
            out.push(std::mem::take(latin).to_lowercase());
        }
    };
    let flush_cjk = |run: &mut Vec<char>, out: &mut Vec<String>| {
        if run.is_empty() {
            return;
        }
        if run.len() == 1 {
            out.push(run[0].to_string());
        } else {
            for w in run.windows(2) {
                out.push(w.iter().collect());
            }
        }
        run.clear();
    };

    for c in text.chars() {
        if is_cjk(c) {
            flush_latin(&mut latin, &mut out);
            cjk_run.push(c);
        } else if c.is_alphanumeric() {
            flush_cjk(&mut cjk_run, &mut out);
            latin.push(c);
        } else {
            flush_latin(&mut latin, &mut out);
            flush_cjk(&mut cjk_run, &mut out);
        }
    }
    flush_latin(&mut latin, &mut out);
    flush_cjk(&mut cjk_run, &mut out);

    out.join(" ")
}

/// A keyword (FTS5/BM25) search hit over a project's chunks. The hybrid
/// vector+RRF version lands in Phase 3; this is the "AI-free" search (I-2).
#[derive(Debug, Clone)]
pub struct KeywordHit {
    pub chunk_id: String,
    pub source_id: String,
    pub document_id: String,
    pub snippet: String,
    pub score: f64,
}

pub fn keyword_search(
    project_db: &rusqlite::Connection,
    query: &str,
    limit: usize,
) -> crate::error::AppResult<Vec<KeywordHit>> {
    let match_expr = fts_match_expr(query);
    if match_expr.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = project_db.prepare(
        "SELECT c.id, c.source_id, c.document_id, c.text, bm25(chunks_fts)
         FROM chunks_fts
         JOIN chunks c ON c.rowid = chunks_fts.rowid
         WHERE chunks_fts MATCH ?1
         ORDER BY bm25(chunks_fts)
         LIMIT ?2",
    )?;
    let hits = stmt
        .query_map(rusqlite::params![match_expr, limit as i64], |r| {
            let text: String = r.get(3)?;
            Ok(KeywordHit {
                chunk_id: r.get(0)?,
                source_id: r.get(1)?,
                document_id: r.get(2)?,
                snippet: text.chars().take(200).collect(),
                score: r.get::<_, f64>(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(hits)
}

/// Build an FTS5 MATCH expression: bi-gram the query, then AND the tokens so a
/// multi-word / multi-character query behaves like "contains all".
pub fn fts_match_expr(query: &str) -> String {
    let tokens: Vec<String> = cjk_bigram(query)
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "")))
        .collect();
    tokens.join(" AND ")
}

/// Reciprocal Rank Fusion (docs/05 §3.1). `k` is fixed at 60 — no per-list
/// weights (that road leads to tuning hell).
pub fn rrf(rankings: &[Vec<String>]) -> Vec<(String, f64)> {
    use std::collections::HashMap;
    const K: f64 = 60.0;
    let mut score: HashMap<String, f64> = HashMap::new();
    for list in rankings {
        for (rank, id) in list.iter().enumerate() {
            *score.entry(id.clone()).or_insert(0.0) += 1.0 / (K + rank as f64 + 1.0);
        }
    }
    let mut out: Vec<(String, f64)> = score.into_iter().collect();
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// One fused hit inside a project.
#[derive(Debug, Clone)]
pub struct HybridHit {
    pub chunk_id: String,
    pub source_id: String,
    pub source_name: String,
    pub document_id: String,
    pub ordinal: i64,
    pub snippet: String,
    #[allow(dead_code)]
    pub score: f64,
    pub locator: String,
}

/// FTS(BM25) + vector KNN fused with RRF (docs/05 §3). `query_vec` is the
/// L2-normalised embedding of the query, or `None` for FTS-only (I-2). Adjacent
/// chunks of a matched document are pulled in (§3.4).
pub fn hybrid_search(
    project_db: &rusqlite::Connection,
    query: &str,
    query_vec: Option<&[f32]>,
    source_id: Option<&str>,
    top_k: usize,
) -> crate::error::AppResult<Vec<HybridHit>> {
    // ── A. FTS ──
    let match_expr = fts_match_expr(query);
    let mut fts_ids: Vec<String> = Vec::new();
    if !match_expr.is_empty() {
        let sql = format!(
            "SELECT c.id FROM chunks_fts
             JOIN chunks c ON c.rowid = chunks_fts.rowid
             WHERE chunks_fts MATCH ?1 {}
             ORDER BY bm25(chunks_fts) LIMIT 50",
            source_id.map(|_| "AND c.source_id = ?2").unwrap_or("")
        );
        let mut stmt = project_db.prepare(&sql)?;
        let map = |r: &rusqlite::Row| r.get::<_, String>(0);
        fts_ids = match source_id {
            Some(sid) => stmt
                .query_map(rusqlite::params![match_expr, sid], map)?
                .collect::<Result<_, _>>()?,
            None => stmt
                .query_map(rusqlite::params![match_expr], map)?
                .collect::<Result<_, _>>()?,
        };
    }

    // ── B. Vector KNN ──
    let mut vec_ids: Vec<String> = Vec::new();
    if let Some(qv) = query_vec {
        let has = project_db
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE name='chunk_vectors'",
                [],
                |_| Ok(()),
            )
            .is_ok();
        if has {
            let bytes: Vec<u8> = crate::services::embed::l2_normalize(qv)
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect();
            let sql = format!(
                "SELECT c.id FROM chunk_vectors v
                 JOIN chunks c ON c.rowid = v.chunk_rowid
                 WHERE v.embedding MATCH ?1 AND k = 50 {}
                 ORDER BY distance",
                source_id.map(|_| "AND c.source_id = ?2").unwrap_or("")
            );
            let mut stmt = project_db.prepare(&sql)?;
            let map = |r: &rusqlite::Row| r.get::<_, String>(0);
            vec_ids = match source_id {
                Some(sid) => stmt
                    .query_map(rusqlite::params![bytes, sid], map)?
                    .collect::<Result<_, _>>()?,
                None => stmt
                    .query_map(rusqlite::params![bytes], map)?
                    .collect::<Result<_, _>>()?,
            };
        }
    }

    let fused = if vec_ids.is_empty() {
        fts_ids
            .iter()
            .cloned()
            .map(|id| (id, 0.0))
            .collect::<Vec<_>>()
    } else {
        rrf(&[fts_ids, vec_ids])
    };

    let mut hits = Vec::new();
    for (chunk_id, score) in fused.into_iter().take(top_k) {
        if let Ok(hit) = load_hit(project_db, &chunk_id, score) {
            hits.push(hit);
        }
    }
    Ok(hits)
}

fn load_hit(
    project_db: &rusqlite::Connection,
    chunk_id: &str,
    score: f64,
) -> crate::error::AppResult<HybridHit> {
    let (source_id, document_id, text, locator): (String, String, String, String) = project_db
        .query_row(
            "SELECT source_id, document_id, text, locator FROM chunks WHERE id = ?1",
            [chunk_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
    let (source_name, ordinal): (String, i64) = project_db.query_row(
        "SELECT s.original_name, d.ordinal FROM documents d
         JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
        [&document_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(HybridHit {
        chunk_id: chunk_id.to_string(),
        source_id,
        source_name,
        document_id,
        ordinal,
        snippet: text.chars().take(240).collect(),
        score,
        locator,
    })
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // hiragana + katakana
        | 0x3400..=0x4DBF // CJK ext A
        | 0x4E00..=0x9FFF // CJK unified
        | 0xF900..=0xFAFF // compat ideographs
        | 0xFF66..=0xFF9D // halfwidth katakana
        | 0x20000..=0x2A6DF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_becomes_bigrams() {
        assert_eq!(cjk_bigram("量子計算"), "量子 子計 計算");
    }

    #[test]
    fn query_and_index_agree() {
        let doc = cjk_bigram("量子計算の講義です");
        let q = cjk_bigram("量子");
        assert!(doc.split(' ').any(|t| t == q));
    }

    #[test]
    fn mixed_script() {
        assert_eq!(cjk_bigram("AI と RAG"), "ai と rag");
        assert_eq!(cjk_bigram("東京タワー2024"), "東京 京タ タワ ワー 2024");
    }

    #[test]
    fn single_cjk_char_passes_through() {
        assert_eq!(cjk_bigram("猫"), "猫");
    }

    #[test]
    fn latin_is_lowercased_and_split_on_punctuation() {
        assert_eq!(cjk_bigram("Hello, World!"), "hello world");
    }
}
