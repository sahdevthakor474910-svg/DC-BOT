-- Migration 003: Add nsfw, rule34, porn, and hentai channel columns
ALTER TABLE guild_config ADD COLUMN nsfw_channel_id TEXT;
ALTER TABLE guild_config ADD COLUMN rule34_channel_id TEXT;
ALTER TABLE guild_config ADD COLUMN porn_channel_id TEXT;
ALTER TABLE guild_config ADD COLUMN hentai_channel_id TEXT;
