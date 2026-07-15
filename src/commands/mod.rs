pub mod checks;
pub mod setup;
pub mod unset;
pub mod post;
pub mod status;
pub mod settings;
pub mod help;
pub mod ping;

// Legacy modules kept for reference — no longer registered as commands
// pub mod admin;
// pub mod config;
// pub mod memes;

use crate::data::{Data, Error};

/// Returns all registered top-level slash commands.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        // ── Core commands ──────────────────────────────────────────────────
        setup::setup(),
        unset::unset(),
        post::post(),
        status::status(),
        settings::settings(),
        // ── Utilities ─────────────────────────────────────────────────────
        help::help(),
        ping::ping(),
    ]
}
