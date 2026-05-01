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
        gwg: 5,
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
            .with_team_stints(snap.team_stints);
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
            .with_team_stints(snap.team_stints);
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
}
