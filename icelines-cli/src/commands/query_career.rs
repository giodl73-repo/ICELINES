//! Phase Calder.4 — `icelines query career --league X --season Y`.
//!
//! Cross-league cohort leaderboard. Walks the local
//! `~/.icelines/career_history.json` store, filters to one
//! (league, season, regular-season) tuple, sorts by points (or any of
//! goals/assists/gp/ppg), prints top-N. CSV/JSON output options
//! mirror `query leaders`.
//!
//! Cohort scope: only players whose career history is in the local
//! store — i.e. who appeared on an NHL roster in one of the last 5
//! bundled seasons (the `fetch career --bundled-seasons 5` target).
//! Career-only players who never reached the NHL aren't in scope.
//!
//! Source-of-truth data is the NHL landing endpoint (`/v1/player/
//! {id}/landing.seasonTotals`); see icelines-fetch/src/career_landing.rs.

use anyhow::{anyhow, Context, Result};
use icelines_core::career_history::{CareerGameType, CareerHistory, CareerStint};
use icelines_fetch::career_landing::CareerHistoryStore;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Points,
    Goals,
    Assists,
    Gp,
    Ppg,
}

impl SortKey {
    fn parse(s: &str) -> Result<Self> {
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "points" | "p" | "pts" => Ok(Self::Points),
            "goals" | "g" => Ok(Self::Goals),
            "assists" | "a" => Ok(Self::Assists),
            "gp" | "games" => Ok(Self::Gp),
            "ppg" | "points-per-game" => Ok(Self::Ppg),
            _ => Err(anyhow!(
                "unknown --sort '{s}' — try: points, goals, assists, gp, ppg"
            )),
        }
    }

    fn header(self) -> &'static str {
        match self {
            Self::Points => "P (sort)",
            Self::Goals => "G (sort)",
            Self::Assists => "A (sort)",
            Self::Gp => "GP (sort)",
            Self::Ppg => "PPG (sort)",
        }
    }
}

/// One leaderboard row. Pure projection — formatting decisions live
/// in the renderers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CareerRow {
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub points_per_game: Option<f32>,
}

/// Resolve the league/season window into a sorted Vec<CareerRow>.
/// Pure — testable from a fixture store.
pub fn project_career_rows<'a>(
    store: &'a CareerHistoryStore,
    league_query: &str,
    season_query: Option<u32>,
    sort: SortKey,
) -> Vec<(u32, &'a CareerStint)> {
    // Case-insensitive league match — users might type "ohl" or "OHL".
    let needle = league_query.to_ascii_uppercase();

    // First pass: collect every (pid, stint) that matches league.
    // A player can have multiple stints in the same season (e.g.,
    // a J20 loan + WHL main-club year); we keep them all so the
    // user sees the full picture, not just the "main" one. The
    // renderer can dedupe later if it wants.
    let mut matched: Vec<(u32, &CareerStint)> = Vec::new();
    for (pid_str, history) in store.histories.iter() {
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };
        for stint in &history.stints {
            if stint.league.0.to_ascii_uppercase() != needle {
                continue;
            }
            // Only regular-season stints — playoff numbers warrant
            // their own surface (out of scope here).
            if !matches!(stint.game_type, CareerGameType::Regular) {
                continue;
            }
            if let Some(want) = season_query {
                if stint.season.0 != want {
                    continue;
                }
            }
            matched.push((pid, stint));
        }
    }

    // If --season was unset, narrow to the single most-recent season
    // the league appears in. Otherwise the leaderboard mixes 5
    // different seasons of OHL stats and the rankings are nonsense.
    if season_query.is_none() {
        if let Some(latest) = matched.iter().map(|(_, s)| s.season.0).max() {
            matched.retain(|(_, s)| s.season.0 == latest);
        }
    }

    // Sort by the requested metric, descending. Ties broken by pid
    // for determinism (matches the AI-06 universal tiebreak the
    // NHL leaderboards use).
    matched.sort_by(|(pa, a), (pb, b)| {
        let ka = metric(a, sort);
        let kb = metric(b, sort);
        // Higher is better; None sorts last.
        kb.partial_cmp(&ka)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| pa.cmp(pb))
    });
    matched
}

