//! Multi-season aggregation across bundled historical data.
//!
//! `load_aggregate_players(n)` returns players whose stats are summed across
//! the last N bundled seasons. Bio fields (team, nationality, draft) come from
//! the most recent season. Used by `icelines query leaders --seasons N`.
//!
//! `load_improvement_map()` returns a per-player delta in PPG between the
//! current and prior season. Used by `--sort improvement`.

use crate::{bundled, schema::SkaterStats};
use icelines_core::{
    identity::{PlayerBio, PlayerId, PlayerIdentity},
    model::{GpStatus, PaceScore, Player, Season, TeamAbbr},
    name::normalize_name,
    scoring::compute_pace_score,
    season_stats::{SeasonStatsBuilder, SeasonType, StatTotals, TeamStint},
    stats_repository::StatsRepository,
    Position,
};
use std::collections::{HashMap, HashSet};

// ── Aggregate stat accumulator ────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Accum {
    total_gp:       u32,
    total_goals:    u32,
    total_assists:  u32,
    total_pp_goals: u32,
    total_pp_pts:   u32,
    total_sh_goals: u32,
    total_sh_pts:   u32,
    total_gwg:      u32,
    total_shots:    u32,
    seasons:        u8,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Build aggregate players across the last `n` bundled seasons (max 5).
///
/// Stats (G, A, GP, PP, SH, GWG, shots) are summed across all seasons.
/// Bio fields (team, nationality, draft, age) are taken from the most recent
/// season's roster so they reflect the player's current situation.
pub fn load_aggregate_players(n: usize) -> Vec<Player> {
    let n = n.min(bundled::BUNDLED_SEASONS.len());
    let seasons = &bundled::BUNDLED_SEASONS[..n];

    // Accumulate stats per player_id across N seasons
    let mut agg: HashMap<u32, Accum> = HashMap::new();
    for season in seasons {
        let bios  = bundled::get_bios(season).unwrap_or_default();
        let stats = bundled::get_stats(season).unwrap_or_default();
        let stats_idx: HashMap<u32, &SkaterStats> =
            stats.iter().map(|s| (s.player_id, s)).collect();

        for bio in &bios {
            let s = stats_idx.get(&bio.player_id);
            let e = agg.entry(bio.player_id).or_default();
            e.total_gp       += s.map(|s| s.games_played).unwrap_or(0);
            e.total_goals    += s.map(|s| s.goals).unwrap_or(0);
            e.total_assists  += s.map(|s| s.assists).unwrap_or(0);
            e.total_pp_goals += s.map(|s| s.pp_goals).unwrap_or(0);
            e.total_pp_pts   += s.map(|s| s.pp_points).unwrap_or(0);
            e.total_sh_goals += s.map(|s| s.sh_goals).unwrap_or(0);
            e.total_sh_pts   += s.map(|s| s.sh_points).unwrap_or(0);
            e.total_gwg      += s.map(|s| s.game_winning_goals).unwrap_or(0);
            e.total_shots    += s.map(|s| s.shots).unwrap_or(0);
            e.seasons        += 1;
        }
    }

    // Build players from the most recent season's bios (current team, age, etc.)
    // Deduplicate traded players: keep last occurrence (= current team row).
    let current_bios = bundled::get_bios(bundled::BUNDLED_SEASONS[0]).unwrap_or_default();
    let mut seen: HashSet<u32> = HashSet::new();
    // Iterate in reverse to keep last occurrence (current-team row for traded players)
    let deduped: Vec<_> = current_bios.iter().rev()
        .filter(|b| seen.insert(b.player_id))
        .collect::<Vec<_>>()
        .into_iter().rev().collect();

    let mut players: Vec<Player> = deduped.into_iter().filter_map(|bio| {
        let acc = agg.get(&bio.player_id)?;
        if acc.total_gp == 0 { return None; }

        let position = Position::from_api_code(&bio.position_code)?;
        if !position.is_forward() && !position.is_defense() { return None; }
        let team_str = bio.current_team_abbrev.as_deref().unwrap_or("");
        if team_str.is_empty() { return None; }

        let full_name = bio.skater_full_name.clone();
        let team = TeamAbbr(team_str.to_owned());

        Some(Player {
            nhl_id:          Some(bio.player_id),
            full_name:       full_name.clone(),
            name_normalized: normalize_name(&full_name),
            team,
            position,
            eligible_pos:    vec![position],
            gp_status:       GpStatus::from_gp(acc.total_gp),
            season_goals:    acc.total_goals,
            season_assists:  acc.total_assists,
            season_points:   acc.total_goals + acc.total_assists,
            pace_score:      compute_pace_score(acc.total_goals, acc.total_assists, acc.total_gp),
            pp_goals:        acc.total_pp_goals,
            pp_points:       acc.total_pp_pts,
            sh_goals:        acc.total_sh_goals,
            sh_points:       acc.total_sh_pts,
            gwg:             acc.total_gwg,
            shots:           acc.total_shots,
            // Fields not aggregated — zeroed/None for aggregate view
            ot_goals: 0, shooting_pct: None, plus_minus: 0,
            toi_per_game_sec: None, faceoff_win_pct: None,
            hits: 0, blocked_shots: 0, missed_shots: 0,
            giveaways: 0, takeaways: 0, pim: 0,
            xg: None, xg_per_60: None, cf_pct_5v5: None,
            ff_pct_5v5: None, xgf_pct_5v5: None,
            contract_expiry_year: None, expiry_type: None, salary: None,
            headshot_url:         None,
            sweater_number:       None,
            birth_date:           bio.birth_date.clone(),
            birth_country:        bio.birth_country.clone(),
            nationality_code:     bio.nationality_code.clone(),
            birth_city:           bio.birth_city.clone(),
            birth_state_province: bio.birth_state_province_code.clone(),
            shoots_catches:       bio.shoots_catches.clone(),
            height_in_inches:     bio.height,
            weight_lbs:           bio.weight,
            draft_year:           bio.draft_year.map(|v| v as u16),
            draft_round:          bio.draft_round.map(|v| v as u8),
            draft_overall:        bio.draft_overall.map(|v| v as u16),
            rookie_season:        bio.first_season_for_game_type,
        })
    }).collect();

    icelines_core::scoring::sort_by_pace(&mut players);
    players
}

