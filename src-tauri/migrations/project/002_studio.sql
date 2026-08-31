-- project.db · 002 · Studio tab scope (docs/06 §6, P6)
-- A Studio tab keeps its own retrieval scope ("project" or "source:<id>") so the
-- selector persists across restarts and a paused tool loop can be resumed with
-- the same context. Additive, nullable-with-default — schemaVersion stays 1.0.0
-- (docs/03 §6.3: no major/minor bump for a compatible column add).
ALTER TABLE studio_tabs ADD COLUMN scope TEXT NOT NULL DEFAULT 'project';
