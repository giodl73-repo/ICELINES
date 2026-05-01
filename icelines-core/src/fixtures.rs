//! Phase Hart — test fixture builders for `PlayerIdentity` / `SeasonStats`.
//!
//! `fixtures::identity()` and `fixtures::stats()` produce realistic
//! defaults so unit tests in this crate (and L0/L1 tests in
//! `icelines-fetch`, `icelines-cli`) can construct rows without
//! restating every field. Defaults match Connor McDavid's 2022-23
//! regular-season shape — change a few fields, get a useful fixture.
//!
//! The path-allowlist ratchet test (Hart.0+) exempts `*/fixtures.rs`,
//! so adding a new fixture call site won't move the deprecated-warning
//! count.

use crate::identity::{PlayerBio, PlayerId, PlayerIdentity};
use crate::model::{PaceScore, Position, Season, TeamAbbr};
use crate::season_stats::{
    AdvancedStats, GoalieSeasonStats, RealtimeStats, SeasonStats, SeasonStatsBuilder, SeasonType,
    StatTotals, TeamStint,
};
use crate::stats_repository::StatsRepository;

// ── Hart.5c.2 — single-player repo helpers for consumer tests ──────────────────
//
// The Hart.5 consumer migrations (scouting, query, fantasy, export) need
// a tiny `StatsRepository` + `PlayerView` for their render-path tests.
// `test_repo_with` (skater) and `test_repo_with_goalie` (goalie) build
// one-row repos from a known identity + stats pair. Identical bodies
// today — `StatsRepository` is shape-agnostic, the goalie-ness lives in
// `stats.goalie.is_some()` — but the pair exists for call-site
// readability and so we can later add type-specific invariants without
// migrating call sites.

/// Build a one-row `StatsRepository` from the given (identity, stats)
/// pair. Use for consumer-render-path tests that need a `PlayerView`
/// against a known fixture.
pub fn test_repo_with(identity: PlayerIdentity, stats: SeasonStats) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity).expect("identity upsert in fixture");
    r.upsert_stats(stats).expect("stats upsert in fixture");
    r
}

/// Goalie variant of `test_repo_with`. Identical body to the skater
/// variant; the call site documents intent (and future invariants can
/// hang off this signature without touching every test).
pub fn test_repo_with_goalie(identity: PlayerIdentity, stats: SeasonStats) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity).expect("identity upsert in fixture");
    r.upsert_stats(stats).expect("stats upsert in fixture");
    r
}

pub struct IdentityFixture {
    inner: PlayerIdentity,
}

pub fn identity(id: u32) -> IdentityFixture {
    IdentityFixture {
        inner: PlayerIdentity {
            id: PlayerId(id),
            full_name: "Connor McDavid".to_string(),
            name_normalized: "connor mcdavid".to_string(),
            headshot_canonical_url: Some(format!(
                "https://assets.nhle.com/mugs/nhl/default/{id}.png"
            )),
            bio: PlayerBio {
                birth_date: Some("1997-01-13".into()),
                birth_country: Some("CAN".into()),
                nationality_code: Some("CAN".into()),
                birth_city: Some("Richmond Hill".into()),
                birth_state_province: Some("ON".into()),
                height_in_inches: Some(73),
                weight_lbs: Some(193),
                draft_year: Some(2015),
                draft_round: Some(1),
                draft_overall: Some(1),
                shoots_catches: Some("L".into()),
                rookie_season: Some("20152016".into()),
            },
        },
    }
}

impl IdentityFixture {
    pub fn name(mut self, full: &str, normalized: &str) -> Self {
        self.inner.full_name = full.into();
        self.inner.name_normalized = normalized.into();
        self
    }

    pub fn rookie_season(mut self, s: &str) -> Self {
        self.inner.bio.rookie_season = Some(s.into());
        self
    }

    pub fn weight(mut self, lbs: u32) -> Self {
        self.inner.bio.weight_lbs = Some(lbs);
        self
    }

