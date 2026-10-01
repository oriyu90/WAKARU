-- project.db · 004 · notes (sticky) + interactive visuals (v1.6.0).
-- Additive only: older builds ignore the new tables, newer builds read older
-- databases with zero notes / zero visuals. Bumps PROJECT_SCHEMA_VERSION to
-- 1.1.0 (forward migration; a backup is taken by the caller before migrate).
-- Notes are viewer-private memos bound to a source + locator (plan §5). They
-- are never fed to AI search/prompts automatically and cascade on source
-- delete. Visuals are small self-contained HTML/CSS/JS artifacts shared by
-- Live and Studio (plan §4), referenced from messages and illustrations.

CREATE TABLE notes (
  id          TEXT PRIMARY KEY,
  source_id   TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  locator     TEXT NOT NULL DEFAULT '{}',
  anchor_kind TEXT NOT NULL DEFAULT 'page',
  anchor_json TEXT NOT NULL DEFAULT '{}',
  body        TEXT NOT NULL DEFAULT '',
  color       TEXT NOT NULL DEFAULT 'yellow',
  stack_order INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  deleted_at  TEXT
);
CREATE INDEX idx_notes_source ON notes(source_id, locator);

CREATE TABLE visual_previews (
  id            TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL DEFAULT 1,
  title         TEXT NOT NULL DEFAULT '',
  html          TEXT NOT NULL DEFAULT '',
  css           TEXT NOT NULL DEFAULT '',
  js            TEXT NOT NULL DEFAULT '',
  data_json     TEXT NOT NULL DEFAULT '{}',
  aspect_ratio  TEXT NOT NULL DEFAULT '16:9',
  source_refs   TEXT NOT NULL DEFAULT '[]',
  initial_state TEXT NOT NULL DEFAULT '{}',
  model         TEXT NOT NULL DEFAULT '',
  created_at    TEXT NOT NULL
);

CREATE TABLE message_visuals (
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  visual_id  TEXT NOT NULL REFERENCES visual_previews(id) ON DELETE CASCADE,
  ordinal    INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (message_id, visual_id)
);

ALTER TABLE illustrations ADD COLUMN visual_id TEXT REFERENCES visual_previews(id) ON DELETE SET NULL;
