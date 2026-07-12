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
        if !ch.nsfw {
            warnings.push(format!(
                "⚠️  **NSFW** skipped — {} is **not Age-Restricted**! \
                Enable it via: Channel Settings → Overview → Age-Restricted Channel.",
                ch.id.mention()
            ));
        } else {
            queries::set_nsfw_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            queries::set_rule34_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            queries::set_porn_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            queries::set_hentai_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            lines.push(format!("🔞  **NSFW** → {} *(r/nsfw, r/gonewild, r/rule34, r/hentai, r/porn)*", ch.id.mention()));
        }
    }

    // ── JAV ────────────────────────────────────────────────────────────────
    if let Some(ch) = &jav {
        if !ch.nsfw {
            warnings.push(format!(
                "⚠️  **JAV** skipped — {} is **not Age-Restricted**!",
                ch.id.mention()
            ));
        } else {
            queries::set_jav_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            lines.push(format!("🎌  **JAV Videos** → {} *(eporner: Japanese & Asian JAV)*", ch.id.mention()));
        }
    }

    // ── Porn Videos ────────────────────────────────────────────────────────
    if let Some(ch) = &porn_videos {
        if !ch.nsfw {
            warnings.push(format!(
                "⚠️  **Porn Videos** skipped — {} is **not Age-Restricted**!",
                ch.id.mention()
            ));
        } else {
            queries::set_porn_video_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            lines.push(format!("🔥  **Porn Videos** → {} *(RedTube: Brazzers, MILF, NaughtyAmerica…)*", ch.id.mention()));
        }
    }

    // ── OK.XXX ─────────────────────────────────────────────────────────────
    if let Some(ch) = &okxxx {
        if !ch.nsfw {
            warnings.push(format!(
                "⚠️  **OK.XXX** skipped — {} is **not Age-Restricted**!",
                ch.id.mention()
            ));
        } else {
            queries::set_okxxx_channel(db, &guild_id, Some(ch.id.to_string().as_str())).await?;
            lines.push(format!("🌶️  **OK.XXX** → {} *(top studio videos — Brazzers, Reality Kings…)*", ch.id.mention()));
        }
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