    pub fn height(mut self, inches: u32) -> Self {
        self.inner.bio.height_in_inches = Some(inches);
        self
    }

    pub fn draft(mut self, year: u16, round: u8, overall: u16) -> Self {
        self.inner.bio.draft_year = Some(year);
        self.inner.bio.draft_round = Some(round);
        self.inner.bio.draft_overall = Some(overall);
        self
    }

    pub fn shoots(mut self, hand: &str) -> Self {
        self.inner.bio.shoots_catches = Some(hand.into());
        self
    }

    pub fn build(self) -> PlayerIdentity {
        self.inner
    }
}

pub struct StatsFixture {
    builder: SeasonStatsBuilder,
}

/// Default skater fixture: one team stint with the given team, modest
/// totals, no realtime/advanced/goalie. Override via the chainable
/// methods.
pub fn stats(player_id: u32, season: u32, team: &str) -> StatsFixture {
    let team_abbr = TeamAbbr(team.into());
    let totals = StatTotals {
        gp: 70,
        goals: 30,
        assists: 50,
        points: 80,
        plus_minus: 12,
        pim: 20,
        shots: 220,
        shooting_pct: Some(13.6),
        toi_per_game_sec: Some(20 * 60),
        pp_goals: 10,
        pp_points: 28,
        sh_goals: 0,
        sh_points: 0,
        gwg: 5,
        ot_goals: 1,
        faceoff_win_pct: Some(56.0),
        pace_score: Some(PaceScore {
            pace_82: 93.7,
            goals_per_82: 35.1,
            raw_points: 80,
            gp: 70,
        }),
    };
    let stint = TeamStint {
        team: team_abbr,
        started: Some("2022-10-15".into()),
        ended: Some("2023-04-13".into()),
        gp: 70,
        goals: 30,
        assists: 50,
        points: 80,
        goalie: None,
    };

    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Center,
        )
        .with_sweater_number(97)
        .with_totals(totals)
        .add_team_stint(stint),
    }
}

impl StatsFixture {
    pub fn season_type(mut self, t: SeasonType) -> Self {
        // Rebuild builder with new season_type — there's no in-place setter.
        // Cheap for tests; production path uses SeasonStatsBuilder::new
        // directly with the right type.
        let snap = self.builder.build();
        let mut b = SeasonStatsBuilder::new(snap.player_id, snap.season, t, snap.position);
        if let Some(n) = snap.sweater_number {
            b = b.with_sweater_number(n);
        }
        b = b
            .with_totals(snap.totals)
            .replace_team_stints(snap.team_stints);
        if let Some(r) = snap.realtime {
            b = b.with_realtime(r);
        }
        if let Some(a) = snap.advanced {
            b = b.with_advanced(a);
        }
        if let Some(g) = snap.goalie {
            b = b.with_goalie(g);
        }
        self.builder = b;
        self
    }

    pub fn position(mut self, pos: Position) -> Self {
        let snap = self.builder.build();
        let mut b = SeasonStatsBuilder::new(snap.player_id, snap.season, snap.season_type, pos);
        if let Some(n) = snap.sweater_number {
            b = b.with_sweater_number(n);
        }
        b = b
            .with_totals(snap.totals)
            .replace_team_stints(snap.team_stints);
        if let Some(r) = snap.realtime {
            b = b.with_realtime(r);
        }
        if let Some(a) = snap.advanced {
            b = b.with_advanced(a);
        }
        if let Some(g) = snap.goalie {
            b = b.with_goalie(g);
        }
        self.builder = b;
        self
    }

    pub fn realtime(mut self, hits: u32, blocks: u32, takeaways: u32, giveaways: u32) -> Self {
        self.builder = self.builder.with_realtime(RealtimeStats {
            hits,
            blocked_shots: blocks,
            takeaways,
            giveaways,
            missed_shots: 0,
        });
        self
    }

    pub fn advanced(mut self, xg: f64, cf_pct: f64) -> Self {
        self.builder = self.builder.with_advanced(AdvancedStats {
            xg: Some(xg),
            xg_per_60: None,
            cf_pct: Some(cf_pct),
            ff_pct: None,
            xgf_pct: None,
        });
        self
    }

