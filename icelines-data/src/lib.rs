//! Portable normalization shared by native and browser IceLines loaders.
//! Inputs are decoded source records; no paths, network, clock, or persistence.
#![deny(unsafe_code)]
use icelines_core::identity::{PlayerBio, PlayerId, PlayerIdentity};
use icelines_core::model::{Position, Season, TeamAbbr};
use icelines_core::name::normalize_name;
use icelines_core::scoring::compute_pace_score;
use icelines_core::season_stats::{
    AdvancedStats, GoalieSeasonStats, RealtimeStats, SeasonStatsBuildError, SeasonStatsBuilder,
    SeasonType, StatTotals, TeamStint, TimeOnIceStats, SYNTHETIC_DATE_PREFIX,
};
use icelines_sources::moneypuck::MoneyPuckStats;
use icelines_sources::schema::{
    GoalieStats, SkaterBio, SkaterRealtime, SkaterStats, SkaterTimeOnIce,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NormalizationError {
    #[error("season stats build failed for player {player_id}: {source}")]
    SeasonStatsBuild {
        player_id: PlayerId,
        #[source]
        source: SeasonStatsBuildError,
    },
}

pub fn build_identity(pid: PlayerId, bio: &SkaterBio) -> PlayerIdentity {
    PlayerIdentity {
        id: pid,
        full_name: bio.skater_full_name.clone(),
        name_normalized: normalize_name(&bio.skater_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            pid.0
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
    }
}

pub fn build_goalie_identity(g: &GoalieStats) -> PlayerIdentity {
    // **Sparse path** — used only when `/goalie/bios` data isn't loaded.
    // Pre-Lindsay this was the only goalie-identity source. Phase Lindsay
    // L.2.6 introduces `merge_goalie_bios_into_identity` for the full-data
    // path that pulls birth/draft/height/weight from the dedicated
    // `goalie_bios` substruct on `SeasonStats`. When both are available,
    // the adapter overwrites the sparse fields below with goalie/bios
    // data — see `merge_goalie_bios_into_identity`.
    PlayerIdentity {
        id: PlayerId(g.player_id),
        full_name: g.goalie_full_name.clone(),
        name_normalized: normalize_name(&g.goalie_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            g.player_id
        )),
        bio: PlayerBio {
            shoots_catches: g.shoots_catches.clone(),
            ..Default::default()
        },
    }
}

#[allow(clippy::too_many_arguments)] // typed identity keys plus independently optional sources
pub fn build_skater_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    position: Position,
    bio: &SkaterBio,
    stats: Option<&SkaterStats>,
    realtime: Option<&SkaterRealtime>,
    time_on_ice: Option<&SkaterTimeOnIce>,
    mp: Option<&MoneyPuckStats>,
) -> Result<icelines_core::season_stats::SeasonStats, NormalizationError> {
    // Field-for-field parity with the OLD `make_player` path so the
    // parallel-run field-parity test holds.
    let goals = stats.map(|s| s.goals).unwrap_or(bio.goals);
    let assists = stats.map(|s| s.assists).unwrap_or(bio.assists);
    let gp = stats.map(|s| s.games_played).unwrap_or(bio.games_played);
    let pp_goals = stats.map(|s| s.pp_goals).unwrap_or(0);
    let pp_points = stats.map(|s| s.pp_points).unwrap_or(0);
    let sh_goals = stats.map(|s| s.sh_goals).unwrap_or(0);
    let sh_points = stats.map(|s| s.sh_points).unwrap_or(0);
    let gwg = stats.map(|s| s.game_winning_goals).unwrap_or(0);
    let ot_goals = stats.map(|s| s.ot_goals).unwrap_or(0);
    let plus_minus = stats.map(|s| s.plus_minus).unwrap_or(0);
    let shots = stats.map(|s| s.shots).unwrap_or(0);
    let shooting_pct = stats.and_then(|s| s.shooting_pctg);
    let toi_per_game_sec = stats.and_then(|s| s.time_on_ice_per_game).map(|v| v as u32);
    let faceoff_win_pct = stats.and_then(|s| s.faceoff_win_pct);

    let totals = StatTotals {
        gp,
        goals,
        assists,
        points: goals + assists,
        plus_minus,
        // L.7a — `realtime.pim` is Option<u32> after the upstream API
        // removed pim from the realtime endpoint (PIM now lives only on
        // `/skater/summary`). Bind via `and_then` so a missing value
        // collapses to 0; downstream summary-merge can replace this once
        // the merge path lands.
        pim: realtime.and_then(|r| r.pim).unwrap_or(0),
        shots,
        shooting_pct,
        toi_per_game_sec,
        pp_goals,
        pp_points,
        sh_goals,
        sh_points,
        gwg,
        ot_goals,
        faceoff_win_pct,
        pace_score: compute_pace_score(goals, assists, gp),
    };

    // Per-season team resolution (UX bug fix 2026-05-04):
    // `bio.current_team_abbrev` is the player's CURRENT team regardless of
    // which season the bio row belongs to — so e.g. Tye Kartye's 2024-25
    // bundled bio reports NYR even though he played that whole season for
    // SEA. The per-season `stats.team_abbrevs` field carries the actual
    // historical team(s) for the season ("SEA" or "SEA,NYR" mid-season
    // trade). Prefer it whenever it's present.
    //
    // For multi-team rows we synthesize a SINGLE TeamStint per season
    // because the bundled feed doesn't break out date ranges. The
    // career-table renderer formats the comma-separated abbrev as-is
    // (e.g. "SEA/NYR"). True per-stint splits would need the roster
    // history endpoint and is out of scope for this fix.
    let team_str = stats
        .and_then(|s| s.team_abbrevs.as_deref())
        .filter(|t| !t.is_empty())
        .map(|t| t.replace(',', "/"))
        .unwrap_or_else(|| {
            bio.current_team_abbrev
                .as_deref()
                .unwrap_or("RET")
                .to_owned()
        });
    let stint = TeamStint {
        team: TeamAbbr(team_str),
        started: None,
        ended: None,
        gp,
        goals,
        assists,
        points: goals + assists,
        goalie: None,
    };

    let mut builder = SeasonStatsBuilder::new(pid, season, season_type, position)
        .with_totals(totals)
        .add_team_stint(stint);

    if let Some(rt) = realtime {
        builder = builder.with_realtime(RealtimeStats {
            hits: rt.hits,
            blocked_shots: rt.blocked_shots,
            takeaways: rt.takeaways,
            giveaways: rt.giveaways,
            missed_shots: rt.missed_shots,
        });
    }
    if let Some(toi) = time_on_ice {
        builder = builder.with_time_on_ice(TimeOnIceStats {
            time_on_ice_sec: toi.time_on_ice,
            time_on_ice_per_game_sec: toi.time_on_ice_per_game.round() as u32,
            ev_time_on_ice_sec: toi.ev_time_on_ice,
            ev_time_on_ice_per_game_sec: toi.ev_time_on_ice_per_game.round() as u32,
            pp_time_on_ice_sec: toi.pp_time_on_ice,
            pp_time_on_ice_per_game_sec: toi.pp_time_on_ice_per_game.round() as u32,
            sh_time_on_ice_sec: toi.sh_time_on_ice,
            sh_time_on_ice_per_game_sec: toi.sh_time_on_ice_per_game.round() as u32,
            ot_time_on_ice_sec: toi.ot_time_on_ice,
            shifts: toi.shifts,
            shifts_per_game: toi.shifts_per_game,
            time_on_ice_per_shift_sec: toi.time_on_ice_per_shift,
        });
    }
    if let Some(m) = mp {
        builder = builder.with_advanced(AdvancedStats {
            xg: Some(m.xg_all as f64),
            xg_per_60: Some(m.xg_per_60 as f64),
            cf_pct: Some(m.cf_pct_5v5 as f64),
            ff_pct: Some(m.ff_pct_5v5 as f64),
            on_ice_xg_for: Some(m.on_ice_xg_for_5v5 as f64),
            on_ice_xg_against: Some(m.on_ice_xg_against_5v5 as f64),
            xgf_pct: Some(m.xgf_pct_5v5 as f64),
        });
    }

    builder
        .try_build()
        .map_err(|source| NormalizationError::SeasonStatsBuild {
            player_id: pid,
            source,
        })
}

