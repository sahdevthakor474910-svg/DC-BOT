-- Migration 017: Toggle for automatic gali/slang responses
ALTER TABLE guild_config ADD COLUMN gali_response_enabled INTEGER NOT NULL DEFAULT 1;
