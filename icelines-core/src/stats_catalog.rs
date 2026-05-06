//! Phase Lindsay stat catalog.
//!
//! L.1 shipped the dispatch infrastructure: `ReportKind`, `Tier`,
//! `MergeTarget`, `Tier1ReportFile`, `Tier1Row`. L.2 (this module's
//! current surface) adds the catalog itself: `StatId` enum, `StatCategory`
//! taxonomy, `StatUnit`, label/key accessor methods, the `read(view)`
//! dispatch table, and the filter-grammar types.
//!
//! Spec: `design/specs/stat-catalog.md` v0.4 §"Public types" + §"Stat
//! enumeration".
//!
//! **Stat count: 108**. Spec v0.4 §"Stat enumeration" prose totals 108
//! with Goalie at 22; explicit enumeration shows Goalie at 23 (19 base
//! plus 4 GSAx) and double-lists `PpToiPerGame`/`ShToiPerGame` in both
//! SpecialTeams AND TimeOnIce. The L.2 implementation rationalizes:
//!
//! - Goalie 23 (follow the explicit list over the prose total),
//! - `PpToiPerGame`/`ShToiPerGame` in TimeOnIce only (natural domain
//!   fit — TOI is deployment, not output).
//!
//! Net (post-L.4.1): 15 + 11 + 17 + 12 + 8 + 15 + 23 + 7 = 108 stats.
//! L.2.1 rationalized to 107; L.4.1 added `Games` (skater GP) to
//! Scoring → 108. HART/FORGE checkpoint pass-confirmed all calls.

use serde::{Deserialize, Serialize};

use crate::model::{Position, Season, MIN_GP};
use crate::season_stats::SeasonType;
use crate::stats_repository::PlayerView;

// ─── L.2.1 — StatCategory + StatUnit ────────────────────────────────────────

/// Hockey-domain categorization of a stat. The 9 categories drive TUI
/// section grouping (Queries screen) and site-page column groupings.
/// Iteration order matches `StatId::all()` — Identity first, then
/// Scoring, etc., for stable UI rendering (AI-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatCategory {
    /// Bios — never selectable for sort/filter; kept as a category for
    /// completeness so the TUI Queries screen can render the identity
    /// panel under the same structure.
    Identity,
    /// G, A, P, S%, EV/PP/SH goal counts.
    Scoring,
    /// PP/SH/Faceoff splits. `FaceoffWinPct` lives in `TwoWay` per
    /// SCOUT-R2 L2-F2; per-zone splits stay here.
    SpecialTeams,
    /// PlusMinus, Pim, Hits, Blocks, Takeaways, Giveaways, FaceoffWinPct.
    /// Most faceoffs happen at even strength → `FaceoffWinPct` is a
    /// 200-foot stat, not a special-teams one.
    TwoWay,
    /// TOI splits + shifts. `EvenStrengthTimeOnIcePerGame` lives here per
    /// SCOUT-R2 L2-F3 (sourced from `goalsForAgainst` endpoint but the
    /// hockey-domain meaning is deployment, not goals).
    TimeOnIce,
    /// On-ice goals at each strength. **DI-11 — last-stint-only**:
    /// `read()` returns `None` when `view.was_traded_in_window()`.
    /// Guard fires at category boundary; adding a new variant inherits.
    OnIceGoals,
    /// Corsi/Fenwick + zone starts + xG family. Tier-2 sources
    /// (puckPossessions, scoringRates, MoneyPuck CSV).
    Possession,
    /// Goalie-only stats. `applies_to(_, is_goalie=true)` is the gate.
    Goalie,
    /// Computed from Scoring + GP. Per-game and per-82 derived. Every
    /// per-game / per-82 inherits the MIN_GP=10 guard (PACE-B2).
    Derived,
}

impl StatCategory {
    /// User-facing label for TUI section headers + site grouping.
    pub fn label(self) -> &'static str {
        match self {
            Self::Identity => "Identity",
            Self::Scoring => "Scoring",
            Self::SpecialTeams => "Special Teams",
            Self::TwoWay => "Two-way",
            Self::TimeOnIce => "Time on Ice",
            Self::OnIceGoals => "On-ice Goals",
            Self::Possession => "Possession",
            Self::Goalie => "Goalie",
            Self::Derived => "Derived",
        }
    }

    /// Every category in declaration order. Determinism backs UI
    /// section ordering (AI-05).
    pub fn all() -> &'static [StatCategory] {
        use StatCategory::*;
        &[
            Identity,
            Scoring,
            SpecialTeams,
            TwoWay,
            TimeOnIce,
            OnIceGoals,
            Possession,
            Goalie,
            Derived,
        ]
    }
}

/// How a stat's value formats + how `FilterOp::Equals` should compare it.
/// Type-aware tolerance per L2-B1 (PACE-R2 fix replacing `f64::EPSILON`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatUnit {
    /// Integer-valued count. `Equals` tolerance: `< 0.5` (exact integer).
    Count,
    /// Percentage in [0.0, 1.0] or [0, 100] depending on stat. Storage
    /// preserves f32→f64 round-trip. `Equals` tolerance: `1e-6`.
    Pct,
    /// Per-60-minute rate. Source-rounded to 3 decimals. `Equals`
    /// tolerance: `1e-3`.
    Per60,
    /// Integer seconds (TOI fields). `Equals` tolerance: `< 0.5`.
    Seconds,
    /// Generic floating-point rate (per-82, per-game). `Equals`
    /// tolerance: `1e-6`.
    Rate,
    /// Lower-is-better (GAA, etc.). Sort direction defaults reversed.
    /// `Equals` tolerance: `1e-6`.
    Inverted,
}

// ─── L.2.1 + L.4.1 — StatId enum (108 variants) ─────────────────────────────

/// Every selectable stat in the IceLines catalog.
///
/// **Exhaustive — NOT `#[non_exhaustive]`** (per L-B17): the compiler
/// enforces "added a stat → updated everywhere" across every consumer
/// surface that matches on `StatId`.
///
/// **Declaration order is canonical**: `StatId::all()` returns variants
/// in this order, `StatCategory::members(c)` filters but preserves
/// order, and the cross-product fixture in `stat_catalog_variants.rs`
/// iterates in this order. UI surfaces never reshuffle.
///
/// 108 variants total: Scoring 15 + SpecialTeams 11 + TwoWay 17 +
/// TimeOnIce 12 + OnIceGoals 8 + Possession 15 + Goalie 23 + Derived 7.
/// L.2.1 had 107; L.4.1 added `Games` (skater GP — KEEL carry-forward
/// closing the `--filter "games>=70"` catalog gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatId {
    // ─── Scoring (15) ───────────────────────────────────────────────────
    /// Games played — `view.stats.totals.gp`. Phase Lindsay L.4.1
    /// adds this to close the catalog gap KEEL caught in L.3 (the
    /// `--filter "gp>=70"` request errored as UnknownStat). cli_key
    /// is `games` to disambiguate from the goalie-only `goalie-games`.
    Games,
    Goals,
    Assists,
    Points,
    EvGoals,
    EvPoints,
    PpGoals,
    PpAssists,
    PpPoints,
    ShGoals,
    ShPoints,
    Gwg,
    OtGoals,
    Shots,
    ShootingPct,

    // ─── SpecialTeams (11) ──────────────────────────────────────────────
    // Note (L.2.1): spec v0.4 §"Stat enumeration" double-lists
    // `PpToiPerGame` / `ShToiPerGame` here AND in `TimeOnIce`. They're
    // the same conceptual stat (TOI split per game); the natural
    // domain fit is `TimeOnIce`. Removed from `SpecialTeams` to avoid
    // duplicate variants. SpecialTeams count: 13 → 11; total: 109 → 107.
    PpGoalsPer60,
    PpPointsPer60,
    PpAssistsPer60,
    PpShootingPct,
    ShGoalsPer60,
    ShPointsPer60,
    PpGoalsAgainstPer60,
    FaceoffWins,
    FaceoffLosses,
    OffensiveZoneFaceoffPct,
    DefensiveZoneFaceoffPct,

    // ─── TwoWay (17) ────────────────────────────────────────────────────
    PlusMinus,
    Pim,
    Hits,
    BlockedShots,
    Takeaways,
    Giveaways,
    MissedShots,
    HitsPer60,
    BlockedShotsPer60,
    TakeawaysPer60,
    GiveawaysPer60,
    PenaltiesDrawn,
    PenaltiesDrawnPer60,
    PenaltiesTakenPer60,
    NetPenalties,
    NetPenaltiesPer60,
    FaceoffWinPct,

    // ─── TimeOnIce (12) ─────────────────────────────────────────────────
    TotalToi,
    TotalToiPerGame,
    EvToi,
    EvToiPerGame,
    EvenStrengthTimeOnIcePerGame,
    PpToi,
    PpToiPerGame,
    ShToi,
    ShToiPerGame,
    Shifts,
    ShiftsPerGame,
    ToiPerShift,

    // ─── OnIceGoals (8) — DI-11 last-stint-only ────────────────────────
    EvGoalsFor,
    EvGoalsAgainst,
    EvGoalsForPct,
    PpGoalsFor,
    PpGoalsAgainst,
    ShGoalsFor,
    ShGoalsAgainst,
    EvenStrengthGoalDifference,

    // ─── Possession (15) — Tier-2 sources ──────────────────────────────
    SatPct,
    UsatPct,
    OffensiveZoneStartPct,
    DefensiveZoneStartPct,
    NeutralZoneStartPct,
    OnIceShootingPct,
    Goals5v5,
    Assists5v5,
    Points5v5,
    PointsPer60_5v5,
    // xG family (SCOUT-B2 addition)
    IxG,
    IxgPer60,
    OnIceXgFor,
    OnIceXgAgainst,
    XgForPct,

    // ─── Goalie (23) ────────────────────────────────────────────────────
    GoalieGames,
    GoalieStarts,
    Wins,
    Losses,
    OtLosses,
    Ties,
    Saves,
    ShotsAgainst,
    GoalsAgainst,
    SavePct,
    Gaa,
    Shutouts,
    EvSavePct,
    PpSavePct,
    ShSavePct,
    QualityStarts,
    QualityStartPct,
    RegulationWins,
    RegulationLosses,
    // GSAx family (SCOUT-B1 addition)
    GoalieXgAgainst,
    GoalieXgAgainstPer60,
    GoalsSavedAboveExpected,
    Gsax60,

    // ─── Derived (7) ────────────────────────────────────────────────────
    Pace82,
    GoalsPer82,
    AssistsPer82,
    PointsPerGame,
    GoalsPerGame,
    AssistsPerGame,
    PaceSortKey,
}

impl StatId {
    /// Hockey-domain category. Drives TUI section grouping + DI-11
    /// trade-window guard placement (OnIceGoals fires the guard).
    pub fn category(self) -> StatCategory {
        use StatCategory::*;
        use StatId::*;
        match self {
            // Scoring
            Games | Goals | Assists | Points | EvGoals | EvPoints | PpGoals | PpAssists
            | PpPoints | ShGoals | ShPoints | Gwg | OtGoals | Shots | ShootingPct => Scoring,

            // SpecialTeams
            PpGoalsPer60
            | PpPointsPer60
            | PpAssistsPer60
            | PpShootingPct
            | ShGoalsPer60
            | ShPointsPer60
            | PpGoalsAgainstPer60
            | FaceoffWins
            | FaceoffLosses
            | OffensiveZoneFaceoffPct
            | DefensiveZoneFaceoffPct => SpecialTeams,

            // TwoWay
            PlusMinus | Pim | Hits | BlockedShots | Takeaways | Giveaways | MissedShots
            | HitsPer60 | BlockedShotsPer60 | TakeawaysPer60 | GiveawaysPer60 | PenaltiesDrawn
            | PenaltiesDrawnPer60 | PenaltiesTakenPer60 | NetPenalties | NetPenaltiesPer60
            | FaceoffWinPct => TwoWay,

            // TimeOnIce
            TotalToi
            | TotalToiPerGame
            | EvToi
            | EvToiPerGame
            | EvenStrengthTimeOnIcePerGame
            | PpToi
            | PpToiPerGame
            | ShToi
            | ShToiPerGame
            | Shifts
            | ShiftsPerGame
            | ToiPerShift => TimeOnIce,

            // OnIceGoals — DI-11 fires here
            EvGoalsFor
            | EvGoalsAgainst
            | EvGoalsForPct
            | PpGoalsFor
            | PpGoalsAgainst
            | ShGoalsFor
            | ShGoalsAgainst
            | EvenStrengthGoalDifference => OnIceGoals,

            // Possession
            SatPct
            | UsatPct
            | OffensiveZoneStartPct
            | DefensiveZoneStartPct
            | NeutralZoneStartPct
            | OnIceShootingPct
            | Goals5v5
            | Assists5v5
            | Points5v5
            | PointsPer60_5v5
            | IxG
            | IxgPer60
            | OnIceXgFor
            | OnIceXgAgainst
            | XgForPct => Possession,

            // Goalie
            GoalieGames
            | GoalieStarts
            | Wins
            | Losses
            | OtLosses
            | Ties
            | Saves
            | ShotsAgainst
            | GoalsAgainst
            | SavePct
            | Gaa
            | Shutouts
            | EvSavePct
            | PpSavePct
            | ShSavePct
            | QualityStarts
            | QualityStartPct
            | RegulationWins
            | RegulationLosses
            | GoalieXgAgainst
            | GoalieXgAgainstPer60
            | GoalsSavedAboveExpected
            | Gsax60 => Goalie,

            // Derived
            Pace82 | GoalsPer82 | AssistsPer82 | PointsPerGame | GoalsPerGame | AssistsPerGame
            | PaceSortKey => Derived,
        }
    }

    /// Storage unit + format hint. Drives `FilterOp::Equals` tolerance
    /// (per L2-B1 — type-aware, replacing `f64::EPSILON`) and display
    /// rendering. `Inverted` flags lower-is-better for sort defaults.
    pub fn unit(self) -> StatUnit {
        use StatId::*;
        use StatUnit::*;
        match self {
            // Counts
            Games
            | Goals
            | Assists
            | Points
            | EvGoals
            | EvPoints
            | PpGoals
            | PpAssists
            | PpPoints
            | ShGoals
            | ShPoints
            | Gwg
            | OtGoals
            | Shots
            | PlusMinus
            | Pim
            | Hits
            | BlockedShots
            | Takeaways
            | Giveaways
            | MissedShots
            | PenaltiesDrawn
            | NetPenalties
            | FaceoffWins
            | FaceoffLosses
            | Shifts
            | EvGoalsFor
            | EvGoalsAgainst
            | PpGoalsFor
            | PpGoalsAgainst
            | ShGoalsFor
            | ShGoalsAgainst
            | EvenStrengthGoalDifference
            | Goals5v5
            | Assists5v5
            | Points5v5
            | GoalieGames
            | GoalieStarts
            | Wins
            | Losses
            | OtLosses
            | Ties
            | Saves
            | ShotsAgainst
            | GoalsAgainst
            | Shutouts
            | QualityStarts
            | RegulationWins
            | RegulationLosses => Count,

            // Percentages [0.0, 1.0]
            ShootingPct
            | PpShootingPct
            | OffensiveZoneFaceoffPct
            | DefensiveZoneFaceoffPct
            | FaceoffWinPct
            | EvGoalsForPct
            | SatPct
            | UsatPct
            | OffensiveZoneStartPct
            | DefensiveZoneStartPct
            | NeutralZoneStartPct
            | OnIceShootingPct
            | XgForPct
            | SavePct
            | EvSavePct
            | PpSavePct
            | ShSavePct
            | QualityStartPct => Pct,

            // Per-60 rates
            PpGoalsPer60 | PpPointsPer60 | PpAssistsPer60 | ShGoalsPer60 | ShPointsPer60
            | PpGoalsAgainstPer60 | HitsPer60 | BlockedShotsPer60 | TakeawaysPer60
            | GiveawaysPer60 | PenaltiesDrawnPer60 | PenaltiesTakenPer60 | NetPenaltiesPer60
            | PointsPer60_5v5 | IxgPer60 | GoalieXgAgainstPer60 | Gsax60 => Per60,

            // Seconds
            TotalToi
            | TotalToiPerGame
            | EvToi
            | EvToiPerGame
            | EvenStrengthTimeOnIcePerGame
            | PpToi
            | PpToiPerGame
            | ShToi
            | ShToiPerGame
            | ShiftsPerGame
            | ToiPerShift => Seconds,

            // xG (floating expected-goals — generic Rate)
            IxG | OnIceXgFor | OnIceXgAgainst | GoalieXgAgainst | GoalsSavedAboveExpected => Rate,

            // Inverted (lower-is-better)
            Gaa => Inverted,

            // Derived per-82 / per-game (floating-point Rate)
            Pace82 | GoalsPer82 | AssistsPer82 | PointsPerGame | GoalsPerGame | AssistsPerGame
            | PaceSortKey => Rate,
        }
    }

    /// True for stats where bigger is better (goals, hits, save pct).
    /// False for inverted stats (GAA, goals against, losses).
    ///
    /// **Exhaustive — no `_` wildcard** (FORGE-R checkpoint #1). Each
    /// new `StatId` variant MUST add an explicit arm here so a future
    /// lower-is-better stat (e.g. `PpGoalsAgainst60` if added later)
    /// can't silently default to "higher better."
    pub fn higher_is_better(self) -> bool {
        use StatId::*;
        match self {
            // ─── Lower-is-better (false) ────────────────────────────
            Pim
            | Giveaways | GiveawaysPer60
            | Losses | OtLosses
            | GoalsAgainst | Gaa
            | PenaltiesTakenPer60
            | EvGoalsAgainst | PpGoalsAgainst | ShGoalsAgainst
            | OnIceXgAgainst
            | GoalieXgAgainst | GoalieXgAgainstPer60
            | DefensiveZoneFaceoffPct  // starting in own zone is "harder"
            | DefensiveZoneStartPct => false,

            // ─── Higher-is-better (true) — exhaustive list ──────────
            // Scoring (15)
            Games
            | Goals | Assists | Points
            | EvGoals | EvPoints
            | PpGoals | PpAssists | PpPoints
            | ShGoals | ShPoints
            | Gwg | OtGoals
            | Shots | ShootingPct
            // SpecialTeams (11) — all higher-better in the catalog
            | PpGoalsPer60 | PpPointsPer60 | PpAssistsPer60 | PpShootingPct
            | ShGoalsPer60 | ShPointsPer60 | PpGoalsAgainstPer60
            | FaceoffWins | FaceoffLosses
            | OffensiveZoneFaceoffPct
            // TwoWay — minus Pim/Giveaways/PenaltiesTaken (lower) above
            | PlusMinus
            | Hits | BlockedShots | Takeaways | MissedShots
            | HitsPer60 | BlockedShotsPer60 | TakeawaysPer60
            | PenaltiesDrawn | PenaltiesDrawnPer60
            | NetPenalties | NetPenaltiesPer60
            | FaceoffWinPct
            // TimeOnIce — all higher-better (more deployment is "better")
            | TotalToi | TotalToiPerGame
            | EvToi | EvToiPerGame | EvenStrengthTimeOnIcePerGame
            | PpToi | PpToiPerGame
            | ShToi | ShToiPerGame
            | Shifts | ShiftsPerGame | ToiPerShift
            // OnIceGoals — minus *_Against (lower) above
            | EvGoalsFor | EvGoalsForPct
            | PpGoalsFor
            | ShGoalsFor
            | EvenStrengthGoalDifference
            // Possession — minus DefensiveZone* (lower) above
            | SatPct | UsatPct
            | OffensiveZoneStartPct | NeutralZoneStartPct
            | OnIceShootingPct
            | Goals5v5 | Assists5v5 | Points5v5 | PointsPer60_5v5
            | IxG | IxgPer60
            | OnIceXgFor | XgForPct
            // Goalie — minus Losses/OtLosses/GoalsAgainst/Gaa/xG (lower) above
            | GoalieGames | GoalieStarts
            | Wins | Ties
            | Saves | ShotsAgainst
            | SavePct | Shutouts
            | EvSavePct | PpSavePct | ShSavePct
            | QualityStarts | QualityStartPct
            | RegulationWins | RegulationLosses
            | GoalsSavedAboveExpected | Gsax60
            // Derived — all higher-better
            | Pace82 | GoalsPer82 | AssistsPer82
            | PointsPerGame | GoalsPerGame | AssistsPerGame
            | PaceSortKey => true,
        }
    }

