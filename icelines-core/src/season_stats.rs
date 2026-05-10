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
use thiserror::Error;

use crate::identity::PlayerId;
use crate::model::{PaceScore, Position, Season, TeamAbbr};

/// Synthetic-date prefix used by the goalie loader and fixture builders
/// when real `started`/`ended` dates aren't available. Any string starting
/// with this prefix sorts BEFORE any ISO-8601 date `YYYY-MM-DD` for
/// `YYYY >= 1900`, so a stint with `started: Some("AAAA-01")` followed by
/// `Some("2024-04-22")` preserves the synthetic-first ordering through
/// `SeasonStatsBuilder::build()`'s sort.
///
/// Hart.4.1 v0.2: extracted to a const so the loader at
/// `icelines-fetch::stats_loader::build_goalie_season_stats` and the
/// `fixtures::*_trade*` builders share the invariant. Don't change this
/// value without auditing both call sites.
pub const SYNTHETIC_DATE_PREFIX: &str = "AAAA";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

impl std::fmt::Display for SeasonType {
    /// User-facing form (lowercase). Matches `label()` and the JSON
    /// wire shape — keeps CLI/TUI banners consistent with persisted
    /// data and avoids the Debug-format `Regular`/`Playoff` PascalCase
    /// leaking into error messages.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
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
    /// Shorthanded goals. Hart.3.2 — added so fantasy SH-scoring
    /// schemes survive the Hart.5 legacy-Player deletion.
    #[serde(default)]
    pub sh_goals: u32,
    /// Shorthanded points (sh_goals + sh_assists). Hart.3.2.
    #[serde(default)]
    pub sh_points: u32,
    /// Game-winning goals.
    pub gwg: u32,
    /// Overtime goals. Hart.3.2 — added for fantasy OT-scoring.
    #[serde(default)]
    pub ot_goals: u32,
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
    /// Shots this player took that missed the net. Hart.3.2 — was on
    /// legacy Player but absent from Hart.1 RealtimeStats.
    #[serde(default)]
    pub missed_shots: u32,
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
    /// Total time on ice in seconds. Suffix is mandatory — matches
    /// `StatTotals.toi_per_game_sec` so no caller has to remember which
    /// goalie field is in seconds vs minutes.
    pub time_on_ice_sec: u32,
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

// ─── Phase Lindsay L.1.1 — Tier-1 typed substructs ─────────────────────────
//
// Each of the five substructs below is per-(player_id, season, season_type),
// per DI-09. They populate from a SEPARATE per-report file under
// `~/.icelines/snapshots/<season>/<season_type>/<filename>` (see Tier-1
// file format in design/specs/stat-catalog.md v0.4 §"Tier-1 file format").
// `None` means the loader hasn't fetched the corresponding report for this
// window — NOT a zero. Per AI-05 + DI-09, downstream code distinguishes
// "no data" from "real zero" via the Option layer.
//
// Per-row nullable fields use `Option<u32>` / `Option<f64>` because the API
// nulls them in older seasons (pre-2005 realtime, pre-2007 advanced). Per-60
// rates use `f32`/`f64` because they're already pre-divided at the source.

/// Skater time-on-ice splits, per (player, season, season_type).
/// Sourced from `/skater/timeonice`. All `*_sec` fields are integer seconds
/// (mandatory suffix per the Hart convention; `time_on_ice_per_shift_sec`
/// can be < 60). Pre-1997 league data nulls TOI splits; treat absent file
/// as `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TimeOnIceStats {
    pub time_on_ice_sec: u32,
    pub time_on_ice_per_game_sec: u32,
    pub ev_time_on_ice_sec: u32,
    pub ev_time_on_ice_per_game_sec: u32,
    pub pp_time_on_ice_sec: u32,
    pub pp_time_on_ice_per_game_sec: u32,
    pub sh_time_on_ice_sec: u32,
    pub sh_time_on_ice_per_game_sec: u32,
    /// OT TOI is per-game-aggregated by the API, NOT a separate per-game
    /// figure. Older seasons null this out (no league-tracked OT TOI
    /// before the 2005-06 OT format change).
    #[serde(default)]
    pub ot_time_on_ice_sec: Option<u32>,
    pub shifts: u32,
    /// Float — `0.20` = 1 shift every 5 games. Source-rounded to 2dp.
    pub shifts_per_game: f32,
    /// Float — average shift length in seconds. Source-rounded to 1dp.
    pub time_on_ice_per_shift_sec: f32,
}

