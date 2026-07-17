-- Migration 011: Add DMC Boss Analyzer channel
ALTER TABLE guild_config ADD COLUMN dmc_channel_id TEXT;
