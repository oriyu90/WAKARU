//! Background job registry + worker pool skeleton (docs/04 §11). Ingest/transcribe/
//! embed jobs land here in later phases. IPC handlers must never block on a job;
//! jobs run on their own tokio tasks and report via the `job://progress` event.

use crate::domain::{Job, JobKind, JobProgress, JobStatus};
use crate::storage::migrate::now_iso8601;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
pub struct JobRegistry {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    jobs: HashMap<String, Job>,
    tokens: HashMap<String, CancellationToken>,
    order: Vec<String>,
}

#[allow(dead_code)] // methods wired in Phase 1+
impl JobRegistry {
    pub fn create(&self, kind: JobKind, project_id: Option<String>, source_id: Option<String>) -> (String, CancellationToken) {
        let id = Uuid::now_v7().to_string();
        let token = CancellationToken::new();
        let job = Job {
            id: id.clone(),
            kind,
            status: JobStatus::Queued,
            project_id,
            source_id,
            phase: None,
            done: 0,
            total: 0,
            message: None,
            started_at: None,
            ended_at: None,
        };
        let mut inner = self.inner.lock().unwrap();
        inner.jobs.insert(id.clone(), job);
        inner.tokens.insert(id.clone(), token.clone());
        inner.order.push(id.clone());
        (id, token)
    }

    pub fn mark_running(&self, id: &str) {
        if let Some(j) = self.inner.lock().unwrap().jobs.get_mut(id) {
            j.status = JobStatus::Running;
            j.started_at = Some(now_iso8601());
        }
    }

    pub fn finish(&self, id: &str, status: JobStatus, message: Option<String>) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(j) = inner.jobs.get_mut(id) {
            j.status = status;
            j.ended_at = Some(now_iso8601());
            j.message = message;
        }
        inner.tokens.remove(id);
    }

    pub fn progress(&self, app: &AppHandle, p: JobProgress) {
        if let Some(j) = self.inner.lock().unwrap().jobs.get_mut(&p.job_id) {
            j.phase = Some(p.phase.clone());
            j.done = p.done;
            j.total = p.total;
        }
        let _ = app.emit("job://progress", &p);
    }

    pub fn list(&self, project_id: Option<&str>) -> Vec<Job> {
        let inner = self.inner.lock().unwrap();
        inner
            .order
            .iter()
            .filter_map(|id| inner.jobs.get(id))
            .filter(|j| project_id.map_or(true, |pid| j.project_id.as_deref() == Some(pid)))
            .cloned()
            .collect()
    }

    pub fn cancel(&self, id: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        if let Some(tok) = inner.tokens.get(id) {
            tok.cancel();
            true
        } else {
            false
        }
    }
}
