//! Embedding index for a project's chunks (docs/03 §7, docs/05 §2). `chunk_vectors`
//! is a `vec0` table whose dimension follows the configured model; changing the
//! model drops and rebuilds it. When no embedding endpoint is set, this is a
//! no-op and search stays FTS-only (I-2).

use crate::error::AppResult;
use crate::services::ai;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection};

const BATCH: usize = 64;

/// (model, dim) currently indexed, if any.
pub fn current_meta(project_db: &Connection) -> Option<(String, usize)> {
    project_db
        .query_row(
            "SELECT model, dim FROM embedding_meta WHERE id = 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize)),
        )
        .ok()
}

pub fn has_vectors(project_db: &Connection) -> bool {
    project_db
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='chunk_vectors'",
            [],
            |_| Ok(()),
        )
        .is_ok()
}

fn ensure_table(project_db: &Connection, dim: usize) -> AppResult<()> {
    project_db.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS chunk_vectors USING vec0(
            chunk_rowid INTEGER PRIMARY KEY,
            embedding FLOAT[{dim}]
        );"
    ))?;
    Ok(())
}

fn reset_table(project_db: &Connection, model: &str, dim: usize) -> AppResult<()> {
    project_db.execute_batch("DROP TABLE IF EXISTS chunk_vectors;")?;
    ensure_table(project_db, dim)?;
    project_db.execute(
        "INSERT INTO embedding_meta (id, model, dim, normalized, built_at)
         VALUES (1, ?1, ?2, 1, ?3)
         ON CONFLICT(id) DO UPDATE SET model=excluded.model, dim=excluded.dim, built_at=excluded.built_at",
        params![model, dim as i64, now_iso8601()],
    )?;
    Ok(())
}

/// Embed every chunk of one source that isn't in `chunk_vectors` yet. Called at
/// the end of ingest. Reports progress via `on_progress(done, total)`.
pub async fn index_source(
    app_db: &Connection,
    project_db: &Connection,
    source_id: &str,
    mut on_progress: impl FnMut(u32, u32),
) -> AppResult<u32> {
    let mut stmt = project_db.prepare(
        "SELECT c.rowid, c.text FROM chunks c
         WHERE c.source_id = ?1
           AND (NOT EXISTS (SELECT 1 FROM sqlite_master WHERE name='chunk_vectors')
                OR c.rowid NOT IN (SELECT chunk_rowid FROM chunk_vectors))",
    )?;
    let pending: Vec<(i64, String)> = stmt
        .query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);
    if pending.is_empty() {
        return Ok(0);
    }

    let total = pending.len() as u32;
    let mut done = 0u32;

    for batch in pending.chunks(BATCH) {
        let texts: Vec<String> = batch.iter().map(|(_, t)| t.clone()).collect();
        let Some((model, vectors)) = ai::embed(app_db, &texts, false).await? else {
            return Ok(0); // no endpoint -> FTS only
        };
        let dim = vectors.first().map(|v| v.len()).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }
        // (Re)build the table if the model/dim changed (docs/03 §7).
        match current_meta(project_db) {
            Some((m, d)) if m == model && d == dim => {}
            Some(_) => reset_table(project_db, &model, dim)?,
            None => {
                ensure_table(project_db, dim)?;
                project_db.execute(
                    "INSERT OR REPLACE INTO embedding_meta (id, model, dim, normalized, built_at)
                     VALUES (1, ?1, ?2, 1, ?3)",
                    params![model, dim as i64, now_iso8601()],
                )?;
            }
        }

        for ((rowid, _), vec) in batch.iter().zip(vectors.iter()) {
            let norm = l2_normalize(vec);
            let bytes = bytemuck_f32(&norm);
            project_db.execute(
                "INSERT OR REPLACE INTO chunk_vectors (chunk_rowid, embedding) VALUES (?1, ?2)",
                params![rowid, bytes],
            )?;
        }
        done += batch.len() as u32;
        on_progress(done, total);
    }
    Ok(done)
}

pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

fn bytemuck_f32(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}
