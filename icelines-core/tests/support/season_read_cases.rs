//! Synthetic boundary cases shared by the legacy oracle and additive-read tests.
use icelines_core::{
    fixtures,
    identity::{PlayerId, PlayerIdentity},
    model::{Position, Season, TeamAbbr},
    scoring::compute_pace_score,
    season_stats::*,
    stats_catalog::{parse_filter_expr, StatId},
    stats_repository::PlayerView,
};
use serde_json::{json, Value};

pub fn cases() -> Vec<(PlayerIdentity, SeasonStats)> {
    let mut cases = Vec::new();
    for gp in [0, 9, 10] {
        for toi in [None, Some(0), Some(299), Some(300)] {
            for traded in [false, true] {
                for goalie in [false, true] {
                    for kind in [SeasonType::Regular, SeasonType::Playoff] {
                        for mode in 0..3 {
                            let id = cases.len() as u32 + 1;
                            let goals = if goalie { 0 } else { gp / 2 };
                            let assists = if goalie { 0 } else { gp / 3 };
                            let totals = StatTotals {
                                gp,
                                goals,
                                assists,
                                points: goals + assists,
                                plus_minus: -7,
                                pim: 13,
                                shots: gp * 7,
                                shooting_pct: (mode > 0).then_some(if mode == 1 {
                                    0.0
                                } else {
                                    0.156789
                                }),
                                faceoff_win_pct: (mode > 0).then_some(if mode == 1 {
                                    0.0
                                } else {
                                    0.512345
                                }),
                                toi_per_game_sec: (mode > 0).then_some(if mode == 1 {
                                    0
                                } else {
                                    999
                                }),
                                pp_goals: goals / 3,
                                pp_points: (goals + assists) / 2,
                                sh_goals: 1,
                                sh_points: 1,
                                gwg: 1,
                                ot_goals: 1,
                                pace_score: compute_pace_score(goals, assists, gp),
                            };
                            let position = if goalie {
                                Position::Goalie
                            } else if id % 2 == 0 {
                                Position::Center
                            } else {
                                Position::Defense
                            };
                            let mut builder = SeasonStatsBuilder::new(
                                PlayerId(id),
                                Season(if id % 2 == 0 { 20252026 } else { 19931994 }),
                                kind,
                                position,
                            )
                            .with_totals(totals);
                            let stint = |team: &str, gp, goals, assists| TeamStint {
                                team: TeamAbbr(team.into()),
                                started: None,
                                ended: None,
                                gp,
                                goals,
                                assists,
                                points: goals + assists,
                                goalie: None,
                            };
                            if traded {
                                builder = builder
                                    .add_team_stint(stint("OLD", gp / 2, goals / 2, assists / 2))
                                    .add_team_stint(stint(
                                        "NEW",
                                        gp - gp / 2,
                                        goals - goals / 2,
                                        assists - assists / 2,
                                    ));
                            } else {
                                builder = builder.add_team_stint(stint("OLD", gp, goals, assists));
                            }
                            let mut row = builder.try_build().unwrap();
                            row.sweater_number =
                                (mode > 0).then_some(if mode == 1 { 0 } else { 97 });
                            let n = if mode == 1 { 0 } else { 7 };
                            let pct = if mode == 1 { 0.0 } else { 0.92345 };
                            if goalie {
                                row.goalie = Some(GoalieSeasonStats {
                                    games_started: gp,
                                    wins: gp / 2,
                                    losses: gp - gp / 2,
                                    ot_losses: (mode > 0).then_some(n),
                                    ties: (mode > 0).then_some(0),
                                    shots_against: gp * 20,
                                    goals_against: gp * 2,
                                    saves: gp * 18,
                                    save_pct: (mode > 0).then_some(pct),
                                    goals_against_average: (mode > 0).then_some(if mode == 1 {
                                        0.0
                                    } else {
                                        2.34567
                                    }),
                                    shutouts: n,
                                    time_on_ice_sec: toi.unwrap_or(0),
                                });
                                if mode > 0 {
                                    row.goalie_advanced = Some(GoalieAdvancedStats {
                                        quality_starts: n,
                                        quality_starts_pct: Some(pct),
                                        regulation_wins: n,
                                        regulation_losses: n,
                                        complete_games: n,
                                        incomplete_games: n,
                                        complete_game_pct: Some(pct),
                                        shots_against_per_60: Some(pct),
                                    });
                                    row.goalie_saves_by_strength =
                                        Some(GoalieSavesByStrengthStats {
                                            ev_saves: n,
                                            ev_shots_against: n,
                                            ev_goals_against: n,
                                            ev_save_pct: Some(pct),
                                            pp_saves: n,
                                            pp_shots_against: n,
                                            pp_goals_against: n,
                                            pp_save_pct: Some(pct),
                                            sh_saves: n,
                                            sh_shots_against: n,
                                            sh_goals_against: n,
                                            sh_save_pct: Some(pct),
                                        });
                                    row.goalie_bios = Some(GoalieBios {
                                        birth_city: Some("Synthetic city".into()),
                                        draft_overall: Some("Undrafted".into()),
                                        ..Default::default()
                                    });
                                }
                            } else if mode > 0 {
                                row.realtime = Some(RealtimeStats {
                                    hits: n,
                                    blocked_shots: n,
                                    takeaways: n,
                                    giveaways: n,
                                    missed_shots: n,
                                });
                                row.advanced = Some(AdvancedStats {
                                    xg: Some(f64::from(pct)),
                                    xg_per_60: Some(1.23456789),
                                    cf_pct: Some(51.25),
                                    ff_pct: Some(49.875),
                                    on_ice_xg_for: Some(12.3456789),
                                    on_ice_xg_against: Some(23.456789),
                                    xgf_pct: Some(0.456789),
                                });
                                row.time_on_ice = toi.map(|total| TimeOnIceStats {
                                    time_on_ice_sec: total,
                                    time_on_ice_per_game_sec: n,
                                    ev_time_on_ice_sec: n,
                                    ev_time_on_ice_per_game_sec: n,
                                    pp_time_on_ice_sec: n,
                                    pp_time_on_ice_per_game_sec: n,
                                    sh_time_on_ice_sec: n,
                                    sh_time_on_ice_per_game_sec: n,
                                    ot_time_on_ice_sec: Some(0),
                                    shifts: n,
                                    shifts_per_game: pct,
                                    time_on_ice_per_shift_sec: pct,
                                });
                                row.goals_for_against = Some(GoalsForAgainstStats {
                                    ev_goals_for: n,
                                    ev_goals_against: n,
                                    ev_goals_for_pct: Some(pct),
                                    pp_goals_for: n,
                                    pp_goals_against: n,
                                    sh_goals_for: n,
                                    sh_goals_against: n,
                                    even_strength_goal_difference: -7,
                                    ev_time_on_ice_per_game_sec: n,
                                    offensive_points: Some(n),
                                    defensive_points: Some(n),
                                });
                            }
                            cases.push((fixtures::identity(id).build(), row));
                        }
                    }
                }
            }
        }
    }
    cases
}
pub fn filters() -> Vec<String> {
    vec![
        format!("{}>=10", StatId::Games.cli_key()),
        format!("{}>=0", StatId::Hits.cli_key()),
        format!("NOT ({}>=1)", StatId::Hits.cli_key()),
        format!(
            "{}>=1 AND {}>=1",
            StatId::Goals.cli_key(),
            StatId::Hits.cli_key()
        ),
        format!("{}>=1", StatId::EvGoalsFor.cli_key()),
        format!("{}>=0.9", StatId::SavePct.cli_key()),
        format!("{}>=0.5", StatId::PointsPerGame.cli_key()),
        format!("{}>=1", StatId::HitsPer60.cli_key()),
    ]
}
pub fn legacy_observations(cases: &[(PlayerIdentity, SeasonStats)]) -> Value {
    let views: Vec<_> = cases
        .iter()
        .map(|(identity, stats)| PlayerView {
            identity,
            stats,
            contract: None,
        })
        .collect();
    let stats:Vec<_>=StatId::all().iter().map(|stat| {
        let values:Vec<_>=views.iter().map(|v|stat.read(v).map(f64::to_bits)).collect();
        let mut sorted=views.clone();sorted.sort_by(|a,b|stat.sort_cmp(a,b));
        json!({"stat":stat.cli_key(),"value_bits":values,"sorted_ids":sorted.iter().map(|v|v.id().0).collect::<Vec<_>>()})
    }).collect();
    let filters:Vec<_>=filters().iter().map(|filter|{
        let parsed=parse_filter_expr(filter).unwrap();json!({"filter":filter,"matches":views.iter().map(|v|parsed.matches(v)).collect::<Vec<_>>()})
    }).collect();
    json!({"schema_version":1,"scope":"synthetic-legacy-catalog-boundaries","rows":cases.len(),"stats":stats,"filters":filters})
}
