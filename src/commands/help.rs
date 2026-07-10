use poise::serenity_prelude as serenity;
use crate::data::{Context, Error};

/// 📖 Show all bot commands and how to use the bot.
#[poise::command(slash_command, guild_only)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("📖 Bot Help — All Commands")
        .description(
            "Welcome! This bot auto-posts **memes**, **gaming news**, **free games**, and **NSFW content** \
            straight into your chosen channels.\n\n\
            Use the commands below to set it up. All channel-setup commands require **Manage Server** permission."
        )
        .color(0x5865F2)

        // ── QUICK START ──────────────────────────────────────────────────────
        .field(
            "🚀 Quick Start (do this first!)",
            "```\n\
            1. /admin set-meme-channel      → pick your #memes channel\n\
            2. /admin set-news-channel      → pick your #gaming-news channel\n\
            3. /admin set-free-games-channel → pick your #free-games channel\n\
            4. /admin set-nsfw-channel      → pick your 🔞 channel (must be Age-Restricted)\n\
            5. /admin force-refresh         → trigger first post immediately\n\
            ```",
            false,
        )

        // ── /admin channel setters ───────────────────────────────────────────
        .field(
            "📺 /admin — Channel Setup  *(Manage Server required)*",
            "`/admin set-meme-channel` `#channel`\n→ Posts r/memes, r/dankmemes, r/196, r/whenthe\n\n\
             `/admin set-brainrot-channel` `#channel`\n→ Posts r/brainrot content\n\n\
             `/admin set-shitposting-channel` `#channel`\n→ Posts r/shitposting\n\n\
             `/admin set-instagram-channel` `#channel`\n→ Posts r/196 (Instagram-style memes)\n\n\
             `/admin set-news-channel` `#channel`\n→ Posts gaming news (every 15 min)\n\n\
             `/admin set-free-games-channel` `#channel`\n→ Posts free game alerts (every 30 min)",
            false,
        )

        // ── /admin NSFW ──────────────────────────────────────────────────────
        .field(
            "🔞 /admin — NSFW Channel Setup  *(channel must be Age-Restricted)*",
            "`/admin set-nsfw-channel` `#channel`\n→ General NSFW (r/nsfw, r/gonewild). Also acts as fallback for rule34/porn/hentai if those aren't set.\n\n\
             `/admin set-rule34-channel` `#channel`\n→ Dedicated channel for r/rule34\n\n\
             `/admin set-porn-channel` `#channel`\n→ Dedicated channel for r/porn\n\n\
             `/admin set-hentai-channel` `#channel`\n→ Dedicated channel for r/hentai\n\n\
             `/admin set-jav-channel` `#channel`\n→ Dedicated channel for 🎌 JAV Videos (popular and latest releases from R18.dev)\n\n\
             💡 **Tip:** If you only set `/admin set-nsfw-channel`, all NSFW content goes there (except JAV which requires a dedicated channel or won't post).\n\
             Set the others only if you want to split them into separate channels.",
            false,
        )

        // ── /admin utilities ─────────────────────────────────────────────────
        .field(
            "⚙️ /admin — Utilities",
            "`/admin status`\n→ Shows all configured channels and settings in a dashboard\n\n\
             `/admin force-refresh`\n→ Immediately posts new memes, news, free games & JAV videos right now\n\n\
             `/admin toggle-auto-react`\n→ Turn automatic emoji reactions ON or OFF for this server\n\n\
             `/admin add-reaction-user` `@user`\n→ Bot will always react to this user's messages\n\n\
             `/admin remove-reaction-user` `@user`\n→ Remove a user from auto-react list",
            false,
        )

        // ── /config ──────────────────────────────────────────────────────────
        .field(
            "🛠️ /config — Advanced Settings  *(Manage Server required)*",
            "`/config meme-channel` `#channel`\n→ Same as /admin set-meme-channel\n\n\
             `/config interval` `<seconds>`\n→ Set how often memes post (min 60s, default 300s = 5 min)\n\n\
             `/config add-reaction-channel` `#channel`\n→ Bot reacts to all messages in this channel\n\n\
             `/config remove-reaction-channel` `#channel`\n→ Stop reacting in that channel\n\n\
             `/config add-user` `@user`\n→ React to all messages from this user\n\n\
             `/config remove-user` `@user`\n→ Remove user from react list\n\n\
             `/config add-emoji` `🔥`\n→ Add an emoji to the bot's reaction pool\n\n\
             `/config remove-emoji` `🔥`\n→ Remove an emoji from the pool\n\n\
             `/config show`\n→ Show current config summary",
            false,
        )

        // ── /memes ───────────────────────────────────────────────────────────
        .field(
            "🖼️ /memes — Meme Controls",
            "`/memes status`\n→ Show which subreddits are active and the fetch interval\n\n\
             `/memes fetch-now`\n→ Manually trigger a meme fetch right now *(Manage Server)*",
            false,
        )

        // ── /ping ────────────────────────────────────────────────────────────
        .field(
            "🏓 Other",
            "`/ping`\n→ Check if the bot is alive and see response latency\n\n\
             `/help`\n→ Show this help message",
            false,
        )

        // ── What the bot watches ─────────────────────────────────────────────
        .field(
            "📡 What Subreddits and Sources Does the Bot Watch?",
            "**SFW Memes:** r/memes • r/dankmemes • r/shitposting • r/brainrot • r/196 • r/whenthe\n\
             **NSFW General:** r/nsfw • r/gonewild\n\
             **NSFW Rule34:** r/rule34\n\
             **NSFW Porn:** r/porn\n\
             **NSFW Hentai:** r/hentai\n\
             **🎌 JAV Videos:** Popular & latest releases from R18.dev\n\n\
             🕐 **Post frequency:** Memes every ~5 min • News every 15 min • Free Games every 30 min • JAV every 2 hours",
            false,
        )

        // ── NSFW setup tip ───────────────────────────────────────────────────
        .field(
            "⚠️ How to Make an Age-Restricted Channel",
            "Right-click the channel → **Edit Channel** → **Overview** → Enable **Age-Restricted Channel**\n\
             The bot will refuse to post NSFW/JAV content in non-age-restricted channels.",
            false,
        )

        .footer(serenity::CreateEmbedFooter::new(
            "Tip: Use /admin status anytime to see your full setup at a glance."
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
