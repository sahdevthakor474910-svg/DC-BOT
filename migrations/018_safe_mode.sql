-- Migration 018: Add safe_mode_enabled to guild_config
-- When safe_mode_enabled = 1, bot suppresses gali auto-responses and operates in clean/family-friendly mode.
ALTER TABLE guild_config ADD COLUMN safe_mode_enabled INTEGER NOT NULL DEFAULT 0;
