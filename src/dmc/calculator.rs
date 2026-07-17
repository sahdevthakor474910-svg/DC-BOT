use super::gemini::BossResult;

// ─────────────────────────────────────────────────────────────────────────────
// Boss time limits (seconds)
// ─────────────────────────────────────────────────────────────────────────────

/// Normalizes a boss name by removing all non-alphanumeric characters and converting to lowercase.
fn normalize_boss_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Returns the total time limit in seconds for the given boss name.
fn boss_time_limit(name: &str) -> u64 {
    let norm = normalize_boss_name(name);
    match norm.as_str() {
        "vergil" | "dante" | "hellcommander" => 300,
        _ => 240, // devilmite, cerberus, calibur, minotaur, nevan, hellshade, beowulf, plutone
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Final computed stats
// ─────────────────────────────────────────────────────────────────────────────

pub struct BossStats {
    pub boss_name: String,
    pub total_damage: i64,   // DMG PTS
    pub kill_time_secs: f64, // seconds it took to kill the boss
    pub kill_time_fmt: String,
    pub dps: f64,
    pub boss_pts: i64,
    pub has_bonus: bool,
    pub reward_pts: i64,
    pub secs_remaining: f64,
    pub boss_time_limit: u64,
}

fn boss_max_hp(name: &str) -> i64 {
    let limit = boss_time_limit(name);
    if limit == 300 {
        2_892_440_140
    } else {
        1_022_497_809
    }
}

impl BossStats {
    pub fn compute(result: &BossResult) -> Self {
        // 1. Infer has_bonus if not provided
        let has_bonus = result.has_bonus.unwrap_or_else(|| {
            let max_hp = boss_max_hp(&result.boss_name);
            result.boss_pts > (max_hp as f64 * 1.1).round() as i64
        });

        // 2. Infer dmg_pts if not provided (e.g. from a leaderboard)
        let total_damage = result.dmg_pts.unwrap_or_else(|| {
            let max_hp = boss_max_hp(&result.boss_name);
            let score_to_check = if has_bonus {
                (result.boss_pts as f64 / 1.20).round() as i64
            } else {
                result.boss_pts
            };
            if score_to_check >= max_hp {
                max_hp
            } else {
                score_to_check
            }
        });

        // ── Reward PTS ───────────────────────────────────────────────────────
        let (reward_pts, base_boss_pts) = if has_bonus {
            // Strip the 20 % bonus to recover the pre-bonus total
            let pre_bonus = (result.boss_pts as f64 / 1.20).round() as i64;
            (pre_bonus - total_damage, pre_bonus)
        } else {
            (result.boss_pts - total_damage, result.boss_pts)
        };

        // ── Seconds remaining ────────────────────────────────────────────────
        // Formula: Reward PTS × 10 ÷ 489530
        let secs_remaining = (reward_pts as f64 * 10.0) / 489_530.0;

        // ── Kill time ───────────────────────────────────────────────────────
        let total_secs = boss_time_limit(&result.boss_name) as f64;
        let kill_time_secs = (total_secs - secs_remaining).max(0.0);

        // ── DPS ─────────────────────────────────────────────────────────────
        let dps = if kill_time_secs > 0.0 {
            total_damage as f64 / kill_time_secs
        } else {
            0.0
        };

        // ── Format kill time as M:SS ─────────────────────────────────────────
        let kill_time_fmt = format_time(kill_time_secs);

        Self {
            boss_name: result.boss_name.clone(),
            total_damage,
            kill_time_secs,
            kill_time_fmt,
            dps,
            boss_pts: base_boss_pts,
            has_bonus,
            reward_pts,
            secs_remaining,
            boss_time_limit: boss_time_limit(&result.boss_name),
        }
    }

    /// Build the formatted Discord reply string.
    pub fn discord_message(&self) -> String {
        let damage_m = self.total_damage as f64 / 1_000_000.0;
        let dps_m = self.dps / 1_000_000.0;
        let bonus_str = if self.has_bonus { "x120% ✅" } else { "None ❌" };

        format!(
            "```\n\
═══════════════════════════════\n\
  DMC: Peak of Combat Results\n\
═══════════════════════════════\n\
  Boss        : {boss}\n\
  Total Damage: {dmg:.2}M\n\
  Kill Time   : {time}\n\
  DPS         : {dps:.2}M/s\n\
  Boss PTS    : {pts}\n\
  Bonus       : {bonus}\n\
  Reward PTS  : {reward}\n\
  Secs Left   : {secs:.2}s\n\
═══════════════════════════════\n\
```",
            boss   = self.boss_name,
            dmg    = damage_m,
            time   = self.kill_time_fmt,
            dps    = dps_m,
            pts    = self.boss_pts,
            bonus  = bonus_str,
            reward = self.reward_pts,
            secs   = self.secs_remaining,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Format a duration in seconds with 1 decimal place precision (e.g. "16.8s").
fn format_time(secs: f64) -> String {
    format!("{:.1}s", secs)
}