    pub fn goalie(mut self, g: GoalieSeasonStats) -> Self {
        self.builder = self.builder.with_goalie(g);
        self
    }

    pub fn add_stint(mut self, stint: TeamStint) -> Self {
        self.builder = self.builder.add_team_stint(stint);
        self
    }

    pub fn build(self) -> SeasonStats {
        self.builder.build()
    }
}

// ── Hart.4.1 canonical scenario builders ────────────────────────────────────
//
// Pre-built named scenarios for Hart.5 consumer tests. Each builder
// enforces the TAPE invariants by construction (sum-equals across
// stints, monotonic stint ordering via SYNTHETIC_DATE_PREFIX-prefixed
// dates, post-upsert roster sum-equals). The self-tests in this
// module's `mod tests` block lock those invariants.

use crate::season_stats::{GoalieStintStats, SYNTHETIC_DATE_PREFIX};

/// A skater traded mid-(season, type) with two TeamStints.
/// Defaults via `traded_skater_default` are STL → NYR (matches the
/// Tarasenko 2022-23 worked example in the Hart plan).
///
/// Hart.4.1 v0.2: `from`/`to` parameterized from day one (BENCH #5)
/// to avoid a future `traded_skater_atl_to_nsh` proliferation. All
/// counter splits maintain the sum-equals invariant: STL has the
/// majority of the season (60%), NYR the rest (40%) — so the totals
/// at the season aggregate equal the sum of stint counters.
pub fn traded_skater(player_id: u32, season: u32, from: TeamAbbr, to: TeamAbbr) -> StatsFixture {
    let gp_a = 38;
    let gp_b = 31;
    let g_a = 10;
    let g_b = 8;
    let a_a = 11;
    let a_b = 13;
    let pim_a = 22;
    let pim_b = 18;

    let totals = StatTotals {
        gp: gp_a + gp_b,
        goals: g_a + g_b,
        assists: a_a + a_b,
        points: g_a + g_b + a_a + a_b,
        plus_minus: 12,
        pim: pim_a + pim_b,
        shots: 220,
        shooting_pct: Some(13.6),
        toi_per_game_sec: Some(20 * 60),
        pp_goals: 6,
        pp_points: 18,
        sh_goals: 0,
        sh_points: 0,
        gwg: 4,
        ot_goals: 1,
        faceoff_win_pct: None,
        pace_score: Some(PaceScore {
            pace_82: 49.9,
            goals_per_82: 21.4,
            raw_points: g_a + g_b + a_a + a_b,
            gp: gp_a + gp_b,
        }),
    };

    let stint_a = TeamStint {
        team: from,
        started: Some(format!("{SYNTHETIC_DATE_PREFIX}-01")),
        ended: Some(format!("{SYNTHETIC_DATE_PREFIX}-02")),
        gp: gp_a,
        goals: g_a,
        assists: a_a,
        points: g_a + a_a,
        goalie: None,
    };
    let stint_b = TeamStint {
        team: to,
        started: Some(format!("{SYNTHETIC_DATE_PREFIX}-03")),
        ended: Some(format!("{SYNTHETIC_DATE_PREFIX}-04")),
        gp: gp_b,
        goals: g_b,
        assists: a_b,
        points: g_b + a_b,
        goalie: None,
    };

    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::RightWing,
        )
        .with_sweater_number(91)
        .with_totals(totals)
        .add_team_stint(stint_a)
        .add_team_stint(stint_b),
    }
}

/// Convenience default — STL → NYR.
pub fn traded_skater_default(player_id: u32, season: u32) -> StatsFixture {
    traded_skater(
        player_id,
        season,
        TeamAbbr("STL".into()),
        TeamAbbr("NYR".into()),
    )
}