    /// Full human-readable label — used for dropdowns, table headers
    /// (wide), and **site templates** (SI-03 / L.5b sweep).
    pub fn label(self) -> &'static str {
        use StatId::*;
        match self {
            // Scoring
            Games => "Games Played",
            Goals => "Goals",
            Assists => "Assists",
            Points => "Points",
            EvGoals => "Even-Strength Goals",
            EvPoints => "Even-Strength Points",
            PpGoals => "Power-Play Goals",
            PpAssists => "Power-Play Assists",
            PpPoints => "Power-Play Points",
            ShGoals => "Short-Handed Goals",
            ShPoints => "Short-Handed Points",
            Gwg => "Game-Winning Goals",
            OtGoals => "Overtime Goals",
            Shots => "Shots",
            ShootingPct => "Shooting %",

            // SpecialTeams
            PpGoalsPer60 => "PP Goals / 60",
            PpPointsPer60 => "PP Points / 60",
            PpAssistsPer60 => "PP Assists / 60",
            PpShootingPct => "PP Shooting %",
            ShGoalsPer60 => "SH Goals / 60",
            ShPointsPer60 => "SH Points / 60",
            PpGoalsAgainstPer60 => "PP Goals Against / 60",
            FaceoffWins => "Faceoff Wins",
            FaceoffLosses => "Faceoff Losses",
            OffensiveZoneFaceoffPct => "Offensive-Zone Faceoff %",
            DefensiveZoneFaceoffPct => "Defensive-Zone Faceoff %",

            // TwoWay
            PlusMinus => "+/-",
            Pim => "Penalty Minutes",
            Hits => "Hits",
            BlockedShots => "Blocked Shots",
            Takeaways => "Takeaways",
            Giveaways => "Giveaways",
            MissedShots => "Missed Shots",
            HitsPer60 => "Hits / 60",
            BlockedShotsPer60 => "Blocked Shots / 60",
            TakeawaysPer60 => "Takeaways / 60",
            GiveawaysPer60 => "Giveaways / 60",
            PenaltiesDrawn => "Penalties Drawn",
            PenaltiesDrawnPer60 => "Penalties Drawn / 60",
            PenaltiesTakenPer60 => "Penalties Taken / 60",
            NetPenalties => "Net Penalties",
            NetPenaltiesPer60 => "Net Penalties / 60",
            FaceoffWinPct => "Faceoff Win %",

            // TimeOnIce
            TotalToi => "Total TOI",
            TotalToiPerGame => "TOI / Game",
            EvToi => "EV TOI",
            EvToiPerGame => "EV TOI / Game",
            EvenStrengthTimeOnIcePerGame => "EV Deployment TOI / Game",
            PpToi => "PP TOI",
            PpToiPerGame => "PP TOI / Game",
            ShToi => "SH TOI",
            ShToiPerGame => "SH TOI / Game",
            Shifts => "Shifts",
            ShiftsPerGame => "Shifts / Game",
            ToiPerShift => "TOI / Shift",

            // OnIceGoals
            EvGoalsFor => "EV Goals For",
            EvGoalsAgainst => "EV Goals Against",
            EvGoalsForPct => "EV Goals-For %",
            PpGoalsFor => "PP Goals For",
            PpGoalsAgainst => "PP Goals Against",
            ShGoalsFor => "SH Goals For",
            ShGoalsAgainst => "SH Goals Against",
            EvenStrengthGoalDifference => "EV Goal Differential",

            // Possession
            SatPct => "Corsi %",
            UsatPct => "Fenwick %",
            OffensiveZoneStartPct => "Offensive-Zone Start %",
            DefensiveZoneStartPct => "Defensive-Zone Start %",
            NeutralZoneStartPct => "Neutral-Zone Start %",
            OnIceShootingPct => "On-Ice Shooting %",
            Goals5v5 => "Goals (5v5)",
            Assists5v5 => "Assists (5v5)",
            Points5v5 => "Points (5v5)",
            PointsPer60_5v5 => "Points / 60 (5v5)",
            IxG => "Individual xG",
            IxgPer60 => "Individual xG / 60",
            OnIceXgFor => "On-Ice xG For",
            OnIceXgAgainst => "On-Ice xG Against",
            XgForPct => "xG For %",

            // Goalie
            GoalieGames => "Games",
            GoalieStarts => "Starts",
            Wins => "Wins",
            Losses => "Losses",
            OtLosses => "OT Losses",
            Ties => "Ties",
            Saves => "Saves",
            ShotsAgainst => "Shots Against",
            GoalsAgainst => "Goals Against",
            SavePct => "Save %",
            Gaa => "GAA",
            Shutouts => "Shutouts",
            EvSavePct => "EV Save %",
            PpSavePct => "PP Save %",
            ShSavePct => "SH Save %",
            QualityStarts => "Quality Starts",
            QualityStartPct => "Quality Start %",
            RegulationWins => "Regulation Wins",
            RegulationLosses => "Regulation Losses",
            GoalieXgAgainst => "xG Against",
            GoalieXgAgainstPer60 => "xG Against / 60",
            GoalsSavedAboveExpected => "Goals Saved Above Expected",
            Gsax60 => "GSAx / 60",

            // Derived
            Pace82 => "Points / 82",
            GoalsPer82 => "Goals / 82",
            AssistsPer82 => "Assists / 82",
            PointsPerGame => "Points / Game",
            GoalsPerGame => "Goals / Game",
            AssistsPerGame => "Assists / Game",
            PaceSortKey => "Pace Score",
        }
    }

    /// Terse column-header label — used for wide tables. ~5-7 chars.
    pub fn short_label(self) -> &'static str {
        use StatId::*;
        match self {
            Games => "GP",
            Goals => "G",
            Assists => "A",
            Points => "P",
            EvGoals => "EV G",
            EvPoints => "EV P",
            PpGoals => "PPG",
            PpAssists => "PPA",
            PpPoints => "PPP",
            ShGoals => "SHG",
            ShPoints => "SHP",
            Gwg => "GWG",
            OtGoals => "OTG",
            Shots => "Shots",
            ShootingPct => "S%",

            PpGoalsPer60 => "PPG/60",
            PpPointsPer60 => "PPP/60",
            PpAssistsPer60 => "PPA/60",
            PpShootingPct => "PP S%",
            ShGoalsPer60 => "SHG/60",
            ShPointsPer60 => "SHP/60",
            PpGoalsAgainstPer60 => "PPGA/60",
            FaceoffWins => "FOW",
            FaceoffLosses => "FOL",
            OffensiveZoneFaceoffPct => "OZ FO%",
            DefensiveZoneFaceoffPct => "DZ FO%",

            PlusMinus => "+/-",
            Pim => "PIM",
            Hits => "Hits",
            BlockedShots => "Blk",
            Takeaways => "TkA",
            Giveaways => "GvA",
            MissedShots => "Mis",
            HitsPer60 => "Hits/60",
            BlockedShotsPer60 => "Blk/60",
            TakeawaysPer60 => "TkA/60",
            GiveawaysPer60 => "GvA/60",
            PenaltiesDrawn => "PenD",
            PenaltiesDrawnPer60 => "PenD/60",
            PenaltiesTakenPer60 => "PenT/60",
            NetPenalties => "NetPen",
            NetPenaltiesPer60 => "NetPen/60",
            FaceoffWinPct => "FO%",

            TotalToi => "TOI",
            TotalToiPerGame => "TOI/g",
            EvToi => "EV TOI",
            EvToiPerGame => "EV TOI/g",
            EvenStrengthTimeOnIcePerGame => "EV Dep/g",
            PpToi => "PP TOI",
            PpToiPerGame => "PP TOI/g",
            ShToi => "SH TOI",
            ShToiPerGame => "SH TOI/g",
            Shifts => "Shft",
            ShiftsPerGame => "Shft/g",
            ToiPerShift => "TOI/Shft",

            EvGoalsFor => "EV GF",
            EvGoalsAgainst => "EV GA",
            EvGoalsForPct => "EV GF%",
            PpGoalsFor => "PP GF",
            PpGoalsAgainst => "PP GA",
            ShGoalsFor => "SH GF",
            ShGoalsAgainst => "SH GA",
            EvenStrengthGoalDifference => "EV +/-",

            SatPct => "CF%",
            UsatPct => "FF%",
            OffensiveZoneStartPct => "OZS%",
            DefensiveZoneStartPct => "DZS%",
            NeutralZoneStartPct => "NZS%",
            OnIceShootingPct => "On-Ice S%",
            Goals5v5 => "G 5v5",
            Assists5v5 => "A 5v5",
            Points5v5 => "P 5v5",
            PointsPer60_5v5 => "P/60 5v5",
            IxG => "ixG",
            IxgPer60 => "ixG/60",
            OnIceXgFor => "xGF",
            OnIceXgAgainst => "xGA",
            XgForPct => "xGF%",

            GoalieGames => "GP",
            GoalieStarts => "GS",
            Wins => "W",
            Losses => "L",
            OtLosses => "OTL",
            Ties => "T",
            Saves => "Sv",
            ShotsAgainst => "SA",
            GoalsAgainst => "GA",
            SavePct => "SV%",
            Gaa => "GAA",
            Shutouts => "SO",
            EvSavePct => "EV SV%",
            PpSavePct => "PP SV%",
            ShSavePct => "SH SV%",
            QualityStarts => "QS",
            QualityStartPct => "QS%",
            RegulationWins => "RW",
            RegulationLosses => "RL",
            GoalieXgAgainst => "xGA",
            GoalieXgAgainstPer60 => "xGA/60",
            GoalsSavedAboveExpected => "GSAx",
            Gsax60 => "GSAx/60",

            Pace82 => "P/82",
            GoalsPer82 => "G/82",
            AssistsPer82 => "A/82",
            PointsPerGame => "PPG",
            GoalsPerGame => "GPG",
            AssistsPerGame => "APG",
            PaceSortKey => "Pace",
        }
    }

    /// Narrowest column-header label — used when terminal width < 90
    /// cols. 1-3 chars, often single-letter. Default falls through to
    /// `short_label` when no narrower form is available.
    pub fn narrow_label(self) -> &'static str {
        use StatId::*;
        match self {
            Goals => "G",
            Assists => "A",
            Points => "P",
            Pim => "Pm",
            Shots => "Sh",
            ShootingPct => "S%",
            Hits => "H",
            BlockedShots => "B",
            FaceoffWinPct => "FO",
            TotalToiPerGame => "TOI",
            // Most variants don't have a distinct narrow form; reuse short.
            _ => self.short_label(),
        }
    }

    /// Hyphen-case canonical key — used for `--filter` / `--sort`
    /// parsing, **CSS class suffix** (`.stat-pp-goals`), **URL anchor**
    /// (`#hits-per-60`), and **HTTP JSON key**. Spec §"L.5b sweep
    /// enumeration" pins these as the four string surfaces.
    ///
    /// Uniqueness is enforced by an L0 test
    /// (`l0_lindsay_cli_keys_unique_across_catalog`).
    pub fn cli_key(self) -> &'static str {
        use StatId::*;
        match self {
            Games => "games",
            Goals => "goals",
            Assists => "assists",
            Points => "points",
            EvGoals => "ev-goals",
            EvPoints => "ev-points",
            PpGoals => "pp-goals",
            PpAssists => "pp-assists",
            PpPoints => "pp-points",
            ShGoals => "sh-goals",
            ShPoints => "sh-points",
            Gwg => "gwg",
            OtGoals => "ot-goals",
            Shots => "shots",
            ShootingPct => "shooting-pct",

            PpGoalsPer60 => "pp-goals-per-60",
            PpPointsPer60 => "pp-points-per-60",
            PpAssistsPer60 => "pp-assists-per-60",
            PpShootingPct => "pp-shooting-pct",
            ShGoalsPer60 => "sh-goals-per-60",
            ShPointsPer60 => "sh-points-per-60",
            PpGoalsAgainstPer60 => "pp-goals-against-per-60",
            FaceoffWins => "faceoff-wins",
            FaceoffLosses => "faceoff-losses",
            OffensiveZoneFaceoffPct => "offensive-zone-faceoff-pct",
            DefensiveZoneFaceoffPct => "defensive-zone-faceoff-pct",

            PlusMinus => "plus-minus",
            Pim => "pim",
            Hits => "hits",
            BlockedShots => "blocked-shots",
            Takeaways => "takeaways",
            Giveaways => "giveaways",
            MissedShots => "missed-shots",
            HitsPer60 => "hits-per-60",
            BlockedShotsPer60 => "blocked-shots-per-60",
            TakeawaysPer60 => "takeaways-per-60",
            GiveawaysPer60 => "giveaways-per-60",
            PenaltiesDrawn => "penalties-drawn",
            PenaltiesDrawnPer60 => "penalties-drawn-per-60",
            PenaltiesTakenPer60 => "penalties-taken-per-60",
            NetPenalties => "net-penalties",
            NetPenaltiesPer60 => "net-penalties-per-60",
            FaceoffWinPct => "faceoff-win-pct",

            TotalToi => "total-toi",
            TotalToiPerGame => "total-toi-per-game",
            EvToi => "ev-toi",
            EvToiPerGame => "ev-toi-per-game",
            EvenStrengthTimeOnIcePerGame => "even-strength-time-on-ice-per-game",
            PpToi => "pp-toi",
            PpToiPerGame => "pp-toi-per-game",
            ShToi => "sh-toi",
            ShToiPerGame => "sh-toi-per-game",
            Shifts => "shifts",
            ShiftsPerGame => "shifts-per-game",
            ToiPerShift => "toi-per-shift",

            EvGoalsFor => "ev-goals-for",
            EvGoalsAgainst => "ev-goals-against",
            EvGoalsForPct => "ev-goals-for-pct",
            PpGoalsFor => "pp-goals-for",
            PpGoalsAgainst => "pp-goals-against",
            ShGoalsFor => "sh-goals-for",
            ShGoalsAgainst => "sh-goals-against",
            EvenStrengthGoalDifference => "even-strength-goal-difference",

            SatPct => "sat-pct",
            UsatPct => "usat-pct",
            OffensiveZoneStartPct => "offensive-zone-start-pct",
            DefensiveZoneStartPct => "defensive-zone-start-pct",
            NeutralZoneStartPct => "neutral-zone-start-pct",
            OnIceShootingPct => "on-ice-shooting-pct",
            Goals5v5 => "goals-5v5",
            Assists5v5 => "assists-5v5",
            Points5v5 => "points-5v5",
            PointsPer60_5v5 => "points-per-60-5v5",
            IxG => "ixg",
            IxgPer60 => "ixg-per-60",
            OnIceXgFor => "on-ice-xg-for",
            OnIceXgAgainst => "on-ice-xg-against",
            XgForPct => "xg-for-pct",

            GoalieGames => "goalie-games",
            GoalieStarts => "goalie-starts",
            Wins => "wins",
            Losses => "losses",
            OtLosses => "ot-losses",
            Ties => "ties",
            Saves => "saves",
            ShotsAgainst => "shots-against",
            GoalsAgainst => "goals-against",
            SavePct => "save-pct",
            Gaa => "gaa",
            Shutouts => "shutouts",
            EvSavePct => "ev-save-pct",
            PpSavePct => "pp-save-pct",
            ShSavePct => "sh-save-pct",
            QualityStarts => "quality-starts",
            QualityStartPct => "quality-start-pct",
            RegulationWins => "regulation-wins",
            RegulationLosses => "regulation-losses",
            GoalieXgAgainst => "goalie-xg-against",
            GoalieXgAgainstPer60 => "goalie-xg-against-per-60",
            GoalsSavedAboveExpected => "goals-saved-above-expected",
            Gsax60 => "gsax-per-60",

            Pace82 => "pace-82",
            GoalsPer82 => "goals-per-82",
            AssistsPer82 => "assists-per-82",
            PointsPerGame => "points-per-game",
            GoalsPerGame => "goals-per-game",
            AssistsPerGame => "assists-per-game",
            PaceSortKey => "pace-sort-key",
        }
    }

    /// Every variant in declaration order. Determinism backs UI
    /// rendering (AI-05) and the cross-product fixture in
    /// `stat_catalog_variants.rs`. Zero-alloc — `&'static`.
    pub fn all() -> &'static [StatId] {
        use StatId::*;
        &[
            // Scoring
            Games,
            Goals,
            Assists,
            Points,
            EvGoals,
            EvPoints,
            PpGoals,
            PpAssists,
            PpPoints,
            ShGoals,
            ShPoints,
            Gwg,
            OtGoals,
            Shots,
            ShootingPct,
            // SpecialTeams
            PpGoalsPer60,
            PpPointsPer60,
            PpAssistsPer60,
            PpShootingPct,
            ShGoalsPer60,
            ShPointsPer60,
            PpGoalsAgainstPer60,
            FaceoffWins,
            FaceoffLosses,
            OffensiveZoneFaceoffPct,
            DefensiveZoneFaceoffPct,
            // TwoWay
            PlusMinus,
            Pim,
            Hits,
            BlockedShots,
            Takeaways,
            Giveaways,
            MissedShots,
            HitsPer60,
            BlockedShotsPer60,
            TakeawaysPer60,
            GiveawaysPer60,
            PenaltiesDrawn,
            PenaltiesDrawnPer60,
            PenaltiesTakenPer60,
            NetPenalties,
            NetPenaltiesPer60,
            FaceoffWinPct,
            // TimeOnIce
            TotalToi,
            TotalToiPerGame,
            EvToi,
            EvToiPerGame,
            EvenStrengthTimeOnIcePerGame,
            PpToi,
            PpToiPerGame,
            ShToi,
            ShToiPerGame,
            Shifts,
            ShiftsPerGame,
            ToiPerShift,
            // OnIceGoals
            EvGoalsFor,
            EvGoalsAgainst,
            EvGoalsForPct,
            PpGoalsFor,
            PpGoalsAgainst,
            ShGoalsFor,
            ShGoalsAgainst,
            EvenStrengthGoalDifference,
            // Possession
            SatPct,
            UsatPct,
            OffensiveZoneStartPct,
            DefensiveZoneStartPct,
            NeutralZoneStartPct,
            OnIceShootingPct,
            Goals5v5,
            Assists5v5,
            Points5v5,
            PointsPer60_5v5,
            IxG,
            IxgPer60,
            OnIceXgFor,
            OnIceXgAgainst,
            XgForPct,
            // Goalie
            GoalieGames,
            GoalieStarts,
            Wins,
            Losses,
            OtLosses,
            Ties,
            Saves,
            ShotsAgainst,
            GoalsAgainst,
            SavePct,
            Gaa,
            Shutouts,
            EvSavePct,
            PpSavePct,
            ShSavePct,
            QualityStarts,
            QualityStartPct,
            RegulationWins,
            RegulationLosses,
            GoalieXgAgainst,
            GoalieXgAgainstPer60,
            GoalsSavedAboveExpected,
            Gsax60,
            // Derived
            Pace82,
            GoalsPer82,
            AssistsPer82,
            PointsPerGame,
            GoalsPerGame,
            AssistsPerGame,
            PaceSortKey,
        ]
    }

    /// Parse a `cli_key()` string back to a `StatId`. `None` for
    /// unknown keys — the CLI front-end maps `None` to
    /// `FilterParseError::UnknownStat` (L.2.4 grammar). Zero-alloc;
    /// linear over `all()` (108 elements is fine for a one-shot parse).
    ///
    /// Gaps.1 — also accepts the short common aliases users naturally
    /// type (`g`, `a`, `p`, `gp`, `ppg`, `s`, `blk`, `tk`, `gv`, `+/-`).
    /// The aliases resolve to the full cli_key without surfacing in
    /// `cli_key()` output (round-trip stays unambiguous).
    pub fn from_cli_key(s: &str) -> Option<StatId> {
        let lower = s.to_ascii_lowercase();
        // Hand-curated alias table — only the keys users naturally
        // shorten. New aliases need a corresponding catalog cli_key
        // they map to; conflicts with an existing cli_key would shadow
        // it (none today).
        let resolved: &str = match lower.as_str() {
            // Scoring shortcuts
            "g" => "goals",
            "a" => "assists",
            "p" | "pts" => "points",
            "s" | "sog" => "shots",
            "shootingpct" | "shooting%" | "sh%" => "shooting-pct",
            "ppg" => "points-per-game",
            "gpg" => "goals-per-game",
            "apg" => "assists-per-game",
            // Schedule shortcuts
            "gp" | "games-played" => "games",
            // Two-way shortcuts
            "+/-" | "plusminus" => "plus-minus",
            "pen" | "penaltyminutes" | "penalty-minutes" => "pim",
            "blk" | "blocks" => "blocked-shots",
            "tk" => "takeaways",
            "gv" => "giveaways",
            "mis" | "missed" => "missed-shots",
            "fow%" | "fow-pct" => "faceoff-win-pct",
            // Special teams shortcuts
            "ppg-rate" | "pp-goals-rate" => "pp-goals-per-60",
            // Goalie shortcuts (the `gp`/`games` collision is resolved
            // by the goalie command's filter parser preferring
            // goalie-games when both available — see Gaps.4).
            "sv%" | "save%" => "save-pct",
            "sv" => "saves",
            "sa" => "shots-against",
            "ga" => "goals-against",
            "w" => "wins",
            "l" => "losses",
            "ot" => "ot-losses",
            "so" => "shutouts",
            // Rate / derived
            "pace" => "pace-82",
            // Anything else: pass through unchanged so existing
            // catalog cli_keys still work directly.
            _ => lower.as_str(),
        };
        Self::all()
            .iter()
            .copied()
            .find(|sid| sid.cli_key() == resolved)
    }

    /// Phase Reports — which Tier-1 ReportKind (if any) provides the
    /// data backing this stat. `None` means the stat is sourced from
    /// `/skater/summary` / `/goalie/summary` / a Tier-2 endpoint /
    /// or computed from other stats — in any of those cases the user
    /// can't toggle it off via the Reports overlay (it's either always
    /// available or fetched on-demand).
    ///
    /// Used by:
    /// - The Reports overlay column-visibility filter (Reports.5): a
    ///   StatId whose `report_source()` is `Some(kind)` and whose
    ///   `kind` is disabled in `ReportToggles` is hidden from sort
    ///   pickers / career tables / query outputs.
    /// - The missing-source banner gate (Reports.3): a snapshot read
    ///   for a disabled report is silently skipped (returns empty)
    ///   instead of pushing `MissingSource::*` to the status bar.
    ///
    /// The mapping mirrors the spec at `design/specs/stat-catalog.md`
    /// §"Stat enumeration". Cross-cutting stats (e.g.
    /// `EvenStrengthTimeOnIcePerGame` is in `TimeOnIce` category but
    /// sourced from the `goalsForAgainst` endpoint) return their
    /// actual data source, not their hockey-domain category.
    pub fn report_source(self) -> Option<ReportKind> {
        use ReportKind::*;
        use StatId::*;
        match self {
            // ── Realtime — Hits/Blocks/Take/Give/Missed + per-60 derivatives ──
            Hits | BlockedShots | Takeaways | Giveaways | MissedShots | HitsPer60
            | BlockedShotsPer60 | TakeawaysPer60 | GiveawaysPer60 => Some(SkaterRealtime),

            // ── TimeOnIce — splits + shifts ──
            // TotalToiPerGame is intentionally NOT here: summary already
            // carries `timeOnIcePerGame`, so users without timeonice
            // enabled still see it.
            TotalToi | EvToi | EvToiPerGame | PpToi | PpToiPerGame | ShToi | ShToiPerGame
            | Shifts | ShiftsPerGame | ToiPerShift => Some(SkaterTimeOnIce),

            // ── GoalsForAgainst — on-ice goals + EV TOI per game ──
            // EvenStrengthTimeOnIcePerGame is sourced here per spec
            // SCOUT-R2 L2-F3 (deployment stat, not a goal stat).
            EvGoalsFor
            | EvGoalsAgainst
            | EvGoalsForPct
            | PpGoalsFor
            | PpGoalsAgainst
            | ShGoalsFor
            | ShGoalsAgainst
            | EvenStrengthGoalDifference
            | EvenStrengthTimeOnIcePerGame => Some(SkaterGoalsForAgainst),

            // ── GoalieAdvanced — quality starts + regulation W/L ──
            QualityStarts | QualityStartPct | RegulationWins | RegulationLosses => {
                Some(GoalieAdvanced)
            }

            // ── GoalieSavesByStrength — situational save percentages ──
            EvSavePct | PpSavePct | ShSavePct => Some(GoalieSavesByStrength),

            // Everything else: summary / Tier-2 / MoneyPuck / derived.
            _ => None,
        }
    }

    /// Per-row applicability. Goalie-category stats apply when the view
    /// IS a goalie (per-row, via `view.is_goalie()` — covers the
    /// emergency-backup-goalie case where a skater is goalie for one
    /// game). Faceoff-takers gated to centers; on-ice stats apply to
    /// every skater regardless of position.
    pub fn applies_to(self, pos: Position, is_goalie: bool) -> bool {
        match self.category() {
            StatCategory::Goalie => is_goalie,
            StatCategory::Identity => true,
            _ if is_goalie => false, // skater-only stats hidden on goalies
            _ => match self {
                // Faceoff-taker stats — centers only.
                StatId::FaceoffWinPct | StatId::FaceoffWins | StatId::FaceoffLosses => {
                    pos == Position::Center
                }
                // Zone-start / on-ice stats apply to all skaters.
                _ => true,
            },
        }
    }

    /// First season with reliable data. `Season(0)` for always-available
    /// (Scoring totals, Identity). Pre-2005 nulls realtime
    /// (Hits/Blocks/Takeaways/Giveaways/MissedShots); pre-2007 nulls
    /// possession + xG family. Per L2-F4 (SCOUT-R2): use `20052006`
    /// for realtime — the data exists 1997+ but is unreliable until
    /// the lockout era.
    /// Phase Lindsay L.4.1 — curated default columns for the career
    /// table on the player card. Returns true for the per-position
    /// "must-have" stats that show by default; users cycle through
    /// preset templates via `[`/`]` to see other categories.
    ///
    /// Per-position defaults (matching hockey-reference convention):
    /// - **Skater (C/LW/RW/D)**: GP G A P PPG GWG +/- PIM PPG PPP Shots
    ///   S% TOI Hits Blk (15 columns).
    /// - **Center additionally**: FaceoffWinPct (16 columns).
    /// - **Defense additionally**: EvGoalsForPct (16 columns) — SCOUT-8.
    /// - **Goalie**: GP GS W L OTL Sv SA Sv% GAA SO QS (11 columns).
    ///
    /// Identity / non-selectable stats always return false.
    pub fn default_in_career_table(self, pos: Position) -> bool {
        use Position::*;
        use StatId::*;
        // Common skater defaults (excludes position-specific stats).
        //
        // SCOUT L.4 review: added Gwg (canonical career-glance counter).
        // SCOUT-3 L.5b post-fix: added PointsPerGame (cross-season
        // legibility — without it, raw season totals compress rookie
        // years into noise).
        let skater_common = matches!(
            self,
            Games
                | Goals
                | Assists
                | Points
                | PointsPerGame
                | Gwg
                | PlusMinus
                | Pim
                | PpGoals
                | PpPoints
                | Shots
                | ShootingPct
                | TotalToiPerGame
                | Hits
                | BlockedShots
        );
        // Goalie defaults. SCOUT L.4 review: dropped RegulationWins
        // (non-canonical, fantasy-derived), added Saves + ShotsAgainst
        // (volume context required to interpret SV%/GAA).
        let goalie_default = matches!(
            self,
            GoalieGames
                | GoalieStarts
                | Wins
                | Losses
                | OtLosses
                | Saves
                | ShotsAgainst
                | SavePct
                | Gaa
                | Shutouts
                | QualityStarts
        );
        match pos {
            Goalie => goalie_default,
            Center => skater_common || self == FaceoffWinPct,
            // SCOUT-8 L.5b post-fix: defenseman default adds
            // EvGoalsForPct — closest hockey analog to "RBI", a
            // single-number measure of how much a D drives play. The
            // OnIceGoals category fires DI-11 for traded-window views
            // (returns None there); that's the right semantic.
            Defense => skater_common || self == EvGoalsForPct,
            LeftWing | RightWing => skater_common,
        }
    }

    pub fn available_since(self) -> Season {
        use StatId::*;
        match self {
            // Realtime — 2005-06 (data exists 1997+ but unreliable)
            Hits | BlockedShots | Takeaways | Giveaways | MissedShots | HitsPer60
            | BlockedShotsPer60 | TakeawaysPer60 | GiveawaysPer60 | PenaltiesDrawn
            | PenaltiesDrawnPer60 | PenaltiesTakenPer60 | NetPenalties | NetPenaltiesPer60 => {
                Season(20052006)
            }
            // Possession family — Corsi tracking starts 2007-08.
            SatPct
            | UsatPct
            | OffensiveZoneStartPct
            | DefensiveZoneStartPct
            | NeutralZoneStartPct
            | OnIceShootingPct
            | Goals5v5
            | Assists5v5
            | Points5v5
            | PointsPer60_5v5 => Season(20072008),
            // xG family — MoneyPuck / NHL Edge from ~2007-08.
            IxG
            | IxgPer60
            | OnIceXgFor
            | OnIceXgAgainst
            | XgForPct
            | GoalieXgAgainst
            | GoalieXgAgainstPer60
            | GoalsSavedAboveExpected
            | Gsax60 => Season(20072008),
            // Goalie advanced (QS, complete-game) — 2009-10.
            QualityStarts | QualityStartPct => Season(20092010),
            // Save% by strength — 2014-15 when API exposed it.
            EvSavePct | PpSavePct | ShSavePct => Season(20142015),
            // OT goals — modern OT format from 2005-06.
            OtGoals | OtLosses => Season(20052006),
            // Scoring/Identity/TimeOnIce/SpecialTeams basics — always.
            _ => Season(0),
        }
    }

    /// Per-row era applicability.
    pub fn applies_to_era(self, season: Season) -> bool {
        season.0 >= self.available_since().0
    }

    /// Read the value for this stat off the given view. Returns `None`
    /// when the underlying data isn't loaded, the stat isn't applicable
    /// to the row, or a guard fires (DI-11 trade-window, MIN_GP, etc.).
    ///
    /// `read()` is the **only** function that knows where the value
    /// lives. Sort, filter, display, export all call into this — a
    /// change to where `Hits` is stored only touches one match arm.
    ///
    /// **DI-11 enforcement at category boundary**: OnIceGoals stats
    /// short-circuit to `None` when the view was traded mid-window,
    /// regardless of which OnIceGoals variant. Adding a new OnIceGoals
    /// stat inherits the guard; moving a stat OUT of OnIceGoals (as
    /// `EvenStrengthTimeOnIcePerGame` did in v0.4) removes the guard
    /// automatically.
    ///
    /// **MIN_GP guard**: derived per-game / per-82 stats return `None`
    /// when `gp < MIN_GP` (10). Inherited from the existing PaceScore
    /// guard (PACE-B2 R2 fix).
    ///
    /// **Per-60 TOI floor**: per-60 rates require `total_toi_sec >= 300`
    /// (PACE-F1 — soft floor). Below that, stats are statistical noise.
    pub fn read(self, view: &PlayerView<'_>) -> Option<f64> {
        use StatId::*;

        // DI-11 enforcement at category boundary. OnIceGoals stats are
        // last-stint-only; summing across stints is wrong-data.
        if self.category() == StatCategory::OnIceGoals && view.was_traded_in_window() {
            return None;
        }

        let stats = view.stats;

        match self {
            // ─── Scoring (15) ───────────────────────────────────────
            Games => Some(stats.totals.gp as f64),
            Goals => Some(stats.totals.goals as f64),
            Assists => Some(stats.totals.assists as f64),
            Points => Some(stats.totals.points as f64),
            EvGoals => {
                // Even-strength = total minus PP minus SH.
                let g = stats.totals.goals;
                Some(
                    g.saturating_sub(stats.totals.pp_goals)
                        .saturating_sub(stats.totals.sh_goals) as f64,
                )
            }
            EvPoints => {
                let p = stats.totals.points;
                Some(
                    p.saturating_sub(stats.totals.pp_points)
                        .saturating_sub(stats.totals.sh_points) as f64,
                )
            }
            PpGoals => Some(stats.totals.pp_goals as f64),
            PpAssists => Some(view.pp_assists() as f64),
            PpPoints => Some(stats.totals.pp_points as f64),
            ShGoals => Some(stats.totals.sh_goals as f64),
            ShPoints => Some(stats.totals.sh_points as f64),
            Gwg => Some(stats.totals.gwg as f64),
            OtGoals => Some(stats.totals.ot_goals as f64),
            Shots => Some(stats.totals.shots as f64),
            ShootingPct => stats.totals.shooting_pct.map(f64::from),

            // ─── SpecialTeams (11) — all Tier-2, deferred to L.6 ───
            // These need PP-TOI / PK-TOI denominators which only the
            // powerplay / penaltykill endpoints emit. Computing per-60
            // from per-82 (i.e. season totals) would need PP-TOI /
            // total-TOI ratio — not the same numerator. HART-R checkpoint
            // caught a numeric bug here in L.2.2's first draft; gate
            // every SpecialTeams arm to `None` until the L.6 cache lands
            // the powerplay/penaltykill rows with proper PP-TOI.
            PpGoalsPer60 => None,            // L.6: powerplay endpoint
            PpPointsPer60 => None,           // L.6: powerplay endpoint
            PpAssistsPer60 => None,          // L.6: powerplay endpoint
            PpShootingPct => None,           // L.6: powerplay endpoint
            ShGoalsPer60 => None,            // L.6: penaltykill endpoint
            ShPointsPer60 => None,           // L.6: penaltykill endpoint
            PpGoalsAgainstPer60 => None,     // L.6: penaltykill endpoint
            FaceoffWins => None,             // L.6: faceoffwins endpoint
            FaceoffLosses => None,           // L.6: faceoffwins endpoint
            OffensiveZoneFaceoffPct => None, // L.6: faceoffpercentages
            DefensiveZoneFaceoffPct => None, // L.6: faceoffpercentages

            // ─── TwoWay (17) ────────────────────────────────────────
            PlusMinus => Some(stats.totals.plus_minus as f64),
            Pim => Some(stats.totals.pim as f64),
            Hits => view.hits().map(f64::from),
            BlockedShots => view.blocked_shots().map(f64::from),
            Takeaways => view.takeaways().map(f64::from),
            Giveaways => view.giveaways().map(f64::from),
            MissedShots => stats.realtime.as_ref().map(|r| r.missed_shots as f64),
            HitsPer60 => per_60(view, view.hits()),
            BlockedShotsPer60 => per_60(view, view.blocked_shots()),
            TakeawaysPer60 => per_60(view, view.takeaways()),
            GiveawaysPer60 => per_60(view, view.giveaways()),
            PenaltiesDrawn => None,      // L.6: penalties endpoint
            PenaltiesDrawnPer60 => None, // L.6
            PenaltiesTakenPer60 => None, // L.6
            NetPenalties => None,        // L.6
            NetPenaltiesPer60 => None,   // L.6
            FaceoffWinPct => stats.totals.faceoff_win_pct.map(f64::from),

            // ─── TimeOnIce (12) ─────────────────────────────────────
            TotalToi => stats.time_on_ice.as_ref().map(|t| t.time_on_ice_sec as f64),
            TotalToiPerGame => stats.totals.toi_per_game_sec.map(f64::from).or_else(|| {
                stats
                    .time_on_ice
                    .as_ref()
                    .map(|t| t.time_on_ice_per_game_sec as f64)
            }),
            EvToi => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.ev_time_on_ice_sec as f64),
            EvToiPerGame => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.ev_time_on_ice_per_game_sec as f64),
            // SCOUT-R2 L2-F3: sourced from goalsForAgainst, lives in
            // TimeOnIce category (NOT subject to DI-11 since this
            // category isn't OnIceGoals).
            EvenStrengthTimeOnIcePerGame => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.ev_time_on_ice_per_game_sec as f64),
            PpToi => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.pp_time_on_ice_sec as f64),
            PpToiPerGame => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.pp_time_on_ice_per_game_sec as f64),
            ShToi => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.sh_time_on_ice_sec as f64),
            ShToiPerGame => stats
                .time_on_ice
                .as_ref()
                .map(|t| t.sh_time_on_ice_per_game_sec as f64),
            Shifts => stats.time_on_ice.as_ref().map(|t| t.shifts as f64),
            ShiftsPerGame => stats
                .time_on_ice
                .as_ref()
                .map(|t| f64::from(t.shifts_per_game)),
            ToiPerShift => stats
                .time_on_ice
                .as_ref()
                .map(|t| f64::from(t.time_on_ice_per_shift_sec)),

            // ─── OnIceGoals (8) — DI-11 already short-circuited ─────
            EvGoalsFor => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.ev_goals_for as f64),
            EvGoalsAgainst => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.ev_goals_against as f64),
            EvGoalsForPct => stats
                .goals_for_against
                .as_ref()
                .and_then(|g| g.ev_goals_for_pct.map(f64::from)),
            PpGoalsFor => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.pp_goals_for as f64),
            PpGoalsAgainst => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.pp_goals_against as f64),
            ShGoalsFor => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.sh_goals_for as f64),
            ShGoalsAgainst => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.sh_goals_against as f64),
            EvenStrengthGoalDifference => stats
                .goals_for_against
                .as_ref()
                .map(|g| g.even_strength_goal_difference as f64),

            // ─── Possession (15) — Tier-2 (L.6) ─────────────────────
            // Most populate from extra_reports in L.6. The MoneyPuck
            // CSV path (xg/cf_pct/ff_pct/xgf_pct on AdvancedStats) is
            // the L.1 path that already exists.
            SatPct => view.cf_pct(),
            UsatPct => view.ff_pct(),
            OffensiveZoneStartPct => None, // L.6: puckPossessions
            DefensiveZoneStartPct => None,
            NeutralZoneStartPct => None,
            OnIceShootingPct => None,
            Goals5v5 => None, // L.6: scoringRates
            Assists5v5 => None,
            Points5v5 => None,
            PointsPer60_5v5 => None,
            IxG => view.xg(), // existing MoneyPuck path
            IxgPer60 => view.xg_per_60(),
            OnIceXgFor => None, // L.6: distinct from individual xG
            OnIceXgAgainst => None,
            XgForPct => view.xgf_pct(),

            // ─── Goalie (23) ────────────────────────────────────────
            GoalieGames => stats.goalie.as_ref().map(|_| stats.totals.gp as f64),
            GoalieStarts => stats.goalie.as_ref().map(|g| g.games_started as f64),
            Wins => stats.goalie.as_ref().map(|g| g.wins as f64),
            Losses => stats.goalie.as_ref().map(|g| g.losses as f64),
            OtLosses => stats
                .goalie
                .as_ref()
                .and_then(|g| g.ot_losses.map(f64::from)),
            Ties => stats.goalie.as_ref().and_then(|g| g.ties.map(f64::from)),
            Saves => stats.goalie.as_ref().map(|g| g.saves as f64),
            ShotsAgainst => stats.goalie.as_ref().map(|g| g.shots_against as f64),
            GoalsAgainst => stats.goalie.as_ref().map(|g| g.goals_against as f64),
            SavePct => stats
                .goalie
                .as_ref()
                .and_then(|g| g.save_pct.map(f64::from)),
            Gaa => stats
                .goalie
                .as_ref()
                .and_then(|g| g.goals_against_average.map(f64::from)),
            Shutouts => stats.goalie.as_ref().map(|g| g.shutouts as f64),
            EvSavePct => stats
                .goalie_saves_by_strength
                .as_ref()
                .and_then(|g| g.ev_save_pct.map(f64::from)),
            PpSavePct => stats
                .goalie_saves_by_strength
                .as_ref()
                .and_then(|g| g.pp_save_pct.map(f64::from)),
            ShSavePct => stats
                .goalie_saves_by_strength
                .as_ref()
                .and_then(|g| g.sh_save_pct.map(f64::from)),
            QualityStarts => stats
                .goalie_advanced
                .as_ref()
                .map(|g| g.quality_starts as f64),
            QualityStartPct => stats
                .goalie_advanced
                .as_ref()
                .and_then(|g| g.quality_starts_pct.map(f64::from)),
            RegulationWins => stats
                .goalie_advanced
                .as_ref()
                .map(|g| g.regulation_wins as f64),
            RegulationLosses => stats
                .goalie_advanced
                .as_ref()
                .map(|g| g.regulation_losses as f64),
            // GSAx family — Tier-2 (L.6 MoneyPuck/NHL Edge).
            GoalieXgAgainst => None,         // L.6
            GoalieXgAgainstPer60 => None,    // L.6
            GoalsSavedAboveExpected => None, // L.6
            Gsax60 => None,                  // L.6

            // ─── Derived (7) ────────────────────────────────────────
            // All inherit MIN_GP guard.
            Pace82 => view.pace_82(),
            GoalsPer82 => view.goals_per_82(),
            AssistsPer82 => {
                let gp = view.gp();
                if gp < MIN_GP {
                    None
                } else {
                    Some(view.assists() as f64 / gp as f64 * 82.0)
                }
            }
            PointsPerGame => {
                let gp = view.gp();
                if gp < MIN_GP {
                    None
                } else {
                    Some(view.points() as f64 / gp as f64)
                }
            }
            GoalsPerGame => {
                let gp = view.gp();
                if gp < MIN_GP {
                    None
                } else {
                    Some(view.goals() as f64 / gp as f64)
                }
            }
            AssistsPerGame => {
                let gp = view.gp();
                if gp < MIN_GP {
                    None
                } else {
                    Some(view.assists() as f64 / gp as f64)
                }
            }
            PaceSortKey => Some(view.pace_sort_key()),
        }
    }

    /// Universal sort comparator (AI-06). Tiebreak: `(stat_value
    /// desc/asc, nhl_id asc)`. `None` sorts last regardless of
    /// `higher_is_better`. Codified here so every catalog-driven sort
    /// across the app inherits the same ordering.
    pub fn sort_cmp(self, a: &PlayerView<'_>, b: &PlayerView<'_>) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let va = self.read(a);
        let vb = self.read(b);
        match (va, vb) {
            (Some(x), Some(y)) => {
                let primary = if self.higher_is_better() {
                    y.partial_cmp(&x).unwrap_or(Ordering::Equal)
                } else {
                    x.partial_cmp(&y).unwrap_or(Ordering::Equal)
                };
                primary.then_with(|| a.id().0.cmp(&b.id().0))
            }
            // None sorts last — direction-agnostic per AI-06.
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.id().0.cmp(&b.id().0),
        }
    }

    /// Multi-window aggregate (AI-09 strict propagation). Returns
    /// `Some(blend)` only when EVERY window in `views` has `Some`
    /// from `read()`. ANY `None` (missing data, era gate, trade
    /// guard, MIN_GP floor) propagates as `None` — no silent zeros.
    ///
    /// Blend semantics by unit (L.2.2 v1):
    /// - **Count / Seconds**: integer sum.
    /// - **Pct / Per60 / Rate / Inverted**: GP-weighted average.
    ///   Falls back to simple mean when total GP is 0 (e.g. all
    ///   views in a goalie career-window with `gp` defaulting low).
    ///
    /// **Future work** (FORGE-R follow-up): TOI-weighting for Per60
    /// rates is the more accurate blend when TOI varies meaningfully
    /// across windows (PP-only specialists, etc.). Lands when L.6
    /// brings reliable PP-TOI / SH-TOI denominators online; for L.2
    /// the GP weighting matches the spec's strict-propagation contract
    /// without lying about a richer blend that isn't implemented.
    pub fn aggregate_read(self, views: &[PlayerView<'_>]) -> Option<f64> {
        if views.is_empty() {
            return None;
        }
        // Read all — strict propagation: any None aborts.
        let vals: Option<Vec<f64>> = views.iter().map(|v| self.read(v)).collect();
        let vals = vals?;

        match self.unit() {
            // Sum semantics.
            StatUnit::Count | StatUnit::Seconds => Some(vals.iter().sum()),

            // Weighted blend. Use GP weights when applicable.
            StatUnit::Pct | StatUnit::Per60 | StatUnit::Rate | StatUnit::Inverted => {
                // GP-weighted blend.
                let gps: Vec<f64> = views.iter().map(|v| v.gp() as f64).collect();
                let total_gp: f64 = gps.iter().sum();
                if total_gp <= 0.0 {
                    // No weights — fall back to simple mean.
                    return Some(vals.iter().sum::<f64>() / vals.len() as f64);
                }
                let weighted: f64 = vals.iter().zip(gps.iter()).map(|(v, w)| v * w).sum();
                Some(weighted / total_gp)
            }
        }
    }
}