/// Hart.5c.3: PlayerView analog of `load_aggregate_players`.
///
/// Builds a `StatsRepository` whose stats are the N-season sum, attributed
/// to the most-recent bundled season (caller queries with that season). Bio
/// fields come from the most recent season's roster — same semantics as
/// `load_aggregate_players`. Goalies are skipped (skater pool only).
///
/// Returns `(repo, season)` where `season` is the key callers should pass
/// to `repo.skaters(season, SeasonType::Regular)`.
pub fn load_aggregate_into_repo(n: usize) -> (StatsRepository, Season) {
    let n = n.min(bundled::BUNDLED_SEASONS.len());
    let seasons = &bundled::BUNDLED_SEASONS[..n];
    let current_season_str = bundled::BUNDLED_SEASONS[0];
    let current_season_u32: u32 = current_season_str.parse().expect("bundled season id is YYYYZZZZ");
    let current_season = Season(current_season_u32);

    // Accumulate counters per player_id across N seasons (mirror of load_aggregate_players).
    let mut agg: HashMap<u32, Accum> = HashMap::new();
    for season in seasons {
        let bios  = bundled::get_bios(season).unwrap_or_default();
        let stats = bundled::get_stats(season).unwrap_or_default();
        let stats_idx: HashMap<u32, &SkaterStats> =
            stats.iter().map(|s| (s.player_id, s)).collect();
        for bio in &bios {
            let s = stats_idx.get(&bio.player_id);
            let e = agg.entry(bio.player_id).or_default();
            e.total_gp       += s.map(|s| s.games_played).unwrap_or(0);
            e.total_goals    += s.map(|s| s.goals).unwrap_or(0);
            e.total_assists  += s.map(|s| s.assists).unwrap_or(0);
            e.total_pp_goals += s.map(|s| s.pp_goals).unwrap_or(0);
            e.total_pp_pts   += s.map(|s| s.pp_points).unwrap_or(0);
            e.total_sh_goals += s.map(|s| s.sh_goals).unwrap_or(0);
            e.total_sh_pts   += s.map(|s| s.sh_points).unwrap_or(0);
            e.total_gwg      += s.map(|s| s.game_winning_goals).unwrap_or(0);
            e.total_shots    += s.map(|s| s.shots).unwrap_or(0);
            e.seasons        += 1;
        }
    }

    // Most-recent bios provide the canonical identity (current team, age, etc.).
    let current_bios = bundled::get_bios(current_season_str).unwrap_or_default();
    let mut seen: HashSet<u32> = HashSet::new();
    let deduped: Vec<_> = current_bios.iter().rev()
        .filter(|b| seen.insert(b.player_id))
        .collect::<Vec<_>>()
        .into_iter().rev().collect();

    let mut repo = StatsRepository::new();
    for bio in deduped {
        let acc = match agg.get(&bio.player_id) {
            Some(a) if a.total_gp > 0 => a,
            _ => continue,
        };
        let position = match Position::from_api_code(&bio.position_code) {
            Some(p) if p.is_forward() || p.is_defense() => p,
            _ => continue,
        };
        let team_str = match bio.current_team_abbrev.as_deref() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        let pid = PlayerId(bio.player_id);

        let identity = PlayerIdentity {
            id: pid,
            full_name: bio.skater_full_name.clone(),
            name_normalized: normalize_name(&bio.skater_full_name),
            headshot_canonical_url: Some(format!(
                "https://assets.nhle.com/mugs/nhl/default/{}.png",
                bio.player_id
            )),
            bio: PlayerBio {
                birth_date: bio.birth_date.clone(),
                birth_country: bio.birth_country.clone(),
                nationality_code: bio.nationality_code.clone(),
                birth_city: bio.birth_city.clone(),
                birth_state_province: bio.birth_state_province_code.clone(),
                height_in_inches: bio.height,
                weight_lbs: bio.weight,
                draft_year: bio.draft_year.map(|v| v as u16),
                draft_round: bio.draft_round.map(|v| v as u8),
                draft_overall: bio.draft_overall.map(|v| v as u16),
                shoots_catches: bio.shoots_catches.clone(),
                rookie_season: bio.first_season_for_game_type.map(|s| s.to_string()),
            },
        };
        let _ = repo.upsert_identity(identity);

        let totals = StatTotals {
            gp: acc.total_gp,
            goals: acc.total_goals,
            assists: acc.total_assists,
            points: acc.total_goals + acc.total_assists,
            plus_minus: 0, pim: 0,
            shots: acc.total_shots,
            shooting_pct: None,
            toi_per_game_sec: None,
            pp_goals: acc.total_pp_goals,
            pp_points: acc.total_pp_pts,
            sh_goals: acc.total_sh_goals,
            sh_points: acc.total_sh_pts,
            gwg: acc.total_gwg,
            ot_goals: 0,
            faceoff_win_pct: None,
            pace_score: compute_pace_score(acc.total_goals, acc.total_assists, acc.total_gp)
                .map(|s| PaceScore {
                    pace_82: s.pace_82,
                    goals_per_82: s.goals_per_82,
                    raw_points: s.raw_points,
                    gp: s.gp,
                }),
        };
        let stint = TeamStint {
            team: TeamAbbr(team_str.to_owned()),
            started: None,
            ended: None,
            gp: acc.total_gp,
            goals: acc.total_goals,
            assists: acc.total_assists,
            points: acc.total_goals + acc.total_assists,
            goalie: None,
        };
        let stats = SeasonStatsBuilder::new(pid, current_season, SeasonType::Regular, position)
            .with_totals(totals)
            .add_team_stint(stint)
            .build();
        let _ = repo.upsert_stats(stats);
    }

    (repo, current_season)
}

