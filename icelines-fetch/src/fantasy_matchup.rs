use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::timeframe::Timeframe;
use icelines_core::view_model::{
    Completeness, FantasyMatchupScheduleInput, FantasyMatchupTeamTotalInput,
    FantasyMatchupWeekInput, FantasyMatchupWeekView, SourceKind, SourceState,
};

use crate::datastore::DataStore;
use crate::fantasy_daily::build_fantasy_daily_delta_view;
use crate::fantasy_db::{FantasyDb, LeagueRow};

pub fn build_fantasy_matchup_week_view(
    db: &FantasyDb,
    store: &DataStore,
    date: NaiveDate,
    season: Season,
    season_type: SeasonType,
    league_name: Option<&str>,
) -> Result<FantasyMatchupWeekView> {
    let league = resolve_league(db, league_name)?;
    let snapshot = db.league_snapshot(Some(&league.name))?;
    let (week_start, week_end) = Timeframe::Week.range(date);
    let schedule_rows = db.list_matchups(&league.id, week_start)?;

    let mut totals = snapshot
        .teams
        .iter()
        .map(|team| {
            (
                team.name.clone(),
                FantasyMatchupTeamTotalInput {
                    team: team.name.clone(),
                    owner: team.owner.clone(),
                    is_user_team: team.name == snapshot.user_team,
                    weekly_points: 0.0,
                    days_scored: 0,
                    rostered_players: team.roster.len() as u16,
                    scored_players: 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut warnings = Vec::new();
    let mut any_boxscore_missing = false;
    let mut any_unfinalized = false;
    let mut day = week_start;
    while day <= week_end {
        let daily =
            build_fantasy_daily_delta_view(db, store, day, season, season_type, Some(&league.name))
                .with_context(|| format!("build fantasy daily delta for {day}"))?;
        let day_has_complete_boxscore = daily.source_state.iter().any(|source| {
            source.source == SourceKind::Boxscore && source.state == Completeness::Complete
        });
        if daily.source_state.iter().any(|source| {
            source.source == SourceKind::Boxscore && source.state == Completeness::Unavailable
        }) {
            any_boxscore_missing = true;
        }
        for warning in daily.warnings {
            if warning.contains("not finalized") {
                any_unfinalized = true;
            }
            warnings.push(format!("{day}: {warning}"));
        }
        for team in daily.teams {
            let total =
                totals
                    .entry(team.team.clone())
                    .or_insert_with(|| FantasyMatchupTeamTotalInput {
                        team: team.team.clone(),
                        owner: team.owner.clone(),
                        is_user_team: team.is_user_team,
                        weekly_points: 0.0,
                        days_scored: 0,
                        rostered_players: team.rostered_players,
                        scored_players: 0,
                    });
            total.weekly_points += team.daily_points;
            if day_has_complete_boxscore {
                total.days_scored = total.days_scored.saturating_add(1);
            }
            total.rostered_players = total.rostered_players.max(team.rostered_players);
            total.scored_players = total.scored_players.saturating_add(team.scored_players);
        }
        day += Duration::days(1);
    }

    let schedule = schedule_rows
        .into_iter()
        .map(|row| FantasyMatchupScheduleInput {
            matchup_id: Some(row.id),
            home_team: row.home_team,
            away_team: row.away_team,
        })
        .collect::<Vec<_>>();

    Ok(FantasyMatchupWeekView::from_input(
        FantasyMatchupWeekInput {
            season,
            season_type,
            week_start,
            week_end,
            league: snapshot.league,
            scoring_scheme: snapshot.scoring_scheme,
            team_totals: totals.into_values().collect(),
            schedule,
            warnings,
            source_state: weekly_source_state(
                any_boxscore_missing,
                any_unfinalized,
                db.list_matchups(&league.id, week_start)?.is_empty(),
            ),
        },
    ))
}

fn resolve_league(db: &FantasyDb, league_name: Option<&str>) -> Result<LeagueRow> {
    if let Some(name) = league_name {
        db.list_leagues()?
            .into_iter()
            .find(|league| league.name == name)
            .ok_or_else(|| anyhow::anyhow!("fantasy league '{name}' not found"))
    } else {
        db.get_active_league()?
            .ok_or_else(|| anyhow::anyhow!("no active fantasy league found"))
    }
}

fn weekly_source_state(
    any_boxscore_missing: bool,
    any_unfinalized: bool,
    missing_schedule: bool,
) -> Vec<SourceState> {
    let schedule = if missing_schedule {
        SourceState::missing(SourceKind::Schedule)
    } else {
        SourceState::complete(SourceKind::Schedule)
    };
    let boxscore = if any_boxscore_missing {
        SourceState {
            source: SourceKind::Boxscore,
            state: Completeness::Unavailable,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some(
                "one or more matchup-week dates are missing cached boxscores".to_string(),
            ),
        }
    } else if any_unfinalized {
        SourceState {
            source: SourceKind::Boxscore,
            state: Completeness::Partial,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some("one or more cached boxscores are not finalized".to_string()),
        }
    } else {
        SourceState::complete(SourceKind::Boxscore)
    };

    vec![
        SourceState::complete(SourceKind::FantasyImport),
        schedule,
        boxscore,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atomic_write::write_bytes_atomic;
    use crate::fantasy_db::FantasyDb;
    use crate::manifest::{DataKey, DataKind, ManifestEntry};
    use icelines_core::freshness::{FetchSource, Freshness, Ttl};
    use icelines_core::identity::GameId;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).expect("valid date")
    }

    fn setup_db(with_schedule: bool) -> FantasyDb {
        let db = FantasyDb::open_in_memory().expect("open db");
        let league_id = db
            .create_league("Matchup League", "yahoo-standard")
            .expect("create league");
        db.set_active_league("Matchup League")
            .expect("set active league");
        let my_team = db
            .create_team(&league_id, "My Team", "Me")
            .expect("create my team");
        db.set_user_team(&league_id, "My Team")
            .expect("set user team");
        db.add_player(&my_team, "matty beniers")
            .expect("add skater");
        db.add_player(&my_team, "joey daccord").expect("add goalie");
        db.create_team(&league_id, "Rival", "Them")
            .expect("create rival");
        if with_schedule {
            db.schedule_matchup(&league_id, d(12), "My Team", Some("Rival"))
                .expect("schedule matchup");
        }
        db
    }

    fn store_with_week_boxscores(live_day: Option<u32>) -> (tempfile::TempDir, DataStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(dir.path()).expect("open store");
        for day in 12..=18 {
            let date = format!("2026-01-{day:02}");
            let game_id = 2025020300 + day as u64;
            let path = dir
                .path()
                .join("boxscores")
                .join(&date)
                .join(format!("{game_id}.json"));
            let has_rostered_players = day == 15;
            let state = if live_day == Some(day) {
                "LIVE"
            } else {
                "FINAL"
            };
            let home_stats = if has_rostered_players {
                serde_json::json!({
                    "forwards": [{
                        "playerId": 8482665,
                        "name": { "default": "Matty Beniers" },
                        "position": "C",
                        "toi": "18:20",
                        "goals": 2,
                        "assists": 1,
                        "plusMinus": 1,
                        "sog": 5,
                        "hits": 3,
                        "blockedShots": 2,
                        "takeaways": 1,
                        "giveaways": 0,
                        "pim": 0
                    }],
                    "defense": [],
                    "goalies": [{
                        "playerId": 8478916,
                        "name": { "default": "Joey Daccord" },
                        "saves": 30,
                        "shotsAgainst": 31,
                        "decision": "W"
                    }]
                })
            } else {
                serde_json::json!({ "forwards": [], "defense": [], "goalies": [] })
            };
            let raw = serde_json::json!({
                "id": game_id,
                "gameDate": date,
                "gameState": state,
                "gameOutcome": { "lastPeriodType": "REG" },
                "awayTeam": { "abbrev": "VAN", "score": 1 },
                "homeTeam": { "abbrev": "SEA", "score": 4 },
                "playerByGameStats": {
                    "awayTeam": { "forwards": [], "defense": [], "goalies": [] },
                    "homeTeam": home_stats
                }
            });
            write_bytes_atomic(&path, raw.to_string().as_bytes()).expect("write boxscore");
            store
                .manifest()
                .upsert(
                    DataKind::Boxscore,
                    ManifestEntry {
                        key: DataKey::Game(GameId(game_id)),
                        path,
                        freshness: Freshness {
                            fetched_at: chrono::Utc::now(),
                            source: FetchSource::Live,
                            ttl: Ttl::Static,
                        },
                    },
                )
                .expect("upsert manifest");
        }
        (dir, store)
    }

    #[test]
    fn l1_fantasy_matchup_week_scores_cached_final_week() {
        let db = setup_db(true);
        let (_dir, store) = store_with_week_boxscores(None);

        let view = build_fantasy_matchup_week_view(
            &db,
            &store,
            d(15),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("matchup view");

        assert_eq!(view.week_start, d(12));
        assert_eq!(view.week_end, d(18));
        assert_eq!(view.matchups.len(), 1);
        assert_eq!(view.matchups[0].winner.as_deref(), Some("My Team"));
        assert!((view.matchups[0].home.weekly_points.unwrap_or_default() - 19.0).abs() < 0.001);
        assert_eq!(view.context.completeness, Completeness::Complete);
    }

    #[test]
    fn l1_fantasy_matchup_week_missing_schedule_is_explicit_empty_state() {
        let db = setup_db(false);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::open(dir.path()).expect("open store");

        let view = build_fantasy_matchup_week_view(
            &db,
            &store,
            d(15),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("matchup view");

        assert!(view.matchups.is_empty());
        assert!(view.empty_state.is_some());
        assert!(view.source_state.iter().any(|source| {
            source.source == SourceKind::Schedule && source.state == Completeness::Unavailable
        }));
    }

    #[test]
    fn l1_fantasy_matchup_week_unfinalized_cache_keeps_result_pending() {
        let db = setup_db(true);
        let (_dir, store) = store_with_week_boxscores(Some(15));

        let view = build_fantasy_matchup_week_view(
            &db,
            &store,
            d(15),
            Season(20252026),
            SeasonType::Regular,
            None,
        )
        .expect("matchup view");

        assert_eq!(view.matchups[0].winner, None);
        assert_eq!(
            view.matchups[0].home.outcome,
            icelines_core::view_model::FantasyMatchupOutcome::Pending
        );
        assert!(view.source_state.iter().any(|source| {
            source.source == SourceKind::Boxscore && source.state == Completeness::Partial
        }));
    }
}