fn metric(s: &CareerStint, sort: SortKey) -> Option<f64> {
    match sort {
        SortKey::Points => s.points.map(|n| n as f64),
        SortKey::Goals => s.goals.map(|n| n as f64),
        SortKey::Assists => s.assists.map(|n| n as f64),
        SortKey::Gp => Some(s.gp as f64),
        SortKey::Ppg => s.points_per_game().map(|p| p as f64),
    }
}

/// Phase Calder.4 — main entry point dispatched from main.rs.
pub async fn run(
    league: String,
    season: Option<String>,
    top: usize,
    sort: String,
    json: bool,
    csv: bool,
) -> Result<()> {
    let sort = SortKey::parse(&sort)?;
    let season_u32 = match season.as_deref() {
        None => None,
        Some(s) => Some(
            s.parse::<u32>()
                .with_context(|| format!("season '{s}' is not a YYYYZZZZ id"))?,
        ),
    };

    let store = icelines_fetch::career_landing::load_local_store();
    if store.is_empty() {
        return Err(anyhow!(
            "career history store is empty — run `icelines fetch career --bundled-seasons 5` first \
             to populate ~/.icelines/career_history.json"
        ));
    }

    let matched = project_career_rows(&store, &league, season_u32, sort);
    if matched.is_empty() {
        eprintln!(
            "No career-history rows matched league '{league}'{} in the local store.",
            season_u32
                .map(|s| format!(" / season {s}"))
                .unwrap_or_default()
        );
        eprintln!(
            "Tip: leagues are case-insensitive; try one of {}.",
            sample_leagues(&store).join(", ")
        );
        return Ok(());
    }

    // Resolve names from bundled bios (single eager scan keyed on
    // the pids we actually care about — much cheaper than per-row).
    let pids: Vec<u32> = matched.iter().map(|(pid, _)| *pid).collect();
    let names = resolve_names(&pids);

    let resolved_season = matched[0].1.season.0;
    let rows: Vec<CareerRow> = matched
        .iter()
        .take(top)
        .map(|(pid, s)| CareerRow {
            player_id: *pid,
            name: names
                .get(pid)
                .cloned()
                .unwrap_or_else(|| format!("player:{pid}")),
            team: s.team.clone(),
            gp: s.gp,
            goals: s.goals,
            assists: s.assists,
            points: s.points,
            points_per_game: s.points_per_game(),
        })
        .collect();

    if json {
        return emit_json(&league, resolved_season, sort, &rows, matched.len());
    }
    if csv {
        return emit_csv(&rows);
    }
    print_table(&league, resolved_season, sort, &rows, matched.len());
    Ok(())
}

fn print_table(league: &str, season: u32, sort: SortKey, rows: &[CareerRow], total_matched: usize) {
    let season_label = if season.to_string().len() == 8 {
        format!("{}-{}", &season.to_string()[..4], &season.to_string()[6..])
    } else {
        season.to_string()
    };
    println!(
        "{} LEADERS — {}  ·  {} of {} rows",
        league.to_uppercase(),
        season_label,
        rows.len(),
        total_matched
    );
    println!(
        "{:<4} {:<24} {:<22} {:<4} {:<4} {:<4} {:<5} {:<6}  ({})",
        "Rank",
        "Player",
        "Team",
        "GP",
        "G",
        "A",
        "P",
        "PPG",
        sort.header()
    );
    println!("{}", "─".repeat(82));
    for (i, r) in rows.iter().enumerate() {
        let name = if r.name.len() > 24 {
            &r.name[..24]
        } else {
            &r.name[..]
        };
        let team = if r.team.len() > 22 {
            &r.team[..22]
        } else {
            &r.team[..]
        };
        let ppg = r
            .points_per_game
            .map(|p| format!("{p:.2}"))
            .unwrap_or_else(|| "—".into());
        println!(
            "{:<4} {:<24} {:<22} {:<4} {:<4} {:<4} {:<5} {:<6}",
            i + 1,
            name,
            team,
            r.gp,
            r.goals.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
            r.assists
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            r.points
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            ppg,
        );
    }
}

