//! File logging with daily rotation (docs/07 §6). Never log API keys, prompt bodies
//! or document text — only operation names, durations, error codes, ids, model names,
//! token counts.

use std::path::Path;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(logs_dir: &Path) {
    let file_appender = tracing_appender::rolling::daily(logs_dir, "wakaru.log");
    let filter = EnvFilter::try_from_env("WAKARU_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,wakaru_lib=debug"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_ansi(false)
                .with_writer(file_appender),
        )
        .with(fmt::layer().with_target(false))
        .try_init();
}
