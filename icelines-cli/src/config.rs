use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Raw TOML shape ────────────────────────────────────────────────────────────

/// The shape read directly from `~/.icelines/config.toml`.
/// All fields are optional so that a partial file (or no file at all) still works.
#[derive(Debug, Deserialize, Serialize, Default)]
#[allow(dead_code)]
struct RawConfig {
    csv_path: Option<String>,
    cache_dir: Option<String>,
    season: Option<u32>,
    /// Phase 8f.1: when explicitly `false`, live NHL API fetches are
    /// disabled. Absent or `true` keeps live feeds on (the default).
    live: Option<bool>,
    /// Phase 8j: when explicitly `true`, the experimental proof-compiled
    /// dashboard panels render in supporting screens (currently the player
    /// detail card). Absent or `false` keeps them hidden — feature is
    /// off by default while the integration matures.
    dashboards: Option<bool>,
    /// Phase Reports — per-Tier-1 report toggles. Controls which reports
    /// the loader pulls from the snapshot store and which `StatId`
    /// columns are visible in TUI / query output. Absent section defaults
    /// to `realtime=true, others=false`.
    #[serde(default)]
    reports: Option<RawReportToggles>,
    /// Phase Foster.0.7 — `[sync]` section.
    #[serde(default)]
    sync: Option<RawSync>,
    /// Phase Adams.6 — `[ai]` section. Optional and disabled by
    /// default; absent section means "AI fallback is off".
    #[serde(default)]
    ai: Option<RawAi>,
}

/// `[ai]` TOML section for the cmdbar AI fallback. Each field
/// optional with a sensible default in `AiConfig::from_raw`.
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[allow(dead_code)]
struct RawAi {
    enabled: Option<bool>,
    provider: Option<String>,
    model: Option<String>,
    timeout_secs: Option<u64>,
}

/// `[reports]` TOML section — every field optional with a sensible default.
#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy)]
#[allow(dead_code)]
struct RawReportToggles {
    realtime: Option<bool>,
    timeonice: Option<bool>,
    goals_for_against: Option<bool>,
    goalie_advanced: Option<bool>,
    goalie_saves_by_strength: Option<bool>,
}

// ── Phase Foster.0.7 — sync + capability matrix raw shape ────────────────────

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[allow(dead_code)]
struct RawSync {
    policy: Option<String>,
    banner: Option<String>,
    season_transition: Option<String>,
    #[serde(default)]
    capabilities: Option<RawCapabilities>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[allow(dead_code)]
struct RawCapabilities {
    stats: Option<String>,
    scores_schedule: Option<String>,
    transactions: Option<String>,
    boxscores: Option<String>,
    shifts: Option<String>,
    career_history: Option<String>,
}

// ── Phase Adams.6 — AI config raw → typed conversion ─────────────────────────

fn ai_from_raw(raw: Option<RawAi>) -> crate::ai::AiConfig {
    let d = crate::ai::AiConfig::default();
    let Some(r) = raw else { return d };
    crate::ai::AiConfig {
        enabled: r.enabled.unwrap_or(d.enabled),
        provider: r.provider.unwrap_or(d.provider),
        model: r.model.unwrap_or(d.model),
        timeout: r
            .timeout_secs
            .map(std::time::Duration::from_secs)
            .unwrap_or(d.timeout),
    }
}

// ── Public Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Path to a Yahoo CSV export (optional).
    pub csv_path: Option<PathBuf>,
    /// Directory used to cache API responses.
    pub cache_dir: PathBuf,
    /// Season identifier (8-digit YYYYZZZZ, e.g. 20252026).
    pub season: Option<u32>,
    /// Phase 8f.1: explicit value of the `live` config key, if set.
    /// `None` means "respect the defaults / env var / CLI flag".
    pub live: Option<bool>,
    /// Phase 8j: explicit value of the `dashboards` config key, if set.
    /// `None` defers to env / CLI / default(off).
    pub dashboards: Option<bool>,
    /// Phase Reports — fully-resolved per-report toggle set.
    /// Defaults applied here so callers don't need to know about
    /// the absent-section case.
    pub reports: ReportToggles,
    /// Phase Foster.0.7 — sync engine + capability matrix.
    pub sync: SyncConfig,
    /// Phase Adams.6 — AI fallback for the cmdbar. Default
    /// off (deterministic grammar is the canonical UX).
    pub ai: crate::ai::AiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            csv_path: None,
            cache_dir: PathBuf::new(),
            season: None,
            live: None,
            dashboards: None,
            reports: ReportToggles::default(),
            sync: SyncConfig::default(),
            ai: crate::ai::AiConfig::default(),
        }
    }
}

impl Config {
    /// Test and fallback constructor for callers that do not want to read
    /// `~/.icelines/config.toml`. Keep this as the one struct-literal shield
    /// so adding a config field does not break every test fixture.
    pub fn test_default() -> Self {
        Self::default()
    }
}

