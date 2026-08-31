-- app.db · initial schema (docs/03 §3)
-- schema_meta and schema_migrations are created by the migration runner.

CREATE TABLE projects (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  description    TEXT NOT NULL DEFAULT '',
  color          TEXT NOT NULL DEFAULT 'accent-1',
  dir_name       TEXT NOT NULL UNIQUE,
  schema_version TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL,
  opened_at      TEXT,
  archived_at    TEXT,
  sort_order     INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_projects_archived ON projects(archived_at, sort_order);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL           -- JSON
);

CREATE TABLE ai_profiles (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  base_url       TEXT NOT NULL,
  api_key_ref    TEXT,          -- keychain entry name; never a plaintext key (I-4)
  default_model  TEXT,
  supports_vision INTEGER NOT NULL DEFAULT 0,
  supports_tools  INTEGER NOT NULL DEFAULT 0,
  supports_embed  INTEGER NOT NULL DEFAULT 0,
  json_schema     INTEGER NOT NULL DEFAULT 0,
  extra_headers  TEXT NOT NULL DEFAULT '{}',
  timeout_ms     INTEGER NOT NULL DEFAULT 120000,
  created_at     TEXT NOT NULL,
  last_ok_at     TEXT
);

CREATE TABLE model_roles (
  role       TEXT PRIMARY KEY,   -- 'chat' | 'vision' | 'embedding' | 'organizer'
  profile_id TEXT NOT NULL REFERENCES ai_profiles(id) ON DELETE CASCADE,
  model      TEXT NOT NULL,
  params     TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE whisper_models (
  name          TEXT PRIMARY KEY,  -- 'base' | 'small' | 'medium' | 'large-v2'
  file_name     TEXT NOT NULL,
  size_bytes    INTEGER NOT NULL,
  sha256        TEXT NOT NULL,
  downloaded_at TEXT
);

CREATE TABLE mcp_servers (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  transport  TEXT NOT NULL,       -- 'stdio' | 'http'
  command    TEXT,
  args       TEXT NOT NULL DEFAULT '[]',
  env        TEXT NOT NULL DEFAULT '{}',
  url        TEXT,
  enabled    INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE mcp_tool_policies (
  server_id TEXT NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
  tool_name TEXT NOT NULL,
  policy    TEXT NOT NULL,        -- 'ask' | 'always_allow' | 'deny'
  PRIMARY KEY (server_id, tool_name)
);

-- Cross-project search mirror (FR-N3). Source of truth stays in each project.db.
CREATE VIRTUAL TABLE global_index USING fts5(
  project_id UNINDEXED,
  source_id  UNINDEXED,
  kind       UNINDEXED,           -- 'source' | 'chunk' | 'message' | 'artifact'
  ref_id     UNINDEXED,
  title,
  body,
  body_raw   UNINDEXED,
  tokenize = "unicode61 remove_diacritics 2"
);
