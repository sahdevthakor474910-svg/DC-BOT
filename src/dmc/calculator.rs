use super::gemini::{LeaderboardPlayer, ScreenshotData};

// ─────────────────────────────────────────────────────────────────────────────
// Boss constants
// ─────────────────────────────────────────────────────────────────────────────

/// Known max damage points per boss (i.e. their HP pool).
fn boss_dmg_pts(name: &str) -> i64 {
    let n = name.to_lowercase();
    if n.contains("devil mite")           { 1_022_497_809 }
    else if n.contains("cerberus")        { 1_335_976_271 }
    else if n.contains("minotaur")        { 951_865_962   }
    else if n.contains("nevan")           { 864_589_190   }
    else if n.contains("hell shade")      { 905_640_916   }
    else if n.contains("beowulf")         { 946_374_652   }
    else if n.contains("plutone")         { 934_691_016   }
    else if n.contains("calibur")         { 1_022_497_809 }
    else if n.contains("vergil")          { 2_892_440_140 }
    else if n.contains("dante")           { 2_892_440_140 }
    else if n.contains("hell commander") || n.contains("hell·commander") { 2_892_440_140 }
    else { 1_022_497_809 } // safe fallback
}

/// Battle time limit in seconds.
fn boss_time_limit(name: &str) -> f64 {
    let n = name.to_lowercase();
    if n.contains("vergil")
        || n.contains("dante")
        || n.contains("hell commander")
        || n.contains("hell·commander")
    {
        300.0
    } else {
        240.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core math
// ─────────────────────────────────────────────────────────────────────────────

/// Given a leaderboard total_pts (and whether the 120% bonus is active),
/// return (reward_pts, secs_remaining, kill_time_secs, dps).
///
/// kill_time_secs is negative when the total_pts is lower than the boss HP
/// used in our formula (i.e. the player used a different damage cap).
fn calc_stats(
    total_pts: i64,
    dmg_pts: i64,
    has_bonus: bool,
    time_limit: f64,
) -> (f64, f64, f64, f64) {
    let reward_pts = if has_bonus {
        let pre_bonus = total_pts as f64 / 1.20;
        pre_bonus - dmg_pts as f64
    } else {
        total_pts as f64 - dmg_pts as f64
    };

    let secs_remaining = (reward_pts * 10.0) / 489_530.0;
    let kill_time = time_limit - secs_remaining;
    let dps = if kill_time > 0.0 {
        dmg_pts as f64 / kill_time
    } else {
        0.0
    };

    (reward_pts, secs_remaining, kill_time, dps)
}

/// For a results screen where DMG PTS is directly provided.
fn calc_stats_results(
    dmg_pts: i64,
    boss_pts: i64,
    has_bonus: bool,
    time_limit: f64,
) -> (f64, f64, f64, f64) {
    let reward_pts = if has_bonus {
        let pre_bonus = boss_pts as f64 / 1.20;
        pre_bonus - dmg_pts as f64
    } else {
        boss_pts as f64 - dmg_pts as f64
    };

    let secs_remaining = (reward_pts * 10.0) / 489_530.0;
    let kill_time = time_limit - secs_remaining;
    let dps = if kill_time > 0.0 {
        dmg_pts as f64 / kill_time
    } else {
        0.0
    };

    (reward_pts, secs_remaining, kill_time, dps)
}

// ─────────────────────────────────────────────────────────────────────────────
// Time formatting
// ─────────────────────────────────────────────────────────────────────────────

fn format_kill_time(secs: f64) -> String {
    if secs <= 0.0 {
        "0s".to_string()
    } else if secs >= 60.0 {
        let mins = (secs / 60.0) as u64;
        let s = (secs % 60.0) as u64;
        format!("{}m {}s", mins, s)
    } else {
        format!("{:.0}s", secs)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Discord message builders
// ─────────────────────────────────────────────────────────────────────────────

/// Format the results-screen reply.
fn format_results(
    boss_name: &str,
    dmg_pts: i64,
    boss_pts: i64,
    has_bonus: bool,
) -> String {
    let time_limit = boss_time_limit(boss_name);
    let (reward_pts, secs_remaining, kill_time, dps) =
        calc_stats_results(dmg_pts, boss_pts, has_bonus, time_limit);

    format!(
        "```\n\
╔══════════════════════════════════════╗\n\
      DMC - {} Results\n\
╠══════════════════════════════════════╣\n\
  Boss PTS    : {}\n\
  Kill Time   : {}\n\
  DPS         : {:.0}\n\
  Reward PTS  : {:.0}\n\
  Secs Left   : {:.1}s\n\
  Bonus       : {}\n\
╚══════════════════════════════════════╝\n\
```",
        boss_name,
        boss_pts,
        format_kill_time(kill_time),
        dps,
        reward_pts,
        secs_remaining,
        if has_bonus { "X120% ✓" } else { "None" }
    )
}

/// Format the leaderboard-screen reply.
fn format_leaderboard(
    boss_name: &str,
    has_bonus: bool,
    players: &[LeaderboardPlayer],
) -> String {
    let time_limit = boss_time_limit(boss_name);
    let dmg_pts = boss_dmg_pts(boss_name);
    let bonus_str = if has_bonus { "X120% | " } else { "" };
    let time_str = if time_limit >= 300.0 { "5min" } else { "4min" };

    const RANK_EMOJIS: [&str; 10] = [
        "🥇", "🥈", "🥉", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟",
    ];

    let mut out = format!(
        "```\n\
╔══════════════════════════════════════╗\n\
      DMC - {} Leaderboard\n\
      {}Time Limit: {}\n\
╠══════════════════════════════════════╣",
        boss_name, bonus_str, time_str
    );

    for player in players {
        let emoji = RANK_EMOJIS
            .get((player.rank as usize).saturating_sub(1))
            .unwrap_or(&"🔢");

        let (_, _, kill_time, dps) =
            calc_stats(player.total_pts, dmg_pts, has_bonus, time_limit);

        if kill_time < 0.0 {
            // Negative kill time means the player's total_pts doesn't match
            // our expected DMG PTS constant → different cap / assumption.
            out.push_str(&format!(
                "\n {} {}\n    Total PTS : {}\n    Kill Time : ❌ Different DMG cap\n\
╠──────────────────────────────────────╣",
                emoji, player.name, player.total_pts
            ));
        } else {
            out.push_str(&format!(
                "\n {} {}\n    Total PTS : {}\n    Kill Time : {}\n    DPS       : {:.0}\n\
╠──────────────────────────────────────╣",
                emoji,
                player.name,
                player.total_pts,
                format_kill_time(kill_time),
                dps
            ));
        }
    }

    out.push_str(
        "\n⚠️ Kill times estimated using known DMG PTS\n\
╚══════════════════════════════════════╝\n\
```",
    );

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Take the parsed [`ScreenshotData`] and return the ready-to-send Discord
/// message string.
pub fn build_discord_message(data: &ScreenshotData) -> String {
    match data {
        ScreenshotData::Results {
            boss_name,
            dmg_pts,
            boss_pts,
            has_bonus,
        } => format_results(boss_name, *dmg_pts, *boss_pts, *has_bonus),

        ScreenshotData::Leaderboard {
            boss_name,
            has_bonus,
            players,
        } => format_leaderboard(boss_name, *has_bonus, players),
    }
}