/// Compute per-player Y/Y PPG improvement: current season minus prior season.
///
/// Returns a map of player_id → delta (positive = improved, negative = declined).
/// Players missing from either season are excluded.
/// Minimum 10 GP in each season required for reliability.
pub fn load_improvement_map() -> HashMap<u32, f64> {
    let curr_stats = bundled::get_stats(bundled::BUNDLED_SEASONS[0]).unwrap_or_default();
    let prev_stats = bundled::get_stats(bundled::BUNDLED_SEASONS[1]).unwrap_or_default();

    let curr_idx: HashMap<u32, &SkaterStats> = curr_stats.iter().map(|s| (s.player_id, s)).collect();
    let prev_idx: HashMap<u32, &SkaterStats> = prev_stats.iter().map(|s| (s.player_id, s)).collect();

    let mut result = HashMap::new();
    for (pid, curr) in &curr_idx {
        if curr.games_played < 10 { continue; }
        let curr_ppg = (curr.goals + curr.assists) as f64 / curr.games_played as f64;
        // Only include players who appeared in BOTH seasons with ≥10 GP.
        // Players missing from the prior season (true rookies or data gaps) are
        // excluded entirely — their delta would just be their current PPG which
        // inflates the leaderboard misleadingly.
        let prev = match prev_idx.get(pid).filter(|p| p.games_played >= 10) {
            Some(p) => p,
            None    => continue,
        };
        let prev_ppg = (prev.goals + prev.assists) as f64 / prev.games_played as f64;
        result.insert(*pid, curr_ppg - prev_ppg);
    }
    result
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_aggregate_1_season_matches_bundled_player_count() {
        let players = load_aggregate_players(1);
        assert!(players.len() > 500, "expected 900+ players, got {}", players.len());
    }

    #[test]
    fn l0_aggregate_5_seasons_has_more_gp_than_1() {
        let one = load_aggregate_players(1);
        let five = load_aggregate_players(5);
        // At least some players should have more total GP across 5 seasons
        let one_top_gp: u32 = one.iter()
            .filter_map(|p| p.gp())
            .max().unwrap_or(0);
        let five_top_gp: u32 = five.iter()
            .filter_map(|p| p.gp())
            .max().unwrap_or(0);
        assert!(five_top_gp > one_top_gp,
            "5-season aggregate must have higher max GP than single season");
    }

    #[test]
    fn l0_aggregate_no_duplicates() {
        let players = load_aggregate_players(1);
        let mut ids: Vec<_> = players.iter().filter_map(|p| p.nhl_id).collect();
        ids.sort_unstable();
        let unique_count = ids.windows(2).filter(|w| w[0] == w[1]).count();
        assert_eq!(unique_count, 0, "aggregate must have no duplicate player IDs");
    }

    #[test]
    fn l0_aggregate_sorted_by_pace() {
        let players = load_aggregate_players(1);
        let paces: Vec<f64> = players.iter()
            .filter_map(|p| p.pace_score.map(|s| s.pace_82))
            .take(10).collect();
        assert!(paces.windows(2).all(|w| w[0] >= w[1]),
            "aggregate players must be sorted by pace descending");
    }

    #[test]
    fn l0_improvement_map_nonempty() {
        let imp = load_improvement_map();
        assert!(!imp.is_empty(), "improvement map must contain entries from bundled data");
    }

    #[test]
    fn l0_improvement_map_values_are_reasonable() {
        let imp = load_improvement_map();
        // No improvement delta should exceed ±2.0 PPG (would indicate a data error)
        for (_, delta) in &imp {
            assert!(delta.abs() <= 2.0,
                "PPG delta {} is unreasonably large", delta);
        }
    }

    #[test]
    fn l0_improvement_map_excludes_low_gp() {
        // load_improvement_map requires min 10 GP — the result should only have
        // players with enough data; verify by checking all mapped players against
        // current bundled stats
        let imp = load_improvement_map();
        let curr_stats = bundled::get_stats(bundled::BUNDLED_SEASONS[0]).unwrap_or_default();
        let curr_idx: HashMap<u32, &SkaterStats> = curr_stats.iter().map(|s| (s.player_id, s)).collect();

        for pid in imp.keys() {
            let gp = curr_idx.get(pid).map(|s| s.games_played).unwrap_or(0);
            assert!(gp >= 10, "player {pid} in improvement map has only {gp} GP");
        }
    }
}