fn emit_json(
    league: &str,
    season: u32,
    sort: SortKey,
    rows: &[CareerRow],
    total: usize,
) -> Result<()> {
    // King.2.4 envelope shape — locked by T2 (system_tests).
    #[derive(serde::Serialize)]
    struct Meta<'a> {
        league: &'a str,
        season: u32,
        sort: &'static str,
        count: usize,
        total: usize,
    }
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        schema_version: u32,
        route: &'static str,
        data: &'a [CareerRow],
        meta: Meta<'a>,
    }
    let env = Envelope {
        schema_version: 1,
        route: "career",
        data: rows,
        meta: Meta {
            league,
            season,
            sort: match sort {
                SortKey::Points => "points",
                SortKey::Goals => "goals",
                SortKey::Assists => "assists",
                SortKey::Gp => "gp",
                SortKey::Ppg => "ppg",
            },
            count: rows.len(),
            total,
        },
    };
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

fn emit_csv(rows: &[CareerRow]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record([
        "rank",
        "player_id",
        "name",
        "team",
        "gp",
        "goals",
        "assists",
        "points",
        "points_per_game",
    ])?;
    for (i, r) in rows.iter().enumerate() {
        wtr.write_record([
            (i + 1).to_string(),
            r.player_id.to_string(),
            r.name.clone(),
            r.team.clone(),
            r.gp.to_string(),
            r.goals.map(|n| n.to_string()).unwrap_or_default(),
            r.assists.map(|n| n.to_string()).unwrap_or_default(),
            r.points.map(|n| n.to_string()).unwrap_or_default(),
            r.points_per_game
                .map(|p| format!("{p:.3}"))
                .unwrap_or_default(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

/// Walk bundled bios newest-first, returning a name for each pid in
/// `wanted`. Single linear scan over the bundle — much cheaper than
/// per-row lookups for a 20-row leaderboard.
fn resolve_names(wanted: &[u32]) -> HashMap<u32, String> {
    use icelines_fetch::bundled;
    use std::collections::HashSet;
    let want: HashSet<u32> = wanted.iter().copied().collect();
    let mut out: HashMap<u32, String> = HashMap::new();
    for season_id in bundled::BUNDLED_SEASONS {
        if let Some(bios) = bundled::get_bios(season_id) {
            for b in bios {
                if want.contains(&b.player_id) {
                    out.entry(b.player_id)
                        .or_insert_with(|| b.skater_full_name.clone());
                }
            }
        }
        if let Some(goalies) = bundled::get_goalie_stats(season_id) {
            for g in goalies {
                if want.contains(&g.player_id) {
                    out.entry(g.player_id)
                        .or_insert_with(|| g.goalie_full_name.clone());
                }
            }
        }
        if out.len() == want.len() {
            break;
        }
    }
    out
}

/// Sample of leagues actually present in the store — used in the
/// "no rows matched" hint so the user isn't guessing at what to type.
fn sample_leagues(store: &CareerHistoryStore) -> Vec<String> {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for h in store.histories.values() {
        for s in &h.stints {
            *counts.entry(s.league.0.clone()).or_insert(0) += 1;
        }
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(8).map(|(l, _)| l).collect()
}

// Suppress unused-import warning when no callers reference these helpers
// directly from outside this module.
#[allow(dead_code)]
fn _silence(_: &CareerHistory) {}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::career_history::{CareerStint, LeagueAbbrev};
    use icelines_core::model::Season;

    fn stint(season: u32, league: &str, points: u32, gp: u32, pid_marker: u8) -> CareerStint {
        let _ = pid_marker;
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "Test".into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(points / 2),
            assists: Some(points - points / 2),
            points: Some(points),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    fn store_with(stints: Vec<(u32, CareerStint)>) -> CareerHistoryStore {
        let mut s = CareerHistoryStore::new();
        let mut by_pid: std::collections::HashMap<u32, Vec<CareerStint>> = Default::default();
        for (pid, st) in stints {
            by_pid.entry(pid).or_default().push(st);
        }
        for (pid, ss) in by_pid {
            s.upsert(CareerHistory {
                player_id: pid,
                stints: ss,
            });
        }
        s
    }

    /// Calder.4 / l0_filters_to_league_case_insensitive
    /// — "ohl" matches OHL stints; non-OHL filtered out.
    #[test]
    fn l0_filters_to_league_case_insensitive() {
        let s = store_with(vec![
            (1, stint(20142015, "OHL", 100, 60, 0)),
            (2, stint(20142015, "WHL", 90, 60, 0)),
            (3, stint(20142015, "ohl", 80, 60, 0)),
        ]);
        let rows = project_career_rows(&s, "ohl", Some(20142015), SortKey::Points);
        assert_eq!(rows.len(), 2, "OHL + ohl matched, WHL filtered");
    }

    /// Calder.4 / l0_default_season_is_most_recent
    /// — Without --season, narrows to the latest season for that
    ///   league (otherwise leaderboards are nonsense).
    #[test]
    fn l0_default_season_is_most_recent() {
        let s = store_with(vec![
            (1, stint(20142015, "OHL", 120, 60, 0)),
            (2, stint(20242025, "OHL", 80, 60, 0)),
        ]);
        let rows = project_career_rows(&s, "OHL", None, SortKey::Points);
        assert_eq!(rows.len(), 1, "only most-recent OHL season");
        assert_eq!(rows[0].1.season.0, 20242025);
    }

    /// Calder.4 / l0_sorts_descending_by_points
    #[test]
    fn l0_sorts_descending_by_points() {
        let s = store_with(vec![
            (1, stint(20142015, "OHL", 80, 60, 0)),
            (2, stint(20142015, "OHL", 120, 60, 0)),
            (3, stint(20142015, "OHL", 100, 60, 0)),
        ]);
        let rows = project_career_rows(&s, "OHL", Some(20142015), SortKey::Points);
        let pts: Vec<u32> = rows.iter().map(|(_, s)| s.points.unwrap()).collect();
        assert_eq!(pts, vec![120, 100, 80]);
    }

    /// Calder.4 / l0_sort_by_ppg_orders_by_rate
    #[test]
    fn l0_sort_by_ppg_orders_by_rate() {
        let s = store_with(vec![
            (1, stint(20142015, "OHL", 60, 60, 0)),  // 1.00 ppg
            (2, stint(20142015, "OHL", 80, 40, 0)),  // 2.00 ppg
            (3, stint(20142015, "OHL", 100, 80, 0)), // 1.25 ppg
        ]);
        let rows = project_career_rows(&s, "OHL", Some(20142015), SortKey::Ppg);
        let pids: Vec<u32> = rows.iter().map(|(p, _)| *p).collect();
        assert_eq!(pids, vec![2, 3, 1], "highest ppg first");
    }

    /// Calder.4 / l0_skips_playoff_stints
    #[test]
    fn l0_skips_playoff_stints() {
        let mut playoff = stint(20142015, "OHL", 50, 20, 0);
        playoff.game_type = CareerGameType::Playoff;
        let s = store_with(vec![(1, stint(20142015, "OHL", 100, 60, 0)), (2, playoff)]);
        let rows = project_career_rows(&s, "OHL", Some(20142015), SortKey::Points);
        assert_eq!(rows.len(), 1);
    }

    /// Calder.4 / l0_sort_key_aliases_resolve
    #[test]
    fn l0_sort_key_aliases_resolve() {
        assert_eq!(SortKey::parse("p").unwrap(), SortKey::Points);
        assert_eq!(SortKey::parse("PTS").unwrap(), SortKey::Points);
        assert_eq!(SortKey::parse("goals").unwrap(), SortKey::Goals);
        assert_eq!(SortKey::parse("g").unwrap(), SortKey::Goals);
        assert_eq!(SortKey::parse("ppg").unwrap(), SortKey::Ppg);
        assert_eq!(SortKey::parse("gp").unwrap(), SortKey::Gp);
        assert!(SortKey::parse("xyz").is_err());
    }

    /// Calder.4 / l0_sample_leagues_returns_top_by_count
    #[test]
    fn l0_sample_leagues_returns_top_by_count() {
        let s = store_with(vec![
            (1, stint(20142015, "OHL", 100, 60, 0)),
            (2, stint(20142015, "OHL", 90, 60, 0)),
            (3, stint(20142015, "WHL", 80, 60, 0)),
        ]);
        let leagues = sample_leagues(&s);
        assert_eq!(leagues[0], "OHL", "OHL count 2 should rank first");
        assert!(leagues.contains(&"WHL".to_owned()));
    }
}