/// Per-60 helper — divides a `Some(count)` by total TOI seconds
/// (multiplied to per-60 rate). Returns `None` when:
///   - the count is `None` (e.g. realtime data absent for pre-2005),
///   - total TOI is unavailable,
///   - total TOI is below the 300s soft floor (PACE-F1 — statistical
///     noise from microscopic ice time).
fn per_60(view: &PlayerView<'_>, count: Option<u32>) -> Option<f64> {
    let toi_sec = view
        .stats
        .time_on_ice
        .as_ref()
        .map(|t| t.time_on_ice_sec)
        .or_else(|| {
            view.stats
                .totals
                .toi_per_game_sec
                .map(|per_g| per_g.saturating_mul(view.gp()))
        })?;
    if toi_sec < 300 {
        return None;
    }
    let n = count? as f64;
    Some(n * 3600.0 / toi_sec as f64)
}

// ─── L.2.4 — Filter grammar primitives ──────────────────────────────────────

/// Comparison op for `StatFilter`. Min = `>=`, Max = `<=`, Equals = `==` / `=`.
/// `Equals` tolerance is unit-aware (per L2-B1 — replacing `f64::EPSILON`):
/// Count/Seconds use exact integer compare; Pct/Rate/Inverted use 1e-6;
/// Per60 uses 1e-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterOp {
    Min,
    Max,
    Equals,
}

