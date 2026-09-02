//! App settings (docs/07 §1). Stored as one JSON blob in `app.db` `settings`
//! under the key `app`. The *display* subset (theme / scale / monochrome /
//! reading font) is also mirrored client-side in localStorage so the very first
//! paint has no flash — see D-12.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub general: General,
    pub language: Language,
    pub illustrator: IllustratorSettings,
    pub ingest: IngestSettings,
    pub sandbox: SandboxSettings,
    pub transcription: Transcription,
    pub ai_budget: AiBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct General {
    /// "home" | "last_project"
    pub startup_view: String,
    pub check_updates_on_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Language {
    /// "en" | "ja" | "zh-Hans"
    pub ui: String,
    /// "follow_ui" | "match_source" | "en" | "ja" | "zh-Hans"
    pub ai_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IllustratorSettings {
    pub enabled: bool,
    /// "simple" | "standard" | "detailed"
    pub default_level: String,
    /// "page" | "source" | "project"
    pub default_scope: String,
    pub prefetch_next: bool,
    pub carry_over_previous_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IngestSettings {
    pub concurrency: u32,
    pub fetch_web_images: bool,
    pub allow_private_network: bool,
    /// P12: run OCR on imported images, and offer it for scanned PDFs.
    #[serde(default = "default_true")]
    pub ocr: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    pub command_timeout_sec: u32,
    pub auto_allow_new_file_writes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    /// "base" | "small" | "medium" | "large-v2"
    pub whisper_model: String,
    /// "auto" | ISO code
    pub language: String,
    pub hardware_accel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AiBudget {
    pub context_tokens: u32,
    pub max_output_tokens: u32,
    pub max_images_per_request: u32,
    pub rerank: bool,
    pub concurrency: u32,
    pub warn_above_tokens: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            general: General {
                startup_view: "home".into(),
                check_updates_on_start: true,
            },
            language: Language {
                ui: "ja".into(),
                ai_response: "follow_ui".into(),
            },
            illustrator: IllustratorSettings {
                enabled: false,
                default_level: "standard".into(),
                default_scope: "source".into(),
                prefetch_next: true,
                carry_over_previous_page: true,
            },
            ingest: IngestSettings {
                concurrency: (num_cpus() / 2).max(1),
                fetch_web_images: false,
                allow_private_network: false,
                ocr: true,
            },
            sandbox: SandboxSettings {
                command_timeout_sec: 60,
                auto_allow_new_file_writes: false,
            },
            transcription: Transcription {
                whisper_model: "small".into(),
                language: "auto".into(),
                hardware_accel: true,
            },
            ai_budget: AiBudget {
                context_tokens: 32768,
                max_output_tokens: 4096,
                max_images_per_request: 4,
                rerank: false,
                concurrency: 3,
                warn_above_tokens: None,
            },
        }
    }
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
