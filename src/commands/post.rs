use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::data::{Context, Error};
use crate::reddit;
use crate::news;
use crate::freegames;
use crate::jav;
use crate::porn;
use crate::okxxx;
use crate::coc;

#[derive(poise::ChoiceParameter, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    #[name = "Memes"]
    Memes,
    #[name = "Gaming News"]
    News,
    #[name = "Free Games"]
    FreeGames,
    #[name = "JAV Videos"]
    Jav,
    #[name = "Porn Videos"]
    Porn,
    #[name = "OK.XXX Videos"]
    Okxxx,
    #[name = "CoC Updates"]
    Coc,
    #[name = "X/Twitter Updates"]
    Twitter,
}

// ─────────────────────────────────────────────────────────────────────────────
// /post — instantly post all content right now
// ─────────────────────────────────────────────────────────────────────────────

/// ⚡ Post memes, news, free games & NSFW content right now — don't wait for the timer!
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check"
)]
pub async fn post(
    ctx: Context<'_>,
    #[description = "Specific category to post"]
    category: Option<ContentType>,
    #[description = "Force post even if already posted/seen before (default: false)"]
    force: Option<bool>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data().clone();
    let http: Arc<serenity::Http> = Arc::clone(&ctx.serenity_context().http);
    let force_val = force.unwrap_or(false);

    let mut meme_n = 0;
    let mut news_n = 0;
    let mut fg_n = 0;
    let mut jav_n = 0;
    let mut porn_n = 0;
    let mut okxxx_n = 0;
    let mut coc_n = 0;
    let mut twitter_n = 0;

    if let Some(cat) = category {
        match cat {
            ContentType::Memes => {
                meme_n = reddit::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("Meme refresh: {:#}", e); 0 });
            }
            ContentType::News => {
                news_n = news::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("News refresh: {:#}", e); 0 });
            }
            ContentType::FreeGames => {
                fg_n = freegames::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("FG refresh: {:#}", e); 0 });
            }
            ContentType::Jav => {
                jav_n = jav::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("JAV refresh: {:#}", e); 0 });
            }
            ContentType::Porn => {
                porn_n = porn::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("Porn refresh: {:#}", e); 0 });
            }
            ContentType::Okxxx => {
                okxxx_n = okxxx::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("OK.XXX refresh: {:#}", e); 0 });
            }
            ContentType::Coc => {
                coc_n = coc::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("CoC refresh: {:#}", e); 0 });
            }
            ContentType::Twitter => {
                twitter_n = crate::twitter::task::run_once(&data, &http, force_val).await.unwrap_or_else(|e| { tracing::error!("Twitter refresh: {:#}", e); 0 });
            }
        }
    } else {
        let (meme_res, news_res, fg_res, jav_res, porn_res, okxxx_res, coc_res, twitter_res) = tokio::join!(
            reddit::task::run_once(&data, &http, force_val),
            news::task::run_once(&data, &http, force_val),
            freegames::task::run_once(&data, &http, force_val),
            jav::task::run_once(&data, &http, force_val),
            porn::task::run_once(&data, &http, force_val),
            okxxx::task::run_once(&data, &http, force_val),
            coc::task::run_once(&data, &http, force_val),
            crate::twitter::task::run_once(&data, &http, force_val),
        );

        meme_n = meme_res.unwrap_or_else(|e| { tracing::error!("Meme refresh: {:#}", e); 0 });
        news_n = news_res.unwrap_or_else(|e| { tracing::error!("News refresh: {:#}", e); 0 });
        fg_n   = fg_res.unwrap_or_else(|e|   { tracing::error!("FG refresh: {:#}", e);   0 });
        jav_n  = jav_res.unwrap_or_else(|e|  { tracing::error!("JAV refresh: {:#}", e);  0 });
        porn_n = porn_res.unwrap_or_else(|e| { tracing::error!("Porn refresh: {:#}", e); 0 });
        okxxx_n = okxxx_res.unwrap_or_else(|e| { tracing::error!("OK.XXX refresh: {:#}", e); 0 });
        coc_n   = coc_res.unwrap_or_else(|e| { tracing::error!("CoC refresh: {:#}", e); 0 });
        twitter_n = twitter_res.unwrap_or_else(|e| { tracing::error!("Twitter refresh: {:#}", e); 0 });
    }

    let total = meme_n + news_n + fg_n + jav_n + porn_n + okxxx_n + coc_n + twitter_n;

    if total == 0 {
        ctx.say(
            "📭 Nothing new to post — everything is already up to date!\n\n\
            *The bot tracks what it has already posted to avoid duplicates. \
            New content will appear automatically on the next cycle.*"
        ).await?;
        return Ok(());
    }

    let mut embed = serenity::CreateEmbed::new()
        .title(if category.is_some() { "⚡ Category Posted Right Now!" } else { "⚡ Posted Right Now!" })
        .color(0x57F287); // Discord green

    if category.is_none() || category == Some(ContentType::Memes) {
        embed = embed.field("🖼️ Memes", meme_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::News) {
        embed = embed.field("📰 Gaming News", news_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::FreeGames) {
        embed = embed.field("🎁 Free Games", fg_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::Jav) {
        embed = embed.field("🎌 JAV Videos", jav_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::Porn) {
        embed = embed.field("🔥 Porn Videos", porn_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::Okxxx) {
        embed = embed.field("🌶️ OK.XXX Videos", okxxx_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::Coc) {
        embed = embed.field("⚔️ CoC Updates", coc_n.to_string(), true);
    }
    if category.is_none() || category == Some(ContentType::Twitter) {
        embed = embed.field("📣 X/Twitter", twitter_n.to_string(), true);
    }

    embed = embed.field("📬 Total", total.to_string(), true)
        .footer(serenity::CreateEmbedFooter::new(
            "New content posts automatically — use /post anytime to force it!"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
