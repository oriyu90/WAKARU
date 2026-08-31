//! The thin IPC layer: `#[tauri::command]` functions only. Business logic lives
//! in `services/`. Every command returns `Result<T, AppError>` and takes
//! domain-shaped arguments (docs/02 §4).

mod ai;
mod app;
mod export;
mod file_modifier;
mod illustrator;
mod projects;
mod search;
mod settings;
mod sources;
mod viewer;

pub use ai::*;
pub use app::*;
pub use export::*;
pub use file_modifier::*;
pub use illustrator::*;
pub use projects::*;
pub use search::*;
pub use settings::*;
pub use sources::*;
pub use viewer::*;