/// Phase Reports — resolved per-Tier-1 report toggle set. Lives in
/// `Config::reports`; persisted via `Config::save_reports`.
///
/// `Core` reports (skater summary/bios, goalie summary/bios) are always
/// on — they're the irreducible identity + counting-stats layer. The
/// fields here are the optional Tier-1 layers a user can turn off if
/// they don't care about hits/blocks, time-on-ice splits, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportToggles {
    /// `/skater/realtime` — Hits, Blocks, Takeaways, Giveaways,
    /// MissedShots, PIM. Default: ON.
    pub realtime: bool,
    /// `/skater/timeonice` — TotalToi, EvToi, PpToi, ShToi, Shifts,
    /// ToiPerShift. Default: OFF (require explicit opt-in until L.7c
    /// bundles the per-season files).
    pub timeonice: bool,
    /// `/skater/goalsForAgainst` — EvGoalsFor/Against, EvGoalsForPct,
    /// PpGoalsFor, ShGoalsFor, EvenStrengthGoalDifference. Default: OFF.
    pub goals_for_against: bool,
    /// `/goalie/advanced` — QualityStarts, QualityStartPct,
    /// RegulationWins, RegulationLosses. Default: OFF.
    pub goalie_advanced: bool,
    /// `/goalie/savesByStrength` — EvSavePct, PpSavePct, ShSavePct.
    /// Default: OFF.
    pub goalie_saves_by_strength: bool,
}

impl Default for ReportToggles {
    fn default() -> Self {
        Self {
            realtime: true,
            timeonice: false,
            goals_for_against: false,
            goalie_advanced: false,
            goalie_saves_by_strength: false,
        }
    }
}

impl ReportToggles {
    fn from_raw(raw: Option<RawReportToggles>) -> Self {
        let d = Self::default();
        match raw {
            None => d,
            Some(r) => Self {
                realtime: r.realtime.unwrap_or(d.realtime),
                timeonice: r.timeonice.unwrap_or(d.timeonice),
                goals_for_against: r.goals_for_against.unwrap_or(d.goals_for_against),
                goalie_advanced: r.goalie_advanced.unwrap_or(d.goalie_advanced),
                goalie_saves_by_strength: r
                    .goalie_saves_by_strength
                    .unwrap_or(d.goalie_saves_by_strength),
            },
        }
    }

    fn to_raw(self) -> RawReportToggles {
        RawReportToggles {
            realtime: Some(self.realtime),
            timeonice: Some(self.timeonice),
            goals_for_against: Some(self.goals_for_against),
            goalie_advanced: Some(self.goalie_advanced),
            goalie_saves_by_strength: Some(self.goalie_saves_by_strength),
        }
    }

    /// True iff `kind` is one of the five Tier-1 reports the overlay
    /// controls AND that toggle is on. Returns `false` for any kind
    /// outside the controllable set (the caller's bug if it asks about
    /// SkaterSummary or a Tier-2 report).
    pub fn is_enabled(self, kind: icelines_core::stats_catalog::ReportKind) -> bool {
        use icelines_core::stats_catalog::ReportKind::*;
        match kind {
            SkaterRealtime => self.realtime,
            SkaterTimeOnIce => self.timeonice,
            SkaterGoalsForAgainst => self.goals_for_against,
            GoalieAdvanced => self.goalie_advanced,
            GoalieSavesByStrength => self.goalie_saves_by_strength,
            // Core / Tier-2 — not controlled here.
            _ => false,
        }
    }

    /// True iff the column for `stat` should be visible. Stats whose
    /// `report_source()` is `None` (core / Tier-2 / derived) are always
    /// visible; stats backed by a toggleable Tier-1 report are visible
    /// only when that toggle is on.
    pub fn is_stat_visible(self, stat: icelines_core::stats_catalog::StatId) -> bool {
        match stat.report_source() {
            None => true,
            Some(kind) => self.is_enabled(kind),
        }
    }

    /// Mutate the toggle for `kind`. No-op for kinds outside the
    /// controllable set (Reports overlay only iterates the 5
    /// controllable kinds via `controllable_kinds()`).
    pub fn set(&mut self, kind: icelines_core::stats_catalog::ReportKind, enabled: bool) {
        use icelines_core::stats_catalog::ReportKind::*;
        match kind {
            SkaterRealtime => self.realtime = enabled,
            SkaterTimeOnIce => self.timeonice = enabled,
            SkaterGoalsForAgainst => self.goals_for_against = enabled,
            GoalieAdvanced => self.goalie_advanced = enabled,
            GoalieSavesByStrength => self.goalie_saves_by_strength = enabled,
            _ => {}
        }
    }

    /// The five Tier-1 reports the overlay can toggle, in display order.
    /// Iteration in this exact sequence keeps the overlay rendering
    /// deterministic across renders (no HashMap iteration order).
    pub fn controllable_kinds() -> &'static [icelines_core::stats_catalog::ReportKind] {
        use icelines_core::stats_catalog::ReportKind::*;
        &[
            SkaterRealtime,
            SkaterTimeOnIce,
            SkaterGoalsForAgainst,
            GoalieAdvanced,
            GoalieSavesByStrength,
        ]
    }
}

