//! Phase Foster.2 — Favorites dashboard schemas.
//!
//! Pure data + a projection helper that turns a raw boxscore +
//! roster + transactions trio into a `FavoritesView`. The actual
//! orchestration (group lookup → DataStore reads → events) lives at
//! the icelines-cli layer where `GroupDb` and `DataStore` are both
//! reachable; this module owns the schemas + the per-night
//! classification logic so they're testable without touching disk.
//!
//! See `design/specs/foster-favorites-dashboard.md` for the full
//! spec; this module pins the type bodies the spec sketches.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::entity::EntityRef;
use crate::model::TeamAbbr;
use crate::timeframe::Timeframe;

/// Top-level view that the CLI / TUI / Web all render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritesView {
    pub date: NaiveDate,
    pub range: Timeframe,
    pub players: Vec<PlayerNightRow>,
    pub teams: Vec<TeamNightRow>,
    pub events: Vec<EventRow>,
    pub aggregate: AggregateView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerNightRow {
    Skater(SkaterNightLine),
    Goalie(GoalieNightLine),
    #[serde(rename = "dnp")]
    DidNotPlay {
        player: EntityRef,
        reason: DnpReason,
    },
}

/// Why a favorited skater isn't in tonight's boxscore. The renderer
/// uses this to disambiguate "not in roster tonight" from "data not
/// yet fetched" so the user can tell whether to wait or move on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnpReason {
    /// In the team roster but missing from the boxscore (healthy
    /// scratch / coach's decision).
    Scratched,
    /// On the IR list. Foster.2 can't reliably detect this from the
    /// boxscore alone — surfaced opportunistically when a future
    /// transaction stream feeds it.
    InjuredReserve,
    /// The player's team simply didn't play that night.
    TeamBye,
    /// Recalled / sent down — `team_diff_at_date(player, date)` shows
    /// a transaction within the last 7 days. Heuristic; refine when
    /// transactions feed lands in Foster.3.
    Recalled,
    /// The boxscore for the player's team's game on this date hasn't
    /// been fetched (offline + cache miss) or hasn't been finalized
    /// (game in progress, line not posted yet).
    DataPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HomeAway {
    Home,
    Away,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    Win,
    Loss,
    OtLoss,
    InProgress,
}

/// Mirrors NHL API `gameState` field. Foster treats `Off` and
/// `Final` as interchangeable for stat-gating — both mean the
/// boxscore is locked and hits/blocks/etc. are real values rather
/// than the API's defaulted zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GameState {
    Fut,
    Pre,
    Live,
    Final,
    Off,
}

impl GameState {
    /// True iff the boxscore stats can be trusted as final values
    /// (not the NHL API's defaulted zeros for in-progress games).
    pub fn is_finalized(self) -> bool {
        matches!(self, Self::Final | Self::Off)
    }
}

