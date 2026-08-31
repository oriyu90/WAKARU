//! `search_query` (FR-N3). Global scope searches the `global_index` FTS mirror
//! across every project; project scope runs the hybrid FTS+vector search
//! (docs/05 §3) inside one project. No `rusqlite::Connection` is held across an
//! `.await` (Tauri async commands require `Send`).

use crate::domain::ai::Role;
use crate::domain::search::*;
use crate::error::AppResult;
use crate::services::{ai, projects, retrieval};
use std::path::Path;

pub async fn query(
    app_db_path: &Path,
    projects_root: &Path,
    q: SearchQuery,
) -> AppResult<SearchResults> {
    match q.scope {
        SearchScope::Global => {
            let app_db = crate::storage::open(app_db_path)?;
            global(&app_db, &q)
        }
        SearchScope::Project => project_scoped(app_db_path, projects_root, &q).await,
    }
}

fn global(app_db: &rusqlite::Connection, q: &SearchQuery) -> AppResult<SearchResults> {
    let expr = retrieval::fts_match_expr(&q.q);
    if expr.is_empty() {
        return Ok(SearchResults { hits: vec![], semantic: false });
    }
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    {
        let mut s = app_db.prepare("SELECT id, name FROM projects")?;
        let rows: Vec<(String, String)> = s
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        names.extend(rows);
    }

    let mut stmt = app_db.prepare(
        "SELECT project_id, source_id, kind, ref_id, title, body_raw
         FROM global_index
         WHERE global_index MATCH ?1
         ORDER BY bm25(global_index)
         LIMIT ?2",
    )?;
    let hits: Vec<SearchHit> = stmt
        .query_map(rusqlite::params![expr, q.limit as i64], |r| {
            let project_id: String = r.get(0)?;
            let source_id: String = r.get(1)?;
            let ref_id: String = r.get(3)?;
            let title: String = r.get(4)?;
            let body_raw: String = r.get(5)?;
            let ordinal = ref_id.rsplit('#').next().and_then(|s| s.parse::<u32>().ok());
            Ok(SearchHit {
                project_name: names.get(&project_id).cloned().unwrap_or_default(),
                project_id,
                source_id,
                source_name: title,
                document_id: None,
                ordinal,
                snippet: body_raw.chars().take(240).collect(),
                locator: serde_json::json!({}),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    Ok(SearchResults { hits, semantic: false })
}

async fn project_scoped(
    app_db_path: &Path,
    projects_root: &Path,
    q: &SearchQuery,
) -> AppResult<SearchResults> {
    let Some(pid) = q.project_id.as_deref() else {
        return Ok(SearchResults { hits: vec![], semantic: false });
    };
    let mode = q.mode.as_deref().unwrap_or("hybrid");
    let want_vec = mode != "keyword";

    // ── sync: resolve everything the await needs, then drop the connections ──
    let (embed_role, project_name) = {
        let app_db = crate::storage::open(app_db_path)?;
        let role = if want_vec {
            ai::profiles::resolve(&app_db, Role::Embedding)?
        } else {
            None
        };
        let name: String = app_db
            .query_row("SELECT name FROM projects WHERE id = ?1", [pid], |r| r.get(0))
            .unwrap_or_default();
        (role, name)
    };

    // ── await: embed the query (no Connection held) ──
    let query_vec = match embed_role {
        Some(role) => ai::embed_with(role, std::slice::from_ref(&q.q), true)
            .await
            .ok()
            .and_then(|(_, mut v)| v.pop()),
        None => None,
    };
    let semantic = query_vec.is_some();

    // ── sync: run the fused search ──
    let project_db = projects::open_db(projects_root, pid)?;
    let hits = retrieval::hybrid_search(
        &project_db,
        &q.q,
        query_vec.as_deref(),
        q.source_id.as_deref(),
        q.limit as usize,
    )?
    .into_iter()
    .map(|h| SearchHit {
        project_id: pid.to_string(),
        project_name: project_name.clone(),
        source_id: h.source_id,
        source_name: h.source_name,
        document_id: Some(h.document_id),
        ordinal: Some(h.ordinal as u32),
        snippet: h.snippet,
        locator: serde_json::from_str(&h.locator).unwrap_or(serde_json::json!({})),
    })
    .collect();

    Ok(SearchResults { hits, semantic })
}
