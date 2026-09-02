-- project.db · 003 · OCR status for scanned PDFs (P12).
-- A PDF whose pages have no text layer gets ocr_status='pending' at ingest; the
-- Viewer runs OCR (PDF.js raster -> ocr_page command) and moves it to
-- 'running' -> 'done' / 'partial' / 'failed'. NULL = not applicable (text PDF,
-- image OCR runs inline, or a pre-P12 database). Additive nullable column —
-- schemaVersion stays 1.0.0 (docs/03 §6.3, same as 002_studio).
ALTER TABLE sources ADD COLUMN ocr_status TEXT;