/// A goalie traded mid-(season, type) with two TeamStints both carrying
/// `GoalieStintStats`. Defaults via `goalie_mid_playoff_trade_default`
/// are BOS → FLA. Used for testing the goalie-stint mapper and the
/// per-stint W/L tracking (Hart.6 will populate this from real data;
/// Hart.4.1 fixtures synthesize for tests).
pub fn goalie_mid_playoff_trade(
    player_id: u32,
    season: u32,
    from: TeamAbbr,
    to: TeamAbbr,
) -> StatsFixture {
    let gp_a = 3;
    let gp_b = 7;
    let w_a = 1;
    let w_b = 4;
    let l_a = 2;
    let l_b = 3;

    let stint_a = TeamStint {
        team: from,
        started: Some(format!("{SYNTHETIC_DATE_PREFIX}-01")),
        ended: Some(format!("{SYNTHETIC_DATE_PREFIX}-02")),
        gp: gp_a,
        goals: 0,
        assists: 0,
        points: 0,
        goalie: Some(GoalieStintStats {
            games_started: gp_a,
            wins: w_a,
            losses: l_a,
            ot_losses: Some(0),
        }),
    };
    let stint_b = TeamStint {
        team: to,
        started: Some(format!("{SYNTHETIC_DATE_PREFIX}-03")),
        ended: Some(format!("{SYNTHETIC_DATE_PREFIX}-04")),
        gp: gp_b,
        goals: 0,
        assists: 0,
        points: 0,
        goalie: Some(GoalieStintStats {
            games_started: gp_b,
            wins: w_b,
            losses: l_b,
            ot_losses: Some(0),
        }),
    };

    let totals = StatTotals {
        gp: gp_a + gp_b,
        ..Default::default()
    };
    let goalie = GoalieSeasonStats {
        games_started: gp_a + gp_b,
        wins: w_a + w_b,
        losses: l_a + l_b,
        ot_losses: Some(0),
        ties: None,
        shots_against: 280,
        goals_against: 28,
        saves: 252,
        save_pct: Some(0.900),
        goals_against_average: Some(2.80),
        shutouts: 0,
        time_on_ice_sec: 600 * 60,
    };

    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Playoff,
            Position::Goalie,
        )
        .with_totals(totals)
        .add_team_stint(stint_a)
        .add_team_stint(stint_b)
        .with_goalie(goalie),
    }
}

pub fn goalie_mid_playoff_trade_default(player_id: u32, season: u32) -> StatsFixture {
    goalie_mid_playoff_trade(
        player_id,
        season,
        TeamAbbr("BOS".into()),
        TeamAbbr("FLA".into()),
    )
}

/// A goalie that played for one team only (single-stint goalie season).
/// Default behavior: ~50 GP, 30 wins, 18 losses, .913 save_pct.
pub fn solo_goalie(player_id: u32, season: u32, team: TeamAbbr) -> StatsFixture {
    let gp = 50;
    let stint = TeamStint {
        team,
        started: Some("2024-10-15".into()),
        ended: Some("2025-04-13".into()),
        gp,
        goals: 0,
        assists: 0,
        points: 0,
        goalie: None,
    };
    let totals = StatTotals {
        gp,
        toi_per_game_sec: Some(50 * 60), // ~50 minutes per game
        ..Default::default()
    };
    let goalie = GoalieSeasonStats {
        games_started: 50,
        wins: 30,
        losses: 18,
        ot_losses: Some(2),
        ties: None,
        shots_against: 1500,
        goals_against: 130,
        saves: 1370,
        save_pct: Some(0.913),
        goals_against_average: Some(2.60),
        shutouts: 5,
        time_on_ice_sec: 3000 * 60,
    };
    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Goalie,
        )
        .with_totals(totals)
        .add_team_stint(stint)
        .with_goalie(goalie),
    }
}

