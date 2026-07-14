use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;

// ─────────────────────────────────────────────────────────────────────────────
// /setup — configure all channels in one command
// ─────────────────────────────────────────────────────────────────────────────

/// 🛠️ Set up the bot — pick which channels to post in. All fields are optional.
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check"
)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "Channel for 🖼️ Memes (r/memes, r/dankmemes, r/brainrot, r/shitposting)"]
    memes: Option<serenity::GuildChannel>,

    #[description = "Channel for 📰 Gaming News (every 5 min)"]
    news: Option<serenity::GuildChannel>,

    #[description = "Channel for 🎁 Free Games (Epic, Steam alerts — every 15 min)"]
    free_games: Option<serenity::GuildChannel>,

    #[description = "🔞 NSFW channel — must be Age-Restricted! (r/nsfw, r/gonewild, r/rule34, r/hentai, r/porn)"]
    nsfw: Option<serenity::GuildChannel>,

    #[description = "🎌 JAV channel — must be Age-Restricted! (eporner: Japanese & Asian JAV — every 15 min)"]
    jav: Option<serenity::GuildChannel>,

    #[description = "🔥 Porn Video channel — must be Age-Restricted! (RedTube: Brazzers, MILF etc — every 20 min)"]
    porn_videos: Option<serenity::GuildChannel>,

    #[description = "🌶️ OK.XXX channel — must be Age-Restricted! (ok.xxx: top studio videos — every 25 min)"]
    okxxx: Option<serenity::GuildChannel>,

    #[description = "⚔️ Clash of Clans channel — updates, events, free rewards (every 10 min)"]
    coc: Option<serenity::GuildChannel>,

    #[description = "📣 X / Twitter updates — @dmc_poc (Global) & @dmc_poc_jp (Asia) every 10 min"]
    twitter: Option<serenity::GuildChannel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let mut lines: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // ── Memes ──────────────────────────────────────────────────────────────
    if let Some(ch) = &memes {
        queries::set_meme_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        queries::set_brainrot_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        queries::set_shitposting_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        queries::set_instagram_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        lines.push(format!("🖼️  **Memes** → {}", ch.id.mention()));
    }

    // ── News ───────────────────────────────────────────────────────────────
    if let Some(ch) = &news {
        queries::set_news_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        lines.push(format!("📰  **Gaming News** → {}", ch.id.mention()));
    }

    // ── Free Games ─────────────────────────────────────────────────────────
    if let Some(ch) = &free_games {
        queries::set_free_games_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        lines.push(format!("🎁  **Free Games** → {}", ch.id.mention()));
    }

    // ── NSFW (general) ─────────────────────────────────────────────────────
    if let Some(ch) = &nsfw {
        match ensure_nsfw(ctx, ch).await {
            Ok(auto_configured) => {
                queries::set_nsfw_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_rule34_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_porn_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_hentai_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                if auto_configured {
                    lines.push(format!("🔞  **NSFW** → {} [Automatically Set Age-Restricted]", ch.id.mention()));
                } else {
                    lines.push(format!("🔞  **NSFW** → {} *(r/nsfw, r/gonewild, r/rule34, r/hentai, r/porn)*", ch.id.mention()));
                }
            }
            Err(warn_msg) => {
                warnings.push(format!("⚠️  **NSFW** warning: {}", warn_msg));
                queries::set_nsfw_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_rule34_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_porn_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                queries::set_hentai_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                lines.push(format!("🔞  **NSFW** → {} *(not Age-Restricted yet!)*", ch.id.mention()));
            }
        }
    }

    // ── JAV ────────────────────────────────────────────────────────────────
    if let Some(ch) = &jav {
        match ensure_nsfw(ctx, ch).await {
            Ok(auto_configured) => {
                queries::set_jav_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                if auto_configured {
                    lines.push(format!("🎌  **JAV Videos** → {} [Automatically Set Age-Restricted]", ch.id.mention()));
                } else {
                    lines.push(format!("🎌  **JAV Videos** → {} *(eporner: Japanese & Asian JAV)*", ch.id.mention()));
                }
            }
            Err(warn_msg) => {
                warnings.push(format!("⚠️  **JAV** warning: {}", warn_msg));
                queries::set_jav_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                lines.push(format!("🎌  **JAV Videos** → {} *(not Age-Restricted yet!)*", ch.id.mention()));
            }
        }
    }

    // ── Porn Videos ────────────────────────────────────────────────────────
    if let Some(ch) = &porn_videos {
        match ensure_nsfw(ctx, ch).await {
            Ok(auto_configured) => {
                queries::set_porn_video_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                if auto_configured {
                    lines.push(format!("🔥  **Porn Videos** → {} [Automatically Set Age-Restricted]", ch.id.mention()));
                } else {
                    lines.push(format!("🔥  **Porn Videos** → {} *(RedTube: Brazzers, MILF, NaughtyAmerica…)*", ch.id.mention()));
                }
            }
            Err(warn_msg) => {
                warnings.push(format!("⚠️  **Porn Videos** warning: {}", warn_msg));
                queries::set_porn_video_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                lines.push(format!("🔥  **Porn Videos** → {} *(not Age-Restricted yet!)*", ch.id.mention()));
            }
        }
    }

    // ── OK.XXX ─────────────────────────────────────────────────────────────
    if let Some(ch) = &okxxx {
        match ensure_nsfw(ctx, ch).await {
            Ok(auto_configured) => {
                queries::set_okxxx_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                if auto_configured {
                    lines.push(format!("🌶️  **OK.XXX** → {} [Automatically Set Age-Restricted]", ch.id.mention()));
                } else {
                    lines.push(format!("🌶️  **OK.XXX** → {} *(top studio videos — Brazzers, Reality Kings…)*", ch.id.mention()));
                }
            }
            Err(warn_msg) => {
                warnings.push(format!("⚠️  **OK.XXX** warning: {}", warn_msg));
                queries::set_okxxx_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
                lines.push(format!("🌶️  **OK.XXX** → {} *(not Age-Restricted yet!)*", ch.id.mention()));
            }
        }
    }

    // ── Clash of Clans ─────────────────────────────────────────────────────
    if let Some(ch) = &coc {
        queries::set_coc_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        lines.push(format!("⚔️  **Clash of Clans** → {} *(updates, events & free rewards — every 10 min)*", ch.id.mention()));
    }

    // ── Twitter / X ────────────────────────────────────────────────────────
    if let Some(ch) = &twitter {
        queries::set_twitter_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
        lines.push(format!("📣  **X Updates** → {} *(@dmc_poc Global & @dmc_poc_jp Asia — every 10 min)*", ch.id.mention()));
    }

    // ── Nothing provided ───────────────────────────────────────────────────
    if lines.is_empty() && warnings.is_empty() {
        ctx.say(
            "❌ You didn't pick any channels!\n\
            Run `/setup` again and choose at least one channel to post in.\n\n\
            💡 **Tip:** All fields are optional — only fill in the ones you want!"
        ).await?;
        return Ok(());
    }

    // ── Build response ─────────────────────────────────────────────────────
    let mut response = String::from("✅ **Setup complete!**\n\n");

    if !lines.is_empty() {
        response.push_str("**Channels configured:**\n");
        for l in &lines {
            response.push_str(&format!("{}\n", l));
        }
    }

    if !warnings.is_empty() {
        response.push('\n');
        for w in &warnings {
            response.push_str(&format!("{}\n", w));
        }
    }

    if !lines.is_empty() {
        response.push_str(
            "\n⚡ Run `/post` to post content **right now** without waiting!\n\
              📊 Run `/status` to see your full configuration."
        );
    }

    ctx.say(response).await?;
    Ok(())
}

/// Helper to ensure a channel is marked as Age-Restricted (NSFW).
/// Returns Ok(true) if it had to be automatically configured as NSFW.
/// Returns Ok(false) if it was already NSFW.
/// Returns Err(String) with a warning message if it is not NSFW and we failed to edit it.
async fn ensure_nsfw(
    ctx: Context<'_>,
    ch: &serenity::GuildChannel,
) -> Result<bool, String> {
    if ch.nsfw {
        return Ok(false);
    }

    // Try to update channel to NSFW
    let builder = serenity::EditChannel::new().nsfw(true);
    match ch.id.edit(&ctx.serenity_context().http, builder).await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!(
            "{} is not Age-Restricted and the bot failed to enable it automatically: {}. Please enable it manually in Channel Settings.",
            ch.id.mention(),
            e
        )),
    }
}

