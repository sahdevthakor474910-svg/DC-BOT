-- Migration 010: Split X / Twitter update channels into Global and Asia
ALTER TABLE guild_config ADD COLUMN twitter_global_channel_id TEXT;
ALTER TABLE guild_config ADD COLUMN twitter_asia_channel_id TEXT;