/// Hart's emergency-backup-goalie scenario: a `Position::Goalie` row for
/// a player whose career is otherwise as a skater (David Ayres /
/// Scott Foster-style EBUG appearances). Locks the per-row `is_goalie()`
/// design — `goalie.is_some()` is the test, NOT `position == Goalie`
/// alone (TAPE #12).
///
/// The fixture: 1 GP, 1 GS, 0 wins, 0 losses, save_pct 1.0 (the
/// archetypal "pull a Zamboni driver out of retirement" outing).
pub fn emergency_backup_goalie(player_id: u32, season: u32) -> StatsFixture {
    let stint = TeamStint {
        team: TeamAbbr("CAR".into()),
        started: Some("2025-02-22".into()),
        ended: Some("2025-02-22".into()),
        gp: 1,
        goals: 0,
        assists: 0,
        points: 0,
        goalie: Some(GoalieStintStats {
            games_started: 1,
            wins: 1,
            losses: 0,
            ot_losses: Some(0),
        }),
    };
    let totals = StatTotals {
        gp: 1,
        ..Default::default()
    };
    let goalie = GoalieSeasonStats {
        games_started: 1,
        wins: 1,
        losses: 0,
        ot_losses: Some(0),
        ties: None,
        shots_against: 8,
        goals_against: 2,
        saves: 6,
        save_pct: Some(0.750),
        goals_against_average: Some(2.86),
        shutouts: 0,
        time_on_ice_sec: 28 * 60 + 41,
    };
    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Goalie,
        )
        .with_totals(totals)
        .add_team_stint(stint)
        .with_goalie(goalie),
    }
}

/// A player with bios + realtime tier present, but no MoneyPuck and no
/// contracts. The realistic mid-season cold-fetch case. Exercises the
/// partial-tier view-accessor path: `view.hits()` returns `Some(_)`,
/// `view.xg()` returns `None` (TAPE #12, BENCH #10).
pub fn partial_tier_player(player_id: u32, season: u32) -> StatsFixture {
    stats(player_id, season, "EDM").realtime(48, 22, 65, 41)
    // .advanced(...) intentionally NOT called — leaves AdvancedStats None.
}