pub fn build_goalie_season_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    g: &GoalieStats,
) -> Result<icelines_core::season_stats::SeasonStats, NormalizationError> {
    // The legacy goalie row carries `team_abbrevs` as a comma-separated
    // string for traded goalies (e.g. "BOS,OTT"). For Hart.3 we synthesize
    // one TeamStint per token; per-stint goalie counts (W/L/GS split by
    // team) are NOT in the bundled data — only the season-aggregate
    // GoalieSeasonStats has them. Hart.6 captures real per-stint history.
    let teams: Vec<&str> = g
        .team_abbrevs
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let n = teams.len().max(1) as u32;

    let stints: Vec<TeamStint> = if teams.is_empty() {
        vec![TeamStint {
            team: TeamAbbr("RET".into()),
            started: None,
            ended: None,
            gp: g.games_played,
            goals: g.goals,
            assists: g.assists,
            points: g.points,
            goalie: None,
        }]
    } else {
        // Roughly equal split — sum-equals invariant on (gp, goals,
        // assists, points). The remainder lands on the LAST stint so
        // current-home semantics stay correct.
        //
        // FORGE: synthesize monotonically-increasing `started` strings
        // ("AAAA-01", "AAAA-02", …) so the builder's stint sort
        // preserves chronological insertion order. `team_abbrevs` is
        // documented chronological (legacy schema.rs comment) — without
        // synthetic dates the (None, None) sort tiebreak goes
        // alphabetical, flipping `last()` for traded goalies whose
        // chronological order is non-alphabetical (e.g. "OTT,BOS"
        // would sort to [BOS, OTT] and report BOS as the destination
        // when OTT was). Hart.6 replaces with real start/end dates.
        teams
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_last = i == teams.len() - 1;
                let take_n = |total: u32| -> u32 {
                    if is_last {
                        total - (total / n) * (n - 1)
                    } else {
                        total / n
                    }
                };
                TeamStint {
                    team: TeamAbbr((*t).to_owned()),
                    started: Some(format!("{SYNTHETIC_DATE_PREFIX}-{:02}", i + 1)),
                    ended: None,
                    gp: take_n(g.games_played),
                    goals: take_n(g.goals),
                    assists: take_n(g.assists),
                    points: take_n(g.points),
                    goalie: None,
                }
            })
            .collect()
    };

    let totals = StatTotals {
        gp: g.games_played,
        goals: g.goals,
        assists: g.assists,
        points: g.points,
        plus_minus: 0,
        pim: g.penalty_minutes,
        shots: 0,
        shooting_pct: None,
        toi_per_game_sec: g.time_on_ice.checked_div(g.games_played),
        pp_goals: 0,
        pp_points: 0,
        sh_goals: 0,
        sh_points: 0,
        gwg: 0,
        ot_goals: 0,
        faceoff_win_pct: None,
        pace_score: None,
    };

    let goalie = GoalieSeasonStats {
        games_started: g.games_started,
        wins: g.wins,
        losses: g.losses,
        ot_losses: g.ot_losses,
        ties: g.ties,
        shots_against: g.shots_against,
        goals_against: g.goals_against,
        saves: g.saves,
        save_pct: g.save_pct,
        goals_against_average: g.goals_against_average,
        shutouts: g.shutouts,
        time_on_ice_sec: g.time_on_ice,
    };

    SeasonStatsBuilder::new(pid, season, season_type, Position::Goalie)
        .with_totals(totals)
        .replace_team_stints(stints)
        .with_goalie(goalie)
        .try_build()
        .map_err(|source| NormalizationError::SeasonStatsBuild {
            player_id: pid,
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goalie_trade_retains_order_and_sum_without_inventing_dates() {
        let mut rows: Vec<GoalieStats> = serde_json::from_str(include_str!(
            "../../data/seasons/20252026/goalie-stats.json"
        ))
        .expect("fixed bundled goalie fixture");
        let mut row = rows.remove(0);
        row.team_abbrevs = "OTT,BOS".into();
        row.games_played = 7;
        row.goals = 1;
        row.assists = 2;
        row.points = 3;
        let normalized = build_goalie_season_stats(
            PlayerId(row.player_id),
            Season(20_252_026),
            SeasonType::Regular,
            &row,
        )
        .expect("valid two-team fixture");
        assert_eq!(normalized.team_stints.last().unwrap().team.0, "BOS");
        assert_eq!(normalized.team_stints.iter().map(|s| s.gp).sum::<u32>(), 7);
        assert_eq!(
            normalized.team_stints.iter().map(|s| s.points).sum::<u32>(),
            3
        );
        assert!(normalized.goalie.is_some());
        // Source lacks team-level counts: synthetic splits must remain explicit.
        assert!(normalized.team_stints[0]
            .started
            .as_ref()
            .unwrap()
            .starts_with(SYNTHETIC_DATE_PREFIX));
    }

    #[test]
    fn goalie_zero_games_preserves_nullable_rates() {
        let mut rows: Vec<GoalieStats> = serde_json::from_str(include_str!(
            "../../data/seasons/20252026/goalie-stats.json"
        ))
        .expect("fixed bundled goalie fixture");
        let mut row = rows.remove(0);
        row.games_played = 0;
        row.save_pct = None;
        row.goals_against_average = None;
        let stats = build_goalie_season_stats(
            PlayerId(row.player_id),
            Season(20_252_026),
            SeasonType::Regular,
            &row,
        )
        .expect("zero games fixture");
        assert_eq!(stats.totals.toi_per_game_sec, None);
        assert_eq!(stats.goalie.unwrap().save_pct, None);
    }
}
