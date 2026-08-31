//! The thin IPC layer: `#[tauri::command]` functions only. Business logic lives
//! in `services/`. Every command returns `Result<T, AppError>` and takes
//! domain-shaped arguments (docs/02 §4).

mod ai;
mod app;
mod illustrator;
mod projects;
mod search;
mod sources;
mod viewer;

pub use ai::*;
pub use app::*;
pub use illustrator::*;
pub use projects::*;
pub use search::*;
pub use sources::*;
pub use viewer::*;