// ── Phase Foster.0.7 — sync + capability matrix typed forms ──────────────────

/// One of three modes per capability. `Off` skips the dataset entirely;
/// `Favorites` fetches only for entities in the user's Favorites group;
/// `League` fetches the entire NHL slate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityMode {
    Off,
    Favorites,
    League,
}

impl CapabilityMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Favorites => "favorites",
            Self::League => "league",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CapabilityError> {
        match s {
            "off" => Ok(Self::Off),
            "favorites" => Ok(Self::Favorites),
            "league" => Ok(Self::League),
            other => Err(CapabilityError::UnknownMode(other.to_string())),
        }
    }
}

/// The 6 dataset capabilities the sync engine cares about. Ordered to
/// match the matrix display in CLI / setup wizard output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Stats,
    ScoresSchedule,
    Transactions,
    Boxscores,
    Shifts,
    CareerHistory,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stats => "stats",
            Self::ScoresSchedule => "scores_schedule",
            Self::Transactions => "transactions",
            Self::Boxscores => "boxscores",
            Self::Shifts => "shifts",
            Self::CareerHistory => "career_history",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CapabilityError> {
        match s {
            "stats" => Ok(Self::Stats),
            "scores_schedule" => Ok(Self::ScoresSchedule),
            "transactions" => Ok(Self::Transactions),
            "boxscores" => Ok(Self::Boxscores),
            "shifts" => Ok(Self::Shifts),
            "career_history" => Ok(Self::CareerHistory),
            other => Err(CapabilityError::UnknownCapability(other.to_string())),
        }
    }

    pub fn all() -> &'static [Capability] {
        &[
            Self::Stats,
            Self::ScoresSchedule,
            Self::Transactions,
            Self::Boxscores,
            Self::Shifts,
            Self::CareerHistory,
        ]
    }
}

/// Resolved capability matrix. Defaults documented in the spec
/// (`stats=league`, `scores_schedule=league`, `transactions=favorites`,
/// `boxscores=favorites`, `shifts=off`, `career_history=favorites`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityMatrix {
    pub stats: CapabilityMode,
    pub scores_schedule: CapabilityMode,
    pub transactions: CapabilityMode,
    pub boxscores: CapabilityMode,
    pub shifts: CapabilityMode,
    pub career_history: CapabilityMode,
}

impl Default for CapabilityMatrix {
    fn default() -> Self {
        Self {
            stats: CapabilityMode::League,
            scores_schedule: CapabilityMode::League,
            transactions: CapabilityMode::Favorites,
            boxscores: CapabilityMode::Favorites,
            shifts: CapabilityMode::Off,
            career_history: CapabilityMode::Favorites,
        }
    }
}

impl CapabilityMatrix {
    pub fn get(&self, cap: Capability) -> CapabilityMode {
        match cap {
            Capability::Stats => self.stats,
            Capability::ScoresSchedule => self.scores_schedule,
            Capability::Transactions => self.transactions,
            Capability::Boxscores => self.boxscores,
            Capability::Shifts => self.shifts,
            Capability::CareerHistory => self.career_history,
        }
    }

    /// Apply a new mode. `shifts` enforces off-only with the literal
    /// error string the spec requires (BENCH H3 — capability-matrix
    /// tests pin the wording).
    pub fn set(&mut self, cap: Capability, mode: CapabilityMode) -> Result<(), CapabilityError> {
        if matches!(cap, Capability::Shifts) && !matches!(mode, CapabilityMode::Off) {
            return Err(CapabilityError::ShiftsLocked {
                chosen: mode.as_str().to_string(),
            });
        }
        match cap {
            Capability::Stats => self.stats = mode,
            Capability::ScoresSchedule => self.scores_schedule = mode,
            Capability::Transactions => self.transactions = mode,
            Capability::Boxscores => self.boxscores = mode,
            Capability::Shifts => self.shifts = mode,
            Capability::CareerHistory => self.career_history = mode,
        }
        Ok(())
    }

    /// True iff sync should fetch this dataset for `is_favorite` (the
    /// caller-supplied predicate). `League` mode → always; `Favorites`
    /// → only if the entity is favorited; `Off` → never.
    /// Foster.4's sync engine consumes this; tests in
    /// `foster_capability_matrix.rs` pin the truth table.
    #[allow(dead_code)]
    pub fn allowed(&self, cap: Capability, is_favorite: bool) -> bool {
        match self.get(cap) {
            CapabilityMode::Off => false,
            CapabilityMode::Favorites => is_favorite,
            CapabilityMode::League => true,
        }
    }