/// Filter-grammar parse failure (FORGE-R2-B4 / EDGE-R2). Seven variants —
/// every malformed-input class in II-05 / II-06 maps to exactly one variant.
/// `Display` impl produces the user-facing error message; the CLI front-end
/// renders `eprintln!("error: {}", e)` and exits non-zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterParseError {
    /// `""` — empty `--filter` value.
    EmptyInput,
    /// `">=10"`, `"  >= 10"` — whitespace-only stat key.
    EmptyStatKey,
    /// `"hits10"`, `"hits 10"` — no op token.
    MissingOp { input: String },
    /// `"hits>=>5"`, `"hits===5"` — more than one op token.
    MultipleOps { input: String },
    /// `"hots-per-60>=1"` — parses but `from_cli_key` returns `None`.
    UnknownStat { key: String },
    /// `"hits>=abc"`, `"hits>=1,5"` (locale-comma decimals) — `f64::from_str` fails.
    BadNumber { token: String },
    /// `"hits>=NaN"`, `"hits>=inf"`, `"hits>=-inf"` — parsed but not finite.
    NotFinite { token: String },
    /// Filter.OR — open paren without a matching close, e.g. `"(g>=30 AND a>=30"`.
    UnclosedParen,
    /// Filter.OR — close paren without a matching open, e.g. `"g>=30)"`.
    UnexpectedRParen,
    /// Filter.OR — expression ends mid-parse (e.g. `"g>=30 AND"` or `"NOT"`).
    UnexpectedEnd,
    /// Filter.OR — token sequence the parser can't make sense of.
    UnexpectedToken { token: String },
}

impl std::fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput =>
                write!(f, "filter is empty — expected `<stat-key><op><value>` (e.g. \"goals>=30\")"),
            Self::EmptyStatKey =>
                write!(f, "filter stat-key is empty — expected `<stat-key><op><value>` (e.g. \"goals>=30\")"),
            Self::MissingOp { input } =>
                write!(f, "filter {input:?} has no op — expected one of `>=`, `<=`, `==`, `=`"),
            Self::MultipleOps { input } => {
                // KEEL D2 (L.5b post-fix) — typo hint for `=>` / `=<`.
                // Both transpositions classify as MultipleOps in the
                // current parser (the `=` and `>` each detect as ops).
                // The user's mental model has `=` first because it's
                // spelled "is greater than or equal"; without this
                // hint the generic "multiple ops" message doesn't
                // help them spot the transposition.
                if input.contains("=>") {
                    write!(f, "filter {input:?} has multiple ops — did you mean `>=`? \
                              (got `=>`; the equals-sign comes second)")
                } else if input.contains("=<") {
                    write!(f, "filter {input:?} has multiple ops — did you mean `<=`? \
                              (got `=<`; the equals-sign comes second)")
                } else {
                    write!(f, "filter {input:?} has multiple ops — expected exactly \
                              one of `>=`, `<=`, `==`, `=`")
                }
            }
            Self::UnknownStat { key } =>
                write!(f, "unknown stat key {key:?} — see `--help` or use one of the catalog cli_keys"),
            Self::BadNumber { token } =>
                write!(f, "filter value {token:?} is not a valid number (locale-comma `,` is not accepted; use `.`)"),
            Self::NotFinite { token } =>
                write!(f, "filter value {token:?} is not finite (NaN/inf rejected)"),
            Self::UnclosedParen =>
                write!(f, "filter expression has an unclosed `(` — every `(` needs a matching `)`"),
            Self::UnexpectedRParen =>
                write!(f, "filter expression has an unexpected `)` — no matching `(` opened"),
            Self::UnexpectedEnd =>
                write!(f, "filter expression ends mid-grammar (e.g. `g>=30 AND` with nothing after AND)"),
            Self::UnexpectedToken { token } =>
                write!(f, "unexpected token {token:?} in filter expression"),
        }
    }
}

impl std::error::Error for FilterParseError {}

impl FilterParseError {
    /// Structured hint for the front-end. Extracted from the variant
    /// shape (not by parsing the `Display` string) so CLI and web
    /// error paths can both surface the same hint without text-search.
    ///
    /// Phase King Clancy King.1.x patch (post-review): edge flagged
    /// the original "hint lives only in `Display`" pattern as fragile
    /// — the web `From<FilterParseError> for WebError` bridge would
    /// have to substring-match the rendered message to extract the
    /// typo hint. This accessor surfaces the hint directly.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::MultipleOps { input } => {
                // The typo case is `=>` / `=<` where the user has the
                // equals-sign first. If the input ALSO contains the
                // canonical `>=` / `<=` form, this isn't a typo —
                // they typed an actual multi-op (e.g. `g>=>50`).
                let has_typo_ge = input.contains("=>") && !input.contains(">=");
                let has_typo_le = input.contains("=<") && !input.contains("<=");
                if has_typo_ge {
                    Some("did you mean `>=`? The equals sign comes second.")
                } else if has_typo_le {
                    Some("did you mean `<=`? The equals sign comes second.")
                } else {
                    None
                }
            }
            Self::EmptyInput | Self::EmptyStatKey => {
                Some("expected `<stat-key><op><value>`, e.g. \"goals>=30\"")
            }
            Self::MissingOp { .. } => Some("the operator must be one of `>=`, `<=`, `==`, `=`"),
            Self::UnknownStat { .. } => {
                Some("see `icelines docs` or `--help` for the StatId catalog cli_keys")
            }
            Self::BadNumber { .. } => Some("locale-comma `,` is not accepted; use `.`"),
            Self::NotFinite { .. } => Some("NaN and infinity are rejected"),
            Self::UnclosedParen => Some("every `(` needs a matching `)`"),
            Self::UnexpectedRParen => Some("no matching `(` was opened"),
            Self::UnexpectedEnd => {
                Some("the expression ended mid-grammar (e.g. `g>=30 AND` with nothing after)")
            }
            Self::UnexpectedToken { .. } => None,
        }
    }
}

#[cfg(test)]
mod filter_parse_error_hint_tests {
    use super::*;

    /// l0_filter_parse_error_hint_arrow_typo
    /// — King.1.x patch fence: the `=>` typo hint must be reachable
    ///   structurally, not by string-grepping `Display`. CLI and web
    ///   error paths share this surface.
    #[test]
    fn l0_filter_parse_error_hint_arrow_typo() {
        let err = FilterParseError::MultipleOps {
            input: "g=>50".into(),
        };
        assert!(err.hint().unwrap().contains(">="));

        let err = FilterParseError::MultipleOps {
            input: "g=<50".into(),
        };
        assert!(err.hint().unwrap().contains("<="));

        // Multiple ops without a typo pattern → no hint
        let err = FilterParseError::MultipleOps {
            input: "g>=>50".into(),
        };
        assert!(err.hint().is_none());
    }

    /// l0_filter_parse_error_hint_unknown_stat_points_at_catalog
    #[test]
    fn l0_filter_parse_error_hint_unknown_stat_points_at_catalog() {
        let err = FilterParseError::UnknownStat { key: "hots".into() };
        assert!(err.hint().unwrap().contains("catalog"));
    }
}

/// One stat-vs-value filter. Constructed only via `StatFilter::new`
/// (the finite-value gate) or `parse_filter` (which routes through
/// `new`). Downstream code can assume `value.is_finite()` always.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatFilter {
    pub stat: StatId,
    pub op: FilterOp,
    pub value: f64,
}

impl StatFilter {
    /// Construct a `StatFilter`. Rejects NaN / infinity at the gate
    /// (II-05 / EDGE-R2). The CLI parser routes through this; the TUI
    /// numeric-input field validates before calling.
    pub fn new(stat: StatId, op: FilterOp, value: f64) -> Result<Self, FilterParseError> {
        if !value.is_finite() {
            return Err(FilterParseError::NotFinite {
                token: value.to_string(),
            });
        }
        Ok(Self { stat, op, value })
    }
}

/// Parse a filter expression `<stat-key><op><value>` per II-05 grammar.
///
/// **Op tokens** in priority order: `>=`, `<=`, `==`, `=`. Whitespace
/// allowed around the op AND surrounding the whole input.
///
/// **Trigger inputs** for each error variant:
/// - `EmptyInput`: empty/whitespace-only string.
/// - `EmptyStatKey`: stat-key portion empty after trim.
/// - `MissingOp`: no recognized op token.
/// - `MultipleOps`: more than one op token.
/// - `UnknownStat`: stat-key not in `StatId::from_cli_key`.
/// - `BadNumber`: number portion fails `f64::from_str` (includes
///   locale comma `1,5`, alphabetic, empty).
/// - `NotFinite`: parses to NaN / +inf / -inf.
pub fn parse_filter(input: &str) -> Result<StatFilter, FilterParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(FilterParseError::EmptyInput);
    }

    // Find first op match. Order matters: `>=` / `<=` / `==` before `=`
    // (single-char `=` is a substring of `==`).
    const OPS: &[(&str, FilterOp)] = &[
        (">=", FilterOp::Min),
        ("<=", FilterOp::Max),
        ("==", FilterOp::Equals),
        ("=", FilterOp::Equals),
    ];

    let (op, op_pos, op_len) = {
        let mut best: Option<(FilterOp, usize, usize)> = None;
        for (token, op) in OPS {
            if let Some(pos) = trimmed.find(token) {
                match best {
                    None => best = Some((*op, pos, token.len())),
                    Some((_, prev_pos, prev_len)) => {
                        // Prefer the EARLIEST op; on tie, prefer the
                        // LONGER op (so `>=` wins over `=` at the same
                        // position). This routes `hits>=10` to `>=`,
                        // not `=`.
                        if pos < prev_pos || (pos == prev_pos && token.len() > prev_len) {
                            best = Some((*op, pos, token.len()));
                        }
                    }
                }
            }
        }
        match best {
            Some(b) => b,
            None => {
                return Err(FilterParseError::MissingOp {
                    input: trimmed.to_owned(),
                })
            }
        }
    };

    let key_part = &trimmed[..op_pos];
    let value_part = &trimmed[op_pos + op_len..];

    // MultipleOps: any of `=`, `>`, `<` in the value part means a
    // second op token leaked through. Catches `hits>=>5`, `hits===5`,
    // `hits=5=`.
    if value_part.contains('=') || value_part.contains('>') || value_part.contains('<') {
        return Err(FilterParseError::MultipleOps {
            input: trimmed.to_owned(),
        });
    }
    // Same check on the key part (defensive — splitting at first op
    // means key_part shouldn't normally contain ops, but a leading op
    // could leak; e.g. ">=hits>=5" would split as ""-">="-"hits>=5").
    if key_part.contains('=') || key_part.contains('>') || key_part.contains('<') {
        return Err(FilterParseError::MultipleOps {
            input: trimmed.to_owned(),
        });
    }

    let stat_key = key_part.trim();
    if stat_key.is_empty() {
        return Err(FilterParseError::EmptyStatKey);
    }

    let value_str = value_part.trim();
    if value_str.is_empty() {
        return Err(FilterParseError::BadNumber {
            token: value_str.to_owned(),
        });
    }

    // f64::from_str rejects locale comma `1,5` and bare alphabetic.
    let value: f64 = value_str.parse().map_err(|_| FilterParseError::BadNumber {
        token: value_str.to_owned(),
    })?;

    if !value.is_finite() {
        return Err(FilterParseError::NotFinite {
            token: value_str.to_owned(),
        });
    }

    let stat = StatId::from_cli_key(stat_key).ok_or_else(|| FilterParseError::UnknownStat {
        key: stat_key.to_owned(),
    })?;

    StatFilter::new(stat, op, value)
}

// ── Phase Foster.5 — windowed filter atoms ──────────────────────────────────

/// Phase Foster.5 — a stat filter scoped to a specific window
/// (`g.week>=10`). When `window` is `None`, the caller binds it to
/// the active CLI timeframe at apply time (defaulting to season).
/// Mirrors `StatFilter` shape so the existing apply-views logic can
/// AND it with the other buckets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowedAtom {
    pub stat: StatId,
    pub window: Option<crate::timeframe::Timeframe>,
    pub op: FilterOp,
    pub value: f64,
}

impl WindowedAtom {
    /// Parse `<stat-key>[.<window>]<op><value>`. The optional
    /// `.window` segment is one of `season` / `week` / `month` /
    /// `day`; absent → `None` (caller resolves to the active CLI
    /// timeframe at apply time).
    pub fn parse(input: &str) -> Result<Self, FilterParseError> {
        // Reuse `parse_filter` to handle op + value parsing; the
        // window suffix attaches to the stat key.
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(FilterParseError::EmptyInput);
        }
        // Find the op position via the same priority order as
        // `parse_filter` so we can split off the key portion.
        const OPS: &[(&str, FilterOp)] = &[
            (">=", FilterOp::Min),
            ("<=", FilterOp::Max),
            ("==", FilterOp::Equals),
            ("=", FilterOp::Equals),
        ];
        let (op, op_pos, op_len) = {
            let mut best: Option<(FilterOp, usize, usize)> = None;
            for (token, op) in OPS {
                if let Some(pos) = trimmed.find(token) {
                    match best {
                        None => best = Some((*op, pos, token.len())),
                        Some((_, prev_pos, prev_len)) => {
                            if pos < prev_pos
                                || (pos == prev_pos && token.len() > prev_len)
                            {
                                best = Some((*op, pos, token.len()));
                            }
                        }
                    }
                }
            }
            best.ok_or_else(|| FilterParseError::MissingOp {
                input: trimmed.to_owned(),
            })?
        };
        let key_part = trimmed[..op_pos].trim();
        let value_part = trimmed[op_pos + op_len..].trim();
        if value_part.is_empty() {
            return Err(FilterParseError::BadNumber {
                token: value_part.to_owned(),
            });
        }
        if value_part.contains('=')
            || value_part.contains('>')
            || value_part.contains('<')
        {
            return Err(FilterParseError::MultipleOps {
                input: trimmed.to_owned(),
            });
        }
        let value: f64 =
            value_part
                .parse()
                .map_err(|_| FilterParseError::BadNumber {
                    token: value_part.to_owned(),
                })?;
        if !value.is_finite() {
            return Err(FilterParseError::NotFinite {
                token: value_part.to_owned(),
            });
        }

