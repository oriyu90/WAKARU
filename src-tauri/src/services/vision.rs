//! Bounded visual analysis for normalized source images. The structured result
//! is appended to the source document and search index, never substituted for
//! original text.

use crate::domain::ai::Role;
use crate::error::{AppError, AppResult};
use crate::services::ai::{client::AiClient, profiles};
use crate::services::retrieval::cjk_bigram;
use crate::storage::migrate::now_iso8601;
use base64::Engine as _;
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::Path;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
const MAX_ANALYSIS_CHARS: usize = 24_000;

pub async fn analyze_normalized_image(
    app_db: &Connection,
    project_db: &Connection,
    project_dir: &Path,
    project_id: &str,
    source_id: &str,
) -> AppResult<bool> {
    let Some(role) = profiles::resolve(app_db, Role::Vision)? else {
        return Ok(false);
    };
    let path = project_dir
        .join("derived")
        .join(source_id)
        .join("pages")
        .join("0001.png");
    let bytes = std::fs::read(path)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::new(
            "VISION_IMAGE_TOO_LARGE",
            "error.vision.tooLarge",
            "normalized image exceeds the visual-analysis limit",
        ));
    }
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    let client = AiClient::new(
        role.protocol,
        &role.base_url,
        role.api_key,
        role.extra_headers,
        role.timeout_ms,
    )?;
    let messages = json!([{
        "role": "system",
        "content": "Analyze the supplied study material faithfully. Do not embellish or infer hidden facts. Return concise Markdown with exactly these headings: ## OCR, ## Layout, ## Diagram or image meaning, ## Uncertainty. Under OCR transcribe every legible string in reading order. Under Layout describe headings, columns, tables and spatial relationships. Under Diagram or image meaning explain labels, arrows, axes, legends and their relationships. Under Uncertainty list unreadable or ambiguous areas. If a section has nothing, write None. Preserve numbers, units, names and symbols exactly."
    }, {
        "role": "user",
        "content": [
            { "type": "text", "text": "Read this source image for later grounded explanation and search." },
            { "type": "image_url", "image_url": { "url": data_url } }
        ]
    }]);
    let cancel = CancellationToken::new();
    let output = std::sync::Mutex::new(String::new());
    let (_, truncated, _) = client
        .chat_stream(
            &role.model,
            messages,
            &role.params,
            &cancel,
            |kind, delta| {
                if kind == "text" {
                    let mut text = output.lock().unwrap_or_else(|error| error.into_inner());
                    if text.chars().count() < MAX_ANALYSIS_CHARS {
                        text.push_str(delta);
                    }
                }
            },
        )
        .await?;
    if truncated {
        return Err(AppError::new(
            "VISION_TRUNCATED",
            "error.ai.truncated",
            "visual analysis stream ended early",
        )
        .retriable());
    }
    let analysis = output
        .into_inner()
        .unwrap_or_else(|error| error.into_inner())
        .trim()
        .chars()
        .take(MAX_ANALYSIS_CHARS)
        .collect::<String>();
    if analysis.is_empty() {
        return Ok(false);
    }

    let (document_id, original): (String, String) = project_db.query_row(
        "SELECT id, text FROM documents WHERE source_id = ?1 ORDER BY ordinal LIMIT 1",
        [source_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let combined = format!("{original}\n\n# Visual analysis\n\n{analysis}");
    let chunk_text = format!("[Visual analysis]\n{analysis}");
    let tx = project_db.unchecked_transaction()?;
    tx.execute(
        "UPDATE documents SET text = ?2 WHERE id = ?1",
        params![document_id, combined],
    )?;
    tx.execute(
        "INSERT INTO chunks (id, source_id, document_id, ordinal, text, text_bigram, tokens, locator, created_at)
         VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(ordinal), 0) + 1 FROM chunks WHERE document_id = ?3), ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::now_v7().to_string(),
            source_id,
            document_id,
            chunk_text,
            cjk_bigram(&chunk_text),
            (chunk_text.chars().count() / 4).max(1) as i64,
            json!({ "t": "region", "bbox": [0.0, 0.0, 1.0, 1.0], "analysis": "vision" }).to_string(),
            now_iso8601(),
        ],
    )?;
    tx.execute(
        "UPDATE sources SET summary = ?2, status = 'ready' WHERE id = ?1",
        params![source_id, analysis.chars().take(800).collect::<String>()],
    )?;
    tx.commit()?;
    app_db.execute(
        "DELETE FROM global_index WHERE project_id = ?1 AND source_id = ?2 AND kind = 'visual'",
        params![project_id, source_id],
    )?;
    app_db.execute(
        "INSERT INTO global_index (project_id, source_id, kind, ref_id, title, body, body_raw)
         VALUES (?1, ?2, 'visual', ?3, 'Visual analysis', ?4, ?5)",
        params![
            project_id,
            source_id,
            format!("{source_id}:visual"),
            cjk_bigram(&analysis),
            analysis
        ],
    )?;
    Ok(true)
}
