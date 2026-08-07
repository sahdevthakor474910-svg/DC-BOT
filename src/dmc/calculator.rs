use super::gemini::{BossOverviewEntry, LeaderboardPlayer, ScreenshotData};

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
/// All standard bosses share 5.07 B HP; Vergil / Dante / Hell Commander keep their old value.
fn boss_dmg_pts(name: &str) -> i64 {
    let norm = normalize_boss_name(name);
    if norm.contains("vergil")
        || norm.contains("dante")
        || norm.contains("hellcommander") { 2_892_440_140 }
    else if norm.contains("lady")         { 9_038_840_000 } // 5-min boss, own HP pool
    else if norm.contains("plutone")      { 5_783_842_000 } // calibrated from results screen
    else                                  { 5_070_000_000 } // remaining standard bosses
}


/// Battle time limit in seconds.
fn boss_time_limit(name: &str) -> f64 {
    let norm = normalize_boss_name(name);
    if norm.contains("vergil")
        || norm.contains("dante")
        || norm.contains("hellcommander")
        || norm.contains("lady")
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
        total_pts as f64 / 1.20 - dmg_pts as f64
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

/// For a results screen where Reward PTS is directly provided by Gemini.
///
/// Returns `(reward_pts, secs_remaining, kill_time, dps, boss_killed)`.
/// `boss_killed = false` when the player timed out without defeating the boss —
/// the caller shows a dedicated "❌ Boss Not Killed" message in that case.
fn calc_stats_results(
    dmg_pts_raw: i64,
    reward_pts_direct: i64,
    boss_pts_raw: i64,
    _has_bonus: bool,
    time_limit: f64,
    boss_name: &str,
) -> (f64, f64, f64, f64, bool) {
    let hp_cap = boss_dmg_pts(boss_name);

    // ── Sanity check 1: auto-swap if Gemini returned rows in wrong order ──
    let (dmg_pts, boss_pts) = if boss_pts_raw > 0 && dmg_pts_raw > 0 && boss_pts_raw < dmg_pts_raw {
        tracing::warn!(
            "calc_stats_results: boss_pts ({}) < dmg_pts ({}) for {} — auto-swapping fields",
            boss_pts_raw, dmg_pts_raw, boss_name
        );
        (boss_pts_raw, dmg_pts_raw)
    } else {
        (dmg_pts_raw, boss_pts_raw)
    };

    // ── Sanity check 2: filter out implausible reward_pts_direct (> 20M) ──
    // Max possible time reward for 300s limit is ~14.68M.
    let valid_reward_direct = if reward_pts_direct > 0 && reward_pts_direct <= 20_000_000 {
        reward_pts_direct
    } else {
        if reward_pts_direct > 20_000_000 {
            tracing::warn!(
                "calc_stats_results: reward_pts_direct ({}) > 20M limit for {} — ignoring invalid Gemini value",
                reward_pts_direct, boss_name
            );
        }
        0
    };

    // ── Primary: use the directly-read Reward PTS ──────────────────────────
    let reward_pts: f64 = if valid_reward_direct > 0 {
        valid_reward_direct as f64
    } else {
        // ── Boss-not-killed shortcut ───────────────────────────────────────
        // reward_pts = 0 AND dmg < hp_cap → player timed out, boss survived.
        // DPS is still useful, computed over the full time limit.
        if dmg_pts < hp_cap {
            tracing::info!(
                "calc_stats_results: reward_pts=0 and dmg_pts ({}) < hp_cap ({}) for {} — boss not killed",
                dmg_pts, hp_cap, boss_name
            );
            let dps = dmg_pts as f64 / time_limit;
            return (0.0, 0.0, time_limit, dps, false); // boss_killed = false
        }

        // ── Fallback: boss WAS killed (dmg >= hp_cap), Gemini missed the row ─
        let computed = (boss_pts as f64 - dmg_pts as f64).max(0.0);
        if computed > 0.0 {
            tracing::warn!(
                "calc_stats_results: reward_pts=0 from Gemini for {} (full clear) — falling back to boss_pts-dmg_pts ({:.0})",
                boss_name, computed
            );
        }
        computed
    };

    let secs_remaining = (reward_pts * 10.0) / 489_530.0;
    let kill_time = time_limit - secs_remaining;

    // ── Plausibility guard ─────────────────────────────────────────────────
    // kill_time outside [0, time_limit] means the data is still corrupt.
    let (reward_pts, secs_remaining, kill_time) = if kill_time < 0.0 || kill_time > time_limit {
        tracing::warn!(
            "calc_stats_results: implausible kill_time ({:.1}s) for {} — clamping via HP cap",
            kill_time, boss_name
        );
        let cap = hp_cap as f64;
        let rp = (cap - dmg_pts as f64).max(0.0);
        let sr = (rp * 10.0) / 489_530.0;
        let kt = (time_limit - sr).max(0.0);
        (rp, sr, kt)
    } else {
        (reward_pts, secs_remaining, kill_time)
    };

    let dps = if kill_time > 0.0 {
        dmg_pts as f64 / kill_time
    } else {
        0.0
    };

    (reward_pts, secs_remaining, kill_time, dps, true) // boss_killed = true
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
    reward_pts: i64,
    boss_pts: i64,
    has_bonus: bool,
) -> String {
    let time_limit = boss_time_limit(boss_name);
    let (reward_pts_f, secs_remaining, kill_time, dps, boss_killed) =
        calc_stats_results(dmg_pts, reward_pts, boss_pts, has_bonus, time_limit, boss_name);

    if !boss_killed {
        // Player timed out — boss survived. Show a clear "not killed" summary.
        return format!(
            "```\n\
╔══════════════════════════════════════╗\n\
      DMC - {} Results\n\
╠══════════════════════════════════════╣\n\
  Status      : ❌ Boss Not Killed\n\
  DMG PTS     : {}\n\
  DPS         : {:.0}  (over full {}s)\n\
  Bonus       : {}\n\
╚══════════════════════════════════════╝\n\
```",
            boss_name,
            dmg_pts,
            dps,
            time_limit as u64,
            if has_bonus { "X120% ✓" } else { "None" }
        );
    }

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
        reward_pts_f,
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

        if kill_time < 0.0 || kill_time > time_limit {
            out.push_str(&format!(
                "\n {} {}\n    Total PTS : {}\n    Kill Time : ❌ Boss Not Killed\n\
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

/// Format the boss-overview screen reply.
/// Each boss card shows name, PTS, and computed kill time.
fn format_boss_overview(bosses: &[BossOverviewEntry]) -> String {
    if bosses.is_empty() {
        return "```\n⚠️ No boss data found in screenshot.\n```".to_string();
    }

    let mut out = String::from(
        "```\n\
╔══════════════════════════════════════╗\n\
      DMC - Boss Overview\n\
╠══════════════════════════════════════╣",
    );

    for entry in bosses {
        let time_limit = boss_time_limit(&entry.boss_name);
        let dmg_cap = boss_dmg_pts(&entry.boss_name);

        // The PTS on the card is a total_pts (includes time bonus if boss killed).
        // Use the leaderboard formula: back-calculates reward_pts → secs_remaining → kill_time.
        let (_, _, kill_time, dps, resolved_bonus) =
            calc_stats_leaderboard(entry.pts, dmg_cap, entry.has_bonus, time_limit);

        let bonus_label = if resolved_bonus { " [X120%]" } else { "" };
        let time_str = if time_limit >= 300.0 { "5min" } else { "4min" };

        if kill_time < 0.0 || kill_time > time_limit {
            out.push_str(&format!(
                "\n  {}{} ({}limit)\n    PTS       : {}\n    Kill Time : ❌ Boss Not Killed\n\
╠──────────────────────────────────────╣",
                entry.boss_name, bonus_label, time_str, entry.pts
            ));
        } else {
            out.push_str(&format!(
                "\n  {}{} ({}limit)\n    PTS       : {}\n    Kill Time : {}\n    DPS       : {:.0}\n\
╠──────────────────────────────────────╣",
                entry.boss_name,
                bonus_label,
                time_str,
                entry.pts,
                format_kill_time(kill_time),
                dps
            ));
        }
    }

    out.push_str(
        "\n⚠️ Kill times estimated from card PTS scores\n\
╚══════════════════════════════════════╝\n\
```",
    );

    out
}



/// Take the parsed [`ScreenshotData`] and return the ready-to-send Discord
/// message string.
pub fn build_discord_message(data: &ScreenshotData) -> String {
    match data {
        ScreenshotData::Results {
            boss_name,
            dmg_pts,
            reward_pts,
            boss_pts,
            has_bonus,
        } => format_results(boss_name, *dmg_pts, *reward_pts, *boss_pts, *has_bonus),

        ScreenshotData::Leaderboard {
            boss_name,
            has_bonus,
            players,
        } => format_leaderboard(boss_name, *has_bonus, players),

        ScreenshotData::BossOverview { bosses } => format_boss_overview(bosses),
    }
}