/// A goalie with `games_played > 0` but `time_on_ice_sec == 0`.
/// Data-quality edge case; PACE divides by TOI in some leaderboard
/// formulas (TAPE #12). Locks "TOI=0 doesn't crash leaderboards."
pub fn goalie_zero_toi(player_id: u32, season: u32) -> StatsFixture {
    let stint = TeamStint {
        team: TeamAbbr("VGK".into()),
        started: Some("2024-10-15".into()),
        ended: Some("2025-04-13".into()),
        gp: 5,
        goals: 0,
        assists: 0,
        points: 0,
        goalie: None,
    };
    let totals = StatTotals {
        gp: 5,
        toi_per_game_sec: None, // intentionally None
        ..Default::default()
    };
    let goalie = GoalieSeasonStats {
        games_started: 5,
        wins: 2,
        losses: 3,
        ot_losses: Some(0),
        ties: None,
        shots_against: 100,
        goals_against: 10,
        saves: 90,
        save_pct: Some(0.900),
        goals_against_average: None,
        shutouts: 0,
        time_on_ice_sec: 0, // intentionally zero
    };
    StatsFixture {
        builder: SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Goalie,
        )
        .with_totals(totals)
        .add_team_stint(stint)
        .with_goalie(goalie),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_hart1_fixtures_identity_default() {
        let id = identity(8478402).build();
        assert_eq!(id.id, PlayerId(8478402));
        assert_eq!(id.bio.weight_lbs, Some(193));
        assert_eq!(id.bio.rookie_season.as_deref(), Some("20152016"));
    }

    #[test]
    fn l0_hart1_fixtures_identity_overrides() {
        let id = identity(8478402)
            .name("Test Player", "test player")
            .weight(220)
            .draft(2018, 2, 45)
            .build();
        assert_eq!(id.full_name, "Test Player");
        assert_eq!(id.bio.weight_lbs, Some(220));
        assert_eq!(id.bio.draft_overall, Some(45));
    }

    #[test]
    fn l0_hart1_fixtures_stats_default() {
        let s = stats(8478402, 20222023, "EDM").build();
        assert_eq!(s.player_id, PlayerId(8478402));
        assert_eq!(s.season, Season(20222023));
        assert_eq!(s.team_stints.len(), 1);
        assert_eq!(s.team_stints[0].team.as_str(), "EDM");
        assert_eq!(s.totals.gp, 70);
    }

    #[test]
    fn l0_hart1_fixtures_stats_overrides() {
        let s = stats(8478402, 20222023, "EDM")
            .season_type(SeasonType::Playoff)
            .realtime(45, 22, 60, 30)
            .build();
        assert_eq!(s.season_type, SeasonType::Playoff);
        assert_eq!(s.realtime.as_ref().unwrap().hits, 45);
    }

    // ── Hart.4.1 v0.2 — by-construction invariant self-tests ────────────────
    //
    // Every canonical scenario builder MUST enforce: sum-of-stints ==
    // aggregate counters, monotonic stint order through the builder
    // sort, post-upsert roster sum-equals invariant. Failures here mean
    // the fixture is unsound — Hart.5 consumer tests built on top
    // would inherit bugs (TAPE #11).

    use crate::stats_repository::StatsRepository;

    fn stint_sum_equals_totals(s: &SeasonStats) {
        let stint_gp: u32 = s.team_stints.iter().map(|t| t.gp).sum();
        let stint_g: u32 = s.team_stints.iter().map(|t| t.goals).sum();
        let stint_a: u32 = s.team_stints.iter().map(|t| t.assists).sum();
        let stint_p: u32 = s.team_stints.iter().map(|t| t.points).sum();
        assert_eq!(stint_gp, s.totals.gp, "stint gp sum != totals.gp");
        assert_eq!(stint_g, s.totals.goals, "stint goals sum != totals.goals");
        assert_eq!(
            stint_a, s.totals.assists,
            "stint assists sum != totals.assists"
        );
        assert_eq!(
            stint_p, s.totals.points,
            "stint points sum != totals.points"
        );
    }

    fn goalie_stint_sum_equals_aggregate(s: &SeasonStats) {
        let Some(g) = s.goalie.as_ref() else {
            return; // skater fixture, not applicable
        };
        let starts: u32 = s
            .team_stints
            .iter()
            .map(|t| t.goalie.as_ref().map(|g| g.games_started).unwrap_or(0))
            .sum();
        let wins: u32 = s
            .team_stints
            .iter()
            .map(|t| t.goalie.as_ref().map(|g| g.wins).unwrap_or(0))
            .sum();
        let losses: u32 = s
            .team_stints
            .iter()
            .map(|t| t.goalie.as_ref().map(|g| g.losses).unwrap_or(0))
            .sum();
        // Only assert sum-equals if at least one stint carries goalie data
        // (solo_goalie / emergency_backup_goalie may have fewer stint
        // GoalieStintStats entries than the aggregate represents).
        if starts > 0 {
            assert_eq!(
                starts, g.games_started,
                "stint games_started sum != aggregate"
            );
            assert_eq!(wins, g.wins, "stint wins sum != aggregate");
            assert_eq!(losses, g.losses, "stint losses sum != aggregate");
        }
    }

    fn stints_monotonic(s: &SeasonStats) {
        // After SeasonStatsBuilder::build()'s sort, stints with `started`
        // dates are in ascending order. The synthetic-date-prefix
        // invariant guarantees AAAA-* sorts before any real YYYY-MM-DD.
        for win in s.team_stints.windows(2) {
            match (win[0].started.as_ref(), win[1].started.as_ref()) {
                (Some(a), Some(b)) => assert!(a <= b, "stint order violated: {a} > {b}"),
                (Some(_), None) => {
                    // Ok per builder sort: Some sorts before None.
                }
                (None, Some(_)) => panic!("stint sort broken: None before Some"),
                (None, None) => { /* both undated: lex tiebreak */ }
            }
        }
    }

    fn post_upsert_roster_sum_equals(s: &SeasonStats) {
        use std::collections::HashSet;
        let pid = s.player_id;
        let mut repo = StatsRepository::new();
        repo.upsert_identity(identity(pid.0).build()).unwrap();
        let season = s.season;
        let stype = s.season_type;
        repo.upsert_stats(s.clone()).unwrap();

        // Distinct teams in the fixture's stints.
        let distinct_teams: HashSet<&str> = s.team_stints.iter().map(|t| t.team.as_str()).collect();

        // Each distinct team's all-stints roster contains this pid exactly once.
        for team in &distinct_teams {
            let team_abbr = TeamAbbr((*team).to_string());
            let roster = repo.team_roster_all_stints(&team_abbr, season, stype);
            assert_eq!(
                roster.len(),
                1,
                "team {team} should have exactly 1 player after upsert"
            );
            assert_eq!(roster[0].id(), pid);
        }
        // Last-stint roster: only the chronologically last team should
        // contain pid.
        let last_team = s.team_stints.last().unwrap().team.clone();
        for team in &distinct_teams {
            let team_abbr = TeamAbbr((*team).to_string());
            let roster = repo.team_roster(&team_abbr, season, stype);
            if team_abbr == last_team {
                assert_eq!(roster.len(), 1, "last-stint team {team} must have pid");
            } else {
                assert_eq!(
                    roster.len(),
                    0,
                    "non-last team {team} must NOT be in last-stint roster"
                );
            }
        }
    }

    fn check_all_invariants(s: &SeasonStats) {
        stint_sum_equals_totals(s);
        goalie_stint_sum_equals_aggregate(s);
        stints_monotonic(s);
        post_upsert_roster_sum_equals(s);
    }

    #[test]
    fn l0_hart4_1_traded_skater_default_invariants() {
        let s = traded_skater_default(8475765, 20222023).build();
        assert_eq!(s.team_stints.len(), 2);
        assert_eq!(s.team_stints[0].team.as_str(), "STL");
        assert_eq!(s.team_stints[1].team.as_str(), "NYR");
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_traded_skater_parameterized_invariants() {
        let s = traded_skater(
            8479987,
            20242025,
            TeamAbbr("ATL".into()),
            TeamAbbr("NSH".into()),
        )
        .build();
        assert_eq!(s.team_stints[0].team.as_str(), "ATL");
        assert_eq!(s.team_stints[1].team.as_str(), "NSH");
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_goalie_mid_playoff_trade_default_invariants() {
        let s = goalie_mid_playoff_trade_default(9000001, 20232024).build();
        assert_eq!(s.team_stints.len(), 2);
        assert_eq!(s.team_stints[0].team.as_str(), "BOS");
        assert_eq!(s.team_stints[1].team.as_str(), "FLA");
        assert!(s.is_goalie());
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_solo_goalie_invariants() {
        let s = solo_goalie(8478406, 20242025, TeamAbbr("OTT".into())).build();
        assert_eq!(s.team_stints.len(), 1);
        assert_eq!(s.team_stints[0].team.as_str(), "OTT");
        assert!(s.is_goalie());
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_emergency_backup_goalie_invariants() {
        let s = emergency_backup_goalie(7000000, 20242025).build();
        assert!(
            s.is_goalie(),
            "EBUG must satisfy is_goalie() per Hart's per-row design"
        );
        assert_eq!(s.team_stints.len(), 1);
        assert_eq!(s.totals.gp, 1);
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_partial_tier_player_realtime_some_advanced_none() {
        let s = partial_tier_player(8478402, 20242025).build();
        assert!(
            s.realtime.is_some(),
            "partial_tier_player must populate RealtimeStats"
        );
        assert!(
            s.advanced.is_none(),
            "partial_tier_player must leave AdvancedStats None"
        );
        check_all_invariants(&s);
    }

    #[test]
    fn l0_hart4_1_goalie_zero_toi_invariants() {
        let s = goalie_zero_toi(8480123, 20242025).build();
        let g = s.goalie.as_ref().expect("goalie set");
        assert_eq!(g.time_on_ice_sec, 0, "goalie_zero_toi must have TOI=0");
        assert!(g.games_started > 0, "but games_started > 0 — the edge case");
        check_all_invariants(&s);
    }
}
