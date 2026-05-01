//! Phase Hart — normalized per-season stats.
//!
//! `SeasonStats` is keyed on `(player_id, season, season_type)`. Goalie
//! and skater rows share infrastructure; goalie-specific fields hang off
//! `SeasonStats.goalie` rather than living on a parallel species.
//!
//! `RealtimeStats` and `AdvancedStats` keep their fields `pub` so the
//! loader in `icelines-fetch` can populate them without going through
//! `pub(crate)` plumbing. Reads should still flow through `PlayerView`
//! accessors (Hart.2) — the WIRE/TAPE/EDGE reviews flagged direct field
//! reads outside `model` and the migrated allow-list as a CI-guard
//! concern, not a type-system one.

use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{PaceScore, Position, Season, TeamAbbr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SeasonType {
    Regular,
    Playoff,
}

impl SeasonType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Playoff => "playoff",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatTotals {
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub pim: u32,
    pub shots: u32,
    #[serde(default)]
    pub shooting_pct: Option<f32>,
    #[serde(default)]
    pub toi_per_game_sec: Option<u32>,
    pub pp_goals: u32,
    pub pp_points: u32,
    pub gwg: u32,
    #[serde(default)]
    pub faceoff_win_pct: Option<f32>,
    #[serde(default)]
    pub pace_score: Option<PaceScore>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RealtimeStats {
    pub hits: u32,
    pub blocked_shots: u32,
    pub takeaways: u32,
    pub giveaways: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AdvancedStats {
    #[serde(default)]
    pub xg: Option<f64>,
    #[serde(default)]
    pub xg_per_60: Option<f64>,
    #[serde(default)]
    pub cf_pct: Option<f64>,
    #[serde(default)]
    pub ff_pct: Option<f64>,
    #[serde(default)]
    pub xgf_pct: Option<f64>,
}

/// Per-stint goalie counts for a mid-season-traded goalie.
/// `None` for skater stints.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalieStintStats {
    pub games_started: u32,
    pub wins: u32,
    pub losses: u32,
    #[serde(default)]
    pub ot_losses: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamStint {
    pub team: TeamAbbr,
    #[serde(default)]
    pub started: Option<String>,
    #[serde(default)]
    pub ended: Option<String>,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    /// Per-stint goalie counts. `None` for skater stints.
    #[serde(default)]
    pub goalie: Option<GoalieStintStats>,
}

/// Goalie season-aggregate. Lives on `SeasonStats.goalie` rather than as
/// a parallel `Goalie` species. `qualified_for(season_type, gp)` carries
/// the 15/4 GP threshold (regular / playoff). The new shape drops
/// `games_played` — derive from `team_stints.iter().map(|s| s.gp).sum()`
/// or from the parent `SeasonStats.totals.gp`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalieSeasonStats {
    pub games_started: u32,
    pub wins: u32,
    pub losses: u32,
    #[serde(default)]
    pub ot_losses: Option<u32>,
    #[serde(default)]
    pub ties: Option<u32>,
    pub shots_against: u32,
    pub goals_against: u32,
    pub saves: u32,
    #[serde(default)]
    pub save_pct: Option<f32>,
    #[serde(default)]
    pub goals_against_average: Option<f32>,
    pub shutouts: u32,
    pub time_on_ice: u32,
}

impl GoalieSeasonStats {
    /// True iff this goalie cleared the season-type-aware GP minimum.
    /// 15 GP regular season; 4 GP playoff (a Cup-final losing starter
    /// still qualifies). `gp` is passed in by the caller from
    /// `SeasonStats.totals.gp` so the threshold tracks actual games
    /// dressed, not just games started.
    pub fn qualified_for(&self, season_type: SeasonType, gp: u32) -> bool {
        let min = match season_type {
            SeasonType::Regular => 15,
            SeasonType::Playoff => 4,
        };
        gp >= min
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonStats {
    pub player_id: PlayerId,
    pub season: Season,
    pub season_type: SeasonType,

    /// Per-season fact: position can shift across seasons (Marchand
    /// 2017-18 C → 2018-19 LW; emergency-backup-goalie scenarios).
    pub position: Position,

    /// Per-season fact: sweater can change across teams.
    #[serde(default)]
    pub sweater_number: Option<u32>,

    /// One stint per team played for this season+type. `len() >= 1`.
    /// Sorted chronologically by `started`; lexicographic-by-team
    /// tiebreak when both starteds are `None`. The builder enforces this.
    pub team_stints: Vec<TeamStint>,

    pub totals: StatTotals,

    /// Realtime stats (hits, blocks, takeaways, giveaways) when the
    /// realtime endpoint has been fetched. None during cold-start.
    #[serde(default)]
    pub realtime: Option<RealtimeStats>,

    /// MoneyPuck advanced (xG, CF%, FF%, xGF%) when available.
    #[serde(default)]
    pub advanced: Option<AdvancedStats>,

    /// Populated when this player suited up as a goalie this season+type
    /// (matches the TAPE-revised "is_goalie is derived" policy).
    #[serde(default)]
    pub goalie: Option<GoalieSeasonStats>,
}

impl SeasonStats {
    /// True iff this row represents a goalie outing (matches
    /// `goalie.is_some()`). Hart's "is_goalie is per-row, derived"
    /// policy collapses through this accessor.
    pub fn is_goalie(&self) -> bool {
        self.goalie.is_some()
    }
}

/// Tagged projection — a points-per-82 figure for a regular season,
/// versus a per-game figure for a playoff series (where there's no 82).
/// FORGE: tagging prevents silent unit mixing in Phase S's accessor
/// pattern.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Projection {
    Per82(f64),
    PerGame(f64),
}

impl Projection {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Per82(_) => "/82",
            Self::PerGame(_) => "/g",
        }
    }

    pub fn value(&self) -> f64 {
        match self {
            Self::Per82(v) | Self::PerGame(v) => *v,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Per82(v) => format!("{v:.1}/82"),
            Self::PerGame(v) => format!("{v:.2}/g"),
        }
    }
}

impl PaceScore {
    /// Points per game = raw_points / gp. Returns 0.0 if gp is 0
    /// (callers should also check `gp > 0` before relying on this).
    pub fn points_per_game(&self) -> f64 {
        if self.gp == 0 {
            0.0
        } else {
            self.raw_points as f64 / self.gp as f64
        }
    }

    /// Tagged projection for the given season type. Regular-season
    /// projection is per-82; playoff projection is per-game.
    pub fn projected_for(&self, season_type: SeasonType) -> Projection {
        match season_type {
            SeasonType::Regular => Projection::Per82(self.pace_82),
            SeasonType::Playoff => Projection::PerGame(self.points_per_game()),
        }
    }
}

/// Builder for `SeasonStats`. The recommended construction path for the
/// loader in `icelines-fetch` and for fixtures in tests. Validates
/// invariants (`team_stints.len() >= 1`) and sorts stints into the
/// canonical order (by `started`, then by team-abbrev as tiebreak when
/// both `started` are `None`).
pub struct SeasonStatsBuilder {
    player_id: PlayerId,
    season: Season,
    season_type: SeasonType,
    position: Position,
    sweater_number: Option<u32>,
    team_stints: Vec<TeamStint>,
    totals: StatTotals,
    realtime: Option<RealtimeStats>,
    advanced: Option<AdvancedStats>,
    goalie: Option<GoalieSeasonStats>,
}

impl SeasonStatsBuilder {
    pub fn new(
        player_id: PlayerId,
        season: Season,
        season_type: SeasonType,
        position: Position,
    ) -> Self {
        Self {
            player_id,
            season,
            season_type,
            position,
            sweater_number: None,
            team_stints: Vec::new(),
            totals: StatTotals::default(),
            realtime: None,
            advanced: None,
            goalie: None,
        }
    }

    pub fn with_sweater_number(mut self, n: u32) -> Self {
        self.sweater_number = Some(n);
        self
    }

    pub fn with_team_stints(mut self, stints: Vec<TeamStint>) -> Self {
        self.team_stints = stints;
        self
    }

    pub fn add_team_stint(mut self, stint: TeamStint) -> Self {
        self.team_stints.push(stint);
        self
    }

    pub fn with_totals(mut self, totals: StatTotals) -> Self {
        self.totals = totals;
        self
    }

    pub fn with_realtime(mut self, r: RealtimeStats) -> Self {
        self.realtime = Some(r);
        self
    }

    pub fn with_advanced(mut self, a: AdvancedStats) -> Self {
        self.advanced = Some(a);
        self
    }

    pub fn with_goalie(mut self, g: GoalieSeasonStats) -> Self {
        self.goalie = Some(g);
        self
    }

    /// Finalize. Panics in debug if `team_stints` is empty (a SeasonStats
    /// row without a team violates the invariant — caller bug). Sorts
    /// stints into canonical order so consumers can rely on
    /// `team_stints.last()` for the most-recent team.
    pub fn build(mut self) -> SeasonStats {
        debug_assert!(
            !self.team_stints.is_empty(),
            "SeasonStats requires at least one TeamStint (player must have played for someone)"
        );
        self.team_stints
            .sort_by(|a, b| match (a.started.as_ref(), b.started.as_ref()) {
                (Some(x), Some(y)) => x.cmp(y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.team.as_str().cmp(b.team.as_str()),
            });
        SeasonStats {
            player_id: self.player_id,
            season: self.season,
            season_type: self.season_type,
            position: self.position,
            sweater_number: self.sweater_number,
            team_stints: self.team_stints,
            totals: self.totals,
            realtime: self.realtime,
            advanced: self.advanced,
            goalie: self.goalie,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PaceScore;

    fn stl_stint(gp: u32, g: u32, a: u32) -> TeamStint {
        TeamStint {
            team: TeamAbbr("STL".into()),
            started: Some("2022-10-15".into()),
            ended: Some("2023-02-09".into()),
            gp,
            goals: g,
            assists: a,
            points: g + a,
            goalie: None,
        }
    }

    fn nyr_stint(gp: u32, g: u32, a: u32) -> TeamStint {
        TeamStint {
            team: TeamAbbr("NYR".into()),
            started: Some("2023-02-10".into()),
            ended: Some("2023-04-13".into()),
            gp,
            goals: g,
            assists: a,
            points: g + a,
            goalie: None,
        }
    }

    fn skater_totals(gp: u32, g: u32, a: u32) -> StatTotals {
        StatTotals {
            gp,
            goals: g,
            assists: a,
            points: g + a,
            ..Default::default()
        }
    }

    #[test]
    fn l0_hart1_builder_canonical_stint_order_with_dates() {
        // Insert NYR before STL; builder must sort STL first because
        // its started date is earlier.
        let stats = SeasonStatsBuilder::new(
            PlayerId(8475765),
            Season(20222023),
            SeasonType::Regular,
            Position::RightWing,
        )
        .add_team_stint(nyr_stint(31, 8, 13))
        .add_team_stint(stl_stint(38, 10, 11))
        .with_totals(skater_totals(69, 18, 24))
        .build();

        assert_eq!(stats.team_stints[0].team.as_str(), "STL");
        assert_eq!(stats.team_stints[1].team.as_str(), "NYR");
    }

    #[test]
    fn l0_hart1_builder_round_trip_serde() {
        let stats = SeasonStatsBuilder::new(
            PlayerId(8475765),
            Season(20222023),
            SeasonType::Regular,
            Position::RightWing,
        )
        .with_sweater_number(91)
        .add_team_stint(stl_stint(38, 10, 11))
        .add_team_stint(nyr_stint(31, 8, 13))
        .with_totals(skater_totals(69, 18, 24))
        .build();

        let s = serde_json::to_string(&stats).unwrap();
        let back: SeasonStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, stats);
    }

    #[test]
    fn l0_hart1_serde_default_on_missing_optionals() {
        // Pre-Hart bundle wouldn't have realtime/advanced/goalie — should parse.
        let json = r#"{
            "player_id": 8475765,
            "season": 20222023,
            "season_type": "Regular",
            "position": "RightWing",
            "team_stints": [
                {"team": "NYR", "gp": 7, "goals": 1, "assists": 0, "points": 1}
            ],
            "totals": {
                "gp": 7, "goals": 1, "assists": 0, "points": 1,
                "plus_minus": 0, "pim": 0, "shots": 0, "pp_goals": 0,
                "pp_points": 0, "gwg": 0
            }
        }"#;
        let stats: SeasonStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.realtime, None);
        assert_eq!(stats.advanced, None);
        assert_eq!(stats.goalie, None);
        assert_eq!(stats.sweater_number, None);
        assert_eq!(stats.team_stints[0].started, None);
    }

    /// Mid-playoff goalie trade — synthetic. No real bundled goalie
    /// playoff trade exists in the 5 seasons we have, so this builds the
    /// shape from scratch and asserts sum-equals between stints and
    /// goalie-row aggregates.
    #[test]
    fn l0_hart1_mid_playoff_goalie_trade_synthetic() {
        let stint_a = TeamStint {
            team: TeamAbbr("BOS".into()),
            started: Some("2024-04-22".into()),
            ended: Some("2024-04-30".into()),
            gp: 3,
            goals: 0,
            assists: 0,
            points: 0,
            goalie: Some(GoalieStintStats {
                games_started: 3,
                wins: 1,
                losses: 2,
                ot_losses: Some(0),
            }),
        };
        let stint_b = TeamStint {
            team: TeamAbbr("FLA".into()),
            started: Some("2024-05-01".into()),
            ended: Some("2024-06-15".into()),
            gp: 7,
            goals: 0,
            assists: 0,
            points: 0,
            goalie: Some(GoalieStintStats {
                games_started: 7,
                wins: 4,
                losses: 3,
                ot_losses: Some(0),
            }),
        };

        let goalie_totals = GoalieSeasonStats {
            games_started: 10,
            wins: 5,
            losses: 5,
            ot_losses: Some(0),
            ties: None,
            shots_against: 280,
            goals_against: 28,
            saves: 252,
            save_pct: Some(0.900),
            goals_against_average: Some(2.80),
            shutouts: 0,
            time_on_ice: 600 * 60,
        };

        let stats = SeasonStatsBuilder::new(
            PlayerId(9999999),
            Season(20232024),
            SeasonType::Playoff,
            Position::Goalie,
        )
        .add_team_stint(stint_a)
        .add_team_stint(stint_b)
        .with_totals(StatTotals {
            gp: 10,
            ..Default::default()
        })
        .with_goalie(goalie_totals)
        .build();

        // Sum-equals invariant: per-stint goalie counts add up to the
        // season-aggregate goalie row.
        let starts: u32 = stats
            .team_stints
            .iter()
            .map(|s| s.goalie.as_ref().map(|g| g.games_started).unwrap_or(0))
            .sum();
        let wins: u32 = stats
            .team_stints
            .iter()
            .map(|s| s.goalie.as_ref().map(|g| g.wins).unwrap_or(0))
            .sum();
        let losses: u32 = stats
            .team_stints
            .iter()
            .map(|s| s.goalie.as_ref().map(|g| g.losses).unwrap_or(0))
            .sum();
        let g = stats.goalie.as_ref().unwrap();
        assert_eq!(starts, g.games_started);
        assert_eq!(wins, g.wins);
        assert_eq!(losses, g.losses);
        assert!(stats.is_goalie());
    }

    #[test]
    fn l0_hart1_goalie_qualified_for_thresholds() {
        let g = GoalieSeasonStats {
            games_started: 5,
            ..Default::default()
        };
        assert!(!g.qualified_for(SeasonType::Regular, 14));
        assert!(g.qualified_for(SeasonType::Regular, 15));
        assert!(!g.qualified_for(SeasonType::Playoff, 3));
        assert!(g.qualified_for(SeasonType::Playoff, 4));
    }

    #[test]
    fn l0_hart1_projection_render_and_label() {
        let p82 = Projection::Per82(138.05);
        assert_eq!(p82.label(), "/82");
        assert_eq!(p82.render(), "138.1/82");

        let pg = Projection::PerGame(0.954);
        assert_eq!(pg.label(), "/g");
        assert_eq!(pg.render(), "0.95/g");
    }

    #[test]
    fn l0_hart1_pace_score_projected_for() {
        let pace = PaceScore {
            pace_82: 110.0,
            goals_per_82: 40.0,
            raw_points: 25,
            gp: 22,
        };
        match pace.projected_for(SeasonType::Regular) {
            Projection::Per82(v) => assert!((v - 110.0).abs() < f64::EPSILON),
            other => panic!("expected Per82, got {other:?}"),
        }
        match pace.projected_for(SeasonType::Playoff) {
            Projection::PerGame(v) => {
                let want = 25.0_f64 / 22.0_f64;
                assert!((v - want).abs() < 1e-9);
            }
            other => panic!("expected PerGame, got {other:?}"),
        }
    }

    proptest::proptest! {
        /// Round-trip: insert stints in arbitrary order, builder sorts
        /// them by `started` (when present) and falls back to lexicographic
        /// team-abbrev tiebreak when both `started` are None.
        #[test]
        fn teamstint_ordering_none_started_tiebreak(
            t1 in "[A-Z]{3}",
            t2 in "[A-Z]{3}",
        ) {
            proptest::prop_assume!(t1 != t2);
            let s1 = TeamStint {
                team: TeamAbbr(t1.clone()),
                started: None, ended: None,
                gp: 10, goals: 1, assists: 1, points: 2, goalie: None,
            };
            let s2 = TeamStint {
                team: TeamAbbr(t2.clone()),
                started: None, ended: None,
                gp: 10, goals: 1, assists: 1, points: 2, goalie: None,
            };
            let stats = SeasonStatsBuilder::new(
                PlayerId(1),
                Season(20232024),
                SeasonType::Regular,
                Position::Center,
            )
            .add_team_stint(s2.clone())
            .add_team_stint(s1.clone())
            .with_totals(skater_totals(20, 2, 2))
            .build();
            let mut want = [t1.clone(), t2.clone()];
            want.sort();
            proptest::prop_assert_eq!(stats.team_stints[0].team.as_str(), want[0].as_str());
            proptest::prop_assert_eq!(stats.team_stints[1].team.as_str(), want[1].as_str());
        }
    }
}
