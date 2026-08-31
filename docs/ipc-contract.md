# IPC contract (recovered)

The v0.1.0 application source is gone. This is the Tauri command surface **extracted
from the shipped v0.1.0 minified bundle** (`~/Downloads/WAKARU/01_元版_復元ソース/
frontend-assets/index.js`), cross-checked against `docs/02_アーキテクチャ.md §4`.

- Command **names** below are certain (string literals in the bundle).
- Argument / return **field names** are inferred from the bundle + docs and are
  authoritative only once the matching `#[derive(ts_rs::TS)]` type is written in
  `src-tauri/src/domain/`. Adjustments are recorded in `docs/DECISIONS.md`.
- `docs/02 §4` is the canonical spec where it and this file disagree.

## App / settings

| command | args | returns |
|---|---|---|
| `app_get_info` | — | `{ version, buildDate, dataDir, license }` |
| `app_get_settings` / `settings_get` | — | `Settings` |
| `app_update_settings` / `settings_save` | `patch` | `Settings` |
| `app_open_data_dir` | — | `()` |
| `app_move_data_dir` | `{ newPath }` | `()` + progress |

## AI

`ai_list_profiles` · `ai_upsert_profile` · `ai_delete_profile` · `ai_test_profile` ·
`ai_get_role_bindings` · `ai_set_role_binding` · `ai_cancel_request`
`embedding_download_model` · `embedding_delete_model`
`whisper_list_models` · `whisper_download_model` · `whisper_delete_model`

## Projects

`project_list` · `project_create` · `project_update` · `project_archive` /
`project_unarchive` · `project_set_archived` · `project_delete {id, confirmName}` ·
`project_export {ids, destDir}` · `project_import {zipPath}` · `project_export_estimates` ·
`project_get_settings` · `project_update_settings`

## Sources

`source_add_files` · `source_add_url` · `source_list` · `source_get` /
`source_get_document` · `source_get_original_file` · `source_delete` ·
`source_reanalyze` · `source_render_page` · `source_asset_url`

## Viewer

`viewer_get_tabs` · `viewer_open_tab` · `viewer_close_tab` · `viewer_reorder_tabs` ·
`viewer_update_locator` · `viewer_pin_tab`

## Live Illustrator

`illustrator_get_or_create_thread` · `illustrator_generate` · `illustrator_ask` ·
`illustrator_cancel` · `illustrator_import_to_studio`

## Studio

`studio_list_tabs` · `studio_create_tab` · `studio_rename_tab` · `studio_close_tab` ·
`studio_reorder_tabs` · `studio_send` · `studio_cancel` · `studio_resolve_tool` /
`studio_approve_tool_call` · `studio_list_artifacts` ·
`studio_import_artifact_as_source` · `studio_download_artifact`

## MCP

`mcp_list_servers` · `mcp_upsert_server` · `mcp_delete_server` · `mcp_connect` /
`mcp_disconnect` · `mcp_set_tool_policy` · `mcp_call_tool`

## Search

`search_query {scope, projectId?, q, filters?, limit}` · `retrieval_query`

## File Modifier

`fm_allow_image_preview` · `fm_path_exists` · `fm_images_to_pdf` ·
`fm_text_to_markdown` · `fm_save_text`

## Jobs / misc

`jobs_list` · `jobs_cancel` · `run_command` (sandboxed, Studio tool) ·
built-in Studio tools: `search_sources` `read_document` `list_sources` `list_tabs`
`read_tab` `read_file` `list_files` `write_file`

## Events (Rust → front) — `docs/02 §5`

`job://progress` · `job://done` · `job://error` ·
`stream://delta` · `stream://tool_call` · `stream://tool_result` ·
`stream://citations` · `stream://done` · `stream://error` ·
`source://status` · `settings://changed`
