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
    AdvancedStats, GoalieAdvancedStats, GoalieBios, GoalieSavesByStrengthStats, GoalieSeasonStats,
    GoalsForAgainstStats, RealtimeStats, SeasonStats, SeasonStatsBuilder, SeasonType, StatTotals,
    TeamStint, TimeOnIceStats,
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
    r.upsert_identity(identity)
        .expect("identity upsert in fixture");
    r.upsert_stats(stats).expect("stats upsert in fixture");
    r
}

/// Goalie variant of `test_repo_with`. Identical body to the skater
/// variant; the call site documents intent (and future invariants can
/// hang off this signature without touching every test).
pub fn test_repo_with_goalie(identity: PlayerIdentity, stats: SeasonStats) -> StatsRepository {
    let mut r = StatsRepository::new();
    r.upsert_identity(identity)
        .expect("identity upsert in fixture");
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

// ─── Phase Lindsay L.2.3 — stat_catalog_variants (BENCH-R2 deliverable) ────
//
// Six named PlayerView fixtures enumerated to drive the cross-product
// `read()` test in `tests/stat_catalog_variants.rs`. Each variant
// targets a specific gate in `StatId::read`:
//
//   - skater_modern    — full-data path (every Lindsay substruct populated)
//   - skater_pre_2005  — era gate (realtime / possession Nones out)
//   - center           — FaceoffWinPct applies (TwoWay center-only stat)
//   - goalie           — Goalie category populated; skater stats Nones
//   - traded_multistint — DI-11 OnIceGoals trade-window guard fires
//   - low_gp           — MIN_GP guard fires for derived per-game / per-82
//
// Spec: design/specs/stat-catalog.md §"Test contract" — the
// 6-variant fixture catalog backs ~107 stats × 6 variants = ~642 read
// dispatch cells exercised in one cross-product L1 test (BENCH-R2 L2-B22).

pub mod stat_catalog_variants {
    use super::*;

    /// Modern skater (Center, 2024-25, GP=82). All Lindsay Tier-1
    /// substructs populated. Single-team season → DI-11 doesn't fire.
    /// Realtime + possession + xG all present so era gates pass.
    pub fn skater_modern() -> (PlayerIdentity, SeasonStats) {
        let id = identity(8478402).build();
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 82,
            goals: 50,
            assists: 80,
            points: 130,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 82,
            goals: 50,
            assists: 80,
            points: 130,
            plus_minus: 22,
            pim: 28,
            shots: 320,
            shooting_pct: Some(0.156),
            toi_per_game_sec: Some(22 * 60),
            pp_goals: 18,
            pp_points: 50,
            sh_goals: 0,
            sh_points: 0,
            gwg: 9,
            ot_goals: 3,
            faceoff_win_pct: Some(0.563),
            pace_score: Some(PaceScore {
                pace_82: 130.0,
                goals_per_82: 50.0,
                raw_points: 130,
                gp: 82,
            }),
        })
        .with_realtime(RealtimeStats {
            hits: 30,
            blocked_shots: 18,
            takeaways: 80,
            giveaways: 60,
            missed_shots: 25,
        })
        .with_advanced(AdvancedStats {
            xg: Some(35.5),
            xg_per_60: Some(1.42),
            cf_pct: Some(54.2),
            ff_pct: Some(53.8),
            xgf_pct: Some(56.0),
        })
        .with_time_on_ice(TimeOnIceStats {
            time_on_ice_sec: 22 * 60 * 82,
            time_on_ice_per_game_sec: 22 * 60,
            ev_time_on_ice_sec: 17 * 60 * 82,
            ev_time_on_ice_per_game_sec: 17 * 60,
            pp_time_on_ice_sec: 4 * 60 * 82,
            pp_time_on_ice_per_game_sec: 4 * 60,
            sh_time_on_ice_sec: 60 * 82,
            sh_time_on_ice_per_game_sec: 60,
            ot_time_on_ice_sec: Some(120),
            shifts: 1900,
            shifts_per_game: 23.2,
            time_on_ice_per_shift_sec: 41.0,
        })
        .with_goals_for_against(GoalsForAgainstStats {
            ev_goals_for: 110,
            ev_goals_against: 75,
            ev_goals_for_pct: Some(0.595),
            pp_goals_for: 40,
            pp_goals_against: 1,
            sh_goals_for: 1,
            sh_goals_against: 6,
            even_strength_goal_difference: 35,
            ev_time_on_ice_per_game_sec: 17 * 60,
            offensive_points: Some(85),
            defensive_points: Some(20),
        })
        .build();
        (id, stats)
    }

    /// Pre-2005 skater. season=2001-02, no realtime / time_on_ice /
    /// goals_for_against / advanced data (era gate fires for those
    /// stats). Total scoring + Pim still populated.
    pub fn skater_pre_2005() -> (PlayerIdentity, SeasonStats) {
        let id = identity(8467396).build(); // Brendan Shanahan-era id
        let stats = SeasonStatsBuilder::new(
            PlayerId(8467396),
            Season(20012002),
            SeasonType::Regular,
            Position::LeftWing,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("DET".into()),
            started: Some("2001-10-04".into()),
            ended: None,
            gp: 80,
            goals: 37,
            assists: 38,
            points: 75,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 80,
            goals: 37,
            assists: 38,
            points: 75,
            plus_minus: 23,
            pim: 118,
            shots: 282,
            shooting_pct: Some(0.131),
            toi_per_game_sec: Some(20 * 60),
            pp_goals: 13,
            pp_points: 23,
            sh_goals: 0,
            sh_points: 0,
            gwg: 6,
            ot_goals: 1,
            faceoff_win_pct: None,
            pace_score: Some(PaceScore {
                pace_82: 76.9,
                goals_per_82: 37.9,
                raw_points: 75,
                gp: 80,
            }),
        })
        // No `with_realtime`, `with_time_on_ice`, `with_goals_for_against`,
        // `with_advanced` — pre-2005 era doesn't have reliable data here.
        .build();
        (id, stats)
    }

    /// Center with explicit faceoff data — FaceoffWinPct applies, the
    /// Center-only gate. Matches `skater_modern` shape but emphasizes
    /// the position-specific path.
    pub fn center_with_faceoffs() -> (PlayerIdentity, SeasonStats) {
        let mut pair = skater_modern();
        // Override player_id + faceoff data — realistic 60% C.
        pair.0.id = PlayerId(8474564);
        pair.0.full_name = "Center With Faceoffs".into();
        pair.0.name_normalized = "center with faceoffs".into();
        // Tweak totals.faceoff_win_pct to a strong value.
        pair.1.player_id = PlayerId(8474564);
        pair.1.totals.faceoff_win_pct = Some(0.605);
        pair.1.position = Position::Center;
        (pair.0, pair.1)
    }

    /// Goalie — Goalie category populated, skater stats None. Modern
    /// 2024-25 starter shape: GP=55, .913 SV%, 2.50 GAA. All Lindsay
    /// goalie substructs (advanced, saves_by_strength, bios) populated.
    pub fn goalie() -> (PlayerIdentity, SeasonStats) {
        let id = identity(8476434).build(); // Sergei Bobrovsky-era id
        let stats = SeasonStatsBuilder::new(
            PlayerId(8476434),
            Season(20242025),
            SeasonType::Regular,
            Position::Goalie,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("FLA".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 55,
            goals: 0,
            assists: 1,
            points: 1,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 55,
            goals: 0,
            assists: 1,
            points: 1,
            plus_minus: 0,
            pim: 6,
            shots: 0,
            shooting_pct: None,
            toi_per_game_sec: Some(60 * 55),
            pp_goals: 0,
            pp_points: 0,
            sh_goals: 0,
            sh_points: 0,
            gwg: 0,
            ot_goals: 0,
            faceoff_win_pct: None,
            pace_score: None,
        })
        .with_goalie(GoalieSeasonStats {
            games_started: 53,
            wins: 33,
            losses: 18,
            ot_losses: Some(4),
            ties: None,
            shots_against: 1620,
            goals_against: 138,
            saves: 1482,
            save_pct: Some(0.915),
            goals_against_average: Some(2.50),
            shutouts: 5,
            time_on_ice_sec: 60 * 55 * 60,
        })
        .with_goalie_advanced(GoalieAdvancedStats {
            quality_starts: 32,
            quality_starts_pct: Some(0.604),
            regulation_wins: 28,
            regulation_losses: 14,
            complete_games: 48,
            incomplete_games: 5,
            complete_game_pct: Some(0.906),
            shots_against_per_60: Some(29.5),
        })
        .with_goalie_saves_by_strength(GoalieSavesByStrengthStats {
            ev_saves: 1100,
            ev_shots_against: 1180,
            ev_goals_against: 80,
            ev_save_pct: Some(0.932),
            pp_saves: 0,
            pp_shots_against: 0,
            pp_goals_against: 0,
            pp_save_pct: None,
            sh_saves: 380,
            sh_shots_against: 440,
            sh_goals_against: 60,
            sh_save_pct: Some(0.864),
        })
        .with_goalie_bios(GoalieBios {
            birth_city: Some("Helsinki".into()),
            birth_country_code: Some("FIN".into()),
            birth_date: Some("1989-09-04".into()),
            current_team_abbrev: Some("FLA".into()),
            draft_overall: Some("11".into()),
            draft_round: Some("1".into()),
            draft_year: Some("2007".into()),
            first_season_for_game_type: Some(20132014),
            height_in_centimeters: Some(187),
            height_in_inches: Some(74),
            nationality_code: Some("FIN".into()),
            shoots_catches: Some("L".into()),
            weight_in_pounds: Some(196),
        })
        .build();
        (id, stats)
    }

    /// Mid-season-traded skater. TWO team stints → DI-11 trade-window
    /// guard fires for OnIceGoals reads. EvenStrengthTimeOnIcePerGame
    /// (TimeOnIce category) is exempt — TOI sums correctly.
    pub fn traded_multistint() -> (PlayerIdentity, SeasonStats) {
        let id = identity(8475158).build(); // Bo Horvat-style mid-season
        let stats = SeasonStatsBuilder::new(
            PlayerId(8475158),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("VAN".into()),
            started: Some("2024-10-09".into()),
            ended: Some("2025-01-30".into()),
            gp: 49,
            goals: 31,
            assists: 23,
            points: 54,
            goalie: None,
        })
        .add_team_stint(TeamStint {
            team: TeamAbbr("NYI".into()),
            started: Some("2025-01-31".into()),
            ended: None,
            gp: 30,
            goals: 7,
            assists: 9,
            points: 16,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 79,
            goals: 38,
            assists: 32,
            points: 70,
            plus_minus: -3,
            pim: 28,
            shots: 215,
            shooting_pct: Some(0.177),
            toi_per_game_sec: Some(19 * 60),
            pp_goals: 15,
            pp_points: 28,
            sh_goals: 0,
            sh_points: 0,
            gwg: 7,
            ot_goals: 2,
            faceoff_win_pct: Some(0.541),
            pace_score: Some(PaceScore {
                pace_82: 72.7,
                goals_per_82: 39.4,
                raw_points: 70,
                gp: 79,
            }),
        })
        .with_goals_for_against(GoalsForAgainstStats {
            ev_goals_for: 88,
            ev_goals_against: 92,
            ev_goals_for_pct: Some(0.489),
            pp_goals_for: 30,
            pp_goals_against: 1,
            sh_goals_for: 0,
            sh_goals_against: 5,
            even_strength_goal_difference: -4,
            ev_time_on_ice_per_game_sec: 14 * 60 + 30,
            offensive_points: Some(48),
            defensive_points: Some(22),
        })
        .build();
        (id, stats)
    }

    /// Low-GP skater — totals.gp = 5 < MIN_GP (10). Derived per-game /
    /// per-82 stats return None via the MIN_GP guard. Per-60 rates
    /// also None via the 300s TOI floor (5 games × 18min = 5400s
    /// — above floor so that's fine; tweak by reducing TOI).
    pub fn low_gp() -> (PlayerIdentity, SeasonStats) {
        let id = identity(8482001).build(); // Synthetic call-up
        let stats = SeasonStatsBuilder::new(
            PlayerId(8482001),
            Season(20242025),
            SeasonType::Regular,
            Position::RightWing,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 5,
            goals: 1,
            assists: 1,
            points: 2,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 5,
            goals: 1,
            assists: 1,
            points: 2,
            plus_minus: 0,
            pim: 0,
            shots: 8,
            shooting_pct: Some(0.125),
            toi_per_game_sec: Some(8 * 60),
            pp_goals: 0,
            pp_points: 0,
            sh_goals: 0,
            sh_points: 0,
            gwg: 0,
            ot_goals: 0,
            faceoff_win_pct: None,
            pace_score: None, // pace_score is None below MIN_GP
        })
        .build();
        (id, stats)
    }

    /// `(name, builder fn)` pair for a stat-catalog variant. Type alias
    /// keeps the `all()` return type readable without tripping
    /// clippy::type_complexity.
    pub type CatalogVariant = (&'static str, fn() -> (PlayerIdentity, SeasonStats));

    /// Catalog of all 6 variants by name + builder. Drives the
    /// cross-product test in `tests/stat_catalog_variants.rs`.
    pub fn all() -> &'static [CatalogVariant] {
        &[
            ("skater_modern", skater_modern),
            ("skater_pre_2005", skater_pre_2005),
            ("center", center_with_faceoffs),
            ("goalie", goalie),
            ("traded_multistint", traded_multistint),
            ("low_gp", low_gp),
        ]
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
