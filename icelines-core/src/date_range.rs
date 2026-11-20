//! Team scoring over inclusive NHL game dates, across season boundaries.
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameScoring {
    pub player_id: u32,
    pub skater_full_name: String,
    pub position_code: String,
    pub team_abbrev: String,
    pub game_id: u64,
    pub game_date: NaiveDate,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ev_points: u32,
    pub pp_points: u32,
    pub sh_points: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRow {
    pub player_id: u32,
    pub name: String,
    pub position: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ev_points: u32,
    pub pp_points: u32,
    pub sh_points: u32,
    pub points_per_game: f64,
    pub ev_points_per_game: f64,
    pub points_rank: usize,
    pub ev_points_rank: usize,
}

pub const SCORING_COLUMNS: &[(&str, &str)] = &[
    ("points-rank", "P rank"),
    ("name", "Player"),
    ("position", "Pos"),
    ("games", "GP"),
    ("goals", "G"),
    ("assists", "A"),
    ("points", "P"),
    ("ev-points", "EV P"),
    ("ev-rank", "EV rank"),
    ("ppg", "P/G"),
    ("ev-ppg", "EVP/G"),
    ("pp-points", "PP P"),
    ("sh-points", "SH P"),
];
pub fn valid_scoring_sort(sort: &str) -> bool {
    SCORING_COLUMNS.iter().any(|(key, _)| *key == sort)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFilter {
    #[serde(default)]
    pub then_by: String,
    pub position: String,
    pub roster_scope: String,
    pub sort: String,
    #[serde(default = "descending")]
    pub order: String,
}
fn descending() -> String {
    "desc".into()
}
impl Default for ScoringFilter {
    fn default() -> Self {
        Self {
            then_by: String::new(),
            position: String::new(),
            roster_scope: "all".into(),
            sort: "points".into(),
            order: descending(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortTerm {
    pub key: String,
    pub order: String,
}

pub fn parse_sort_chain(
    primary: &str,
    order: &str,
    then_by: &str,
    valid: impl Fn(&str) -> bool,
) -> Result<Vec<SortTerm>, String> {
    let mut chain = vec![SortTerm {
        key: primary.into(),
        order: order.into(),
    }];
    if !then_by.trim().is_empty() {
        for term in then_by.split(',') {
            let (key, order) = term.trim().split_once(':').unwrap_or((term.trim(), "desc"));
            chain.push(SortTerm {
                key: key.trim().into(),
                order: order.trim().into(),
            });
        }
    }
    let mut seen = BTreeSet::new();
    if chain.len() > 32 {
        return Err("too many sort columns".into());
    }
    for term in &chain {
        if !valid(&term.key) {
            return Err(format!("unsupported sort column: {}", term.key));
        }
        if !matches!(term.order.as_str(), "asc" | "desc") {
            return Err("order must be asc or desc".into());
        }
        if !seen.insert(term.key.clone()) {
            return Err(format!("duplicate sort column: {}", term.key));
        }
    }
    Ok(chain)
}
impl ScoringFilter {
    pub fn sort_chain(&self) -> Result<Vec<SortTerm>, String> {
        parse_sort_chain(&self.sort, &self.order, &self.then_by, valid_scoring_sort)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.position.as_str(), "" | "F" | "C" | "LW" | "RW" | "D") {
            return Err("position must be F, C, LW, RW or D (or blank for all skaters)".into());
        }
        if !matches!(self.roster_scope.as_str(), "all" | "current" | "former") {
            return Err("roster scope must be all, current or former".into());
        }
        self.sort_chain()?;
        Ok(())
    }
}

/// Filter player rows, then recompute competition ranks within the selected
/// population. Roster membership never changes which historical games count.
pub fn select_scoring_rows(
    mut rows: Vec<ScoringRow>,
    filter: &ScoringFilter,
    current_roster: Option<&BTreeSet<u32>>,
) -> Result<Vec<ScoringRow>, String> {
    filter.validate()?;
    if filter.roster_scope != "all" && current_roster.is_none() {
        return Err(
            "current-roster evidence unavailable; refresh the roster before applying this scope"
                .into(),
        );
    }
    rows.retain(|r| {
        let position = match r.position.as_str() {
            "L" => "LW",
            "R" => "RW",
            p => p,
        };
        let position_matches = match filter.position.as_str() {
            "" => true,
            "F" => matches!(position, "C" | "LW" | "RW" | "F"),
            p => p == position,
        };
        let on_roster = current_roster.is_some_and(|ids| ids.contains(&r.player_id));
        position_matches
            && match filter.roster_scope.as_str() {
                "current" => on_roster,
                "former" => !on_roster,
                _ => true,
            }
    });
    let counts: Vec<_> = rows.iter().map(|r| (r.points, r.ev_points)).collect();
    for r in &mut rows {
        if r.games == 0 {
            return Err("per-game scoring requires at least one game".into());
        }
        r.points_per_game = f64::from(r.points) / f64::from(r.games);
        r.ev_points_per_game = f64::from(r.ev_points) / f64::from(r.games);
        r.points_rank = 1 + counts.iter().filter(|(p, _)| *p > r.points).count();
        r.ev_points_rank = 1 + counts.iter().filter(|(_, ev)| *ev > r.ev_points).count();
    }
    let chain = filter.sort_chain()?;
    rows.sort_by(|a, b| {
        let primary = {
            let mut result = std::cmp::Ordering::Equal;
            for term in &chain {
                let ascending = match term.key.as_str() {
                    "name" => a.name.cmp(&b.name),
                    "position" => a.position.cmp(&b.position),
                    "games" => a.games.cmp(&b.games),
                    "goals" => a.goals.cmp(&b.goals),
                    "assists" => a.assists.cmp(&b.assists),
                    "ev-points" => a.ev_points.cmp(&b.ev_points),
                    "pp-points" => a.pp_points.cmp(&b.pp_points),
                    "sh-points" => a.sh_points.cmp(&b.sh_points),
                    "points-rank" => a.points_rank.cmp(&b.points_rank),
                    "ev-rank" => a.ev_points_rank.cmp(&b.ev_points_rank),
                    "ppg" => a.points_per_game.total_cmp(&b.points_per_game),
                    "ev-ppg" => a.ev_points_per_game.total_cmp(&b.ev_points_per_game),
                    _ => a.points.cmp(&b.points),
                };
                let primary = if term.order == "desc" {
                    ascending.reverse()
                } else {
                    ascending
                };
                if primary != std::cmp::Ordering::Equal {
                    result = primary;
                    break;
                }
            }
            result
        };
        primary
            .then_with(|| b.points.cmp(&a.points))
            .then_with(|| b.goals.cmp(&a.goals))
            .then_with(|| a.player_id.cmp(&b.player_id))
    });
    Ok(rows)
}

/// Reject incomplete/duplicated or incorrectly scoped source records rather than
/// displaying plausible but wrong counts. EV is the official evPoints field.
pub fn scoring_rows(
    games: &[GameScoring],
    team: &str,
    from: NaiveDate,
    to: NaiveDate,
    game_type: u8,
    sort: &str,
) -> Result<Vec<ScoringRow>, String> {
    if from > to {
        return Err("start date must be on or before end date".into());
    }
    if !matches!(game_type, 2 | 3) {
        return Err("game type must be regular or playoff".into());
    }
    if !valid_scoring_sort(sort) {
        return Err("sort must be a date-range table column key".into());
    }
    let mut seen = BTreeSet::new();
    let mut rows = BTreeMap::<u32, ScoringRow>::new();
    for g in games {
        if g.team_abbrev != team
            || g.game_date < from
            || g.game_date > to
            || g.game_id / 10000 % 100 != u64::from(game_type)
            || g.games_played != 1
        {
            return Err("source row is outside requested team/date/game-type scope".into());
        }
        if !seen.insert((g.game_id, g.player_id)) {
            return Err("duplicate player/game record".into());
        }
        if g.points != g.goals + g.assists || g.points != g.ev_points + g.pp_points + g.sh_points {
            return Err("source scoring breakdown does not reconcile".into());
        }
        let r = rows.entry(g.player_id).or_insert_with(|| ScoringRow {
            player_id: g.player_id,
            name: g.skater_full_name.clone(),
            position: g.position_code.clone(),
            games: 0,
            goals: 0,
            assists: 0,
            points: 0,
            ev_points: 0,
            pp_points: 0,
            sh_points: 0,
            points_per_game: 0.0,
            ev_points_per_game: 0.0,
            points_rank: 0,
            ev_points_rank: 0,
        });
        r.games += 1;
        r.goals += g.goals;
        r.assists += g.assists;
        r.points += g.points;
        r.ev_points += g.ev_points;
        r.pp_points += g.pp_points;
        r.sh_points += g.sh_points;
    }
    select_scoring_rows(
        rows.into_values().collect(),
        &ScoringFilter {
            sort: sort.into(),
            ..Default::default()
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(id: u64, date: &str, player: u32, ev: u32, pp: u32) -> GameScoring {
        GameScoring {
            player_id: player,
            skater_full_name: format!("Skater {player}"),
            position_code: "C".into(),
            team_abbrev: "BOS".into(),
            game_id: id,
            game_date: date.parse().unwrap(),
            games_played: 1,
            goals: ev + pp,
            assists: 0,
            points: ev + pp,
            ev_points: ev,
            pp_points: pp,
            sh_points: 0,
        }
    }
    fn run(games: &[GameScoring], sort: &str) -> Result<Vec<ScoringRow>, String> {
        scoring_rows(
            games,
            "BOS",
            "2026-02-28".parse().unwrap(),
            "2026-11-11".parse().unwrap(),
            2,
            sort,
        )
    }
    #[test]
    fn date_range_cross_season_boundaries_ev_and_tied_ranks() {
        let games = [
            game(2025020001, "2026-02-28", 1, 1, 0),
            game(2026020001, "2026-11-11", 1, 0, 2),
            game(2026020001, "2026-11-11", 2, 2, 1),
        ];
        let rows = run(&games, "ev-points").unwrap();
        assert_eq!(rows[0].player_id, 2);
        assert_eq!(rows[1].games, 2);
        assert_eq!(rows[0].points_rank, 1);
        assert_eq!(rows[1].points_rank, 1);
        assert_eq!(rows[1].ev_points_rank, 2);
    }
    #[test]
    fn date_range_filters_rosters_positions_rates_and_recomputes_ranks() {
        let mut games = vec![
            game(2025020001, "2026-02-28", 1, 1, 0),
            game(2026020001, "2026-11-11", 1, 0, 2),
            game(2026020001, "2026-11-11", 2, 2, 1),
            game(2026020001, "2026-11-11", 3, 10, 0),
            game(2026020001, "2026-11-11", 4, 0, 0),
        ];
        games[0].position_code = "L".into();
        games[1].position_code = "L".into();
        games[2].position_code = "RW".into();
        games[3].position_code = "D".into();
        let rows = run(&games, "ppg").unwrap();
        assert_eq!(rows[0].player_id, 3);
        let roster = BTreeSet::from([1, 3]);
        let mut filter = ScoringFilter {
            then_by: String::new(),
            position: "F".into(),
            roster_scope: "all".into(),
            sort: "ev-ppg".into(),
            order: "desc".into(),
        };
        let forwards = select_scoring_rows(rows.clone(), &filter, None).unwrap();
        assert_eq!(forwards.len(), 3);
        assert_eq!(forwards[0].player_id, 2);
        assert_eq!(forwards[1].points_per_game, 1.5);
        assert_eq!(forwards[1].ev_points_per_game, 0.5);
        assert_eq!(forwards[0].points_rank, 1);
        assert_eq!(forwards[1].points_rank, 1);
        filter.roster_scope = "current".into();
        assert!(select_scoring_rows(rows.clone(), &filter, None).is_err());
        let current = select_scoring_rows(rows.clone(), &filter, Some(&roster)).unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].games, 2);
        assert_eq!(current[0].points, 3);
        assert_eq!(current[0].ev_points_rank, 1);
        filter.roster_scope = "former".into();
        let former = select_scoring_rows(rows.clone(), &filter, Some(&roster)).unwrap();
        assert_eq!(
            former.iter().map(|r| r.player_id).collect::<Vec<_>>(),
            vec![2, 4]
        );
        filter.position = "LW".into();
        filter.roster_scope = "all".into();
        assert_eq!(
            select_scoring_rows(rows, &filter, None).unwrap()[0].player_id,
            1
        );
    }
    #[test]
    fn date_range_sort_direction_changes_order_without_changing_stats_or_ranks() {
        let games = [
            game(2025020001, "2026-02-28", 1, 1, 0),
            game(2026020001, "2026-11-11", 1, 1, 1),
            game(2026020001, "2026-11-11", 2, 2, 0),
        ];
        let rows = run(&games, "points").unwrap();
        let mut filter = ScoringFilter {
            sort: "ppg".into(),
            ..Default::default()
        };
        let desc = select_scoring_rows(rows.clone(), &filter, None).unwrap();
        assert_eq!(
            desc.iter().map(|r| r.player_id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(desc[1].points_per_game, 1.5);
        assert_eq!(desc[1].points, 3);
        assert_eq!(desc[1].points_rank, 1);
        filter.order = "asc".into();
        let asc = select_scoring_rows(rows.clone(), &filter, None).unwrap();
        assert_eq!(
            asc.iter().map(|r| r.player_id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(asc[0].ev_points_per_game, 1.0);
        assert_eq!(asc[0].points_rank, 1);
        filter.sort = "goals".into();
        filter.order = "desc".into();
        assert_eq!(
            select_scoring_rows(rows.clone(), &filter, None).unwrap()[0].goals,
            3
        );
        filter.sort = "name".into();
        filter.order = "asc".into();
        assert_eq!(
            select_scoring_rows(rows, &filter, None).unwrap()[0].name,
            "Skater 1"
        );
        filter.order = "sideways".into();
        assert!(filter.validate().is_err());
    }
    #[test]
    fn date_range_secondary_rates_precede_implicit_tiebreaks_and_reject_duplicate_keys() {
        let games = [
            game(2025020001, "2026-02-28", 1, 1, 2),
            game(2025020001, "2026-02-28", 2, 2, 1),
            game(2025020001, "2026-02-28", 3, 3, 0),
        ];
        let rows = run(&games, "ppg").unwrap();
        let mut filter = ScoringFilter {
            sort: "ppg".into(),
            then_by: "ev-ppg:desc".into(),
            ..Default::default()
        };
        let result = select_scoring_rows(rows.clone(), &filter, None).unwrap();
        assert_eq!(
            result.iter().map(|r| r.player_id).collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert!(result
            .iter()
            .all(|r| r.points_per_game == 3.0 && r.points_rank == 1));
        filter.then_by = "ev-ppg:asc,name:desc".into();
        assert_eq!(
            select_scoring_rows(rows, &filter, None).unwrap()[0].player_id,
            1
        );
        filter.then_by = "ppg:asc".into();
        assert!(filter.validate().is_err());
        filter.then_by = "ev-ppg:desc,ev-ppg:asc".into();
        assert!(filter.validate().is_err());
        filter.then_by = "ev-ppg:sideways".into();
        assert!(filter.validate().is_err());
    }
    #[test]
    fn date_range_rejects_duplicates_wrong_team_dates_type_and_missing_strength() {
        let g = game(2025020001, "2026-02-28", 1, 1, 0);
        assert!(run(&[g.clone(), g.clone()], "points").is_err());
        let mut bad = g.clone();
        bad.team_abbrev = "SEA".into();
        assert!(run(&[bad], "points").is_err());
        let mut bad = g.clone();
        bad.game_date = "2026-02-27".parse().unwrap();
        assert!(run(&[bad], "points").is_err());
        let mut bad = g.clone();
        bad.game_id = 2025030001;
        assert!(run(&[bad], "points").is_err());
        let mut bad = g;
        bad.ev_points = 0;
        assert!(run(&[bad], "points").is_err());
    }
}