/// Skater on-ice goals at each strength + EV deployment TOI.
/// Sourced from `/skater/goalsForAgainst`.
///
/// **DI-11 caveat**: every field on this struct is last-stint-only —
/// summing across stints in a mid-season trade is wrong-data. The
/// `OnIceGoals` `StatCategory` arm in the Lindsay catalog returns
/// `None` from `read()` when `view.was_traded_in_window() == true`.
/// `ev_time_on_ice_per_game_sec` is the exception: per the v0.4
/// recategorization (SCOUT-R2 L2-F3) it lives in `TimeOnIce` category
/// in the catalog and DOES sum correctly across stints — but it's still
/// SOURCED from this endpoint, hence its presence here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalsForAgainstStats {
    pub ev_goals_for: u32,
    pub ev_goals_against: u32,
    /// EV goal-share — `ev_goals_for / (ev_goals_for + ev_goals_against)`.
    /// API-published; we don't recompute (faithfulness over derivation).
    #[serde(default)]
    pub ev_goals_for_pct: Option<f32>,
    pub pp_goals_for: u32,
    pub pp_goals_against: u32,
    pub sh_goals_for: u32,
    pub sh_goals_against: u32,
    pub even_strength_goal_difference: i32,
    /// Lives in this struct because the endpoint emits it; lives in the
    /// `TimeOnIce` category in the Lindsay catalog because it's
    /// hockey-domain a deployment stat (SCOUT-R2 L2-F3).
    pub ev_time_on_ice_per_game_sec: u32,
    /// API-derived "offensive points" tally (G + A weighted toward
    /// production at strengths the player was deployed for). Optional
    /// because it's null on older API rows.
    #[serde(default)]
    pub offensive_points: Option<u32>,
    #[serde(default)]
    pub defensive_points: Option<u32>,
}

/// Goalie advanced stats — Quality Starts, Regulation W/L, Complete-Game %.
/// Sourced from `/goalie/advanced`.
///
/// **Quality Start** (Brodeur metric): SV% ≥ league-average for the start,
/// roughly SV% ≥ .913 in modern eras OR ≤ 3 GA on ≥ 20 SA outing. The
/// API publishes the count + percentage; we don't recompute.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalieAdvancedStats {
    pub quality_starts: u32,
    /// Float — `0.500` = 50% quality-start rate. Optional because the
    /// API nulls it for goalies with 0 starts in the window.
    #[serde(default)]
    pub quality_starts_pct: Option<f32>,
    pub regulation_wins: u32,
    pub regulation_losses: u32,
    pub complete_games: u32,
    pub incomplete_games: u32,
    /// Float — share of starts the goalie finished without being pulled.
    #[serde(default)]
    pub complete_game_pct: Option<f32>,
    /// Float — shots faced per 60 minutes. Per-60 rate, source-rounded.
    #[serde(default)]
    pub shots_against_per_60: Option<f32>,
}

/// Goalie save splits by manpower strength.
/// Sourced from `/goalie/savesByStrength`. EV/PP/SH save% diverge for
/// most goalies — modern evaluation distinguishes a 5v5 anchor from a PK
/// specialist. `*_save_pct` is `Option<f32>` because the API nulls it
/// when shots-against at that strength is zero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalieSavesByStrengthStats {
    pub ev_saves: u32,
    pub ev_shots_against: u32,
    pub ev_goals_against: u32,
    #[serde(default)]
    pub ev_save_pct: Option<f32>,
    pub pp_saves: u32,
    pub pp_shots_against: u32,
    pub pp_goals_against: u32,
    /// **PP save% — saves while the goalie's team is shorthanded
    /// (opponent on PP).** Mirrors the API's `ppSavePercentage`. High
    /// variance because of small sample sizes. Often null for backups
    /// with no PK exposure (which is rare but happens).
    #[serde(default)]
    pub pp_save_pct: Option<f32>,
    pub sh_saves: u32,
    pub sh_shots_against: u32,
    pub sh_goals_against: u32,
    /// **SH save% — saves while the goalie's team is on the PP**
    /// (opponent shorthanded). Rare situation; often null for goalies
    /// who never see PP-against shots.
    #[serde(default)]
    pub sh_save_pct: Option<f32>,
}

