-- Existing profiles used the OpenAI-compatible wire format. Keep that exact
-- behaviour while allowing new Anthropic-compatible profiles.
ALTER TABLE ai_profiles
ADD COLUMN protocol TEXT NOT NULL DEFAULT 'openai'
CHECK (protocol IN ('openai', 'anthropic'));
