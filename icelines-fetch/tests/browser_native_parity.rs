//! Fixed bundle parity across native acquisition and portable browser loading.
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;
use icelines_wasm::{BrowserEngine, QueryRequest, SeasonPackage};

fn compare(kind: SeasonType, bios: &str, stats: &str, goalies: &str) {
    // Fixed completed-season fixtures, deliberately independent of CURRENT_SEASON.
    let season = Season(20_242_025);
    let package = SeasonPackage {
        schema_version: 1,
        season: season.0,
        season_type: kind,
        source: "parity-fixture".into(),
        observed_at: None,
        fetched_at: None,
        bios: serde_json::from_str(bios).unwrap(),
        stats: serde_json::from_str(stats).unwrap(),
        goalies: serde_json::from_str(goalies).unwrap(),
    };
    let mut browser = BrowserEngine::new();
    browser
        .load_bytes(&serde_json::to_vec(&package).unwrap())
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let native = load_into_repo(season, kind, &SnapshotStore::new(directory.path())).unwrap();
    for goalie in [false, true] {
        let result = browser
            .execute(&QueryRequest {
                filter: String::new(),
                sort: if goalie { "wins" } else { "points" }.into(),
                goalies: goalie,
                minimum_games: Some(0),
                today: "2026-10-03".into(),
            })
            .unwrap();
        let expected = native
            .repo
            .league(season, kind)
            .filter(|view| view.is_goalie() == goalie)
            .count();
        assert_eq!(result.rows.len(), expected);
        assert_eq!(result.season_type, kind);
        for row in result.rows {
            let view = native
                .repo
                .league(season, kind)
                .find(|view| view.id().0 == row.player_id)
                .unwrap();
            assert_eq!(row.name, view.full_name());
            assert_eq!(row.team, view.team_display());
            assert_eq!(
                (row.gp, row.goals, row.assists, row.points),
                (view.gp(), view.goals(), view.assists(), view.points())
            );
            assert_eq!(row.pace_82, view.pace_82());
            if let Some(goalie) = &view.stats.goalie {
                assert_eq!(row.save_pct, goalie.save_pct.map(f64::from));
                assert_eq!(row.gaa, goalie.goals_against_average.map(f64::from));
                assert_eq!(row.wins, Some(goalie.wins));
            }
        }
    }
    // Exercise ordering and qualification against native repository views, not
    // merely the browser's exported row values. Zero is an explicit override.
    for goalie in [false, true] {
        let sorts: &[&str] = if goalie {
            &["wins", "gp", "save_pct", "gaa"]
        } else {
            &["points", "goals", "assists", "gp", "pace"]
        };
        for sort in sorts {
            for minimum_games in [None, Some(0), Some(1), Some(5), Some(10)] {
                let floor = minimum_games.unwrap_or(if goalie {
                    icelines_core::view_model::goalies::qualified_goalie_games(kind)
                } else {
                    0
                });
                let result = browser
                    .execute(&QueryRequest {
                        filter: String::new(),
                        sort: (*sort).into(),
                        goalies: goalie,
                        minimum_games,
                        today: "2026-10-03".into(),
                    })
                    .unwrap();
                let mut expected: Vec<_> = native
                    .repo
                    .league(season, kind)
                    .filter(|view| view.is_goalie() == goalie && view.gp() >= floor)
                    .collect();
                if goalie {
                    let metric = icelines_core::GoalieLeaderboardSort::from_key(sort).unwrap();
                    expected.sort_by(|a, b| metric.compare_player_views(a, b));
                } else if *sort == "pace" {
                    icelines_core::sort_views_by_pace(&mut expected);
                } else {
                    let metric = icelines_core::stats_catalog::StatId::from_cli_key(sort).unwrap();
                    expected.sort_by(|a, b| metric.sort_cmp(a, b));
                }
                assert_eq!(result.minimum_games, floor);
                assert_eq!(
                    result.rows.iter().map(|r| r.player_id).collect::<Vec<_>>(),
                    expected.iter().map(|v| v.id().0).collect::<Vec<_>>(),
                    "{kind:?} goalie={goalie}, sort={sort}, minimum={minimum_games:?}"
                );
            }
        }
    }
}

#[test]
fn regular_bundle_has_native_browser_row_parity() {
    compare(
        SeasonType::Regular,
        include_str!("../../data/seasons/20242025/bios.json"),
        include_str!("../../data/seasons/20242025/stats.json"),
        include_str!("../../data/seasons/20242025/goalie-stats.json"),
    );
}

#[test]
fn playoff_bundle_has_native_browser_row_parity() {
    compare(
        SeasonType::Playoff,
        include_str!("../../data/seasons/20242025/playoff-bios.json"),
        include_str!("../../data/seasons/20242025/playoff-stats.json"),
        include_str!("../../data/seasons/20242025/playoff-goalie-stats.json"),
    );
}