/// Goalie biographical fields, per-window.
/// Sourced from `/goalie/bios` — the dedicated goalie identity endpoint.
/// Pre-Lindsay used `/skater/bios` as a fallback for goalie identity, which
/// produced wrong-shape fields (no goalie position context, missing
/// `firstSeasonForGameType` in goalie semantics). The Lindsay loader
/// switches the goalie identity path to read this struct and uses it to
/// populate `PlayerIdentity` for goalie rows.
///
/// Fields are mostly identity-stable, but `current_team_abbrev` and
/// `first_season_for_game_type` ARE per-window (a goalie's "current team"
/// in the 2018-19 window is not the same as in 2024-25). Hence the
/// per-window placement on `SeasonStats` rather than as a single-shot on
/// `PlayerIdentity`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalieBios {
    pub birth_city: Option<String>,
    pub birth_country_code: Option<String>,
    pub birth_date: Option<String>,
    pub current_team_abbrev: Option<String>,
    /// Draft order — string in the API ("103" not 103) because pre-1979
    /// draft is encoded as "Undrafted" or similar. Stays string here;
    /// consumers that need a numeric value parse on read.
    pub draft_overall: Option<String>,
    pub draft_round: Option<String>,
    pub draft_year: Option<String>,
    /// First season the goalie appeared in this `gameTypeId`. NOT
    /// "rookie season overall" — it's per (gameTypeId, league-tracking
    /// availability). Differs from the skater/bios semantic; loader has
    /// an explicit field-mapping table to keep the goalie path correct.
    pub first_season_for_game_type: Option<u32>,
    pub height_in_centimeters: Option<u32>,
    pub height_in_inches: Option<u32>,
    pub nationality_code: Option<String>,
    /// Goalie catches L/R. Pre-Lindsay this was sourced from
    /// `skater/bios.shootsCatches` — same field name, different
    /// semantic (skater shoots, goalie catches).
    pub shoots_catches: Option<String>,
    pub weight_in_pounds: Option<u32>,
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

    // ── Phase Lindsay L.1.1 — Tier-1 typed substructs ──────────────────
    // Each is per (player_id, season, season_type). `None` means the
    // loader hasn't fetched the corresponding report for THIS window
    // (DI-09). Loader populates via `load_report_with_fallback<T>` at
    // `StatsRepository::load_window` time (DI-28 boundary).
    /// Skater TOI splits — sourced from `/skater/timeonice`.
    #[serde(default)]
    pub time_on_ice: Option<TimeOnIceStats>,

    /// Skater on-ice goals at each strength + EV deployment TOI —
    /// sourced from `/skater/goalsForAgainst`. Subject to DI-11
    /// (last-stint-only) when consumed via the catalog `OnIceGoals`
    /// category.
    #[serde(default)]
    pub goals_for_against: Option<GoalsForAgainstStats>,

    /// Goalie quality-starts / regulation-W/L / complete-game stats —
    /// sourced from `/goalie/advanced`. `None` on every skater row.
    #[serde(default)]
    pub goalie_advanced: Option<GoalieAdvancedStats>,

    /// Goalie save splits by manpower strength — sourced from
    /// `/goalie/savesByStrength`. `None` on every skater row.
    #[serde(default)]
    pub goalie_saves_by_strength: Option<GoalieSavesByStrengthStats>,

    /// Goalie biographical fields — sourced from `/goalie/bios`.
    /// `None` on every skater row. Lindsay-loader switches goalie
    /// identity-path source from `/skater/bios` (wrong shape) to here.
    #[serde(default)]
    pub goalie_bios: Option<GoalieBios>,
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
/// pattern. JSON shape: `{"scale": "per_82" | "per_game", "value": f64}` —
/// flat, schema-friendly for export consumers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scale", content = "value")]
pub enum Projection {
    #[serde(rename = "per_82")]
    Per82(f64),
    #[serde(rename = "per_game")]
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
    time_on_ice: Option<TimeOnIceStats>,
    goals_for_against: Option<GoalsForAgainstStats>,
    goalie_advanced: Option<GoalieAdvancedStats>,
    goalie_saves_by_strength: Option<GoalieSavesByStrengthStats>,
    goalie_bios: Option<GoalieBios>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SeasonStatsBuildError {
    #[error("SeasonStats requires at least one TeamStint")]
    MissingTeamStint,
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
            time_on_ice: None,
            goals_for_against: None,
            goalie_advanced: None,
            goalie_saves_by_strength: None,
            goalie_bios: None,
        }
    }

    pub fn with_sweater_number(mut self, n: u32) -> Self {
        self.sweater_number = Some(n);
        self
    }

    /// Replace any previously-added stints with `stints`. Destructive
    /// on purpose (vs additive `add_team_stint`) — name reflects that.
    pub fn replace_team_stints(mut self, stints: Vec<TeamStint>) -> Self {
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

    pub fn with_time_on_ice(mut self, t: TimeOnIceStats) -> Self {
        self.time_on_ice = Some(t);
        self
    }

    pub fn with_goals_for_against(mut self, g: GoalsForAgainstStats) -> Self {
        self.goals_for_against = Some(g);
        self
    }

    pub fn with_goalie_advanced(mut self, g: GoalieAdvancedStats) -> Self {
        self.goalie_advanced = Some(g);
        self
    }

    pub fn with_goalie_saves_by_strength(mut self, g: GoalieSavesByStrengthStats) -> Self {
        self.goalie_saves_by_strength = Some(g);
        self
    }

    pub fn with_goalie_bios(mut self, b: GoalieBios) -> Self {
        self.goalie_bios = Some(b);
        self
    }

    /// Fallible finalize. Returns an error when the row violates
    /// construction invariants. Sorts stints into canonical order so
    /// consumers can rely on `team_stints.last()` for the most-recent
    /// team.
    pub fn try_build(mut self) -> Result<SeasonStats, SeasonStatsBuildError> {
        if self.team_stints.is_empty() {
            return Err(SeasonStatsBuildError::MissingTeamStint);
        }
        self.team_stints
            .sort_by(|a, b| match (a.started.as_ref(), b.started.as_ref()) {
                (Some(x), Some(y)) => x.cmp(y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.team.as_str().cmp(b.team.as_str()),
            });
        Ok(SeasonStats {
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
            time_on_ice: self.time_on_ice,
            goals_for_against: self.goals_for_against,
            goalie_advanced: self.goalie_advanced,
            goalie_saves_by_strength: self.goalie_saves_by_strength,
            goalie_bios: self.goalie_bios,
        })
    }

    /// Finalize for legacy/test call sites that already guarantee a
    /// stint. New production loader paths should prefer `try_build`.
    pub fn build(self) -> SeasonStats {
        self.try_build()
            .expect("SeasonStats requires at least one TeamStint")
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
    fn l0_builder_try_build_rejects_missing_stint() {
        let err = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20252026),
            SeasonType::Regular,
            Position::Center,
        )
        .try_build()
        .unwrap_err();
        assert_eq!(err, SeasonStatsBuildError::MissingTeamStint);
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
            "season_type": "regular",
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
        // Lindsay L.1.1 — Tier-1 substructs all default to None when absent
        // from the on-disk JSON. Serde-default ensures pre-Lindsay bundles
        // load without producing zeroed-out structs masquerading as real
        // data (DI-09 distinction between "not loaded" and "real zero").
        assert_eq!(stats.time_on_ice, None);
        assert_eq!(stats.goals_for_against, None);
        assert_eq!(stats.goalie_advanced, None);
        assert_eq!(stats.goalie_saves_by_strength, None);
        assert_eq!(stats.goalie_bios, None);
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
            time_on_ice_sec: 600 * 60,
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
        // season-aggregate goalie row. Strict equality is fine here —
        // this fixture is hand-built clean.
        // TODO(Hart.2 L1): the *real-API* path needs a fixture with
        // ±1 GP mismatch on game-of-trade plus a sum-equals-with-tolerance
        // proptest that exercises the loader's "trust totals + clamp last
        // stint + tracing::warn" policy (plan §"Mid-playoff-trade worked
        // example", lines 174-180).
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

    /// B5: round-trip a SeasonStats with realtime + advanced + goalie
    /// all populated. The all-None case is covered separately; this
    /// catches a field rename on any of the three sub-types that the
    /// builder smoke test would otherwise miss.
    #[test]
    fn l0_hart1_round_trip_serde_all_optionals_populated() {
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20222023),
            SeasonType::Regular,
            Position::Center,
        )
        .with_sweater_number(97)
        .add_team_stint(stl_stint(70, 30, 50))
        .with_totals(skater_totals(70, 30, 50))
        .with_realtime(RealtimeStats {
            hits: 30,
            blocked_shots: 12,
            takeaways: 80,
            giveaways: 60,
            missed_shots: 25,
        })
        .with_advanced(AdvancedStats {
            xg: Some(28.5),
            xg_per_60: Some(1.07),
            cf_pct: Some(54.2),
            ff_pct: Some(53.8),
            xgf_pct: Some(55.0),
        })
        .with_goalie(GoalieSeasonStats {
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
        })
        .build();

        let s = serde_json::to_string(&stats).unwrap();
        let back: SeasonStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, stats);
        // Sanity-check: all three Optionals serialized non-null.
        assert!(s.contains("\"hits\":30"));
        assert!(s.contains("\"xg\":28.5"));
        assert!(s.contains("\"shutouts\":5"));
    }

    /// B6: pin SeasonType wire shape. lowercase per W2 — matches the
    /// `label()` method and the `ProjectionMode`/`SchemeSource` precedent
    /// elsewhere in icelines-core. On-disk bundle compatibility relies
    /// on this not silently flipping back to PascalCase.
    #[test]
    fn l0_hart1_season_type_serde_shape() {
        assert_eq!(
            serde_json::to_string(&SeasonType::Regular).unwrap(),
            "\"regular\""
        );
        assert_eq!(
            serde_json::to_string(&SeasonType::Playoff).unwrap(),
            "\"playoff\""
        );
        let r: SeasonType = serde_json::from_str("\"regular\"").unwrap();
        let p: SeasonType = serde_json::from_str("\"playoff\"").unwrap();
        assert_eq!(r, SeasonType::Regular);
        assert_eq!(p, SeasonType::Playoff);
    }

    /// B7: Projection JSON shape is `{"scale": ..., "value": ...}` per
    /// W7 — flat, schema-friendly. Plus value() round-trips through both
    /// variants without unit confusion.
    #[test]
    fn l0_hart1_projection_serde_shape_and_value() {
        let p82 = Projection::Per82(138.0);
        let pg = Projection::PerGame(0.954);

        let s82 = serde_json::to_string(&p82).unwrap();
        let sg = serde_json::to_string(&pg).unwrap();
        assert_eq!(s82, r#"{"scale":"per_82","value":138.0}"#);
        assert_eq!(sg, r#"{"scale":"per_game","value":0.954}"#);

        let back82: Projection = serde_json::from_str(&s82).unwrap();
        let backg: Projection = serde_json::from_str(&sg).unwrap();
        assert_eq!(back82, p82);
        assert_eq!(backg, pg);

        assert_eq!(p82.value(), 138.0);
        assert_eq!(pg.value(), 0.954);
    }

    /// Lindsay L.1.1 — `TimeOnIceStats` round-trip. Pinning the
    /// `*_sec` suffix and the `Option<u32>` shape on `ot_time_on_ice_sec`
    /// (older seasons null OT TOI). If anyone collapses this to `u32`
    /// with a default of 0, the test fails because the synthesized JSON
    /// drops to 0 not None.
    #[test]
    fn l0_lindsay_time_on_ice_round_trip() {
        let toi = TimeOnIceStats {
            time_on_ice_sec: 60_000,
            time_on_ice_per_game_sec: 1230,
            ev_time_on_ice_sec: 50_000,
            ev_time_on_ice_per_game_sec: 1000,
            pp_time_on_ice_sec: 6_000,
            pp_time_on_ice_per_game_sec: 180,
            sh_time_on_ice_sec: 4_000,
            sh_time_on_ice_per_game_sec: 50,
            ot_time_on_ice_sec: Some(120),
            shifts: 1500,
            shifts_per_game: 22.5,
            time_on_ice_per_shift_sec: 41.0,
        };
        let s = serde_json::to_string(&toi).unwrap();
        let back: TimeOnIceStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, toi);

        // Pre-OT-tracking era: `ot_time_on_ice_sec` absent from JSON →
        // None, not 0.
        let json_no_ot = r#"{
            "time_on_ice_sec": 0, "time_on_ice_per_game_sec": 0,
            "ev_time_on_ice_sec": 0, "ev_time_on_ice_per_game_sec": 0,
            "pp_time_on_ice_sec": 0, "pp_time_on_ice_per_game_sec": 0,
            "sh_time_on_ice_sec": 0, "sh_time_on_ice_per_game_sec": 0,
            "shifts": 0, "shifts_per_game": 0.0, "time_on_ice_per_shift_sec": 0.0
        }"#;
        let parsed: TimeOnIceStats = serde_json::from_str(json_no_ot).unwrap();
        assert_eq!(parsed.ot_time_on_ice_sec, None);
    }

    /// Lindsay L.1.1 — `GoalsForAgainstStats` round-trip. Pinning the
    /// `i32` on `even_strength_goal_difference` (sign matters, signed),
    /// the `Option<u32>` on offensive/defensive points (API nulls them
    /// for older rows), and the `ev_time_on_ice_per_game_sec` field
    /// that v0.4 SCOUT-recategorized into `TimeOnIce` category but
    /// kept SOURCED here (endpoint provenance, not category).
    #[test]
    fn l0_lindsay_goals_for_against_round_trip() {
        let g = GoalsForAgainstStats {
            ev_goals_for: 80,
            ev_goals_against: 70,
            ev_goals_for_pct: Some(0.5333),
            pp_goals_for: 25,
            pp_goals_against: 2,
            sh_goals_for: 1,
            sh_goals_against: 8,
            even_strength_goal_difference: 10,
            ev_time_on_ice_per_game_sec: 1100,
            offensive_points: Some(45),
            defensive_points: Some(20),
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: GoalsForAgainstStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, g);

        // Negative goal differential — sign survives round-trip.
        let neg = GoalsForAgainstStats {
            even_strength_goal_difference: -15,
            ..GoalsForAgainstStats::default()
        };
        let s = serde_json::to_string(&neg).unwrap();
        let back: GoalsForAgainstStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back.even_strength_goal_difference, -15);
    }

    /// Lindsay L.1.1 — `GoalieAdvancedStats` round-trip. Quality Starts
    /// tracked since 2009-10 in the API; older seasons null `quality_starts_pct`.
    #[test]
    fn l0_lindsay_goalie_advanced_round_trip() {
        let g = GoalieAdvancedStats {
            quality_starts: 35,
            quality_starts_pct: Some(0.5833),
            regulation_wins: 28,
            regulation_losses: 18,
            complete_games: 50,
            incomplete_games: 10,
            complete_game_pct: Some(0.8333),
            shots_against_per_60: Some(28.5),
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: GoalieAdvancedStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, g);

        // 0-start row: quality_starts_pct absent from JSON → None.
        let json_zero = r#"{
            "quality_starts": 0, "regulation_wins": 0, "regulation_losses": 0,
            "complete_games": 0, "incomplete_games": 0
        }"#;
        let parsed: GoalieAdvancedStats = serde_json::from_str(json_zero).unwrap();
        assert_eq!(parsed.quality_starts_pct, None);
        assert_eq!(parsed.complete_game_pct, None);
    }

    /// Lindsay L.1.1 — `GoalieSavesByStrengthStats` round-trip. PP/SH
    /// save% nullable when shots-against at that strength is zero.
    #[test]
    fn l0_lindsay_goalie_saves_by_strength_round_trip() {
        let g = GoalieSavesByStrengthStats {
            ev_saves: 1300,
            ev_shots_against: 1400,
            ev_goals_against: 100,
            ev_save_pct: Some(0.9286),
            pp_saves: 0,
            pp_shots_against: 0,
            pp_goals_against: 0,
            pp_save_pct: None,
            sh_saves: 50,
            sh_shots_against: 60,
            sh_goals_against: 10,
            sh_save_pct: Some(0.8333),
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: GoalieSavesByStrengthStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, g);
        // pp_save_pct=None survives JSON null round-trip.
        assert!(s.contains("\"pp_save_pct\":null"));
    }

    /// Lindsay L.1.1 — `GoalieBios` round-trip. Draft fields stay
    /// `Option<String>` not `Option<u32>` because the API encodes
    /// pre-1979 draftees as non-numeric strings.
    #[test]
    fn l0_lindsay_goalie_bios_round_trip() {
        let b = GoalieBios {
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
        };
        let s = serde_json::to_string(&b).unwrap();
        let back: GoalieBios = serde_json::from_str(&s).unwrap();
        assert_eq!(back, b);
    }

    /// Lindsay L.1.1 — full builder round-trip exercising all 5 new
    /// substructs plus the existing 3. Pins the `SeasonStats` struct
    /// shape against accidental field reorder + serde rename slips.
    /// Field count drift here is the most likely way a future loader
    /// edit silently drops a Tier-1 report.
    #[test]
    fn l0_lindsay_season_stats_round_trip_all_lindsay_substructs_populated() {
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(stl_stint(70, 30, 50))
        .with_totals(skater_totals(70, 30, 50))
        .with_realtime(RealtimeStats {
            hits: 30,
            blocked_shots: 12,
            takeaways: 80,
            giveaways: 60,
            missed_shots: 25,
        })
        .with_time_on_ice(TimeOnIceStats {
            time_on_ice_sec: 90_000,
            time_on_ice_per_game_sec: 1285,
            ev_time_on_ice_sec: 75_000,
            ev_time_on_ice_per_game_sec: 1071,
            pp_time_on_ice_sec: 12_000,
            pp_time_on_ice_per_game_sec: 171,
            sh_time_on_ice_sec: 3_000,
            sh_time_on_ice_per_game_sec: 42,
            ot_time_on_ice_sec: Some(180),
            shifts: 1900,
            shifts_per_game: 27.1,
            time_on_ice_per_shift_sec: 47.4,
        })
        .with_goals_for_against(GoalsForAgainstStats {
            ev_goals_for: 95,
            ev_goals_against: 60,
            ev_goals_for_pct: Some(0.6129),
            pp_goals_for: 35,
            pp_goals_against: 1,
            sh_goals_for: 2,
            sh_goals_against: 5,
            even_strength_goal_difference: 35,
            ev_time_on_ice_per_game_sec: 1071,
            offensive_points: Some(60),
            defensive_points: Some(15),
        })
        .build();

        let s = serde_json::to_string(&stats).unwrap();
        let back: SeasonStats = serde_json::from_str(&s).unwrap();
        assert_eq!(back, stats);
        // Field-name pin: each Lindsay substruct's serde-key matches the
        // SeasonStats field name. If anyone renames `time_on_ice` →
        // `timeOnIce` on the wire, this catches it.
        assert!(s.contains("\"time_on_ice\":{"));
        assert!(s.contains("\"goals_for_against\":{"));
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
