use crate::data::{Context, Error};

/// Simple health-check command.
///
/// Usage: `/ping`
#[poise::command(slash_command, description_localized("en-US", "Check bot latency"))]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let shard_latency = ctx.ping().await;
    let latency_ms = shard_latency.as_millis();

    ctx.say(format!(
        "🏓 Pong! Gateway Latency: **{}ms**",
        latency_ms
    ))
    .await?;

    Ok(())
}
