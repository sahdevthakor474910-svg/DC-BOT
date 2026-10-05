-- Migration 020: Add sassy_mode_enabled to guild_config
-- When sassy_mode_enabled = 1, the bot talks and flirts in a 100000000% cheesy, flamboyant, gay sassy diva persona.
ALTER TABLE guild_config ADD COLUMN sassy_mode_enabled INTEGER NOT NULL DEFAULT 0;
