use super::gemini::{LeaderboardPlayer, ScreenshotData};

// ─────────────────────────────────────────────────────────────────────────────
// Boss constants
// ─────────────────────────────────────────────────────────────────────────────

/// Normalizes a boss name by removing all non-alphanumeric characters and converting to lowercase.
fn normalize_boss_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Known max damage points per boss (i.e. their HP pool).
fn boss_dmg_pts(name: &str) -> i64 {
    let norm = normalize_boss_name(name);
    if norm.contains("devilmite")           { 1_022_497_809 }
    else if norm.contains("cerberus")        { 1_335_976_271 }
    else if norm.contains("minotaur")        { 951_865_962   }
    else if norm.contains("nevan")           { 864_589_190   }
    else if norm.contains("hellshade")       { 905_640_916   }
    else if norm.contains("beowulf")         { 946_374_652   }
    else if norm.contains("plutone")         { 934_691_016   }
    else if norm.contains("calibur")         { 1_022_497_809 }
    else if norm.contains("vergil")          { 2_892_440_140 }
    else if norm.contains("dante")           { 2_892_440_140 }
    else if norm.contains("hellcommander")   { 2_892_440_140 }
    else { 1_022_497_809 } // safe fallback
}

/// Battle time limit in seconds.
fn boss_time_limit(name: &str) -> f64 {
    let norm = normalize_boss_name(name);
    if norm.contains("vergil")
        || norm.contains("dante")
        || norm.contains("hellcommander")
    {
        300.0
    } else {
        240.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core math
// ─────────────────────────────────────────────────────────────────────────────

fn calc_stats_internal(
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

/// Given a leaderboard total_pts (and initial assumption of 120% bonus active),
/// return (reward_pts, secs_remaining, kill_time_secs, dps, resolved_has_bonus).
///
/// If our initial assumption yields an impossible kill time (negative or exceeding limit),
/// it automatically toggles the bonus parameter to find the correct fit.
fn calc_stats_leaderboard(
    total_pts: i64,
    dmg_pts: i64,
    has_bonus: bool,
    time_limit: f64,
) -> (f64, f64, f64, f64, bool) {
    // 1. Try with the parsed bonus setting
    let (reward, secs_rem, kill, dps) = calc_stats_internal(total_pts, dmg_pts, has_bonus, time_limit);
    if kill >= 0.0 && kill <= time_limit {
        return (reward, secs_rem, kill, dps, has_bonus);
    }

    // 2. Try the opposite bonus setting
    let alt_bonus = !has_bonus;
    let (reward_alt, secs_rem_alt, kill_alt, dps_alt) = calc_stats_internal(total_pts, dmg_pts, alt_bonus, time_limit);
    if kill_alt >= 0.0 && kill_alt <= time_limit {
        return (reward_alt, secs_rem_alt, kill_alt, dps_alt, alt_bonus);
    }

    // 3. Fallback to the original calculation
    (reward, secs_rem, kill, dps, has_bonus)
}

/// For a results screen where DMG PTS is directly provided.
/// The Results screen always displays the raw base points, so we do not divide by the 1.20x bonus factor here.
fn calc_stats_results(
    dmg_pts: i64,
    boss_pts: i64,
    _has_bonus: bool,
    time_limit: f64,
) -> (f64, f64, f64, f64) {
    let reward_pts = boss_pts as f64 - dmg_pts as f64;

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
        let s = secs % 60.0;
        format!("{}m {:.1}s", mins, s)
    } else {
        format!("{:.1}s", secs)
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

    // Auto-detect resolved has_bonus state based on the first player's score
    let mut resolved_has_bonus = has_bonus;
    if !players.is_empty() {
        let (_, _, _, _, actual_bonus) = calc_stats_leaderboard(players[0].total_pts, dmg_pts, has_bonus, time_limit);
        resolved_has_bonus = actual_bonus;
    }

    let bonus_str = if resolved_has_bonus { "X120% | " } else { "" };
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

        let (_, _, kill_time, dps, _) =
            calc_stats_leaderboard(player.total_pts, dmg_pts, resolved_has_bonus, time_limit);

        if kill_time < 0.0 {
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
