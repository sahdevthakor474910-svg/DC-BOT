use poise::serenity_prelude as serenity;
use crate::data::{Context, Error};

/// 📖 How to use the bot — quick reference for all commands.
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("📖 Quick Start Guide")
        .description(
            "This bot auto-posts **memes**, **gaming news**, **free games**, **JAV**, and **porn videos** \
            into your chosen channels — hands free!\n\n\
            There are only **4 commands** to learn:"
        )
        .color(0x5865F2)

        // ── /setup ───────────────────────────────────────────────────────────
        .field(
            "🛠️ `/setup` — Pick your channels *(first thing to do!)*",
            "Run `/setup` and fill in whichever channels you want:\n\
             • **memes** → 🖼️ r/memes, r/dankmemes, r/brainrot, r/shitposting\n\
             • **news** → 📰 Gaming news every 5 min\n\
             • **free_games** → 🎁 Epic & Steam free game alerts\n\
             • **nsfw** → 🔞 r/nsfw, r/gonewild, r/rule34, r/hentai, r/porn *(channel must be Age-Restricted!)*\n\
             • **jav** → 🎌 JAV videos from Reddit *(Age-Restricted!)*\n\
             • **porn_videos** → 🔥 RedTube: Brazzers, MILF, NaughtyAmerica *(Age-Restricted!)*\n\n\
             All fields are optional — only fill what you want!",
            false,
        )

        // ── /post ────────────────────────────────────────────────────────────
        .field(
            "⚡ `/post` — Post everything right now!",
            "Don't want to wait? `/post` instantly triggers all content.\n\
             The bot won't re-post anything it already sent.",
            false,
        )

        // ── /status ──────────────────────────────────────────────────────────
        .field(
            "📊 `/status` — See your current setup",
            "Shows all configured channels, post intervals, and reaction settings at a glance.",
            false,
        )

        // ── /settings ────────────────────────────────────────────────────────
        .field(
            "⚙️ `/settings` — Advanced options",
            "`/settings interval 60` → Post memes every 60 seconds (faster!)\n\
             `/settings auto-react` → Toggle auto-reactions ON/OFF\n\
             `/settings add-react-channel #ch` → React to all messages in a channel\n\
             `/settings add-react-user @user` → Always react to someone's messages\n\
             `/settings add-emoji 🔥` → Add emoji to the reaction pool",
            false,
        )

        // ── Tips ─────────────────────────────────────────────────────────────
        .field(
            "💡 Tips",
            "• Use `/setup` again anytime to change channels\n\
             • NSFW channels must have **Age-Restricted** enabled in Discord\n\
               *(Right-click channel → Edit Channel → Overview → Age-Restricted)*\n\
             • `/post` is great for testing — it posts immediately\n\
             • Check `/status` to confirm your setup is correct",
            false,
        )

        .footer(serenity::CreateEmbedFooter::new(
            "4 commands: /setup • /post • /status • /settings"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