        // Split key on the LAST `.` — `points-per-game.week` has the
        // window after the final dot; `pp-pct.season` likewise. The
        // suffix is a window keyword if it parses as one.
        let (stat_key, window) = match key_part.rsplit_once('.') {
            Some((stat, suffix)) => match parse_window_keyword(suffix) {
                Some(w) => (stat.trim(), Some(w)),
                None => (key_part, None),
            },
            None => (key_part, None),
        };
        if stat_key.is_empty() {
            return Err(FilterParseError::EmptyStatKey);
        }
        let stat = StatId::from_cli_key(stat_key)
            .ok_or_else(|| FilterParseError::UnknownStat {
                key: stat_key.to_owned(),
            })?;
        Ok(Self {
            stat,
            window,
            op,
            value,
        })
    }

    /// Resolve `window` against the active CLI timeframe. Returns
    /// the explicit window if set; otherwise `default_window`
    /// (typically Season).
    pub fn resolved_window(&self, default_window: crate::timeframe::Timeframe) -> crate::timeframe::Timeframe {
        self.window.unwrap_or(default_window)
    }
}

fn parse_window_keyword(s: &str) -> Option<crate::timeframe::Timeframe> {
    use crate::timeframe::Timeframe;
    match s.trim() {
        "day" => Some(Timeframe::Day),
        "week" => Some(Timeframe::Week),
        "month" => Some(Timeframe::Month),
        "season" => Some(Timeframe::Season),
        _ => None,
    }
}

// ── Filter.OR — boolean filter expressions (AND / OR / NOT / parens) ──────────

/// A boolean expression over `StatFilter` atoms. Built by
/// `parse_filter_expr` from user input; evaluated against a
/// `PlayerView` via `FilterExpr::matches`. Atoms are the existing
/// single-comparison filters (e.g. `g>=50`); compound expressions
/// combine them with AND / OR / NOT / parens.
///
/// Single-atom expressions (e.g. `parse_filter_expr("g>=50")`) round-
/// trip to `FilterExpr::Atom(StatFilter)` so existing filter inputs
/// stay backward-compatible.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    Atom(StatFilter),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
}

impl FilterExpr {
    /// True iff this expression accepts the given view. Atoms use the
    /// same per-row applicability + Equals-tolerance logic as
    /// `PlayerFilter::matches_stat_filters`. Compound expressions
    /// short-circuit normally.
    ///
    /// **Missing data semantic** matches the legacy
    /// `matches_stat_filters` rule: when `stat.read(view)` returns
    /// `None`, the atom evaluates to `false` (not `None`). This means
    /// `NOT (hits>=200)` accepts pre-2010 rows where hits was never
    /// tracked — they fail `hits>=200` and the NOT flips that to true.
    /// If the user actually wants "tracked AND below threshold,"
    /// they should add a guard like `gp>=1` in the AND chain.
    pub fn matches(&self, v: &crate::stats_repository::PlayerView<'_>) -> bool {
        match self {
            FilterExpr::Atom(f) => atom_matches(f, v),
            FilterExpr::And(a, b) => a.matches(v) && b.matches(v),
            FilterExpr::Or(a, b) => a.matches(v) || b.matches(v),
            FilterExpr::Not(inner) => !inner.matches(v),
        }
    }

    /// True iff this expression is a single Atom (no AND / OR / NOT).
    /// Used by the CLI to route single-atom filters through the
    /// legacy `stat_filters` path (which gets normalization for free).
    pub fn is_atom(&self) -> bool {
        matches!(self, FilterExpr::Atom(_))
    }

    /// Extract the atom if this is an `Atom` variant. `None` for
    /// compound expressions.
    pub fn as_atom(&self) -> Option<&StatFilter> {
        match self {
            FilterExpr::Atom(f) => Some(f),
            _ => None,
        }
    }
}

/// Single-atom evaluator — same logic as
/// `PlayerFilter::matches_stat_filters` but on one atom. Lifted into
/// a free function so `FilterExpr::matches` doesn't need a borrow on
/// PlayerFilter.
fn atom_matches(f: &StatFilter, v: &crate::stats_repository::PlayerView<'_>) -> bool {
    if !f.stat.applies_to(v.position(), v.is_goalie()) {
        // DI-08 — non-applicable filters silently pass for the atom.
        // (matches_stat_filters does `continue`; here we return true
        // so the atom is a no-op when AND-chained.)
        return true;
    }
    let actual = match f.stat.read(v) {
        Some(x) => x,
        None => return false,
    };
    match f.op {
        FilterOp::Min => actual >= f.value,
        FilterOp::Max => actual <= f.value,
        FilterOp::Equals => match f.stat.unit() {
            StatUnit::Count | StatUnit::Seconds => (actual - f.value).abs() < 0.5,
            StatUnit::Per60 => (actual - f.value).abs() < 1e-3,
            StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => (actual - f.value).abs() < 1e-6,
        },
    }
}

/// Parse a boolean filter expression with AND / OR / NOT / parens.
///
/// Grammar (precedence: NOT > AND > OR; standard left-associativity):
/// ```text
///   expr    := or_expr
///   or_expr := and_expr ( OR  and_expr )*
///   and_expr:= unary    ( AND unary    )*
///   unary   := NOT unary | primary
///   primary := '(' expr ')' | atom
///   atom    := <key OP value>  (delegated to parse_filter)
/// ```
///
/// Keywords `AND`, `OR`, `NOT` are case-insensitive and matched only
/// at word boundaries (whitespace / `(` / `)` / start / end), so they
/// don't collide with stat keys. A bare atom like `"g>=50"` parses as
/// `FilterExpr::Atom(StatFilter)` — backward-compatible with the
/// existing single-filter input shape.
///
/// Examples:
/// - `"g>=50"` → Atom
/// - `"g>=50 OR a>=50"` → Or(Atom, Atom)
/// - `"(g>=30 AND a>=30) OR p>=80"` → Or(And(Atom, Atom), Atom)
/// - `"NOT pim>=100"` → Not(Atom)
pub fn parse_filter_expr(input: &str) -> Result<FilterExpr, FilterParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(FilterParseError::EmptyInput);
    }
    let tokens = tokenize_filter_expr(trimmed);
    if tokens.is_empty() {
        return Err(FilterParseError::EmptyInput);
    }
    let mut p = ExprParser {
        tokens: &tokens,
        pos: 0,
    };
    let expr = p.parse_or()?;
    if p.pos < p.tokens.len() {
        let leftover = match &p.tokens[p.pos] {
            ExprToken::Atom(s) => s.clone(),
            ExprToken::LParen => "(".to_owned(),
            ExprToken::RParen => ")".to_owned(),
            ExprToken::And => "AND".to_owned(),
            ExprToken::Or => "OR".to_owned(),
            ExprToken::Not => "NOT".to_owned(),
        };
        return Err(FilterParseError::UnexpectedToken { token: leftover });
    }
    Ok(expr)
}

#[derive(Debug, Clone)]
enum ExprToken {
    LParen,
    RParen,
    And,
    Or,
    Not,
    /// A `key OP value` fragment (or a partial fragment that
    /// `parse_filter` will validate). Whitespace inside is preserved.
    Atom(String),
}

/// Walk the input character-by-character, peeling off parens and
/// keywords (AND/OR/NOT) at word boundaries. Everything else
/// accumulates into the next `Atom` token.
fn tokenize_filter_expr(input: &str) -> Vec<ExprToken> {
    let mut tokens = Vec::new();
    let mut atom = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    let flush = |atom: &mut String, tokens: &mut Vec<ExprToken>| {
        let trimmed = atom.trim();
        if !trimmed.is_empty() {
            tokens.push(ExprToken::Atom(trimmed.to_owned()));
        }
        atom.clear();
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            '(' => {
                flush(&mut atom, &mut tokens);
                tokens.push(ExprToken::LParen);
                i += 1;
            }
            ')' => {
                flush(&mut atom, &mut tokens);
                tokens.push(ExprToken::RParen);
                i += 1;
            }
            _ => {
                let prev_is_boundary = i == 0
                    || chars[i - 1].is_whitespace()
                    || chars[i - 1] == '('
                    || chars[i - 1] == ')';
                if prev_is_boundary {
                    if let Some((kw, kw_len)) = match_keyword_at(&chars, i) {
                        flush(&mut atom, &mut tokens);
                        tokens.push(kw);
                        i += kw_len;
                        continue;
                    }
                }
                atom.push(c);
                i += 1;
            }
        }
    }
    flush(&mut atom, &mut tokens);
    tokens
}

/// If position `i` in `chars` starts a keyword (`AND` / `OR` / `NOT`)
/// followed by a word boundary (whitespace / paren / EOI), return the
/// token and length. Else `None`. Case-insensitive.
fn match_keyword_at(chars: &[char], i: usize) -> Option<(ExprToken, usize)> {
    fn match_at(chars: &[char], i: usize, kw: &str) -> bool {
        if i + kw.len() > chars.len() {
            return false;
        }
        for (j, kc) in kw.chars().enumerate() {
            if chars[i + j].to_ascii_uppercase() != kc {
                return false;
            }
        }
        true
    }
    fn next_is_boundary(chars: &[char], i: usize) -> bool {
        match chars.get(i) {
            None => true,
            Some(&c) => c.is_whitespace() || c == '(' || c == ')',
        }
    }

    if match_at(chars, i, "AND") && next_is_boundary(chars, i + 3) {
        return Some((ExprToken::And, 3));
    }
    if match_at(chars, i, "NOT") && next_is_boundary(chars, i + 3) {
        return Some((ExprToken::Not, 3));
    }
    if match_at(chars, i, "OR") && next_is_boundary(chars, i + 2) {
        return Some((ExprToken::Or, 2));
    }
    None
}

struct ExprParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&ExprToken> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&ExprToken> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_or(&mut self) -> Result<FilterExpr, FilterParseError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(ExprToken::Or)) {
            self.advance();
            let right = self.parse_and()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, FilterParseError> {
        let mut left = self.parse_unary()?;
        while matches!(self.peek(), Some(ExprToken::And)) {
            self.advance();
            let right = self.parse_unary()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<FilterExpr, FilterParseError> {
        if matches!(self.peek(), Some(ExprToken::Not)) {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(FilterExpr::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, FilterParseError> {
        match self.peek() {
            Some(ExprToken::LParen) => {
                self.advance();
                let expr = self.parse_or()?;
                match self.peek() {
                    Some(ExprToken::RParen) => {
                        self.advance();
                        Ok(expr)
                    }
                    _ => Err(FilterParseError::UnclosedParen),
                }
            }
            Some(ExprToken::RParen) => Err(FilterParseError::UnexpectedRParen),
            Some(ExprToken::Atom(_)) => {
                let s = match self.advance() {
                    Some(ExprToken::Atom(s)) => s.clone(),
                    _ => unreachable!(),
                };
                let atom = parse_filter(&s)?;
                Ok(FilterExpr::Atom(atom))
            }
            None => Err(FilterParseError::UnexpectedEnd),
            Some(t) => Err(FilterParseError::UnexpectedToken {
                token: format!("{t:?}"),
            }),
        }
    }
}

/// One raw NHL API row from a Tier-1 endpoint. Implementors expose
/// the `seasonId` field for the per-endpoint fence (DI-29).
///
/// `load_report_with_fallback<R>` (in `icelines-fetch::stats_loader`)
/// requires `R: Tier1Row + DeserializeOwned`. For each row in a loaded
/// file, the loader calls `row.season_id()` and fails
/// `LoadError::SeasonIdMismatch` if any row's id disagrees with the
/// requested season. `None` is the bundled-trust path (pre-Hart.6
/// fixtures didn't carry seasonId on every row); the loader skips
/// the fence for those rows.
pub trait Tier1Row {
    /// `seasonId` from the API row if present. `None` for hand-edited
    /// or pre-Hart.6 fixtures.
    fn season_id(&self) -> Option<u32>;
}

/// One report tier — Tier 1 lives in typed substructs on `SeasonStats`
/// (loaded into the typed-window LRU); Tier 2 lives in `extra_reports`
/// on `StatsRepository` (runtime-only `BTreeMap`, DI-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Loaded eagerly with the typed window. Persisted to disk.
    Tier1,
    /// Fetched on demand. Lives in `extra_reports` (BTreeMap, DI-12
    /// cascade-evicted with the window). Never persisted.
    Tier2,
}

/// One per NHL stats-API endpoint we exercise. Tier-1 variants populate
/// typed substructs on `SeasonStats`; Tier-2 variants live in
/// `extra_reports` (runtime-only). Variant order is the canonical
/// `StatId::all()` enumeration order — the spec asserts iteration is
/// deterministic (AI-05).
///
/// `serde` shape is camelCase — matches the JSON nesting key in
/// `ChunkedManifest::reports` (e.g. `"skaterTimeOnIce"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReportKind {
    // ── Tier 1 — typed substructs on `SeasonStats` ────────────────────
    /// `/skater/summary` → `SeasonStats.totals` (existing, bundled).
    SkaterSummary,
    /// `/skater/bios` → `PlayerIdentity` (existing, bundled).
    SkaterBios,
    /// `/skater/realtime` → `SeasonStats.realtime` (existing field;
    /// new bundling target in Lindsay).
    SkaterRealtime,
    /// `/skater/timeonice` → `SeasonStats.time_on_ice` (Lindsay-NEW).
    SkaterTimeOnIce,
    /// `/skater/goalsForAgainst` → `SeasonStats.goals_for_against`
    /// (Lindsay-NEW).
    SkaterGoalsForAgainst,
    /// `/goalie/summary` → `SeasonStats.goalie` (existing, bundled).
    GoalieSummary,
    /// `/goalie/bios` → `SeasonStats.goalie_bios` (Lindsay-NEW;
    /// previously skater/bios was misused as goalie identity source).
    GoalieBios,
    /// `/goalie/advanced` → `SeasonStats.goalie_advanced` (Lindsay-NEW).
    GoalieAdvanced,
    /// `/goalie/savesByStrength` → `SeasonStats.goalie_saves_by_strength`
    /// (Lindsay-NEW).
    GoalieSavesByStrength,

    // ── Tier 2 — runtime-only blobs in `extra_reports` ────────────────
    SkaterPuckPossessions,
    SkaterScoringRates,
    SkaterSummaryShooting,
    SkaterPowerPlay,
    SkaterPenaltyKill,
    SkaterPenalties,
    SkaterFaceoffWins,
    SkaterFaceoffPercentages,
    SkaterShotType,
    SkaterScoringPerGame,
    GoalieStartedVsRelieved,
    GoalieDaysRest,
    GoaliePenaltyShots,
    GoalieShootout,
}

impl ReportKind {
    /// Path component appended to the stats base URL — e.g.
    /// `"skater/timeonice"`. Combined with the season-aware cayenneExp
    /// query in the fetch client. The probe artifact at
    /// `data/api-probe-2026-05-02.txt` records the exact verified URLs.
    pub fn url_path(self) -> &'static str {
        match self {
            // Tier 1
            Self::SkaterSummary => "skater/summary",
            Self::SkaterBios => "skater/bios",
            Self::SkaterRealtime => "skater/realtime",
            Self::SkaterTimeOnIce => "skater/timeonice",
            Self::SkaterGoalsForAgainst => "skater/goalsForAgainst",
            Self::GoalieSummary => "goalie/summary",
            Self::GoalieBios => "goalie/bios",
            Self::GoalieAdvanced => "goalie/advanced",
            Self::GoalieSavesByStrength => "goalie/savesByStrength",
            // Tier 2
            Self::SkaterPuckPossessions => "skater/puckPossessions",
            Self::SkaterScoringRates => "skater/scoringRates",
            Self::SkaterSummaryShooting => "skater/summaryshooting",
            Self::SkaterPowerPlay => "skater/powerplay",
            Self::SkaterPenaltyKill => "skater/penaltykill",
            Self::SkaterPenalties => "skater/penalties",
            Self::SkaterFaceoffWins => "skater/faceoffwins",
            Self::SkaterFaceoffPercentages => "skater/faceoffpercentages",
            Self::SkaterShotType => "skater/shottype",
            Self::SkaterScoringPerGame => "skater/scoringpergame",
            Self::GoalieStartedVsRelieved => "goalie/startedVsRelieved",
            Self::GoalieDaysRest => "goalie/daysrest",
            Self::GoaliePenaltyShots => "goalie/penaltyShots",
            Self::GoalieShootout => "goalie/shootout",
        }
    }

    /// Whether this report has data for the given season type. Currently
    /// every documented-working endpoint supports both regular and
    /// playoff (verified 2026-05-02 — see `data/api-probe-2026-05-02.txt`
    /// §"Playoff sanity"). Tier-2 playoff support was NOT exhaustively
    /// re-probed in the artifact; this method conservatively returns
    /// `true` for them — the rate-limit policy + 500-fallback path in
    /// L.1.4 surfaces "no data this season-type" cleanly to the user
    /// rather than fence-tripping at L.1 entry.
    pub fn supports(self, _season_type: SeasonType) -> bool {
        true
    }

    /// Whether the NHL stats API actually serves this endpoint. Eight
    /// endpoints documented in the probe artifact return 500 (Skater:
    /// `advanced`; Goalie: `realtime`, `savePercentage`, `penaltykill`,
    /// `percentages`, `shottype`, `timeonice`, `goalsForAgainstByStrength`)
    /// — they are NOT enumerated as variants of `ReportKind` because we
    /// never call them. This method exists so a future maintainer who
    /// adds a kind variant doesn't accidentally include one of those
    /// without setting this to false.
    ///
    /// Lindsay L.1.6 (TAPE-R3 follow-up): the `fetch report` CLI gates
    /// on this method and refuses to dispatch known-broken endpoints
    /// instead of letting the L.1.5 retry policy burn 5 × 30s ≈ 2.5min.
    pub fn is_known_working(self) -> bool {
        // Every variant in `ReportKind::all()` is currently known-working
        // per the 2026-05-02 probe artifact. The method exists so a
        // future variant addition can flip this without changing the
        // enum surface — set the new arm to `false` until probed.
        true
    }

    /// Whether this report is loaded eagerly into typed-window LRU
    /// (Tier-1) or fetched on demand into `extra_reports` (Tier-2).
    pub fn tier(self) -> Tier {
        match self {
            Self::SkaterSummary
            | Self::SkaterBios
            | Self::SkaterRealtime
            | Self::SkaterTimeOnIce
            | Self::SkaterGoalsForAgainst
            | Self::GoalieSummary
            | Self::GoalieBios
            | Self::GoalieAdvanced
            | Self::GoalieSavesByStrength => Tier::Tier1,
            _ => Tier::Tier2,
        }
    }

    /// Every variant in declaration order. Callers iterating over the
    /// full report set MUST use this — don't roll your own slice.
    /// Determinism backs SI-03 (site rendering) and AI-05 (catalog
    /// iteration).
    pub fn all() -> &'static [ReportKind] {
        use ReportKind::*;
        &[
            // Tier 1
            SkaterSummary,
            SkaterBios,
            SkaterRealtime,
            SkaterTimeOnIce,
            SkaterGoalsForAgainst,
            GoalieSummary,
            GoalieBios,
            GoalieAdvanced,
            GoalieSavesByStrength,
            // Tier 2
            SkaterPuckPossessions,
            SkaterScoringRates,
            SkaterSummaryShooting,
            SkaterPowerPlay,
            SkaterPenaltyKill,
            SkaterPenalties,
            SkaterFaceoffWins,
            SkaterFaceoffPercentages,
            SkaterShotType,
            SkaterScoringPerGame,
            GoalieStartedVsRelieved,
            GoalieDaysRest,
            GoaliePenaltyShots,
            GoalieShootout,
        ]
    }
}

/// Where a Tier-1 report's deserialized rows merge onto `SeasonStats`.
/// Used by `load_report_with_fallback<T>` (L.1.4) to dispatch the
/// `Vec<Row>` into the right typed substruct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeTarget {
    /// `SeasonStats.totals` (and `team_stints`) — built by the existing
    /// `stats_loader` path. Kept as a variant for completeness; L.1.4
    /// does NOT reroute the existing summary loader through
    /// `load_report_with_fallback` (held for L.7 historical bundling).
    SkaterSummaryTotals,
    /// `PlayerIdentity` — built by the existing identity path. Same
    /// caveat as above.
    SkaterIdentity,
    /// `SeasonStats.realtime`.
    SkaterRealtime,
    /// `SeasonStats.time_on_ice`.
    SkaterTimeOnIce,
    /// `SeasonStats.goals_for_against`.
    SkaterGoalsForAgainst,
    /// `SeasonStats.goalie`.
    GoalieSummary,
    /// `SeasonStats.goalie_bios` — Lindsay switches goalie-identity
    /// source from skater/bios to here.
    GoalieBios,
    /// `SeasonStats.goalie_advanced`.
    GoalieAdvanced,
    /// `SeasonStats.goalie_saves_by_strength`.
    GoalieSavesByStrength,
}

