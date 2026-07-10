-- Migration 003: Add nsfw_channel_id and rule34_channel_id
ALTER TABLE guild_config ADD COLUMN nsfw_channel_id TEXT;
ALTER TABLE guild_config ADD COLUMN rule34_channel_id TEXT;
