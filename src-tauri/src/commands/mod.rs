//! The thin IPC layer: `#[tauri::command]` functions only. Business logic lives
//! in `services/`. Every command returns `Result<T, AppError>` and takes
//! domain-shaped arguments (docs/02 §4).

mod app;
mod projects;
mod sources;

pub use app::*;
pub use projects::*;
pub use sources::*;
