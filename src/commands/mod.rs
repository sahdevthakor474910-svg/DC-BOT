pub mod admin;
pub mod config;
pub mod help;
pub mod memes;
pub mod ping;

use crate::data::{Data, Error};

/// Returns all registered top-level slash commands.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        ping::ping(),
        help::help(),
        admin::admin(),
        config::config(),
        memes::memes(),
    ]
}
