-- project.db · initial schema (docs/03 §4)
-- schema_meta and schema_migrations are created by the migration runner.
-- chunk_vectors (vec0) is added in a Phase 3 migration once sqlite-vec is loaded.

CREATE TABLE sources (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL,
  original_name TEXT NOT NULL,
  rel_path      TEXT NOT NULL,
  url           TEXT,
  mime          TEXT,
  bytes         INTEGER NOT NULL DEFAULT 0,
  sha256        TEXT,
  status        TEXT NOT NULL,
  error_code    TEXT,
  error_message TEXT,
  lang          TEXT,
  page_count    INTEGER,
  duration_ms   INTEGER,
  summary       TEXT,
  meta          TEXT NOT NULL DEFAULT '{}',
  added_at      TEXT NOT NULL,
  analyzed_at   TEXT,
  origin        TEXT NOT NULL DEFAULT 'user'
);
CREATE INDEX idx_sources_status ON sources(status);
CREATE UNIQUE INDEX idx_sources_sha ON sources(sha256) WHERE sha256 IS NOT NULL;

CREATE TABLE documents (
  id         TEXT PRIMARY KEY,
  source_id  TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  ordinal    INTEGER NOT NULL,
  kind       TEXT NOT NULL,
  title      TEXT,
  text       TEXT NOT NULL DEFAULT '',
  image_rel  TEXT,
  locator    TEXT NOT NULL,
  analysis   TEXT NOT NULL DEFAULT '{}',
  UNIQUE(source_id, ordinal)
);
CREATE INDEX idx_documents_source ON documents(source_id, ordinal);

CREATE TABLE chunks (
  id          TEXT PRIMARY KEY,
  source_id   TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  ordinal     INTEGER NOT NULL,
  text        TEXT NOT NULL,
  text_bigram TEXT NOT NULL,
  tokens      INTEGER NOT NULL DEFAULT 0,
  locator     TEXT NOT NULL,
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_chunks_doc ON chunks(document_id, ordinal);
CREATE INDEX idx_chunks_source ON chunks(source_id);

CREATE VIRTUAL TABLE chunks_fts USING fts5(
  text_bigram,
  content = 'chunks',
  content_rowid = 'rowid',
  tokenize = "unicode61 remove_diacritics 2"
);
CREATE TRIGGER chunks_ai AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, text_bigram) VALUES (new.rowid, new.text_bigram);
END;
CREATE TRIGGER chunks_ad AFTER DELETE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, text_bigram) VALUES ('delete', old.rowid, old.text_bigram);
END;
CREATE TRIGGER chunks_au AFTER UPDATE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, text_bigram) VALUES ('delete', old.rowid, old.text_bigram);
  INSERT INTO chunks_fts(rowid, text_bigram) VALUES (new.rowid, new.text_bigram);
END;

CREATE TABLE embedding_meta (
  id         INTEGER PRIMARY KEY CHECK (id = 1),
  model      TEXT NOT NULL,
  dim        INTEGER NOT NULL,
  normalized INTEGER NOT NULL DEFAULT 1,
  built_at   TEXT NOT NULL
);

CREATE TABLE threads (
  id          TEXT PRIMARY KEY,
  scope       TEXT NOT NULL,
  source_id   TEXT REFERENCES sources(id) ON DELETE CASCADE,
  locator_key TEXT,
  title       TEXT NOT NULL DEFAULT '',
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_threads_illustrator
  ON threads(source_id, locator_key) WHERE scope = 'illustrator';

CREATE TABLE messages (
  id           TEXT PRIMARY KEY,
  thread_id    TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
  role         TEXT NOT NULL,
  content      TEXT NOT NULL,
  tool_calls   TEXT,
  tool_call_id TEXT,
  citations    TEXT NOT NULL DEFAULT '[]',
  model        TEXT,
  usage        TEXT,
  status       TEXT NOT NULL DEFAULT 'complete',
  created_at   TEXT NOT NULL
);
CREATE INDEX idx_messages_thread ON messages(thread_id, created_at);

CREATE TABLE illustrations (
  id          TEXT PRIMARY KEY,
  source_id   TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  locator_key TEXT NOT NULL,
  lang        TEXT NOT NULL,
  level       TEXT NOT NULL,
  model       TEXT NOT NULL,
  content     TEXT NOT NULL,
  citations   TEXT NOT NULL DEFAULT '[]',
  created_at  TEXT NOT NULL,
  UNIQUE(source_id, locator_key, lang, level, model)
);

CREATE TABLE viewer_tabs (
  id        TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  locator   TEXT NOT NULL DEFAULT '{}',
  pinned    INTEGER NOT NULL DEFAULT 0,
  ordinal   INTEGER NOT NULL,
  opened_at TEXT NOT NULL
);

CREATE TABLE studio_tabs (
  id         TEXT PRIMARY KEY,
  thread_id  TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
  title      TEXT NOT NULL,
  ordinal    INTEGER NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE artifacts (
  id                 TEXT PRIMARY KEY,
  thread_id          TEXT REFERENCES threads(id) ON DELETE SET NULL,
  rel_path           TEXT NOT NULL,
  bytes              INTEGER NOT NULL DEFAULT 0,
  mime               TEXT,
  imported_source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
  created_at         TEXT NOT NULL,
  UNIQUE(rel_path)
);

CREATE TABLE jobs (
  id         TEXT PRIMARY KEY,
  kind       TEXT NOT NULL,
  source_id  TEXT,
  status     TEXT NOT NULL,
  phase      TEXT,
  done       INTEGER NOT NULL DEFAULT 0,
  total      INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  started_at TEXT,
  ended_at   TEXT
);
