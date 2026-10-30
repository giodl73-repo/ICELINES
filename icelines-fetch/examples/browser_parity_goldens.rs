//! Emit native-loader expectations for comparison with the distributed WASM.
//! Completed bundled season only; no live acquisition or browser implementation.
use icelines_core::{model::Season, season_stats::SeasonType};
use icelines_fetch::{snapshot::SnapshotStore, stats_loader::load_into_repo};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).ok_or("expected output path")?;
    let directory = tempfile::tempdir()?;
    let season = Season(20_242_025);
    let mut cases = Vec::new();
    for kind in [SeasonType::Regular, SeasonType::Playoff] {
        let native = load_into_repo(season, kind, &SnapshotStore::new(directory.path()))?;
        for goalies in [false, true] {
            let sorts: &[&str] = if goalies {
                &["wins", "gp", "save_pct", "gaa"]
            } else {
                &["points", "goals", "assists", "gp", "pace"]
            };
            let filters = if goalies {
                ["", "goalie-games >= 10", "goalie-games >= 100"]
            } else {
                ["", "p >= 100", "g >= 30"]
            };
            for sort in sorts {
                for minimum in [None, Some(0), Some(1), Some(5), Some(10)] {
                    for filter in filters {
                        let floor = minimum.unwrap_or(if goalies {
                            icelines_core::view_model::goalies::qualified_goalie_games(kind)
                        } else {
                            0
                        });
                        let mut views: Vec<_> = native
                            .repo
                            .league(season, kind)
                            .filter(|view| view.is_goalie() == goalies && view.gp() >= floor)
                            .filter(|view| match filter {
                                "p >= 100" => view.points() >= 100,
                                "g >= 30" => view.goals() >= 30,
                                "goalie-games >= 10" => view.gp() >= 10,
                                "goalie-games >= 100" => view.gp() >= 100,
                                _ => true,
                            })
                            .collect();
                        if goalies {
                            let metric = icelines_core::GoalieLeaderboardSort::from_key(sort)
                                .ok_or("goalie sort")?;
                            views.sort_by(|a, b| metric.compare_player_views(a, b));
                        } else if *sort == "pace" {
                            icelines_core::sort_views_by_pace(&mut views);
                        } else {
                            let metric = icelines_core::stats_catalog::StatId::from_cli_key(sort)
                                .ok_or("skater sort")?;
                            views.sort_by(|a, b| metric.sort_cmp(a, b));
                        }
                        let rows: Vec<_> = views.into_iter().map(|view| {
                            let goalie = view.stats.goalie.as_ref();
                            json!({"player_id":view.id().0,"name":view.full_name(),
                                "team":view.team_display(),"position":format!("{:?}",view.position()),
                                "gp":view.gp(),"goals":view.goals(),"assists":view.assists(),
                                "points":view.points(),"pace_82":view.pace_82(),
                                "save_pct":goalie.and_then(|g|g.save_pct).map(f64::from),
                                "gaa":goalie.and_then(|g|g.goals_against_average).map(f64::from),
                                "wins":goalie.map(|g|g.wins),"hits":view.hits(),"blocked_shots":view.blocked_shots()})
                        }).collect();
                        cases.push(
                            json!({"season":season.0,"season_type":kind,"minimum_games":floor,
                            "request":{"filter":filter,"sort":sort,"goalies":goalies,
                                "minimum_games":minimum,"today":"2026-10-03"},"rows":rows}),
                        );
                    }
                }
            }
        }
    }
    std::fs::write(output, serde_json::to_vec(&cases)?)?;
    println!("Emitted {} native-loader parity cases", cases.len());
    Ok(())
}