/// The on-disk file and merge target for one Tier-1 report.
/// Used by `load_report_with_fallback<T>` (L.1.4) to read the right
/// per-window file and route the deserialized rows. The constant
/// `TIER1_REPORTS` enumerates every Tier-1 entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier1ReportFile {
    pub kind: ReportKind,
    /// Per-window filename under `<snapshot_dir>/<season>/<season_type>/`.
    /// Pinned by spec §"Tier-1 file format".
    pub filename: &'static str,
    pub merge_target: MergeTarget,
}

/// Per-(ReportKind) Tier-1 file dispatch table. Order matches
/// `ReportKind::all()` for the Tier-1 prefix. The order also defines
/// the load order in `StatsRepository::load_window` — earlier entries
/// land first, so dependent reports (e.g. `GoalieBios` that influences
/// goalie-identity merging) come before consumers.
pub const TIER1_REPORTS: &[Tier1ReportFile] = &[
    Tier1ReportFile {
        kind: ReportKind::SkaterSummary,
        filename: "summary.json",
        merge_target: MergeTarget::SkaterSummaryTotals,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterBios,
        filename: "bios.json",
        merge_target: MergeTarget::SkaterIdentity,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterRealtime,
        filename: "realtime.json",
        merge_target: MergeTarget::SkaterRealtime,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterTimeOnIce,
        filename: "timeonice.json",
        merge_target: MergeTarget::SkaterTimeOnIce,
    },
    Tier1ReportFile {
        kind: ReportKind::SkaterGoalsForAgainst,
        filename: "goalsForAgainst.json",
        merge_target: MergeTarget::SkaterGoalsForAgainst,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieSummary,
        filename: "goalie-summary.json",
        merge_target: MergeTarget::GoalieSummary,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieBios,
        filename: "goalie-bios.json",
        merge_target: MergeTarget::GoalieBios,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieAdvanced,
        filename: "goalie-advanced.json",
        merge_target: MergeTarget::GoalieAdvanced,
    },
    Tier1ReportFile {
        kind: ReportKind::GoalieSavesByStrength,
        filename: "goalie-savesByStrength.json",
        merge_target: MergeTarget::GoalieSavesByStrength,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase Reports — StatId::report_source mapping ────────────────────

    /// Every Tier-1 report kind that the Reports overlay can toggle has
    /// at least one `StatId` whose `report_source()` returns it. If the
    /// mapping ever drifts (e.g. SkaterRealtime is removed but no Hits
    /// re-categorization), this test fires.
    #[test]
    fn l0_reports_each_toggleable_report_owns_at_least_one_stat() {
        use ReportKind::*;
        for kind in [
            SkaterRealtime,
            SkaterTimeOnIce,
            SkaterGoalsForAgainst,
            GoalieAdvanced,
            GoalieSavesByStrength,
        ] {
            let count = StatId::all()
                .iter()
                .filter(|s| s.report_source() == Some(kind))
                .count();
            assert!(
                count > 0,
                "ReportKind::{kind:?} owns 0 StatIds — mapping drift in StatId::report_source"
            );
        }
    }

    /// Sanity-check the canonical mappings: Hits → SkaterRealtime,
    /// EvGoalsFor → SkaterGoalsForAgainst, EvSavePct → GoalieSavesByStrength,
    /// QualityStarts → GoalieAdvanced, PpToi → SkaterTimeOnIce.
    #[test]
    fn l0_reports_canonical_stat_to_report_mappings() {
        use ReportKind::*;
        assert_eq!(StatId::Hits.report_source(), Some(SkaterRealtime));
        assert_eq!(StatId::BlockedShots.report_source(), Some(SkaterRealtime));
        assert_eq!(StatId::PpToi.report_source(), Some(SkaterTimeOnIce));
        assert_eq!(
            StatId::EvGoalsFor.report_source(),
            Some(SkaterGoalsForAgainst)
        );
        assert_eq!(
            StatId::EvSavePct.report_source(),
            Some(GoalieSavesByStrength)
        );
        assert_eq!(StatId::QualityStarts.report_source(), Some(GoalieAdvanced));
    }

    /// Core stats (always available from summary or computed) return
    /// `None` — they're not gated behind any toggle.
    #[test]
    fn l0_reports_core_stats_have_no_report_source() {
        // Summary skater stats
        assert_eq!(StatId::Goals.report_source(), None);
        assert_eq!(StatId::Assists.report_source(), None);
        assert_eq!(StatId::Points.report_source(), None);
        assert_eq!(StatId::PlusMinus.report_source(), None);
        assert_eq!(StatId::Pim.report_source(), None);
        assert_eq!(StatId::FaceoffWinPct.report_source(), None);
        // Summary-derived TOI per game
        assert_eq!(StatId::TotalToiPerGame.report_source(), None);
        // Goalie summary
        assert_eq!(StatId::Wins.report_source(), None);
        assert_eq!(StatId::SavePct.report_source(), None);
        assert_eq!(StatId::Gaa.report_source(), None);
        // Derived
        assert_eq!(StatId::Pace82.report_source(), None);
        assert_eq!(StatId::PointsPerGame.report_source(), None);
    }

    /// Possession + xG stats are Tier-2 / MoneyPuck — not gated by the
    /// overlay toggles (they're on-demand fetched, not bundled).
    #[test]
    fn l0_reports_tier2_and_moneypuck_stats_return_none() {
        assert_eq!(StatId::SatPct.report_source(), None);
        assert_eq!(StatId::IxG.report_source(), None);
        assert_eq!(StatId::OnIceXgFor.report_source(), None);
        assert_eq!(StatId::GoalieXgAgainst.report_source(), None);
        assert_eq!(StatId::GoalsSavedAboveExpected.report_source(), None);
    }

    /// Pin: `ReportKind::all().len()` matches the documented inventory.
    /// 9 Tier-1 + 14 Tier-2 = 23 working endpoints. Matches the spec
    /// §"Endpoint inventory" and the probe artifact.
    #[test]
    fn l0_lindsay_report_kind_count_matches_inventory() {
        assert_eq!(ReportKind::all().len(), 23);
        let tier1: Vec<_> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier1))
            .collect();
        let tier2: Vec<_> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier2))
            .collect();
        assert_eq!(tier1.len(), 9, "9 Tier-1 reports per spec inventory");
        assert_eq!(tier2.len(), 14, "14 Tier-2 reports per spec inventory");
    }

    /// Pin: serde shape is camelCase. JSON keys in
    /// `ChunkedManifest.reports` (L.1.2) and Tier-2 cache keys
    /// (L.1.4 / L.6) round-trip through this representation.
    #[test]
    fn l0_lindsay_report_kind_serde_camel_case() {
        assert_eq!(
            serde_json::to_string(&ReportKind::SkaterTimeOnIce).unwrap(),
            "\"skaterTimeOnIce\"",
        );
        assert_eq!(
            serde_json::to_string(&ReportKind::GoalieSavesByStrength).unwrap(),
            "\"goalieSavesByStrength\"",
        );
        let back: ReportKind = serde_json::from_str("\"skaterGoalsForAgainst\"").unwrap();
        assert_eq!(back, ReportKind::SkaterGoalsForAgainst);
    }

    /// Pin: every endpoint in the probe artifact has a matching `url_path`.
    /// If anyone reorders the variants or renames a path, every snapshot
    /// reading that report kind would break — this test catches the slip
    /// before it ships.
    #[test]
    fn l0_lindsay_url_path_matches_probe_inventory() {
        // Tier-1 sample.
        assert_eq!(ReportKind::SkaterSummary.url_path(), "skater/summary");
        assert_eq!(ReportKind::SkaterTimeOnIce.url_path(), "skater/timeonice");
        assert_eq!(
            ReportKind::SkaterGoalsForAgainst.url_path(),
            "skater/goalsForAgainst",
        );
        assert_eq!(ReportKind::GoalieBios.url_path(), "goalie/bios");
        assert_eq!(
            ReportKind::GoalieSavesByStrength.url_path(),
            "goalie/savesByStrength",
        );
        // Tier-2 sample.
        assert_eq!(
            ReportKind::SkaterPuckPossessions.url_path(),
            "skater/puckPossessions",
        );
        assert_eq!(
            ReportKind::GoalieStartedVsRelieved.url_path(),
            "goalie/startedVsRelieved",
        );
    }

    /// Pin: `supports(season_type)` returns `true` for both regular and
    /// playoff on every Tier-1 endpoint. Verified by the playoff sanity
    /// probe at `data/api-probe-2026-05-02.txt`.
    #[test]
    fn l0_lindsay_tier1_supports_both_season_types() {
        for kind in ReportKind::all() {
            if !matches!(kind.tier(), Tier::Tier1) {
                continue;
            }
            assert!(
                kind.supports(SeasonType::Regular),
                "{:?} must support Regular",
                kind,
            );
            assert!(
                kind.supports(SeasonType::Playoff),
                "{:?} must support Playoff",
                kind,
            );
        }
    }

    /// Pin: `TIER1_REPORTS` covers every Tier-1 `ReportKind`. If anyone
    /// adds a new Tier-1 variant without adding a `Tier1ReportFile`
    /// row, the loader (L.1.4) silently skips that report. This test
    /// catches the omission.
    #[test]
    fn l0_lindsay_tier1_reports_table_is_exhaustive() {
        let tier1_kinds: Vec<ReportKind> = ReportKind::all()
            .iter()
            .filter(|k| matches!(k.tier(), Tier::Tier1))
            .copied()
            .collect();
        let table_kinds: Vec<ReportKind> = TIER1_REPORTS.iter().map(|r| r.kind).collect();
        assert_eq!(
            tier1_kinds, table_kinds,
            "TIER1_REPORTS table must enumerate every Tier-1 ReportKind \
             in declaration order",
        );
    }

    /// Pin: filenames in `TIER1_REPORTS` are unique. Two reports
    /// pointing at the same file would silently overwrite each other
    /// at write time and load the wrong substruct at read time.
    #[test]
    fn l0_lindsay_tier1_reports_filenames_unique() {
        let mut seen = std::collections::HashSet::new();
        for r in TIER1_REPORTS {
            assert!(
                seen.insert(r.filename),
                "duplicate Tier-1 filename: {}",
                r.filename,
            );
        }
    }

    /// BENCH closeout #2: pin that every variant currently returns
    /// `is_known_working() == true`. The 23 enumerated variants ARE
    /// the working endpoints (verified 2026-05-02 — see
    /// `data/api-probe-2026-05-02.txt`). The 8 documented-broken
    /// endpoints are intentionally NOT enum variants, so no false
    /// can sneak in today. If a future maintainer adds a variant
    /// for a still-broken endpoint, this test catches it: they MUST
    /// either probe-and-fix the endpoint OR set the new arm's return
    /// to `false`. Either path forces explicit acknowledgement.
    #[test]
    fn l0_lindsay_every_report_kind_is_known_working() {
        for kind in ReportKind::all() {
            assert!(
                kind.is_known_working(),
                "ReportKind::{kind:?} must be known-working OR get a \
                 dedicated false-return arm in is_known_working() — \
                 see data/api-probe-2026-05-02.txt for the inventory"
            );
        }
    }

    // ─── L.2.1 — StatId / StatCategory / StatUnit tests ─────────────────────

    /// Pin total stat count. L.2.1 rationalized to 107; L.4.1 added
    /// `Games` (skater GP — KEEL carry-forward) → 108. Per-category
    /// breakdown:
    ///   Scoring 15 (was 14, +Games)
    ///   SpecialTeams 11
    ///   TwoWay 17
    ///   TimeOnIce 12
    ///   OnIceGoals 8
    ///   Possession 15
    ///   Goalie 23
    ///   Derived 7
    /// Total: 108.
    #[test]
    fn l0_lindsay_stat_id_total_count() {
        assert_eq!(
            StatId::all().len(),
            108,
            "catalog total — drift from 108 means a variant added/removed \
             without updating StatId::all() OR a category test below"
        );
    }

    /// Per-category counts.
    /// 15 + 11 + 17 + 12 + 8 + 15 + 23 + 7 = 108.
    #[test]
    fn l0_lindsay_stat_id_per_category_counts() {
        let count_in = |c: StatCategory| -> usize {
            StatId::all().iter().filter(|s| s.category() == c).count()
        };
        assert_eq!(count_in(StatCategory::Identity), 0, "Identity");
        assert_eq!(
            count_in(StatCategory::Scoring),
            15,
            "Scoring (L.4.1 +Games)"
        );
        assert_eq!(
            count_in(StatCategory::SpecialTeams),
            11,
            "SpecialTeams (PpToiPerGame/ShToiPerGame relocated to TimeOnIce)"
        );
        assert_eq!(count_in(StatCategory::TwoWay), 17, "TwoWay");
        assert_eq!(count_in(StatCategory::TimeOnIce), 12, "TimeOnIce");
        assert_eq!(count_in(StatCategory::OnIceGoals), 8, "OnIceGoals");
        assert_eq!(count_in(StatCategory::Possession), 15, "Possession");
        assert_eq!(
            count_in(StatCategory::Goalie),
            23,
            "Goalie (19 base + 4 GSAx)"
        );
        assert_eq!(count_in(StatCategory::Derived), 7, "Derived");
    }

    /// Iteration determinism — `StatId::all()` returns variants in a
    /// stable order across runs (declaration order). UI dropdowns +
    /// site rendering rely on this (AI-05).
    #[test]
    fn l0_lindsay_stat_id_iteration_deterministic() {
        let first_run: Vec<StatId> = StatId::all().to_vec();
        let second_run: Vec<StatId> = StatId::all().to_vec();
        assert_eq!(first_run, second_run);
        // First four should always be the Scoring leaders (post-L.4.1
        // Games is first; Goals/Assists/Points follow).
        assert_eq!(first_run[0], StatId::Games);
        assert_eq!(first_run[1], StatId::Goals);
        assert_eq!(first_run[2], StatId::Assists);
        assert_eq!(first_run[3], StatId::Points);
    }

    /// Every variant has a unique `cli_key()`. Collisions would silently
    /// route `--filter "X>=1"` to whichever `cli_key()` arm was checked
    /// first by `from_cli_key`'s linear scan. Pin uniqueness here.
    /// Catches accidental copy-paste duplicates across the 107 arms.
    #[test]
    fn l0_lindsay_cli_keys_unique_across_catalog() {
        let mut seen = std::collections::HashSet::new();
        for sid in StatId::all() {
            let key = sid.cli_key();
            assert!(
                seen.insert(key),
                "duplicate cli_key {key:?} — collision blocks --filter / \
                 --sort routing for one of the colliding StatIds"
            );
        }
        // Sanity: 108 unique keys (L.4.1 +Games).
        assert_eq!(seen.len(), 108);
    }

    /// `from_cli_key` round-trip: every variant's `cli_key()` parses
    /// back to itself. Catches a typo'd arm in either direction.
    #[test]
    fn l0_lindsay_cli_key_round_trip() {
        for sid in StatId::all() {
            let key = sid.cli_key();
            let back = StatId::from_cli_key(key)
                .unwrap_or_else(|| panic!("from_cli_key({key:?}) returned None"));
            assert_eq!(*sid, back, "round-trip failed for cli_key {key:?}");
        }
    }

    /// `from_cli_key` returns `None` for unknown keys. CLI grammar
    /// (L.2.4) maps this to `FilterParseError::UnknownStat`. Gaps.1
    /// — parse is now case-insensitive (HITS resolves to Hits) and
    /// accepts short aliases (g→goals, gp→games).
    #[test]
    fn l0_lindsay_cli_key_unknown_returns_none() {
        assert_eq!(StatId::from_cli_key("bogus-stat"), None);
        assert_eq!(StatId::from_cli_key(""), None);
        // Gaps.1 — case-insensitive now.
        assert_eq!(StatId::from_cli_key("HITS"), Some(StatId::Hits));
        // Aliases resolve to canonical StatIds.
        assert_eq!(StatId::from_cli_key("g"), Some(StatId::Goals));
        assert_eq!(StatId::from_cli_key("gp"), Some(StatId::Games));
        assert_eq!(StatId::from_cli_key("ppg"), Some(StatId::PointsPerGame));
        assert_eq!(StatId::from_cli_key("blk"), Some(StatId::BlockedShots));
        // Truly unknown still returns None.
        assert_eq!(StatId::from_cli_key("does-not-exist"), None);
    }

    /// Every variant has non-empty `label()` / `short_label()` /
    /// `narrow_label()`. Catches a missing arm or accidentally-empty
    /// string (would render as "" in TUI / site columns).
    #[test]
    fn l0_lindsay_every_label_non_empty() {
        for sid in StatId::all() {
            assert!(!sid.label().is_empty(), "{sid:?} has empty label()");
            assert!(
                !sid.short_label().is_empty(),
                "{sid:?} has empty short_label()"
            );
            assert!(
                !sid.narrow_label().is_empty(),
                "{sid:?} has empty narrow_label()"
            );
        }
    }

    /// `cli_key()` is hyphen-case lowercase ASCII. Site CSS class names
    /// (`.stat-X`) and URL anchors (`#X`) need this property.
    #[test]
    fn l0_lindsay_cli_keys_are_kebab_case() {
        for sid in StatId::all() {
            let k = sid.cli_key();
            assert!(
                k.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "cli_key {k:?} has non-kebab-case chars (must be [a-z0-9-])",
            );
            assert!(
                !k.starts_with('-') && !k.ends_with('-'),
                "cli_key {k:?} has leading or trailing '-'"
            );
            assert!(!k.contains("--"), "cli_key {k:?} has consecutive '-'");
        }
    }

    /// Inverted-unit stats (lower-is-better) report `higher_is_better()
    /// == false`. The fix-point is `Gaa` — the canonical inverted stat.
    /// A few Count stats (Pim, Losses, etc.) are also lower-is-better
    /// but unit-typed as `Count`; the `higher_is_better` arm handles
    /// them explicitly.
    #[test]
    fn l0_lindsay_higher_is_better_inverted_stats() {
        assert!(
            !StatId::Gaa.higher_is_better(),
            "GAA must be lower-is-better"
        );
        assert!(!StatId::Losses.higher_is_better(), "Losses lower-is-better");
        assert!(!StatId::OtLosses.higher_is_better());
        assert!(!StatId::GoalsAgainst.higher_is_better());
        assert!(!StatId::Pim.higher_is_better());
        assert!(!StatId::Giveaways.higher_is_better());
        assert!(!StatId::EvGoalsAgainst.higher_is_better());
        // Sanity: the canonical higher-is-better Goals + SavePct.
        assert!(StatId::Goals.higher_is_better());
        assert!(StatId::SavePct.higher_is_better());
        assert!(StatId::Hits.higher_is_better());
    }

    /// `unit()` matches the field-type discipline on the L.1 substructs:
    /// counts → `u32` storage; pcts → `Option<f32>`; per-60 → `f32`;
    /// seconds → `u32`. Pin a few representative arms.
    #[test]
    fn l0_lindsay_unit_classification_sanity() {
        use StatUnit::*;
        assert_eq!(StatId::Goals.unit(), Count);
        assert_eq!(StatId::ShootingPct.unit(), Pct);
        assert_eq!(StatId::HitsPer60.unit(), Per60);
        assert_eq!(StatId::TotalToi.unit(), Seconds);
        assert_eq!(StatId::Pace82.unit(), Rate);
        assert_eq!(StatId::Gaa.unit(), Inverted);
        assert_eq!(StatId::SavePct.unit(), Pct);
        assert_eq!(StatId::IxG.unit(), Rate);
    }

    /// `StatCategory::all()` lists every category in declaration order.
    /// Drives TUI section-stack rendering.
    #[test]
    fn l0_lindsay_stat_category_all_listed() {
        let categories = StatCategory::all();
        assert_eq!(categories.len(), 9);
        assert_eq!(categories[0], StatCategory::Identity);
        assert_eq!(categories.last(), Some(&StatCategory::Derived));
    }

    // ─── L.2.2 — read / applies_to / available_since tests ─────────────────

    use crate::identity::PlayerId;
    use crate::model::{Position, Season, TeamAbbr};
    use crate::season_stats::{SeasonStats, SeasonStatsBuilder, SeasonType, StatTotals};

    /// `applies_to` truth table — Goalie stats only apply to goalies.
    #[test]
    fn l0_lindsay_applies_to_goalie_category() {
        // Goalie stats — only when is_goalie=true.
        for sid in StatId::all()
            .iter()
            .filter(|s| s.category() == StatCategory::Goalie)
        {
            assert!(
                !sid.applies_to(Position::Center, false),
                "{sid:?} must not apply to skater"
            );
            assert!(
                sid.applies_to(Position::Center, true),
                "{sid:?} must apply to goalie (per-row is_goalie=true)"
            );
        }
    }

    /// `applies_to` — skater stats are HIDDEN on goalie views.
    #[test]
    fn l0_lindsay_applies_to_skater_stats_hidden_on_goalies() {
        for sid in [
            StatId::Goals,
            StatId::Assists,
            StatId::Hits,
            StatId::PpToiPerGame,
        ] {
            assert!(
                sid.applies_to(Position::Center, false),
                "{sid:?} must apply to skater"
            );
            assert!(
                !sid.applies_to(Position::Center, true),
                "{sid:?} must be hidden on goalie views"
            );
        }
    }

    /// `applies_to` — faceoff-taker stats gated to centers only.
    #[test]
    fn l0_lindsay_applies_to_faceoff_centers_only() {
        for sid in [
            StatId::FaceoffWinPct,
            StatId::FaceoffWins,
            StatId::FaceoffLosses,
        ] {
            assert!(sid.applies_to(Position::Center, false), "{sid:?} for C");
            assert!(
                !sid.applies_to(Position::LeftWing, false),
                "{sid:?} blocked for LW"
            );
            assert!(
                !sid.applies_to(Position::Defense, false),
                "{sid:?} blocked for D"
            );
        }
    }

    /// `available_since` — pre-2005 era gates realtime stats.
    #[test]
    fn l0_lindsay_available_since_realtime_2005() {
        for sid in [
            StatId::Hits,
            StatId::BlockedShots,
            StatId::Takeaways,
            StatId::Giveaways,
        ] {
            assert!(
                !sid.applies_to_era(Season(20002001)),
                "{sid:?} not pre-2005"
            );
            assert!(
                sid.applies_to_era(Season(20052006)),
                "{sid:?} OK at 2005-06"
            );
            assert!(sid.applies_to_era(Season(20242025)), "{sid:?} OK modern");
        }
    }

    /// `available_since` — pre-2007 era gates possession + xG.
    #[test]
    fn l0_lindsay_available_since_possession_2007() {
        for sid in [
            StatId::SatPct,
            StatId::IxG,
            StatId::OnIceXgFor,
            StatId::Goals5v5,
        ] {
            assert!(
                !sid.applies_to_era(Season(20062007)),
                "{sid:?} pre-2007 gate"
            );
            assert!(
                sid.applies_to_era(Season(20072008)),
                "{sid:?} OK at 2007-08"
            );
        }
    }

    /// `available_since` — Scoring basics always available.
    #[test]
    fn l0_lindsay_available_since_scoring_always() {
        for sid in [
            StatId::Goals,
            StatId::Assists,
            StatId::Points,
            StatId::PlusMinus,
        ] {
            assert_eq!(sid.available_since(), Season(0));
            assert!(sid.applies_to_era(Season(19171918)));
        }
    }

    fn synthetic_skater_view(stats: SeasonStats) -> (crate::identity::PlayerIdentity, SeasonStats) {
        let id = stats.player_id;
        let identity = crate::identity::PlayerIdentity {
            id,
            full_name: "Test Skater".into(),
            name_normalized: "test skater".into(),
            headshot_canonical_url: None,
            bio: crate::identity::PlayerBio::default(),
        };
        (identity, stats)
    }

    /// `read()` for Scoring basics returns the value from `totals`.
    #[test]
    fn l0_lindsay_read_scoring_basics() {
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(crate::season_stats::TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            shots: 280,
            pp_goals: 12,
            pp_points: 36,
            sh_goals: 1,
            sh_points: 1,
            gwg: 8,
            ot_goals: 2,
            ..Default::default()
        })
        .build();
        let (identity, stats) = synthetic_skater_view(stats);
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        assert_eq!(StatId::Goals.read(&view), Some(30.0));
        assert_eq!(StatId::Assists.read(&view), Some(80.0));
        assert_eq!(StatId::Points.read(&view), Some(110.0));
        assert_eq!(StatId::PpGoals.read(&view), Some(12.0));
        assert_eq!(StatId::PpPoints.read(&view), Some(36.0));
        assert_eq!(StatId::ShGoals.read(&view), Some(1.0));
        assert_eq!(StatId::Gwg.read(&view), Some(8.0));
        assert_eq!(StatId::OtGoals.read(&view), Some(2.0));
        assert_eq!(StatId::Shots.read(&view), Some(280.0));
        // EvGoals = total - PP - SH = 30 - 12 - 1 = 17
        assert_eq!(StatId::EvGoals.read(&view), Some(17.0));
    }

    fn build_stats_with_totals(player_id: u32, season: u32, totals: StatTotals) -> SeasonStats {
        SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(crate::season_stats::TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: totals.gp,
            goals: totals.goals,
            assists: totals.assists,
            points: totals.points,
            goalie: None,
        })
        .with_totals(totals)
        .build()
    }

    /// `read()` for derived per-game stats returns None below MIN_GP.
    #[test]
    fn l0_lindsay_read_per_game_below_min_gp() {
        let stats = build_stats_with_totals(
            8400000,
            20242025,
            StatTotals {
                gp: 5,
                goals: 5,
                assists: 5,
                points: 10,
                ..Default::default()
            },
        );
        let (identity, stats) = synthetic_skater_view(stats);
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };
        assert_eq!(
            StatId::PointsPerGame.read(&view),
            None,
            "GP=5 < MIN_GP must return None"
        );
        assert_eq!(StatId::GoalsPerGame.read(&view), None);
        assert_eq!(StatId::AssistsPerGame.read(&view), None);
    }

    /// `read()` for derived per-game stats works at MIN_GP boundary.
    #[test]
    fn l0_lindsay_read_per_game_at_min_gp() {
        let stats = build_stats_with_totals(
            8400000,
            20242025,
            StatTotals {
                gp: 10,
                goals: 5,
                assists: 5,
                points: 10,
                ..Default::default()
            },
        );
        let (identity, stats) = synthetic_skater_view(stats);
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };
        assert_eq!(StatId::PointsPerGame.read(&view), Some(1.0));
        assert_eq!(StatId::GoalsPerGame.read(&view), Some(0.5));
        assert_eq!(StatId::AssistsPerGame.read(&view), Some(0.5));
    }

    /// DI-11 — `read()` returns None for OnIceGoals stats when traded
    /// in window (multi-stint). Category-boundary guard.
    #[test]
    fn l0_lindsay_read_on_ice_goals_di11_traded_in_window() {
        let stats = SeasonStatsBuilder::new(
            PlayerId(8400000),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(crate::season_stats::TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: Some("2025-02-01".into()),
            gp: 40,
            goals: 15,
            assists: 25,
            points: 40,
            goalie: None,
        })
        .add_team_stint(crate::season_stats::TeamStint {
            team: TeamAbbr("FLA".into()),
            started: Some("2025-02-02".into()),
            ended: None,
            gp: 30,
            goals: 10,
            assists: 15,
            points: 25,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 70,
            goals: 25,
            assists: 40,
            points: 65,
            ..Default::default()
        })
        .with_goals_for_against(crate::season_stats::GoalsForAgainstStats {
            ev_goals_for: 100,
            ev_goals_against: 80,
            ev_goals_for_pct: Some(0.555),
            pp_goals_for: 30,
            pp_goals_against: 1,
            sh_goals_for: 1,
            sh_goals_against: 5,
            even_strength_goal_difference: 20,
            ev_time_on_ice_per_game_sec: 1100,
            offensive_points: Some(45),
            defensive_points: Some(20),
        })
        .build();
        let (identity, stats) = synthetic_skater_view(stats);
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        // Multi-stint → was_traded_in_window() is true → DI-11 fires.
        assert!(view.was_traded_in_window());
        assert_eq!(
            StatId::EvGoalsFor.read(&view),
            None,
            "DI-11 — OnIceGoals must short-circuit to None when traded"
        );
        assert_eq!(StatId::EvenStrengthGoalDifference.read(&view), None);

        // EvenStrengthTimeOnIcePerGame is in TimeOnIce category, NOT
        // OnIceGoals. DI-11 must NOT fire — stat reads through.
        assert_eq!(
            StatId::EvenStrengthTimeOnIcePerGame.read(&view),
            Some(1100.0),
            "TimeOnIce category exempt from DI-11"
        );
        // Scoring stats unaffected.
        assert_eq!(StatId::Goals.read(&view), Some(25.0));
    }

    fn build_goalie_stats(player_id: u32, season: u32, gaa: f32, sv_pct: f32) -> SeasonStats {
        SeasonStatsBuilder::new(
            PlayerId(player_id),
            Season(season),
            SeasonType::Regular,
            Position::Goalie,
        )
        .add_team_stint(crate::season_stats::TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 50,
            goals: 0,
            assists: 0,
            points: 0,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp: 50,
            ..Default::default()
        })
        .with_goalie(crate::season_stats::GoalieSeasonStats {
            games_started: 50,
            wins: 30,
            losses: 18,
            ot_losses: Some(2),
            ties: None,
            shots_against: 1500,
            goals_against: 130,
            saves: 1370,
            save_pct: Some(sv_pct),
            goals_against_average: Some(gaa),
            shutouts: 5,
            time_on_ice_sec: 3000 * 60,
        })
        .build()
    }

    /// `sort_cmp`: higher_is_better stats sort descending; None last.
    #[test]
    fn l0_lindsay_sort_cmp_higher_is_better_descending() {
        use std::cmp::Ordering;
        let s1 = build_stats_with_totals(
            8478402,
            20242025,
            StatTotals {
                gp: 82,
                goals: 50,
                ..Default::default()
            },
        );
        let s2 = build_stats_with_totals(
            8479318,
            20242025,
            StatTotals {
                gp: 82,
                goals: 30,
                ..Default::default()
            },
        );
        let (id1, s1c) = synthetic_skater_view(s1);
        let (id2, s2c) = synthetic_skater_view(s2);
        let v1 = PlayerView {
            identity: &id1,
            stats: &s1c,
            contract: None,
        };
        let v2 = PlayerView {
            identity: &id2,
            stats: &s2c,
            contract: None,
        };
        // McDavid (50G) before Marner (30G) — descending Goals.
        assert_eq!(StatId::Goals.sort_cmp(&v1, &v2), Ordering::Less);
        // Reverse direction → reverse cmp.
        assert_eq!(StatId::Goals.sort_cmp(&v2, &v1), Ordering::Greater);
    }

    /// `sort_cmp`: lower_is_better (Gaa) sorts ascending.
    #[test]
    fn l0_lindsay_sort_cmp_inverted_ascending() {
        use std::cmp::Ordering;
        let s1 = build_goalie_stats(8400001, 20242025, 2.50, 0.913);
        let s2 = build_goalie_stats(8400002, 20242025, 3.10, 0.893);
        let (id1, s1c) = synthetic_skater_view(s1);
        let (id2, s2c) = synthetic_skater_view(s2);
        let v1 = PlayerView {
            identity: &id1,
            stats: &s1c,
            contract: None,
        };
        let v2 = PlayerView {
            identity: &id2,
            stats: &s2c,
            contract: None,
        };
        // GAA 2.50 (better) before 3.10 — ascending GAA.
        assert_eq!(StatId::Gaa.sort_cmp(&v1, &v2), Ordering::Less);
    }

    // ── EDGE E1 (L.5b post-fix) — sort_cmp picker edge cases ────────────

    /// EDGE E1.1 — `sort_cmp` for Gaa (Inverted unit / lower_is_better)
    /// sorts ascending AND a None value sorts last regardless of
    /// direction. Without this, picking `gaa` in the picker would put
    /// goalies-without-data at the TOP (artificial low-GAA zero) — the
    /// opposite of what the user wants.
    #[test]
    fn l0_lindsay_l5b_edge_picker_gaa_none_sorts_last() {
        use std::cmp::Ordering;
        let s_with_gaa = build_goalie_stats(8400001, 20242025, 2.50, 0.913);
        let s_no_gaa = build_stats_with_totals(
            8400002,
            20242025,
            StatTotals {
                gp: 70,
                goals: 30,
                assists: 50,
                points: 80,
                ..Default::default()
            },
        );
        let (id1, s1c) = synthetic_skater_view(s_with_gaa);
        let (id2, s2c) = synthetic_skater_view(s_no_gaa);
        let v_gaa = PlayerView {
            identity: &id1,
            stats: &s1c,
            contract: None,
        };
        let v_no = PlayerView {
            identity: &id2,
            stats: &s2c,
            contract: None,
        };
        // GAA reads Some on the goalie view, None on the skater view.
        // Sort: goalie-with-data first, no-data last.
        assert_eq!(
            StatId::Gaa.sort_cmp(&v_gaa, &v_no),
            Ordering::Less,
            "Some(Gaa) before None — None sorts last per AI-06"
        );
        assert_eq!(
            StatId::Gaa.sort_cmp(&v_no, &v_gaa),
            Ordering::Greater,
            "None after Some(Gaa) — None sorts last per AI-06"
        );
    }

    /// EDGE E1.2 — `EvGoalsForPct` picker pick on a multi-stint view
    /// returns None per DI-11 (OnIceGoals category trade-window guard).
    /// `sort_cmp` sorts None last, so traded players don't show
    /// artificial zero values at the top.
    #[test]
    fn l0_lindsay_l5b_edge_picker_ev_goals_for_pct_traded_returns_none() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::traded_multistint();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };
        // DI-11 fires — the OnIceGoals trade-window guard.
        assert_eq!(
            StatId::EvGoalsForPct.read(&view),
            None,
            "DI-11: OnIceGoals reads return None on multi-stint views"
        );
    }

    /// EDGE E1.3 — `PointsPerGame` picker pick on a sub-MIN_GP view
    /// returns None per the MIN_GP=10 derived-rate gate. Sub-MIN_GP
    /// players don't appear at the top of a PPG-sorted leaderboard
    /// with artificial 0.0 values.
    #[test]
    fn l0_lindsay_l5b_edge_picker_points_per_game_below_min_gp_returns_none() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::low_gp();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };
        // GP < 10 → PointsPerGame returns None per MIN_GP guard.
        assert_eq!(
            StatId::PointsPerGame.read(&view),
            None,
            "MIN_GP=10 guard: derived rate returns None below threshold"
        );
        // Sanity — Goals (Count) is unaffected by the gate.
        assert!(
            StatId::Goals.read(&view).is_some(),
            "raw counts not gated by MIN_GP"
        );
    }

    /// `aggregate_read` — strict propagation: any None → None.
    #[test]
    fn l0_lindsay_aggregate_read_strict_propagation() {
        let s1 = build_stats_with_totals(
            8478402,
            20242025,
            StatTotals {
                gp: 82,
                goals: 50,
                ..Default::default()
            },
        );
        let s2 = build_stats_with_totals(
            8478402,
            20232024,
            StatTotals {
                gp: 5,
                goals: 3,
                ..Default::default() // < MIN_GP
            },
        );
        let (id1, s1c) = synthetic_skater_view(s1);
        let (id2, s2c) = synthetic_skater_view(s2);
        let v1 = PlayerView {
            identity: &id1,
            stats: &s1c,
            contract: None,
        };
        let v2 = PlayerView {
            identity: &id2,
            stats: &s2c,
            contract: None,
        };

        // Goals (Count) — sums across windows even when GP varies.
        assert_eq!(StatId::Goals.aggregate_read(&[v1, v2]), Some(53.0));

        // PointsPerGame — second window below MIN_GP returns None →
        // aggregate propagates None.
        assert_eq!(StatId::PointsPerGame.aggregate_read(&[v1, v2]), None);
    }

    /// `aggregate_read` — empty slice returns None.
    #[test]
    fn l0_lindsay_aggregate_read_empty_returns_none() {
        assert_eq!(StatId::Goals.aggregate_read(&[]), None);
    }

    // ─── L.4.1 — Games StatId + default_in_career_table tests ────────────

    /// `Games` parses from cli_key `"games"`. Closes the L.3 catalog
    /// gap KEEL flagged: `--filter "games>=70"` previously errored as
    /// UnknownStat; post-L.4.1 it routes to `StatId::Games`.
    #[test]
    fn l0_lindsay_games_stat_id_from_cli_key() {
        assert_eq!(StatId::from_cli_key("games"), Some(StatId::Games));
    }

    /// `Games::read(view)` returns `view.stats.totals.gp` as f64.
    #[test]
    fn l0_lindsay_games_stat_id_reads_gp_from_totals() {
        let stats = build_stats_with_totals(
            8478402,
            20242025,
            StatTotals {
                gp: 70,
                goals: 30,
                assists: 50,
                points: 80,
                ..Default::default()
            },
        );
        let (identity, stats) = synthetic_skater_view(stats);
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };
        assert_eq!(StatId::Games.read(&view), Some(70.0));
    }

    /// `Games` is in the Scoring category; iteration order places it
    /// before Goals (declaration-first).
    #[test]
    fn l0_lindsay_games_in_scoring_category() {
        assert_eq!(StatId::Games.category(), StatCategory::Scoring);
        let pos_games = StatId::all()
            .iter()
            .position(|&s| s == StatId::Games)
            .unwrap();
        let pos_goals = StatId::all()
            .iter()
            .position(|&s| s == StatId::Goals)
            .unwrap();
        assert!(
            pos_games < pos_goals,
            "Games should iterate before Goals (declaration order)"
        );
    }

    /// `default_in_career_table` for skaters: the 15 expected default
    /// stats return true; non-default stats return false.
    /// (Post-SCOUT-3 L.5b: PointsPerGame added → 15 common + FaceoffWinPct = 16 for C.)
    #[test]
    fn l0_lindsay_default_in_career_table_skater_defaults() {
        use crate::model::Position::*;
        let center_defaults: Vec<StatId> = StatId::all()
            .iter()
            .copied()
            .filter(|s| s.default_in_career_table(Center))
            .collect();
        assert_eq!(
            center_defaults.len(),
            16,
            "Center default = 16 (skater_common 15 + FOWinPct)"
        );
        assert!(center_defaults.contains(&StatId::Games));
        assert!(center_defaults.contains(&StatId::Goals));
        assert!(
            center_defaults.contains(&StatId::Gwg),
            "Gwg in skater default (SCOUT L.4)"
        );
        assert!(
            center_defaults.contains(&StatId::PointsPerGame),
            "PointsPerGame in skater default (SCOUT-3 L.5b)"
        );
        assert!(center_defaults.contains(&StatId::FaceoffWinPct));
        // Non-default stat for Center.
        assert!(!StatId::PpAssistsPer60.default_in_career_table(Center));
        assert!(!StatId::SatPct.default_in_career_table(Center));
    }

    /// LeftWing/RightWing get the 15 skater defaults (no position
    /// extras). Defense gets 15 + EvGoalsForPct = 16 per SCOUT-8 L.5b.
    #[test]
    fn l0_lindsay_default_in_career_table_wingers_no_faceoff() {
        use crate::model::Position::*;
        let lw_defaults: Vec<StatId> = StatId::all()
            .iter()
            .copied()
            .filter(|s| s.default_in_career_table(LeftWing))
            .collect();
        assert_eq!(lw_defaults.len(), 15, "LW default = 15 skater common");
        assert!(
            !lw_defaults.contains(&StatId::FaceoffWinPct),
            "FaceoffWinPct is Center-only"
        );
        assert!(
            !lw_defaults.contains(&StatId::EvGoalsForPct),
            "EvGoalsForPct is Defense-only (SCOUT-8 L.5b)"
        );

        let d_defaults: Vec<StatId> = StatId::all()
            .iter()
            .copied()
            .filter(|s| s.default_in_career_table(Defense))
            .collect();
        assert_eq!(
            d_defaults.len(),
            16,
            "Defense default = 16 (skater_common 15 + EvGoalsForPct)"
        );
        assert!(
            d_defaults.contains(&StatId::EvGoalsForPct),
            "EvGoalsForPct in Defense default (SCOUT-8 L.5b)"
        );
    }

    /// Goalie gets 11 goalie-specific default columns. (Post-SCOUT L.4:
    /// dropped RegulationWins, added Saves + ShotsAgainst — net 10 → 11.)
    #[test]
    fn l0_lindsay_default_in_career_table_goalie() {
        use crate::model::Position::*;
        let goalie_defaults: Vec<StatId> = StatId::all()
            .iter()
            .copied()
            .filter(|s| s.default_in_career_table(Goalie))
            .collect();
        assert_eq!(goalie_defaults.len(), 11, "Goalie default = 11");
        assert!(goalie_defaults.contains(&StatId::SavePct));
        assert!(goalie_defaults.contains(&StatId::Gaa));
        assert!(goalie_defaults.contains(&StatId::QualityStarts));
        assert!(
            goalie_defaults.contains(&StatId::Saves),
            "Saves added per SCOUT L.4 (volume context for SV%)"
        );
        assert!(
            goalie_defaults.contains(&StatId::ShotsAgainst),
            "ShotsAgainst added per SCOUT L.4 (volume context for GAA)"
        );
        assert!(
            !goalie_defaults.contains(&StatId::RegulationWins),
            "RegulationWins dropped from default (non-canonical)"
        );
        // Skater stats are NOT in goalie defaults.
        assert!(!goalie_defaults.contains(&StatId::Games));
        assert!(!goalie_defaults.contains(&StatId::Goals));
    }

    /// Identity / non-selectable stats never appear in any career-table default.
    #[test]
    fn l0_lindsay_default_in_career_table_identity_excluded() {
        use crate::model::Position::*;
        // Pick a few clearly-non-default stats.
        let non_defaults = [
            StatId::PpGoalsAgainstPer60,
            StatId::OnIceXgFor,
            StatId::PaceSortKey,
        ];
        for s in non_defaults {
            for pos in [Center, LeftWing, RightWing, Defense, Goalie] {
                assert!(
                    !s.default_in_career_table(pos),
                    "{s:?} should not be a default for {pos:?}"
                );
            }
        }
    }

    // ─── L.2.4 — FilterParseError + parse_filter tests ─────────────────────

    /// Happy path: all four ops parse + every category exercised.
    #[test]
    fn l0_lindsay_parse_filter_happy_paths() {
        let f = parse_filter("goals>=30").unwrap();
        assert_eq!(f.stat, StatId::Goals);
        assert_eq!(f.op, FilterOp::Min);
        assert_eq!(f.value, 30.0);

        let f = parse_filter("save-pct==0.92").unwrap();
        assert_eq!(f.stat, StatId::SavePct);
        assert_eq!(f.op, FilterOp::Equals);
        assert!((f.value - 0.92).abs() < 1e-9);

        let f = parse_filter("hits-per-60<=2.0").unwrap();
        assert_eq!(f.stat, StatId::HitsPer60);
        assert_eq!(f.op, FilterOp::Max);

        // Single-`=` shorthand for Equals.
        let f = parse_filter("plus-minus=22").unwrap();
        assert_eq!(f.stat, StatId::PlusMinus);
        assert_eq!(f.op, FilterOp::Equals);
        assert_eq!(f.value, 22.0);
    }

    /// Whitespace tolerance: trim, allowed around op.
    #[test]
    fn l0_lindsay_parse_filter_whitespace_tolerated() {
        for input in &[
            "  goals>=30  ",
            "goals >= 30",
            "  goals  >=  30  ",
            "\tgoals\t>=\t30\t",
        ] {
            let f =
                parse_filter(input).unwrap_or_else(|e| panic!("input {input:?} should parse: {e}"));
            assert_eq!(f.stat, StatId::Goals);
            assert_eq!(f.op, FilterOp::Min);
            assert_eq!(f.value, 30.0);
        }
    }

    /// EmptyInput: empty + whitespace-only.
    #[test]
    fn l0_lindsay_parse_filter_empty_input_variant() {
        for input in &["", "   ", "\t\t", " \n\r"] {
            assert_eq!(
                parse_filter(input).unwrap_err(),
                FilterParseError::EmptyInput,
                "{input:?} should be EmptyInput"
            );
        }
    }

    /// EmptyStatKey: op present, stat-key empty/whitespace.
    #[test]
    fn l0_lindsay_parse_filter_empty_stat_key_variant() {
        for input in &[">=10", "<=5", "==1", "  >=  10  "] {
            let err = parse_filter(input).unwrap_err();
            assert_eq!(
                err,
                FilterParseError::EmptyStatKey,
                "{input:?} should be EmptyStatKey, got {err:?}"
            );
        }
    }

    /// MissingOp: no op token.
    #[test]
    fn l0_lindsay_parse_filter_missing_op_variant() {
        for input in &["hits10", "hits 10", "goals", "shots-per-game"] {
            let err = parse_filter(input).unwrap_err();
            assert!(
                matches!(err, FilterParseError::MissingOp { .. }),
                "{input:?} should be MissingOp, got {err:?}"
            );
        }
    }

    /// KEEL D2 (L.5b post-fix) — `=>` / `=<` typos get a "did you mean
    /// `>=` / `<=`?" hint in the error message. Both classify as
    /// MultipleOps in the current parser (the `=` and `>` each detect
    /// as ops).
    #[test]
    fn l0_lindsay_l5b_parse_filter_typo_hint_for_swapped_op() {
        let err = parse_filter("hits=>10").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("did you mean `>=`?"),
            "expected typo hint for `=>`; got {msg:?}"
        );

        let err = parse_filter("hits=<10").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("did you mean `<=`?"),
            "expected typo hint for `=<`; got {msg:?}"
        );
    }

    /// MultipleOps: more than one op token.
    #[test]
    fn l0_lindsay_parse_filter_multiple_ops_variant() {
        for input in &["hits>=>5", "hits===5", "hits>=>=5", "hits<>=5"] {
            let err = parse_filter(input).unwrap_err();
            assert!(
                matches!(err, FilterParseError::MultipleOps { .. }),
                "{input:?} should be MultipleOps, got {err:?}"
            );
        }
    }

    /// UnknownStat: parses but not in catalog. Gaps.1 — UPPERCASE
    /// keys now resolve case-insensitively (HITS → Hits), so
    /// `GOALS>=10` is no longer an UnknownStat. Adjusted inputs to
    /// genuinely-unknown keys.
    #[test]
    fn l0_lindsay_parse_filter_unknown_stat_variant() {
        for input in &[
            "hots-per-60>=2",
            "foo=5",
            "totally-fake-stat>=10",
            "made-up-stat==1",
        ] {
            let err = parse_filter(input).unwrap_err();
            assert!(
                matches!(err, FilterParseError::UnknownStat { .. }),
                "{input:?} should be UnknownStat, got {err:?}"
            );
        }
    }

    /// BadNumber: number portion fails f64 parse. Includes locale comma.
    #[test]
    fn l0_lindsay_parse_filter_bad_number_variant() {
        for input in &[
            "hits>=abc",
            "hits>=1,5", // locale comma — explicitly rejected
            "hits>=",    // empty number
            "hits>=1.2.3",
        ] {
            let err = parse_filter(input).unwrap_err();
            assert!(
                matches!(err, FilterParseError::BadNumber { .. }),
                "{input:?} should be BadNumber, got {err:?}"
            );
        }
    }

    /// NotFinite: parses to NaN / +inf / -inf.
    #[test]
    fn l0_lindsay_parse_filter_not_finite_variant() {
        for input in &["hits>=NaN", "hits>=inf", "hits>=-inf", "hits>=+inf"] {
            let err = parse_filter(input).unwrap_err();
            assert!(
                matches!(err, FilterParseError::NotFinite { .. }),
                "{input:?} should be NotFinite, got {err:?}"
            );
        }
    }

    /// `StatFilter::new` finite-value gate. Direct constructor path.
    #[test]
    fn l0_lindsay_stat_filter_new_finite_gate() {
        // Finite values pass.
        let f = StatFilter::new(StatId::Goals, FilterOp::Min, 30.0).unwrap();
        assert_eq!(f.value, 30.0);
        // NaN rejected.
        let err = StatFilter::new(StatId::Goals, FilterOp::Min, f64::NAN).unwrap_err();
        assert!(matches!(err, FilterParseError::NotFinite { .. }));
        // +inf rejected.
        let err = StatFilter::new(StatId::Goals, FilterOp::Min, f64::INFINITY).unwrap_err();
        assert!(matches!(err, FilterParseError::NotFinite { .. }));
        // -inf rejected.
        let err = StatFilter::new(StatId::Goals, FilterOp::Min, f64::NEG_INFINITY).unwrap_err();
        assert!(matches!(err, FilterParseError::NotFinite { .. }));
    }

    /// `FilterParseError` Display strings include actionable info.
    #[test]
    fn l0_lindsay_filter_parse_error_display_messages() {
        let err = parse_filter("foo=5").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("foo"),
            "UnknownStat msg should mention key: {msg}"
        );
        assert!(
            msg.contains("unknown stat"),
            "msg should label error class: {msg}"
        );

        let err = parse_filter("hits>=NaN").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not finite"),
            "NotFinite msg should label: {msg}"
        );
        assert!(
            msg.contains("NaN"),
            "NotFinite msg should mention token: {msg}"
        );
    }

    /// Op-priority: `>=` wins over `=` at the same position.
    /// Pin the routing for `hits>=10` (which contains `=` as a substring
    /// of `>=`) — it must parse as Min, not Equals.
    #[test]
    fn l0_lindsay_parse_filter_op_priority_two_char_first() {
        let f = parse_filter("hits>=10").unwrap();
        assert_eq!(f.op, FilterOp::Min);
        let f = parse_filter("hits<=10").unwrap();
        assert_eq!(f.op, FilterOp::Max);
        let f = parse_filter("hits==10").unwrap();
        assert_eq!(f.op, FilterOp::Equals);
        let f = parse_filter("hits=10").unwrap();
        assert_eq!(f.op, FilterOp::Equals);
    }

    // ── Filter.OR — boolean grammar (AND / OR / NOT / parens) ──────────

    #[test]
    fn l0_filter_expr_bare_atom_round_trips_to_atom_variant() {
        let e = parse_filter_expr("g>=50").unwrap();
        assert!(e.is_atom());
        let atom = e.as_atom().unwrap();
        assert_eq!(atom.stat, StatId::Goals);
        assert_eq!(atom.value, 50.0);
    }

    #[test]
    fn l0_filter_expr_or_two_atoms() {
        let e = parse_filter_expr("g>=50 OR a>=50").unwrap();
        match e {
            FilterExpr::Or(a, b) => {
                assert_eq!(a.as_atom().unwrap().stat, StatId::Goals);
                assert_eq!(b.as_atom().unwrap().stat, StatId::Assists);
            }
            other => panic!("expected Or, got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_and_two_atoms() {
        let e = parse_filter_expr("g>=30 AND a>=30").unwrap();
        match e {
            FilterExpr::And(a, b) => {
                assert_eq!(a.as_atom().unwrap().stat, StatId::Goals);
                assert_eq!(b.as_atom().unwrap().stat, StatId::Assists);
            }
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_not_unary() {
        let e = parse_filter_expr("NOT pim>=100").unwrap();
        match e {
            FilterExpr::Not(inner) => {
                assert_eq!(inner.as_atom().unwrap().stat, StatId::Pim);
            }
            other => panic!("expected Not, got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_parens_grouping_changes_associativity() {
        // `(g>=30 AND a>=30) OR p>=80` — parens force OR at top.
        let e = parse_filter_expr("(g>=30 AND a>=30) OR p>=80").unwrap();
        match e {
            FilterExpr::Or(a, b) => {
                assert!(matches!(*a, FilterExpr::And(_, _)));
                assert_eq!(b.as_atom().unwrap().stat, StatId::Points);
            }
            other => panic!("expected Or-of-And, got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_precedence_and_binds_tighter_than_or() {
        // No parens: `g>=30 AND a>=30 OR p>=80` parses as
        // `(g>=30 AND a>=30) OR p>=80`.
        let e = parse_filter_expr("g>=30 AND a>=30 OR p>=80").unwrap();
        match e {
            FilterExpr::Or(a, b) => {
                assert!(matches!(*a, FilterExpr::And(_, _)));
                assert_eq!(b.as_atom().unwrap().stat, StatId::Points);
            }
            other => panic!("AND must bind tighter than OR; got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_keywords_case_insensitive() {
        // Both `or` and `OR` work; same for and/AND, not/NOT.
        let e1 = parse_filter_expr("g>=50 or a>=50").unwrap();
        let e2 = parse_filter_expr("g>=50 OR a>=50").unwrap();
        let e3 = parse_filter_expr("g>=50 Or a>=50").unwrap();
        assert!(matches!(e1, FilterExpr::Or(_, _)));
        assert!(matches!(e2, FilterExpr::Or(_, _)));
        assert!(matches!(e3, FilterExpr::Or(_, _)));
    }

    #[test]
    fn l0_filter_expr_keyword_inside_atom_is_not_a_keyword() {
        // `goals` contains "OR" embedded but it's not a word boundary
        // hit — the atom parses as a stat, not as a malformed expr.
        let e = parse_filter_expr("goals>=30").unwrap();
        assert_eq!(e.as_atom().unwrap().stat, StatId::Goals);
    }

    #[test]
    fn l0_filter_expr_double_not_cancels() {
        let e = parse_filter_expr("NOT NOT g>=30").unwrap();
        match e {
            FilterExpr::Not(inner) => match *inner {
                FilterExpr::Not(_) => {}
                other => panic!("expected Not-of-Not, got {other:?}"),
            },
            other => panic!("expected Not at top, got {other:?}"),
        }
    }

    #[test]
    fn l0_filter_expr_unclosed_paren_errors() {
        let err = parse_filter_expr("(g>=30 AND a>=30").unwrap_err();
        assert_eq!(err, FilterParseError::UnclosedParen);
    }

    #[test]
    fn l0_filter_expr_unexpected_rparen_errors() {
        // Trailing `)` after a complete expression — the top-level
        // parser surfaces it as UnexpectedToken because parse_or
        // already returned a clean tree before seeing the rparen.
        // The mid-expression rparen path returns UnexpectedRParen
        // (covered by the leading-rparen test below).
        let err = parse_filter_expr("g>=30)").unwrap_err();
        assert!(
            matches!(
                err,
                FilterParseError::UnexpectedToken { .. } | FilterParseError::UnexpectedRParen
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn l0_filter_expr_leading_rparen_errors() {
        let err = parse_filter_expr(") g>=30").unwrap_err();
        assert_eq!(err, FilterParseError::UnexpectedRParen);
    }

    #[test]
    fn l0_filter_expr_dangling_and_errors() {
        let err = parse_filter_expr("g>=30 AND").unwrap_err();
        assert_eq!(err, FilterParseError::UnexpectedEnd);
    }

    #[test]
    fn l0_filter_expr_dangling_not_errors() {
        let err = parse_filter_expr("NOT").unwrap_err();
        assert_eq!(err, FilterParseError::UnexpectedEnd);
    }

    #[test]
    fn l0_filter_expr_propagates_atom_errors() {
        // Invalid atom inside the expression — propagates via parse_filter.
        let err = parse_filter_expr("totally-fake>=10 OR g>=50").unwrap_err();
        assert!(matches!(err, FilterParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_filter_expr_empty_input_errors() {
        let err = parse_filter_expr("").unwrap_err();
        assert_eq!(err, FilterParseError::EmptyInput);
    }

    #[test]
    fn l0_filter_expr_whitespace_only_errors() {
        let err = parse_filter_expr("   ").unwrap_err();
        assert_eq!(err, FilterParseError::EmptyInput);
    }

    #[test]
    fn l0_filter_expr_aliases_inside_compound() {
        // Short aliases must work inside boolean expressions too.
        let e = parse_filter_expr("g>=50 OR a>=50").unwrap();
        match e {
            FilterExpr::Or(a, b) => {
                assert_eq!(a.as_atom().unwrap().stat, StatId::Goals);
                assert_eq!(b.as_atom().unwrap().stat, StatId::Assists);
            }
            _ => panic!("expected Or"),
        }
    }

    #[test]
    fn l0_filter_expr_three_way_or_left_associative() {
        // `g>=50 OR a>=50 OR p>=80` parses as `((g OR a) OR p)`.
        let e = parse_filter_expr("g>=50 OR a>=50 OR p>=80").unwrap();
        match e {
            FilterExpr::Or(left, right) => {
                assert!(matches!(*left, FilterExpr::Or(_, _)));
                assert_eq!(right.as_atom().unwrap().stat, StatId::Points);
            }
            _ => panic!("expected nested Or"),
        }
    }

    // ── Phase Foster.5 — windowed atom grammar ──────────────────────────────

    use crate::timeframe::Timeframe;

    #[test]
    fn l0_foster5_windowed_atom_no_window_is_none() {
        let a = WindowedAtom::parse("g>=10").unwrap();
        assert_eq!(a.stat, StatId::Goals);
        assert!(a.window.is_none(), "no .window suffix → None");
        assert_eq!(a.value, 10.0);
    }

    #[test]
    fn l0_foster5_windowed_atom_with_week_suffix() {
        let a = WindowedAtom::parse("g.week>=10").unwrap();
        assert_eq!(a.stat, StatId::Goals);
        assert_eq!(a.window, Some(Timeframe::Week));
        assert_eq!(a.value, 10.0);
    }

    #[test]
    fn l0_foster5_windowed_atom_all_window_keywords() {
        for (key, want) in [
            ("g.day>=1", Timeframe::Day),
            ("g.week>=10", Timeframe::Week),
            ("g.month>=20", Timeframe::Month),
            ("g.season>=50", Timeframe::Season),
        ] {
            let a = WindowedAtom::parse(key).unwrap();
            assert_eq!(a.window, Some(want), "input {key}");
        }
    }

    #[test]
    fn l0_foster5_windowed_atom_unknown_window_falls_back_to_full_key() {
        // `pp-pct` is a real stat key; `pp-pct.unrecognized>=0.5` should
        // try `pp-pct.unrecognized` as a stat key (unknown → error).
        let err = WindowedAtom::parse("pp-pct.notawindow>=0.5").unwrap_err();
        assert!(matches!(err, FilterParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_foster5_windowed_atom_resolved_window_picks_default() {
        let bare = WindowedAtom::parse("g>=10").unwrap();
        assert_eq!(bare.resolved_window(Timeframe::Season), Timeframe::Season);
        assert_eq!(bare.resolved_window(Timeframe::Week), Timeframe::Week);

        let explicit = WindowedAtom::parse("g.month>=20").unwrap();
        // Explicit window beats the default.
        assert_eq!(
            explicit.resolved_window(Timeframe::Week),
            Timeframe::Month
        );
    }

    #[test]
    fn l0_foster5_windowed_atom_rejects_garbage() {
        // Empty input
        assert!(matches!(
            WindowedAtom::parse("").unwrap_err(),
            FilterParseError::EmptyInput
        ));
        // Missing op
        assert!(matches!(
            WindowedAtom::parse("g.week").unwrap_err(),
            FilterParseError::MissingOp { .. }
        ));
        // Multiple ops in value
        assert!(matches!(
            WindowedAtom::parse("g.week>=>10").unwrap_err(),
            FilterParseError::MultipleOps { .. }
        ));
        // Non-numeric value
        assert!(matches!(
            WindowedAtom::parse("g.week>=ten").unwrap_err(),
            FilterParseError::BadNumber { .. }
        ));
    }

    #[test]
    fn l0_foster5_windowed_atom_round_trips_through_parse_filter() {
        // Bare atom (no .window) should produce the same StatId+op+value
        // as parse_filter — proves they share the op/value handling.
        let windowed = WindowedAtom::parse("hits>=5").unwrap();
        let plain = parse_filter("hits>=5").unwrap();
        assert_eq!(windowed.stat, plain.stat);
        assert_eq!(windowed.op, plain.op);
        assert_eq!(windowed.value, plain.value);
    }
}