    fn from_raw(raw: Option<RawCapabilities>) -> Self {
        let d = Self::default();
        let Some(r) = raw else { return d };
        Self {
            stats: r
                .stats
                .as_deref()
                .and_then(|s| CapabilityMode::parse(s).ok())
                .unwrap_or(d.stats),
            scores_schedule: r
                .scores_schedule
                .as_deref()
                .and_then(|s| CapabilityMode::parse(s).ok())
                .unwrap_or(d.scores_schedule),
            transactions: r
                .transactions
                .as_deref()
                .and_then(|s| CapabilityMode::parse(s).ok())
                .unwrap_or(d.transactions),
            boxscores: r
                .boxscores
                .as_deref()
                .and_then(|s| CapabilityMode::parse(s).ok())
                .unwrap_or(d.boxscores),
            // shifts: silently coerced to Off if a stray non-off value
            // ends up on disk (e.g. hand-edited TOML from a future
            // build). The CLI `set` path enforces; the loader is
            // defensive.
            shifts: CapabilityMode::Off,
            career_history: r
                .career_history
                .as_deref()
                .and_then(|s| CapabilityMode::parse(s).ok())
                .unwrap_or(d.career_history),
        }
    }

    fn to_raw(self) -> RawCapabilities {
        RawCapabilities {
            stats: Some(self.stats.as_str().to_string()),
            scores_schedule: Some(self.scores_schedule.as_str().to_string()),
            transactions: Some(self.transactions.as_str().to_string()),
            boxscores: Some(self.boxscores.as_str().to_string()),
            shifts: Some(self.shifts.as_str().to_string()),
            career_history: Some(self.career_history.as_str().to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncPolicy {
    Eager,
    Lazy,
    Off,
}

impl SyncPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eager => "eager",
            Self::Lazy => "lazy",
            Self::Off => "off",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CapabilityError> {
        match s {
            "eager" => Ok(Self::Eager),
            "lazy" => Ok(Self::Lazy),
            "off" => Ok(Self::Off),
            other => Err(CapabilityError::UnknownSyncValue {
                key: "policy",
                value: other.to_string(),
                allowed: "eager, lazy, off",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BannerMode {
    Summary,
    Silent,
    Verbose,
}

impl BannerMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Silent => "silent",
            Self::Verbose => "verbose",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CapabilityError> {
        match s {
            "summary" => Ok(Self::Summary),
            "silent" => Ok(Self::Silent),
            "verbose" => Ok(Self::Verbose),
            other => Err(CapabilityError::UnknownSyncValue {
                key: "banner",
                value: other.to_string(),
                allowed: "summary, silent, verbose",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeasonTransition {
    Prompt,
    Auto,
    Ignore,
}

impl SeasonTransition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Auto => "auto",
            Self::Ignore => "ignore",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CapabilityError> {
        match s {
            "prompt" => Ok(Self::Prompt),
            "auto" => Ok(Self::Auto),
            "ignore" => Ok(Self::Ignore),
            other => Err(CapabilityError::UnknownSyncValue {
                key: "season_transition",
                value: other.to_string(),
                allowed: "prompt, auto, ignore",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncConfig {
    pub policy: SyncPolicy,
    pub banner: BannerMode,
    pub season_transition: SeasonTransition,
    pub capabilities: CapabilityMatrix,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            policy: SyncPolicy::Eager,
            banner: BannerMode::Summary,
            season_transition: SeasonTransition::Prompt,
            capabilities: CapabilityMatrix::default(),
        }
    }
}

impl SyncConfig {
    fn from_raw(raw: Option<RawSync>) -> Self {
        let d = Self::default();
        let Some(r) = raw else { return d };
        Self {
            policy: r
                .policy
                .as_deref()
                .and_then(|s| SyncPolicy::parse(s).ok())
                .unwrap_or(d.policy),
            banner: r
                .banner
                .as_deref()
                .and_then(|s| BannerMode::parse(s).ok())
                .unwrap_or(d.banner),
            season_transition: r
                .season_transition
                .as_deref()
                .and_then(|s| SeasonTransition::parse(s).ok())
                .unwrap_or(d.season_transition),
            capabilities: CapabilityMatrix::from_raw(r.capabilities),
        }
    }

    fn to_raw(self) -> RawSync {
        RawSync {
            policy: Some(self.policy.as_str().to_string()),
            banner: Some(self.banner.as_str().to_string()),
            season_transition: Some(self.season_transition.as_str().to_string()),
            capabilities: Some(self.capabilities.to_raw()),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapabilityError {
    #[error(
        "capability `shifts` cannot be set to `{chosen}` yet —\n       per-shift parsing isn't implemented. Reserved for a future\n       phase. Allowed values today: off"
    )]
    ShiftsLocked { chosen: String },

    #[error(
        "unknown capability '{0}' — expected one of: stats, scores_schedule, transactions, boxscores, shifts, career_history"
    )]
    UnknownCapability(String),

    #[error("unknown mode '{0}' — expected one of: off, favorites, league")]
    UnknownMode(String),

    #[error("unknown {key} value '{value}' — expected one of: {allowed}")]
    UnknownSyncValue {
        key: &'static str,
        value: String,
        allowed: &'static str,
    },

    #[error("unknown config key '{0}' — try `icelines config list`")]
    UnknownKey(String),
}

impl Config {
    /// Load configuration from `~/.icelines/config.toml`.
    ///
    /// If the file does not exist the function returns a default `Config`
    /// (not an error).  If the file exists but cannot be parsed, the error
    /// is propagated so the user sees a useful message.
    pub fn load() -> anyhow::Result<Self> {
        let home = home_dir()?;
        let config_path = home.join(".icelines").join("config.toml");

        let raw: RawConfig = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {}", config_path.display(), e))?;
            toml::from_str(&text).map_err(|e| {
                anyhow::anyhow!("invalid config at {}: {}", config_path.display(), e)
            })?
        } else {
            RawConfig::default()
        };

        let default_cache = home.join(".icelines").join("cache");

        Ok(Config {
            csv_path: raw.csv_path.map(PathBuf::from),
            cache_dir: raw.cache_dir.map(PathBuf::from).unwrap_or(default_cache),
            season: raw.season,
            live: raw.live,
            dashboards: raw.dashboards,
            reports: ReportToggles::from_raw(raw.reports),
            sync: SyncConfig::from_raw(raw.sync),
            ai: ai_from_raw(raw.ai),
        })
    }

    /// Phase Reports — persist the current `reports` toggles back to
    /// `~/.icelines/config.toml`. Round-trips the rest of the file:
    /// reads the existing TOML, replaces the `[reports]` section, and
    /// writes atomically via a temp file. Other config sections are
    /// preserved verbatim — even comments are stripped (toml-rs limit),
    /// so we keep the file machine-edited only.
    pub fn save_reports(&self) -> anyhow::Result<()> {
        let home = home_dir()?;
        let icelines_dir = home.join(".icelines");
        std::fs::create_dir_all(&icelines_dir)
            .map_err(|e| anyhow::anyhow!("cannot create {}: {}", icelines_dir.display(), e))?;
        let config_path = icelines_dir.join("config.toml");

        // Read current file (or default) so non-reports keys survive.
        let mut raw: RawConfig = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {}", config_path.display(), e))?;
            toml::from_str(&text).unwrap_or_default()
        } else {
            RawConfig::default()
        };
        raw.reports = Some(self.reports.to_raw());

        let serialized =
            toml::to_string_pretty(&raw).map_err(|e| anyhow::anyhow!("serialize config: {e}"))?;

        // Atomic write via temp + rename to avoid half-written files
        // on crash/power-loss between fopen and fclose.
        let tmp = config_path.with_extension("toml.tmp");
        std::fs::write(&tmp, serialized)
            .map_err(|e| anyhow::anyhow!("write {}: {}", tmp.display(), e))?;
        std::fs::rename(&tmp, &config_path)
            .map_err(|e| anyhow::anyhow!("rename {}: {}", config_path.display(), e))?;
        Ok(())
    }

    /// Phase Foster.0.7 — persist the current `sync` block back to
    /// `~/.icelines/config.toml`, preserving every other key. Mirror
    /// of `save_reports`.
    pub fn save_sync(&self) -> anyhow::Result<()> {
        let home = home_dir()?;
        let icelines_dir = home.join(".icelines");
        std::fs::create_dir_all(&icelines_dir)
            .map_err(|e| anyhow::anyhow!("cannot create {}: {}", icelines_dir.display(), e))?;
        let config_path = icelines_dir.join("config.toml");
        let mut raw: RawConfig = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {}", config_path.display(), e))?;
            toml::from_str(&text).unwrap_or_default()
        } else {
            RawConfig::default()
        };
        raw.sync = Some(self.sync.to_raw());
        let serialized =
            toml::to_string_pretty(&raw).map_err(|e| anyhow::anyhow!("serialize config: {e}"))?;
        let tmp = config_path.with_extension("toml.tmp");
        std::fs::write(&tmp, serialized)
            .map_err(|e| anyhow::anyhow!("write {}: {}", tmp.display(), e))?;
        std::fs::rename(&tmp, &config_path)
            .map_err(|e| anyhow::anyhow!("rename {}: {}", config_path.display(), e))?;
        Ok(())
    }

    /// Resolve a dotted config key to a string value. Used by
    /// `icelines config get`. Returns `Err(UnknownKey)` for keys we
    /// don't recognize so the CLI can surface a "did you mean"
    /// suggestion.
    pub fn get_key(&self, key: &str) -> Result<String, CapabilityError> {
        match key {
            "sync.policy" => Ok(self.sync.policy.as_str().to_string()),
            "sync.banner" => Ok(self.sync.banner.as_str().to_string()),
            "sync.season_transition" => Ok(self.sync.season_transition.as_str().to_string()),
            other => {
                if let Some(rest) = other.strip_prefix("sync.capabilities.") {
                    let cap = Capability::parse(rest)?;
                    return Ok(self.sync.capabilities.get(cap).as_str().to_string());
                }
                Err(CapabilityError::UnknownKey(other.to_string()))
            }
        }
    }

    /// Apply a dotted-key mutation in-memory. Persistence is the
    /// caller's responsibility (call `save_sync` after).
    pub fn set_key(&mut self, key: &str, value: &str) -> Result<(), CapabilityError> {
        match key {
            "sync.policy" => {
                self.sync.policy = SyncPolicy::parse(value)?;
                Ok(())
            }
            "sync.banner" => {
                self.sync.banner = BannerMode::parse(value)?;
                Ok(())
            }
            "sync.season_transition" => {
                self.sync.season_transition = SeasonTransition::parse(value)?;
                Ok(())
            }
            other => {
                if let Some(rest) = other.strip_prefix("sync.capabilities.") {
                    let cap = Capability::parse(rest)?;
                    let mode = CapabilityMode::parse(value)?;
                    return self.sync.capabilities.set(cap, mode);
                }
                Err(CapabilityError::UnknownKey(other.to_string()))
            }
        }
    }

    /// Reset a section to defaults. Recognized keys: `sync`,
    /// `sync.capabilities`. Returns `Err(UnknownKey)` otherwise.
    pub fn reset_key(&mut self, key: &str) -> Result<(), CapabilityError> {
        match key {
            "sync" => {
                self.sync = SyncConfig::default();
                Ok(())
            }
            "sync.capabilities" => {
                self.sync.capabilities = CapabilityMatrix::default();
                Ok(())
            }
            other => Err(CapabilityError::UnknownKey(other.to_string())),
        }
    }

    /// Enumerate every settable config key for `icelines config list`.
    pub fn list_keys(&self) -> Vec<(String, String)> {
        let mut v = vec![
            ("sync.policy".into(), self.sync.policy.as_str().to_string()),
            ("sync.banner".into(), self.sync.banner.as_str().to_string()),
            (
                "sync.season_transition".into(),
                self.sync.season_transition.as_str().to_string(),
            ),
        ];
        for cap in Capability::all() {
            v.push((
                format!("sync.capabilities.{}", cap.as_str()),
                self.sync.capabilities.get(*cap).as_str().to_string(),
            ));
        }
        v
    }

    /// Return the season as an 8-digit string (e.g. "20252026").
    pub fn season_str(&self) -> String {
        self.season
            .unwrap_or(icelines_core::CURRENT_SEASON)
            .to_string()
    }

    /// Directory for named snapshots: ~/.icelines/snapshots/
    pub fn snapshot_dir(&self) -> std::path::PathBuf {
        self.cache_dir
            .parent()
            .unwrap_or(&self.cache_dir)
            .join("snapshots")
    }

    /// Phase Calder.2 — path to the multi-league career-history blob.
    /// Single global file (not per-season) because a player's pre-NHL
    /// career doesn't change with the active season.
    pub fn career_history_path(&self) -> std::path::PathBuf {
        self.cache_dir
            .parent()
            .unwrap_or(&self.cache_dir)
            .join("career_history.json")
    }
}

// ── Live-feeds toggle (Phase 8f.1) ────────────────────────────────────────────

/// Process-wide flag for whether live NHL API fetches are allowed.
/// `Some(true)` = on, `Some(false)` = suppressed, `None` = unset (treat as on).
/// Set once at process startup by `init_live_feeds`; queried by
/// `live_feeds_enabled()` from any caller.
static LIVE_FEEDS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Resolve and stash the live-feeds boolean. Precedence (highest first):
/// 1. `--no-live` CLI flag (wins if set)
/// 2. `ICELINES_NO_LIVE` env var (any non-empty / non-zero value disables)
/// 3. `live = false` in `~/.icelines/config.toml`
/// 4. Default: live feeds ON
///
/// Idempotent — first call wins, subsequent calls are no-ops. Safe to call
/// from tests via `init_live_feeds_for_test`.
pub fn init_live_feeds(cli_no_live: bool, cfg: &Config) {
    let env_disable = std::env::var_os("ICELINES_NO_LIVE")
        .map(|v| {
            let s = v.to_string_lossy();
            !s.is_empty() && s != "0" && !s.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false);
    let resolved = if cli_no_live || env_disable {
        false
    } else {
        cfg.live.unwrap_or(true)
    };
    let _ = LIVE_FEEDS.set(resolved);
}

/// True iff live NHL API fetches are permitted.
/// Defaults to `true` if `init_live_feeds` was never called (e.g. unit tests
/// that bypass the CLI entry point — they shouldn't be hitting the network
/// anyway, but the default keeps existing tests' behavior unchanged).
pub fn live_feeds_enabled() -> bool {
    *LIVE_FEEDS.get().unwrap_or(&true)
}

/// Pure precedence resolver — exposed for tests so they can verify the
/// rule without poking the global OnceLock. `init_live_feeds` inlines
/// the same logic against env/config inputs.
#[allow(dead_code)] // production path uses init_live_feeds; tests use this.
pub fn resolve_live(cli_no_live: bool, env_no_live: bool, config_live: Option<bool>) -> bool {
    if cli_no_live {
        return false;
    }
    if env_no_live {
        return false;
    }
    if let Some(c) = config_live {
        return c;
    }
    true
}

// ── Dashboards feature flag (Phase 8j) ────────────────────────────────────────

/// Process-wide flag for whether experimental dashboard panels render.
/// Off by default — opt in via `--dashboards`, `ICELINES_DASHBOARDS=1`, or
/// `dashboards = true` in `~/.icelines/config.toml`.
static DASHBOARDS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Resolve and stash the dashboards boolean. Precedence (highest first):
/// 1. `--no-dashboards` CLI flag (force-off)
/// 2. `ICELINES_DASHBOARDS=0` env var (force-off)
/// 3. `dashboards = false` in `~/.icelines/config.toml`
/// 4. Default: dashboards ON.
///
/// Idempotent — first call wins, subsequent calls are no-ops.
pub fn init_dashboards(cli_no_dashboards: bool, cfg: &Config) {
    let env_off = std::env::var_os("ICELINES_DASHBOARDS")
        .map(|v| {
            let s = v.to_string_lossy();
            // Only an explicit falsy value disables; empty / unset / non-zero leaves it on.
            s == "0" || s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("off")
        })
        .unwrap_or(false);
    let resolved = if cli_no_dashboards || env_off {
        false
    } else {
        cfg.dashboards.unwrap_or(true)
    };
    let _ = DASHBOARDS.set(resolved);
}

/// True iff dashboard panels should render. Returns `true` when
/// `init_dashboards` was never called — matches the new on-by-default
/// behavior so tests that bypass the CLI entry point see panels by
/// default (most TUI render tests work with explicit fixture setups
/// that don't depend on this).
pub fn dashboards_enabled() -> bool {
    *DASHBOARDS.get().unwrap_or(&true)
}

/// Pure precedence resolver for tests — same rule as `init_dashboards`
/// but free of any global state. Inputs: `cli_no` is `--no-dashboards`,
/// `env_off` is `ICELINES_DASHBOARDS=0|false|off`, `config` is the
/// optional `dashboards` config-file key.
#[allow(dead_code)]
pub fn resolve_dashboards(cli_no: bool, env_off: bool, config: Option<bool>) -> bool {
    if cli_no {
        return false;
    }
    if env_off {
        return false;
    }
    if let Some(c) = config {
        return c;
    }
    true
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the current user's home directory.
fn home_dir() -> anyhow::Result<PathBuf> {
    // `std::env::home_dir` is deprecated (and unreliable on Windows in old Rust).
    // Use the HOME / USERPROFILE env vars directly — portable and straightforward.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory; set HOME or USERPROFILE"))
}

// ── Tests for live-feeds precedence (Phase 8f.1) ──────────────────────────────

#[cfg(test)]
mod live_feeds_tests {
    use super::*;

    #[test]
    fn l0_resolve_live_default_is_on() {
        assert!(resolve_live(false, false, None));
    }

    #[test]
    fn l0_resolve_live_cli_flag_wins_over_everything() {
        // CLI true → live disabled regardless of env / config
        assert!(!resolve_live(true, false, None));
        assert!(!resolve_live(true, false, Some(true)));
        assert!(!resolve_live(true, true, Some(true)));
    }

    #[test]
    fn l0_resolve_live_env_wins_over_config() {
        // env disable beats config = true
        assert!(!resolve_live(false, true, Some(true)));
        // env unset → config wins
        assert!(resolve_live(false, false, Some(true)));
        assert!(!resolve_live(false, false, Some(false)));
    }

    #[test]
    fn l0_resolve_live_config_only_when_no_higher_signal() {
        assert!(!resolve_live(false, false, Some(false)));
        assert!(resolve_live(false, false, Some(true)));
        // No config → default true
        assert!(resolve_live(false, false, None));
    }

    // ── Phase 8j: dashboards flag precedence (now opt-OUT — on by default) ──

    #[test]
    fn l0_resolve_dashboards_default_is_on() {
        // With no CLI flag, no env var, and no config — dashboards render.
        assert!(resolve_dashboards(false, false, None));
    }

    #[test]
    fn l0_resolve_dashboards_cli_no_wins_over_everything() {
        // `--no-dashboards` forces off no matter what env/config say.
        assert!(!resolve_dashboards(true, false, None));
        assert!(!resolve_dashboards(true, false, Some(true)));
        assert!(!resolve_dashboards(true, true, Some(true)));
    }

    #[test]
    fn l0_resolve_dashboards_env_off_wins_over_config() {
        // `ICELINES_DASHBOARDS=0` disables even with `dashboards = true`.
        assert!(!resolve_dashboards(false, true, Some(true)));
        // env unset → config wins.
        assert!(!resolve_dashboards(false, false, Some(false)));
        assert!(resolve_dashboards(false, false, Some(true)));
    }

    #[test]
    fn l0_resolve_dashboards_config_only_when_no_higher_signal() {
        assert!(resolve_dashboards(false, false, Some(true)));
        assert!(!resolve_dashboards(false, false, Some(false)));
        // No config → default true.
        assert!(resolve_dashboards(false, false, None));
    }
}

// ── Tests for Phase Reports — toggle round-trip ────────────────────────────

#[cfg(test)]
mod reports_tests {
    use super::*;

    #[test]
    fn l0_reports_default_realtime_only_on() {
        let d = ReportToggles::default();
        assert!(
            d.realtime,
            "realtime defaults ON (matches historical behavior)"
        );
        assert!(!d.timeonice);
        assert!(!d.goals_for_against);
        assert!(!d.goalie_advanced);
        assert!(!d.goalie_saves_by_strength);
    }

    #[test]
    fn l0_reports_from_raw_none_uses_defaults() {
        let r = ReportToggles::from_raw(None);
        assert_eq!(r, ReportToggles::default());
    }

    #[test]
    fn l0_reports_from_raw_partial_keeps_unset_defaults() {
        // User wrote only realtime=false; the rest stay at the default.
        let raw = RawReportToggles {
            realtime: Some(false),
            ..Default::default()
        };
        let r = ReportToggles::from_raw(Some(raw));
        assert!(!r.realtime);
        assert!(!r.timeonice);
        assert!(!r.goals_for_against);
    }

    #[test]
    fn l0_reports_to_raw_round_trip() {
        let r = ReportToggles {
            realtime: false,
            timeonice: true,
            goals_for_against: true,
            goalie_advanced: false,
            goalie_saves_by_strength: true,
        };
        let back = ReportToggles::from_raw(Some(r.to_raw()));
        assert_eq!(r, back);
    }

    /// Asserts the TOML serializer + deserializer agree on the
    /// `[reports]` section shape — catches a key rename (e.g. someone
    /// adds `#[serde(rename)]` and forgets the matching read path).
    #[test]
    fn l0_reports_is_enabled_matches_field() {
        use icelines_core::stats_catalog::ReportKind::*;
        let mut r = ReportToggles::default(); // realtime=on
        assert!(r.is_enabled(SkaterRealtime));
        assert!(!r.is_enabled(SkaterTimeOnIce));
        r.set(SkaterTimeOnIce, true);
        assert!(r.is_enabled(SkaterTimeOnIce));
        // Non-controllable kinds always read false.
        assert!(!r.is_enabled(SkaterSummary));
        assert!(!r.is_enabled(SkaterPuckPossessions));
    }

    #[test]
    fn l0_reports_is_stat_visible_core_always_on() {
        use icelines_core::stats_catalog::StatId;
        // All toggles off — core stats stay visible.
        let r = ReportToggles {
            realtime: false,
            timeonice: false,
            goals_for_against: false,
            goalie_advanced: false,
            goalie_saves_by_strength: false,
        };
        assert!(r.is_stat_visible(StatId::Goals));
        assert!(r.is_stat_visible(StatId::Points));
        assert!(r.is_stat_visible(StatId::TotalToiPerGame));
        assert!(r.is_stat_visible(StatId::Wins));
        assert!(r.is_stat_visible(StatId::SavePct));
    }

    #[test]
    fn l0_reports_is_stat_visible_tier1_gated_on_toggle() {
        use icelines_core::stats_catalog::StatId;
        let mut r = ReportToggles {
            realtime: false,
            timeonice: false,
            goals_for_against: false,
            goalie_advanced: false,
            goalie_saves_by_strength: false,
        };
        // All hidden when their backing report is off.
        assert!(!r.is_stat_visible(StatId::Hits));
        assert!(!r.is_stat_visible(StatId::PpToi));
        assert!(!r.is_stat_visible(StatId::EvGoalsFor));
        assert!(!r.is_stat_visible(StatId::QualityStarts));
        assert!(!r.is_stat_visible(StatId::EvSavePct));
        // Flip realtime on → Hits visible, others still hidden.
        r.realtime = true;
        assert!(r.is_stat_visible(StatId::Hits));
        assert!(!r.is_stat_visible(StatId::PpToi));
    }

    #[test]
    fn l0_reports_controllable_kinds_count_matches_struct_fields() {
        // Pin: the 5 fields on ReportToggles must match the 5
        // entries in controllable_kinds(). Adding a new field without
        // adding it here would silently drop the toggle from the UI.
        assert_eq!(ReportToggles::controllable_kinds().len(), 5);
    }

    #[test]
    fn l1_reports_toml_round_trip_keys_match() {
        let raw = RawConfig {
            reports: Some(RawReportToggles {
                realtime: Some(false),
                timeonice: Some(true),
                goals_for_against: Some(true),
                goalie_advanced: Some(false),
                goalie_saves_by_strength: Some(true),
            }),
            ..Default::default()
        };
        let s = toml::to_string(&raw).expect("serialize");
        assert!(
            s.contains("[reports]"),
            "must emit a [reports] table, got:\n{s}"
        );
        assert!(s.contains("realtime = false"));
        assert!(s.contains("timeonice = true"));
        assert!(s.contains("goals_for_against = true"));

        let back: RawConfig = toml::from_str(&s).expect("parse");
        let r = ReportToggles::from_raw(back.reports);
        assert!(!r.realtime);
        assert!(r.timeonice);
        assert!(r.goals_for_against);
        assert!(!r.goalie_advanced);
        assert!(r.goalie_saves_by_strength);
    }
}
