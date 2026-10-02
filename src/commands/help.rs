use poise::serenity_prelude as serenity;
use crate::data::{Context, Error};

/// 📖 How to use the bot — quick reference for all commands.
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?; // prevent Discord 3-second timeout on Render cold-starts
    let embed = serenity::CreateEmbed::new()
        .title("📖 Quick Start Guide")
        .description(
            "This bot auto-posts **memes**, **gaming news**, **free games**, **NSFW contents**, **X / Twitter updates**, \
            and **Clash of Clans updates** into your chosen channels — hands free!\n\n\
            Configure and manage the bot with these simple commands:"
        )
        .color(0x5865F2)

        // ── /setup & /unset ──────────────────────────────────────────────────
        .field(
            "🛠️ Setup & Channel Config (`/setup` & `/unset`)",
            "Fill in whichever channels you want to activate (all optional):\n\
             • **memes** → 🖼️ r/memes, r/dankmemes, r/brainrot, r/shitposting\n\
             • **news** → 📰 Gaming news every 5 min\n\
             • **free_games** → 🎁 Epic & Steam free game alerts\n\
             • **coc** → ⚔️ Clash of Clans: updates, news, free rewards every 10 min\n\
             • **twitter_global** → 🌍 X updates of @dmc_poc every 10 min\n\
             • **twitter_asia** → 🌏 X updates of @dmc_poc_jp every 10 min\n\
             • **dmc** → 🎮 DMC boss results leaderboard screenshot calculator\n\
             • **hot_photos** → 📸 18+ amateur slides *(Age-Restricted!)*\n\
             • **jav** → 🎌 JAV videos from Eporner *(Age-Restricted!)*\n\
             • **porn_videos** → 🔥 RedTube: Brazzers, MILF, NaughtyAmerica *(Age-Restricted!)*\n\
             • **okxxx** → 🌶️ OK.XXX: top studio movies *(Age-Restricted!)*\n\
             • **xnxx** → 🔥 XNXX: trending videos *(Age-Restricted!)*\n\
             • **javhd** → 🎌 JAVHD: featured videos *(Age-Restricted!)*\n\
             • **hanime** → 🔮 Hentai: anime & 3D hentai videos with direct video player *(Age-Restricted!)*\n\
             • **pornclips** → 🎬 Short porn clips/reels (RedGIFs — every 10 min) *(Age-Restricted!)*\n\
             • **nsfw** → 🔞 r/nsfw, r/rule34, r/hentai, r/porn *(Age-Restricted!)*\n\n\
             💡 **Stop posting:** Use `/unset` and check **True** next to any channel to disable it.",
            false,
        )

        // ── /post, /status, /ping, /roast, /safemode, /response ────────────
        .field(
            "⚡ Utility Commands",
            "• `/post` — Instantly triggers all active content feeds right now (without double posting)\n\
             • `/status` — Displays configured channels, current reaction lists, and block settings\n\
             • `/ping` — Checks the bot server response latency\n\
             • `/roast [@user]` — 🔥 Unleashes a savage Desi / Hindi roast on someone or yourself!\n\
             • `/safemode [enabled]` — 🛡️ Toggles Safe Mode ON/OFF (clean PG mode vs savage slangs/gali mode)\n\
             • `/response [enabled]` — 💬 Toggles all chat responses ON/OFF (completely silence the bot in chat)",
            false,
        )

        // ── AI Assistant & Server Management ─────────────────────────────────
        .field(
            "🤖 AI Assistant & Server Management",
            "• **Chat & Q&A:** Mention `@Honored one` or reply to any bot message to ask questions or chat!\n\
             • **Gali & Slang Detection:** If anyone uses bad words or slangs in chat, the bot automatically claps back with savage Desi roasts!\n\
             • **Safe Mode:** Ask `@Honored one enable safe mode` to switch to clean PG mode anytime!\n\
             • **Toggle Responses:** Use `/response false` if you want the bot to sit quietly without replying to anyone in chat!\n\
             • **Owner Server Control:** The server owner can ask the bot to manage the server (e.g. `@Honored one create a channel called chill-zone`, `@Honored one delete channel #test`, `@Honored one set slowmode to 5s`). Non-owners cannot run management actions.",
            false,
        )

        // ── /settings ────────────────────────────────────────────────────────
        .field(
            "⚙️ Bot Settings (`/settings <subcommand>`)",
            "• **interval** → `/settings interval <seconds>` (adjust posting frequency; min 60s)\n\
             • **response** → `/settings response [enabled]` (toggle all chat responses ON/OFF)\n\
             • **safe-mode** → `/settings safe-mode [enabled]` (toggle clean PG mode ON/OFF)\n\
             • **gali-response** → `/settings gali-response` (toggle automatic slang/gali roast responses ON/OFF)\n\
             • **clear-cache** → `/settings clear-cache` (wipe posting history to re-post hottest entries)\n\n\
             **Reaction settings:**\n\
             • `/settings auto-react` → Enforce/disable automatic reactions globally\n\
             • `/settings add-react-channel`/`remove-react-channel` → Auto-react in an entire channel\n\
             • `/settings add-react-user`/`remove-react-user` → Auto-react to a specific member's posts\n\
             • `/settings add-emoji`/`remove-emoji` → Manage the emoji pool used for reactions\n\n\
             **Admin & Backup settings:**\n\
             • `/settings export-setup` → Downloads a JSON backup file of your current configuration\n\
             • `/settings import-setup` → Uploads a JSON backup file to immediately restore your configurations\n\
             • `/settings block-user`/`unblock-user` → Prevent/allow specific users from invoking slash commands\n\
             • `/settings blocked-list` → List all currently blocked users",
            false,
        )

        // ── Tips ─────────────────────────────────────────────────────────────
        .field(
            "💡 Tips",
            "• NSFW channels must have **Age-Restricted** enabled in Discord Settings\n\
             • Commands are restrictable by permissions (Administrator only by default)\n\
             • If calculations fail in the DMC analysis channel, ensure the boss name matches the screenshot",
            false,
        )

        .footer(serenity::CreateEmbedFooter::new(
            "Quick Reference: /setup • /unset • /post • /status • /settings • /safemode • /response • /help"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