/// Goalie decision per NHL boxscore. `None` means relief appearance
/// without a decision attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    #[serde(rename = "W")]
    Win,
    #[serde(rename = "L")]
    Loss,
    #[serde(rename = "OTL")]
    OtLoss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkaterNightLine {
    pub player: EntityRef,
    /// Team in THIS game (mid-day-trade-aware — comes from boxscore,
    /// not the favorited-team registry).
    pub team: TeamAbbr,
    pub opponent: TeamAbbr,
    pub home_or_away: HomeAway,
    pub team_score: u32,
    pub opponent_score: u32,
    pub result: GameResult,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub shots: Option<u32>,
    /// `None` when game state ∈ {FUT, PRE, LIVE} — NHL API defaults
    /// these to 0 mid-game. Real value once Final/Off.
    pub hits: Option<u32>,
    pub blocks: Option<u32>,
    pub pim: Option<u32>,
    pub takeaways: Option<u32>,
    pub giveaways: Option<u32>,
    pub toi_seconds: Option<u32>,
    pub power_play_goals: u32,
    pub power_play_assists: u32,
    pub shorthanded_goals: u32,
    pub game_state: GameState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieNightLine {
    pub player: EntityRef,
    pub team: TeamAbbr,
    pub opponent: TeamAbbr,
    pub home_or_away: HomeAway,
    pub team_score: u32,
    pub opponent_score: u32,
    pub games_started: bool,
    pub decision: Option<Decision>,
    pub saves: u32,
    pub shots_against: u32,
    pub goals_against: u32,
    pub save_pct: f32,
    pub gaa: f32,
    pub toi_seconds: Option<u32>,
    pub shutout: bool,
    pub game_state: GameState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamNightRow {
    pub team: EntityRef,
    pub team_abbr: TeamAbbr,
    /// "7-3" style score string; empty when team is on a bye.
    pub score: String,
    pub result: Option<GameResult>,
    pub opponent: Option<TeamAbbr>,
    pub top_skater: Option<EntityRef>,
    pub top_goalie: Option<EntityRef>,
    pub on_bye: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub event_kind: String,
    pub date: NaiveDate,
    pub entity: EntityRef,
    pub payload_version: u32,
    pub summary: String,
}

/// Last-N-days rollup. `range_start..=range_end` covers the active
/// timeframe (`Day` collapses to a single day; `Week`/`Month` widen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateView {
    pub range_start: NaiveDate,
    pub range_end: NaiveDate,
    pub player_rollups: Vec<PlayerRollup>,
    pub team_rollups: Vec<TeamRollup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerRollup {
    Skater(SkaterRollup),
    Goalie(GoalieRollup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkaterRollup {
    pub player: EntityRef,
    /// Counts only nights where the player has a `Skater` row AND
    /// `game_state ∈ {OFF, FINAL}`. DNPs and unfinalized games don't
    /// inflate the denominator (SCOUT M8).
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub shots: u32,
    pub shots_per_game: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieRollup {
    pub player: EntityRef,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub ot_losses: u32,
    pub saves: u32,
    pub shots_against: u32,
    pub save_pct: f32,
    pub gaa: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRollup {
    pub team: EntityRef,
    pub team_abbr: TeamAbbr,
    pub wins: u32,
    pub losses: u32,
    pub ot_losses: u32,
    pub goal_differential: i32,
}

// ── Projection helpers ──────────────────────────────────────────────────────

impl SkaterNightLine {
    /// Compute `result` from team / opponent scores + game state.
    /// Tied scores at non-final state map to `InProgress`; tied at
    /// Final/Off is impossible in the NHL (always a shootout winner)
    /// so we default to `OtLoss` for the trailing perspective if
    /// somehow encountered.
    pub fn classify_result(
        team_score: u32,
        opponent_score: u32,
        game_state: GameState,
    ) -> GameResult {
        if !game_state.is_finalized() {
            return GameResult::InProgress;
        }
        if team_score > opponent_score {
            GameResult::Win
        } else if team_score < opponent_score {
            // OT/SO loss is encoded in NHL boxscores via
            // `gameOutcome.lastPeriodType`; we don't see it here.
            // Caller (Foster.3 boxscore parser) sets `OtLoss` when
            // appropriate. Default to plain Loss; rollup math then
            // double-checks against the boxscore's outcome field.
            GameResult::Loss
        } else {
            GameResult::OtLoss
        }
    }

    /// Hits/blocks/pim/takeaways/giveaways must be `None` mid-game
    /// because the NHL API zero-defaults those fields. Caller should
    /// pass the raw integer + game_state and let this helper decide
    /// what to record.
    pub fn gate_finalized(value: u32, state: GameState) -> Option<u32> {
        if state.is_finalized() {
            Some(value)
        } else {
            None
        }
    }
}

impl GoalieNightLine {
    /// Save % = 1 - GA/SA. Zero-shots = 1.000 (nothing to save means
    /// the shutout is perfect). Caller passes raw integers from the
    /// boxscore.
    pub fn compute_save_pct(saves: u32, shots_against: u32) -> f32 {
        if shots_against == 0 {
            return 1.0;
        }
        saves as f32 / shots_against as f32
    }

    /// GAA = goals_against / (toi_minutes / 60). Zero-TOI returns 0.
    pub fn compute_gaa(goals_against: u32, toi_seconds: u32) -> f32 {
        if toi_seconds == 0 {
            return 0.0;
        }
        let toi_hours = toi_seconds as f32 / 3600.0;
        goals_against as f32 / toi_hours
    }

    /// Multi-goalie picker (SCOUT H5):
    ///
    /// - prefer the goalie with `decision != None` (W/L/OTL)
    /// - if none has a decision, pick the longest TOI
    /// - returns the index into the input slice
    ///
    /// Returns `None` when the slice is empty.
    pub fn primary_goalie<F>(
        goalies: &[F],
        get: impl Fn(&F) -> (Option<Decision>, u32),
    ) -> Option<usize> {
        if goalies.is_empty() {
            return None;
        }
        // First pass: any with a decision?
        let with_decision: Vec<usize> = (0..goalies.len())
            .filter(|i| get(&goalies[*i]).0.is_some())
            .collect();
        if !with_decision.is_empty() {
            // If multiple have decisions, take the longest TOI among them.
            return with_decision
                .into_iter()
                .max_by_key(|i| get(&goalies[*i]).1);
        }
        // No decisions — fall back to longest TOI.
        (0..goalies.len()).max_by_key(|i| get(&goalies[*i]).1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::PlayerId;

    fn skater(id: u32) -> EntityRef {
        EntityRef::Player(PlayerId(id))
    }

    fn team(abbr: &str) -> TeamAbbr {
        TeamAbbr(abbr.into())
    }

    // ── Result classification ─────────────────────────────────────────────

    #[test]
    fn l0_foster2_classify_result_in_progress() {
        // Mid-game: any score combination → InProgress
        assert_eq!(
            SkaterNightLine::classify_result(3, 2, GameState::Live),
            GameResult::InProgress
        );
        assert_eq!(
            SkaterNightLine::classify_result(0, 0, GameState::Pre),
            GameResult::InProgress
        );
    }

    #[test]
    fn l0_foster2_classify_result_finalized_win() {
        assert_eq!(
            SkaterNightLine::classify_result(7, 3, GameState::Final),
            GameResult::Win
        );
        assert_eq!(
            SkaterNightLine::classify_result(7, 3, GameState::Off),
            GameResult::Win
        );
    }

    #[test]
    fn l0_foster2_classify_result_finalized_loss() {
        assert_eq!(
            SkaterNightLine::classify_result(2, 4, GameState::Final),
            GameResult::Loss
        );
    }

    // ── Hits/blocks gating (SCOUT B2) ─────────────────────────────────────

    #[test]
    fn l0_foster2_gate_finalized_returns_value_when_final() {
        assert_eq!(
            SkaterNightLine::gate_finalized(5, GameState::Final),
            Some(5)
        );
        assert_eq!(SkaterNightLine::gate_finalized(0, GameState::Off), Some(0));
    }

    #[test]
    fn l0_foster2_gate_finalized_returns_none_mid_game() {
        // NHL API defaults these fields to 0 mid-game; we must NOT
        // record that as a real "0 hits tonight" value.
        assert_eq!(SkaterNightLine::gate_finalized(0, GameState::Live), None);
        assert_eq!(SkaterNightLine::gate_finalized(0, GameState::Fut), None);
        assert_eq!(SkaterNightLine::gate_finalized(0, GameState::Pre), None);
    }

    #[test]
    fn l0_foster2_game_state_is_finalized_truth_table() {
        assert!(GameState::Final.is_finalized());
        assert!(GameState::Off.is_finalized());
        assert!(!GameState::Live.is_finalized());
        assert!(!GameState::Pre.is_finalized());
        assert!(!GameState::Fut.is_finalized());
    }

    // ── Save % + GAA arithmetic ────────────────────────────────────────────

    #[test]
    fn l0_foster2_save_pct_zero_shots_is_perfect() {
        assert_eq!(GoalieNightLine::compute_save_pct(0, 0), 1.0);
    }

    #[test]
    fn l0_foster2_save_pct_normal_case() {
        // 34 saves on 35 shots = .9714…
        let pct = GoalieNightLine::compute_save_pct(34, 35);
        assert!((pct - 0.9714286).abs() < 0.0001, "got {pct}");
    }

    #[test]
    fn l0_foster2_gaa_zero_toi_is_zero() {
        assert_eq!(GoalieNightLine::compute_gaa(3, 0), 0.0);
    }

    #[test]
    fn l0_foster2_gaa_full_60_min_is_ga() {
        // 1 goal against, 60 minutes (3600 seconds) → GAA = 1.0
        assert_eq!(GoalieNightLine::compute_gaa(1, 3600), 1.0);
        // 2 goals against, 60 minutes → GAA = 2.0
        assert_eq!(GoalieNightLine::compute_gaa(2, 3600), 2.0);
    }

    // ── Goalie pull / multi-goalie (SCOUT H5) ─────────────────────────────

    #[test]
    fn l0_foster2_primary_goalie_picks_decision_holder() {
        // (decision, toi_seconds)
        let goalies = vec![
            (None, 600),                 // relief, 10 min
            (Some(Decision::Win), 2400), // starter, 40 min
        ];
        let idx = GoalieNightLine::primary_goalie(&goalies, |g| *g).unwrap();
        assert_eq!(idx, 1, "starter with decision picked");
    }

    #[test]
    fn l0_foster2_primary_goalie_picks_longest_toi_when_no_decision() {
        // Both goalies pulled mid-game on opposite sides; neither has
        // a decision. Pick the one with longer TOI (the "primary"
        // viewer-perspective goalie).
        let goalies = vec![
            (None, 1200), // 20 min
            (None, 2400), // 40 min
            (None, 600),  // 10 min
        ];
        let idx = GoalieNightLine::primary_goalie(&goalies, |g| *g).unwrap();
        assert_eq!(idx, 1, "longest TOI picked when no decisions");
    }

    #[test]
    fn l0_foster2_primary_goalie_picks_longer_toi_among_decision_holders() {
        // Both goalies somehow have decisions (e.g. mid-game pull
        // recorded as L for the pulled goalie + W for the relief).
        // Pick the one with longer TOI.
        let goalies = vec![
            (Some(Decision::Loss), 1500), // 25 min
            (Some(Decision::Win), 2100),  // 35 min
        ];
        let idx = GoalieNightLine::primary_goalie(&goalies, |g| *g).unwrap();
        assert_eq!(idx, 1, "longest TOI among decision-holders picked");
    }

    #[test]
    fn l0_foster2_primary_goalie_empty_returns_none() {
        let goalies: Vec<(Option<Decision>, u32)> = vec![];
        assert!(GoalieNightLine::primary_goalie(&goalies, |g| *g).is_none());
    }

    // ── Serde envelope shape ───────────────────────────────────────────────

    #[test]
    fn l0_foster2_player_night_row_skater_round_trip() {
        let line = SkaterNightLine {
            player: skater(8478402),
            team: team("EDM"),
            opponent: team("CGY"),
            home_or_away: HomeAway::Home,
            team_score: 7,
            opponent_score: 3,
            result: GameResult::Win,
            goals: 1,
            assists: 2,
            points: 3,
            plus_minus: 2,
            shots: Some(4),
            hits: Some(2),
            blocks: Some(0),
            pim: Some(0),
            takeaways: Some(1),
            giveaways: Some(1),
            toi_seconds: Some(1334),
            power_play_goals: 0,
            power_play_assists: 1,
            shorthanded_goals: 0,
            game_state: GameState::Final,
        };
        let row = PlayerNightRow::Skater(line);
        let s = serde_json::to_string(&row).unwrap();
        // Tag-based discriminator surfaces the kind so the JSON
        // envelope at /api/v1/favorites is self-describing.
        assert!(s.contains("\"kind\":\"skater\""), "envelope: {s}");
        let _back: PlayerNightRow = serde_json::from_str(&s).unwrap();
    }

    #[test]
    fn l0_foster2_player_night_row_dnp_round_trip() {
        let row = PlayerNightRow::DidNotPlay {
            player: skater(8470829),
            reason: DnpReason::Scratched,
        };
        let s = serde_json::to_string(&row).unwrap();
        assert!(s.contains("\"kind\":\"dnp\""), "envelope: {s}");
        assert!(s.contains("\"scratched\""), "snake_case reason: {s}");
        let _back: PlayerNightRow = serde_json::from_str(&s).unwrap();
    }

    #[test]
    fn l0_foster2_dnp_reason_round_trips_all_variants() {
        for r in [
            DnpReason::Scratched,
            DnpReason::InjuredReserve,
            DnpReason::TeamBye,
            DnpReason::Recalled,
            DnpReason::DataPending,
        ] {
            let s = serde_json::to_string(&r).unwrap();
            let back: DnpReason = serde_json::from_str(&s).unwrap();
            assert_eq!(back, r, "round-trip failed for {r:?} via {s}");
        }
    }
}
