use crate::data::{Context, Error};

/// Simple health-check command.
///
/// Usage: `/ping`
#[poise::command(slash_command, description_localized("en-US", "Check bot latency"))]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let now = std::time::Instant::now();
    let msg = ctx.say("🏓 Pinging…").await?;
    let elapsed = now.elapsed().as_millis();

    msg.edit(ctx, poise::CreateReply::default().content(format!(
        "🏓 Pong! Round-trip: **{}ms**",
        elapsed
    )))
    .await?;

    Ok(())
}
