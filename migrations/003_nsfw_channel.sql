-- Migration 003: Add NSFW channel support
ALTER TABLE guild_config ADD COLUMN nsfw_channel_id TEXT;
