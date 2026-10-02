-- Migration 019: Add bot_response_enabled to guild_config
-- When bot_response_enabled = 0, the bot stops replying in chat completely (no AI, no gali comebacks) and silently operates background tasks.
ALTER TABLE guild_config ADD COLUMN bot_response_enabled INTEGER NOT NULL DEFAULT 1;
