//! Phase Hart.2 — `StatsRepository` and `PlayerView`.
//!
//! The single owner of normalized data in memory. Keyed by:
//! - `identities: HashMap<PlayerId, PlayerIdentity>` — once per player ever
//! - `stats: HashMap<(PlayerId, Season, SeasonType), SeasonStats>` — per
//!   (player, season, type) row
//!
//! Two roster indexes (rebuilt incrementally on upsert, never persisted):
//! - `rosters_last_stint` — players whose **last** stint was on a given
//!   team in a given (season, type). Matches "current home" semantics.
//! - `rosters_all_stints` — every player who had ANY stint with a team
//!   that (season, type). Used by trade / historical views.
//!
//! Window LRU bounds memory at `lru_cap` resident (Season, SeasonType)
//! windows. Identities never evict (cheap and stable).
//!
//! `!Send + !Sync` by construction (`PhantomData<*const ()>`). Concurrent
//! upserts could tear the roster indexes; tokio-spawning callers must
//! wrap in `Arc<RwLock<_>>` at the call site.

use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

use static_assertions::assert_not_impl_any;
use thiserror::Error;

use crate::contract::PlayerContract;
use crate::identity::{IdentityMergeError, PlayerId, PlayerIdentity};
use crate::model::{PaceScore, Position, Season, TeamAbbr};
use crate::season_stats::SeasonStats;
use crate::season_stats::SeasonType;

/// Default LRU cap. Five bundled seasons × two types = 10 windows. Cap
/// of 8 leaves room for time-travel into older seasons after a `fetch`
/// install while bounding memory.
pub const DEFAULT_LRU_CAP: usize = 8;

/// Errors returned by repository mutators. Hart.3's `LoadError` (in
/// `icelines-fetch`) wraps this with `#[from]` so the loader's caller
/// gets a single error surface.
#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum RepoError {
    #[error("identity merge failed: {0}")]
    IdentityMerge(#[from] IdentityMergeError),
    #[error(
        "stats upserted before identity for player {id} at \
         {season}/{season_type}"
    )]
    StatsWithoutIdentity {
        id: PlayerId,
        season: Season,
        season_type: SeasonType,
    },
}

#[derive(Debug)]
pub struct StatsRepository {
    // pub(crate) so external code can't bypass roster index + LRU
    // bookkeeping by mutating the HashMaps directly. Read access via
    // `iter_identities()` / `iter_stats()`; mutation via `upsert_*`.
    pub(crate) identities: HashMap<PlayerId, PlayerIdentity>,
    pub(crate) stats: HashMap<(PlayerId, Season, SeasonType), SeasonStats>,
    /// Per-player contracts. Window-independent — contracts don't evict
    /// when a (season, type) window does, and contracts are not keyed
    /// by season. The current NHL landing API rarely populates these.
    pub(crate) contracts: HashMap<PlayerId, PlayerContract>,

    rosters_last_stint: HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>>,
    rosters_all_stints: HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>>,

    window_lru: VecDeque<(Season, SeasonType)>,
    lru_cap: usize,

    /// `*const ()` makes this type both `!Send` AND `!Sync`. Concurrent
    /// upserts can race the roster indexes; the static assertion below
    /// enforces this contract.
    _not_send_sync: PhantomData<*const ()>,
}

assert_not_impl_any!(StatsRepository: Send, Sync);

impl Default for StatsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsRepository {
    pub fn new() -> Self {
        Self::with_lru_cap(DEFAULT_LRU_CAP)
    }

    pub fn with_lru_cap(cap: usize) -> Self {
        assert!(cap > 0, "lru_cap must be > 0");
        Self {
            identities: HashMap::new(),
            stats: HashMap::new(),
            contracts: HashMap::new(),
            rosters_last_stint: HashMap::new(),
            rosters_all_stints: HashMap::new(),
            window_lru: VecDeque::with_capacity(cap),
            lru_cap: cap,
            _not_send_sync: PhantomData,
        }
    }

    pub fn lru_cap(&self) -> usize {
        self.lru_cap
    }

    /// Number of resident (season, type) windows.
    pub fn resident_windows(&self) -> usize {
        self.window_lru.len()
    }

    /// True iff this (season, type) window is currently resident.
    pub fn has_window(&self, s: Season, t: SeasonType) -> bool {
        self.window_lru.iter().any(|&w| w == (s, t))
    }

    // ── Single-row lookups ──────────────────────────────────────────────────

    pub fn identity(&self, id: PlayerId) -> Option<&PlayerIdentity> {
        self.identities.get(&id)
    }

    pub fn season(&self, id: PlayerId, s: Season, t: SeasonType) -> Option<&SeasonStats> {
        self.stats.get(&(id, s, t))
    }

    pub fn contract(&self, id: PlayerId) -> Option<&PlayerContract> {
        self.contracts.get(&id)
    }

    pub fn view(&self, id: PlayerId, s: Season, t: SeasonType) -> Option<PlayerView<'_>> {
        let identity = self.identities.get(&id)?;
        let stats = self.stats.get(&(id, s, t))?;
        let contract = self.contracts.get(&id);
        Some(PlayerView {
            identity,
            stats,
            contract,
        })
    }

    // ── Career iterators (TAPE: typed, never mixed) ─────────────────────────

    /// Regular-season rows for one player, ordered ascending by season.
    /// Returns `None` iff the player's identity is unknown to the repo;
    /// returns `Some(empty_iter)` for an identity that exists but has
    /// no Regular rows (e.g. a drafted prospect, or a player with only
    /// Playoff rows resident in the LRU window). Explicit `<'a>` lifetime
    /// per FORGE — survives 2024-edition RPIT-capture rule changes.
    pub fn career_regular<'a>(
        &'a self,
        id: PlayerId,
    ) -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> {
        self.identities.get(&id)?;
        Some(self.career_filtered(id, Some(SeasonType::Regular)))
    }

    pub fn career_playoff<'a>(
        &'a self,
        id: PlayerId,
    ) -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> {
        self.identities.get(&id)?;
        Some(self.career_filtered(id, Some(SeasonType::Playoff)))
    }

    pub fn career_all<'a>(
        &'a self,
        id: PlayerId,
    ) -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> {
        self.identities.get(&id)?;
        Some(self.career_filtered(id, None))
    }

    fn career_filtered(
        &self,
        id: PlayerId,
        filter: Option<SeasonType>,
    ) -> std::vec::IntoIter<&SeasonStats> {
        let mut rows: Vec<&SeasonStats> = self
            .stats
            .iter()
            .filter(|(&(pid, _, t), _)| pid == id && filter.is_none_or(|ft| t == ft))
            .map(|(_, s)| s)
            .collect();
        rows.sort_by(|a, b| {
            a.season
                .cmp(&b.season)
                .then_with(|| a.season_type.cmp(&b.season_type))
        });
        rows.into_iter()
    }

    // ── League / roster iterators ───────────────────────────────────────────

    /// Every player with stats in the given (season, type), as views.
    /// Order is unspecified — callers that want deterministic ordering
    /// should `.collect::<Vec>()` and sort. Explicit `<'a>` per FORGE.
    pub fn league<'a>(
        &'a self,
        s: Season,
        t: SeasonType,
    ) -> impl Iterator<Item = PlayerView<'a>> + 'a {
        self.stats
            .iter()
            .filter_map(move |(&(pid, ks, kt), stats)| {
                if ks == s && kt == t {
                    let identity = self.identities.get(&pid)?;
                    let contract = self.contracts.get(&pid);
                    Some(PlayerView {
                        identity,
                        stats,
                        contract,
                    })
                } else {
                    None
                }
            })
    }

    pub fn skaters<'a>(
        &'a self,
        s: Season,
        t: SeasonType,
    ) -> impl Iterator<Item = PlayerView<'a>> + 'a {
        self.league(s, t).filter(|v| !v.is_goalie())
    }

    pub fn goalies<'a>(
        &'a self,
        s: Season,
        t: SeasonType,
    ) -> impl Iterator<Item = PlayerView<'a>> + 'a {
        self.league(s, t).filter(|v| v.is_goalie())
    }

    // ── Iter accessors (safe read paths over the locked-down HashMaps) ──────

    /// Iterate every resident `PlayerIdentity`. Order is unspecified.
    pub fn iter_identities<'a>(&'a self) -> impl Iterator<Item = &'a PlayerIdentity> + 'a {
        self.identities.values()
    }

    /// Iterate every resident `SeasonStats` row. Order is unspecified.
    /// For (season, type)-scoped iteration use `league()` instead.
    pub fn iter_stats<'a>(&'a self) -> impl Iterator<Item = &'a SeasonStats> + 'a {
        self.stats.values()
    }

    /// Iterate every resident `(PlayerId, &PlayerContract)`. Order
    /// unspecified. Contracts don't evict with windows.
    pub fn iter_contracts<'a>(
        &'a self,
    ) -> impl Iterator<Item = (PlayerId, &'a PlayerContract)> + 'a {
        self.contracts.iter().map(|(k, v)| (*k, v))
    }

    pub fn identities_len(&self) -> usize {
        self.identities.len()
    }

    pub fn stats_len(&self) -> usize {
        self.stats.len()
    }

    pub fn contracts_len(&self) -> usize {
        self.contracts.len()
    }

    /// Players whose **last** stint was on `team` for (season, type).
    /// Matches today's "current home" UI semantics — the lineup card.
    pub fn team_roster(&self, team: &TeamAbbr, s: Season, t: SeasonType) -> Vec<PlayerView<'_>> {
        self.rosters_last_stint
            .get(&(s, t, team.clone()))
            .map(|ids| {
                ids.iter()
                    .filter_map(|&id| self.view(id, s, t))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    /// Every player who had ANY stint with `team` for (season, type).
    /// Includes players traded in or out mid-window. Used by trade /
    /// historical views.
    pub fn team_roster_all_stints(
        &self,
        team: &TeamAbbr,
        s: Season,
        t: SeasonType,
    ) -> Vec<PlayerView<'_>> {
        self.rosters_all_stints
            .get(&(s, t, team.clone()))
            .map(|ids| {
                ids.iter()
                    .filter_map(|&id| self.view(id, s, t))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    // ── Mutators (loader path) ──────────────────────────────────────────────

    /// Insert or replace a `PlayerContract` for the given player.
    /// Idempotent — same contract twice is a no-op. Window-independent
    /// (contracts don't evict with stats windows). No identity guard
    /// because contracts can land before bios in some loader paths.
    pub fn upsert_contract(&mut self, id: PlayerId, contract: PlayerContract) {
        self.contracts.insert(id, contract);
    }

    /// Insert or merge a `PlayerIdentity`. New IDs insert directly;
    /// existing IDs run through `PlayerIdentity::merge_with`'s
    /// sanity-floor and reissue-detection policy. A `LikelyIdReissue`
    /// propagates as `RepoError::IdentityMerge`.
    pub fn upsert_identity(&mut self, identity: PlayerIdentity) -> Result<(), RepoError> {
        match self.identities.get_mut(&identity.id) {
            Some(prior) => prior.merge_with(identity)?,
            None => {
                self.identities.insert(identity.id, identity);
            }
        }
        Ok(())
    }

    /// Insert or replace a `SeasonStats` row. Errors if no identity
    /// exists for the player_id (loader contract: identity rows always
    /// land first). Updates the roster indexes incrementally and touches
    /// the LRU; if the new row introduces a fresh (season, type) window
    /// that pushes us over `lru_cap`, the LRU window is evicted.
    pub fn upsert_stats(&mut self, stats: SeasonStats) -> Result<(), RepoError> {
        if !self.identities.contains_key(&stats.player_id) {
            return Err(RepoError::StatsWithoutIdentity {
                id: stats.player_id,
                season: stats.season,
                season_type: stats.season_type,
            });
        }

        let key = (stats.player_id, stats.season, stats.season_type);
        let window = (stats.season, stats.season_type);

        // 1. Remove old roster-index entries for this key (if replacing).
        // Clone the old team_stints out and drop the immutable borrow
        // before mutating the index hashmaps.
        let old_team_stints = self.stats.get(&key).map(|s| s.team_stints.clone());
        if let Some(stints) = old_team_stints {
            self.unindex_rosters_for(stats.player_id, stats.season, stats.season_type, &stints);
        }

        // 2. Touch LRU. If this is a fresh window, push and possibly evict.
        self.touch_window_lru(window);

        // 3. Insert new stats; index rosters from new stints.
        self.index_rosters(&stats);
        self.stats.insert(key, stats);

        Ok(())
    }

    fn touch_window_lru(&mut self, window: (Season, SeasonType)) {
        if let Some(pos) = self.window_lru.iter().position(|&w| w == window) {
            // Move existing window to the back (MRU).
            self.window_lru.remove(pos);
            self.window_lru.push_back(window);
            return;
        }
        // Fresh window. Evict front if at cap.
        if self.window_lru.len() >= self.lru_cap {
            if let Some(evicted) = self.window_lru.pop_front() {
                self.evict_window(evicted);
            }
        }
        self.window_lru.push_back(window);
    }

    fn evict_window(&mut self, (season, season_type): (Season, SeasonType)) {
        // Drop every stats row keyed to this window.
        self.stats
            .retain(|&(_, s, t), _| !(s == season && t == season_type));
        // Drop roster indexes for this window.
        self.rosters_last_stint
            .retain(|&(s, t, _), _| !(s == season && t == season_type));
        self.rosters_all_stints
            .retain(|&(s, t, _), _| !(s == season && t == season_type));
    }

    fn index_rosters(&mut self, stats: &SeasonStats) {
        let pid = stats.player_id;
        let s = stats.season;
        let t = stats.season_type;

        // Dedup on insert. A re-acquired-mid-window player's stints can
        // be e.g. [STL, NYR, STL]; without this guard, STL would
        // double-push and `team_roster_all_stints("STL", ...)` would
        // double-count. The retain-based unindex correctly removes a
        // single occurrence regardless, so unindex stays unchanged.
        for stint in &stats.team_stints {
            let key = (s, t, stint.team.clone());
            let entry = self.rosters_all_stints.entry(key).or_default();
            if !entry.contains(&pid) {
                entry.push(pid);
            }
        }

        if let Some(last) = stats.team_stints.last() {
            let key = (s, t, last.team.clone());
            let entry = self.rosters_last_stint.entry(key).or_default();
            if !entry.contains(&pid) {
                entry.push(pid);
            }
        }
    }

    fn unindex_rosters_for(
        &mut self,
        pid: PlayerId,
        s: Season,
        t: SeasonType,
        stints: &[crate::season_stats::TeamStint],
    ) {
        for stint in stints {
            let key = (s, t, stint.team.clone());
            if let Some(v) = self.rosters_all_stints.get_mut(&key) {
                v.retain(|&id| id != pid);
                if v.is_empty() {
                    self.rosters_all_stints.remove(&key);
                }
            }
        }

        if let Some(last) = stints.last() {
            let key = (s, t, last.team.clone());
            if let Some(v) = self.rosters_last_stint.get_mut(&key) {
                v.retain(|&id| id != pid);
                if v.is_empty() {
                    self.rosters_last_stint.remove(&key);
                }
            }
        }
    }

    // ── Atomic replacement ──────────────────────────────────────────────────

    /// Swap the entire repository state. Returns the old repo so callers
    /// can drop, inspect, or roll back. All currently-borrowed
    /// `PlayerView`s are invalidated by the swap; render paths must drop
    /// borrows before calling. The borrow checker enforces this — any
    /// outstanding view holds `&self`, and `repo_swap` requires
    /// `&mut self`, so the call is rejected at compile time.
    ///
    /// ```compile_fail
    /// use icelines_core::stats_repository::StatsRepository;
    /// use icelines_core::identity::PlayerId;
    /// use icelines_core::model::Season;
    /// use icelines_core::season_stats::SeasonType;
    ///
    /// let mut repo = StatsRepository::new();
    /// let view = repo.view(PlayerId(1), Season(20232024), SeasonType::Regular);
    /// repo.repo_swap(StatsRepository::new()); // cannot borrow as mutable
    /// drop(view); // load-bearing: keeps `view`'s &-borrow alive across
    ///             // the swap call. Without it NLL would shorten the
    ///             // borrow and the example would compile, silently
    ///             // turning this contract test into a tautology.
    /// ```
    pub fn repo_swap(&mut self, new_repo: StatsRepository) -> StatsRepository {
        std::mem::replace(self, new_repo)
    }
}

/// Borrowed projection over `(PlayerIdentity, SeasonStats, PlayerContract?)`.
/// Render code never sees raw structs — accesses go through these
/// accessors. Lifetime is render-frame-scoped: a `PlayerView` MUST NOT
/// outlive a frame, and MUST NOT be a field of any struct whose lifetime
/// exceeds one render pass. (Hart.0 surveyed for current Player-storage
/// patterns; none port to view-storage.)
#[derive(Debug, Clone, Copy)]
pub struct PlayerView<'a> {
    pub identity: &'a PlayerIdentity,
    pub stats: &'a SeasonStats,
    pub contract: Option<&'a PlayerContract>,
}

impl PlayerView<'_> {
    pub fn id(&self) -> PlayerId {
        self.identity.id
    }

    pub fn full_name(&self) -> &str {
        &self.identity.full_name
    }

    pub fn name_normalized(&self) -> &str {
        &self.identity.name_normalized
    }

    /// Per-season position. Marchand C→LW round-trips correctly because
    /// position is per-season-fact (TAPE).
    pub fn position(&self) -> Position {
        self.stats.position
    }

    pub fn sweater_number(&self) -> Option<u32> {
        self.stats.sweater_number
    }

    /// Last-stint team — "current home" semantics matching today's UI.
    pub fn team(&self) -> Option<&TeamAbbr> {
        self.stats.team_stints.last().map(|s| &s.team)
    }

    /// Render-side helper — always returns a string. Empty stints
    /// (impossible per the builder invariant but defensive) → em-dash.
    pub fn team_display(&self) -> &str {
        self.team().map(|t| t.0.as_str()).unwrap_or("—")
    }

    /// True iff this player suited up for more than one team in this
    /// (season, type). Drives the "trade banner" decision in the player
    /// card (GLASS).
    pub fn was_traded_in_window(&self) -> bool {
        self.stats.team_stints.len() > 1
    }

    pub fn goals(&self) -> u32 {
        self.stats.totals.goals
    }
    pub fn assists(&self) -> u32 {
        self.stats.totals.assists
    }
    pub fn points(&self) -> u32 {
        self.stats.totals.points
    }
    pub fn gp(&self) -> u32 {
        self.stats.totals.gp
    }
    pub fn plus_minus(&self) -> i32 {
        self.stats.totals.plus_minus
    }
    pub fn shots(&self) -> u32 {
        self.stats.totals.shots
    }

    pub fn pace_score(&self) -> Option<&PaceScore> {
        self.stats.totals.pace_score.as_ref()
    }

    pub fn is_goalie(&self) -> bool {
        self.stats.is_goalie()
    }

    // WIRE: realtime/advanced read through Option-at-leaf accessors,
    // never `.unwrap()`. None during cold-start / partial fetch.

    pub fn hits(&self) -> Option<u32> {
        self.stats.realtime.as_ref().map(|r| r.hits)
    }
    pub fn blocked_shots(&self) -> Option<u32> {
        self.stats.realtime.as_ref().map(|r| r.blocked_shots)
    }
    pub fn takeaways(&self) -> Option<u32> {
        self.stats.realtime.as_ref().map(|r| r.takeaways)
    }
    pub fn giveaways(&self) -> Option<u32> {
        self.stats.realtime.as_ref().map(|r| r.giveaways)
    }

    pub fn xg(&self) -> Option<f64> {
        self.stats.advanced.as_ref().and_then(|a| a.xg)
    }
    pub fn xg_per_60(&self) -> Option<f64> {
        self.stats.advanced.as_ref().and_then(|a| a.xg_per_60)
    }
    pub fn cf_pct(&self) -> Option<f64> {
        self.stats.advanced.as_ref().and_then(|a| a.cf_pct)
    }
    pub fn ff_pct(&self) -> Option<f64> {
        self.stats.advanced.as_ref().and_then(|a| a.ff_pct)
    }
    pub fn xgf_pct(&self) -> Option<f64> {
        self.stats.advanced.as_ref().and_then(|a| a.xgf_pct)
    }

    // ── Contract accessors (Hart.3) ────────────────────────────────────────
    //
    // None during cold-start / when the loader didn't fetch contracts.
    // The current NHL landing API does not populate these fields; values
    // are typically None even when a contract row exists.

    pub fn contract_expiry_year(&self) -> Option<u16> {
        self.contract.and_then(|c| c.expiry_year)
    }
    pub fn contract_expiry_type(&self) -> Option<&str> {
        self.contract.and_then(|c| c.expiry_type.as_deref())
    }
    pub fn contract_salary(&self) -> Option<u64> {
        self.contract.and_then(|c| c.salary)
    }

    // ── Hart.5b2 prep — derived helpers matching legacy Player methods ──
    //
    // These are added so the Hart.5b2 consumer refactor becomes a
    // mechanical rename (`p.foo()` → `view.foo()`) rather than per-site
    // re-derivation. Semantics match the legacy Player impl in model.rs.

    /// True iff this player's stats include a pace_score (legacy
    /// `Player::is_rankable`).
    pub fn is_rankable(&self) -> bool {
        self.pace_score().is_some()
    }

    /// Power-play assists = pp_points - pp_goals.
    pub fn pp_assists(&self) -> u32 {
        self.stats
            .totals
            .pp_points
            .saturating_sub(self.stats.totals.pp_goals)
    }

    /// Per-82 power-play points. None when gp == 0.
    pub fn pp_points_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        if gp == 0 {
            None
        } else {
            Some(self.stats.totals.pp_points as f64 / gp as f64 * 82.0)
        }
    }

    pub fn pp_goals_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        if gp == 0 {
            None
        } else {
            Some(self.stats.totals.pp_goals as f64 / gp as f64 * 82.0)
        }
    }

    pub fn sh_goals_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        if gp == 0 {
            None
        } else {
            Some(self.stats.totals.sh_goals as f64 / gp as f64 * 82.0)
        }
    }

    pub fn gwg_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        if gp == 0 {
            None
        } else {
            Some(self.stats.totals.gwg as f64 / gp as f64 * 82.0)
        }
    }

    pub fn shots_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        if gp == 0 {
            None
        } else {
            Some(self.stats.totals.shots as f64 / gp as f64 * 82.0)
        }
    }

    pub fn hits_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        let hits = self.hits()?;
        if gp == 0 {
            None
        } else {
            Some(hits as f64 / gp as f64 * 82.0)
        }
    }

    pub fn blocked_shots_per_82(&self) -> Option<f64> {
        let gp = self.gp();
        let b = self.blocked_shots()?;
        if gp == 0 {
            None
        } else {
            Some(b as f64 / gp as f64 * 82.0)
        }
    }

    /// TOI per game formatted as "MM:SS", or None if unavailable.
    pub fn toi_mmss(&self) -> Option<String> {
        let sec = self.stats.totals.toi_per_game_sec?;
        Some(format!("{:02}:{:02}", sec / 60, sec % 60))
    }

    /// Pace-projected points — convenience accessor for the
    /// most-common rank/sort key.
    pub fn pace_82(&self) -> Option<f64> {
        self.pace_score().map(|s| s.pace_82)
    }

    pub fn goals_per_82(&self) -> Option<f64> {
        self.pace_score().map(|s| s.goals_per_82)
    }

    /// Sort key for pace-based leaderboards. Mirrors
    /// `PaceScore::sort_key` for views without a pace score
    /// (returns 0.0 — sort places them last).
    pub fn pace_sort_key(&self) -> f64 {
        self.pace_score().map(|s| s.sort_key()).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::season_stats::{GoalieSeasonStats, SeasonStatsBuilder, StatTotals, TeamStint};

    fn skater_stats(player_id: u32, season: u32, t: SeasonType, team: &str) -> SeasonStats {
        fixtures::stats(player_id, season, team)
            .season_type(t)
            .build()
    }

    fn make_repo_with_player(player_id: u32) -> StatsRepository {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(player_id).build())
            .unwrap();
        r
    }

    // ── Send/Sync negative ──────────────────────────────────────────────────

    #[test]
    fn l0_hart2_repository_default_constructs_empty() {
        let r = StatsRepository::new();
        assert_eq!(r.lru_cap(), DEFAULT_LRU_CAP);
        assert_eq!(r.resident_windows(), 0);
    }

    #[test]
    #[should_panic(expected = "lru_cap must be > 0")]
    fn l0_hart2_with_lru_cap_zero_panics() {
        let _ = StatsRepository::with_lru_cap(0);
    }

    // ── Single-row lookups ──────────────────────────────────────────────────

    #[test]
    fn l0_hart2_view_returns_some_after_identity_and_stats() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        let v = r.view(PlayerId(8478402), Season(20222023), SeasonType::Regular);
        assert!(v.is_some());
        let v = v.unwrap();
        assert_eq!(v.id(), PlayerId(8478402));
        assert_eq!(v.full_name(), "Connor McDavid");
        assert_eq!(v.team_display(), "EDM");
        assert_eq!(v.gp(), 70);
        // Single-stint → was_traded_in_window must be false.
        assert!(!v.was_traded_in_window());
        // The fixture seeds a pace_score; accessor should surface it.
        let pace = v.pace_score().expect("fixture pace_score is Some");
        assert_eq!(pace.raw_points, 80);
        assert_eq!(pace.gp, 70);
    }

    #[test]
    fn l0_hart2_view_returns_none_for_missing_stats() {
        let r = make_repo_with_player(8478402);
        let v = r.view(PlayerId(8478402), Season(20222023), SeasonType::Regular);
        assert!(v.is_none(), "no stats yet — view must be None");
    }

    /// B5: full key tuple must match — Regular stats don't satisfy a
    /// Playoff query for the same player/season. Catches a bug where
    /// only the player_id is consulted.
    #[test]
    fn l0_hart2_view_none_for_other_season_type() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        let regular = r.view(PlayerId(8478402), Season(20222023), SeasonType::Regular);
        let playoff = r.view(PlayerId(8478402), Season(20222023), SeasonType::Playoff);
        let other_season = r.view(PlayerId(8478402), Season(20212022), SeasonType::Regular);
        assert!(regular.is_some());
        assert!(
            playoff.is_none(),
            "playoff query must not match a Regular row"
        );
        assert!(other_season.is_none(), "other-season query must not match");
    }

    #[test]
    fn l0_hart2_upsert_stats_without_identity_errors() {
        let mut r = StatsRepository::new();
        let err = r
            .upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap_err();
        assert!(matches!(err, RepoError::StatsWithoutIdentity { .. }));
    }

    #[test]
    fn l0_hart2_upsert_identity_merge_propagates_reissue() {
        let mut r = StatsRepository::new();
        let prior = fixtures::identity(8478402)
            .rookie_season("20152016")
            .build();
        r.upsert_identity(prior).unwrap();
        let mismatched = fixtures::identity(8478402)
            .rookie_season("20212022")
            .build();
        let err = r.upsert_identity(mismatched).unwrap_err();
        assert!(matches!(
            err,
            RepoError::IdentityMerge(IdentityMergeError::LikelyIdReissue { .. })
        ));
    }

    // ── Career iterators ────────────────────────────────────────────────────

    fn seed_career(r: &mut StatsRepository, pid: u32) {
        r.upsert_identity(fixtures::identity(pid).build()).unwrap();
        r.upsert_stats(skater_stats(pid, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(pid, 20212022, SeasonType::Playoff, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(pid, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(pid, 20222023, SeasonType::Playoff, "EDM"))
            .unwrap();
    }

    #[test]
    fn l0_hart2_career_regular_excludes_playoff() {
        let mut r = StatsRepository::new();
        seed_career(&mut r, 8478402);
        let it = r.career_regular(PlayerId(8478402)).unwrap();
        let rows: Vec<_> = it.collect();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.season_type, SeasonType::Regular);
        }
    }

    #[test]
    fn l0_hart2_career_playoff_excludes_regular() {
        let mut r = StatsRepository::new();
        seed_career(&mut r, 8478402);
        let it = r.career_playoff(PlayerId(8478402)).unwrap();
        let rows: Vec<_> = it.collect();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.season_type, SeasonType::Playoff);
        }
    }

    #[test]
    fn l0_hart2_career_all_includes_both_types_sorted() {
        let mut r = StatsRepository::new();
        seed_career(&mut r, 8478402);
        let rows: Vec<_> = r.career_all(PlayerId(8478402)).unwrap().collect();
        assert_eq!(rows.len(), 4);
        // Sorted by (season, type) — Regular < Playoff per derive order.
        assert_eq!(rows[0].season, Season(20212022));
        assert_eq!(rows[0].season_type, SeasonType::Regular);
        assert_eq!(rows[1].season, Season(20212022));
        assert_eq!(rows[1].season_type, SeasonType::Playoff);
        assert_eq!(rows[2].season, Season(20222023));
        assert_eq!(rows[2].season_type, SeasonType::Regular);
        assert_eq!(rows[3].season, Season(20222023));
        assert_eq!(rows[3].season_type, SeasonType::Playoff);
    }

    #[test]
    fn l0_hart2_career_returns_none_for_unknown_player() {
        let r = StatsRepository::new();
        assert!(r.career_regular(PlayerId(99)).is_none());
        assert!(r.career_playoff(PlayerId(99)).is_none());
        assert!(r.career_all(PlayerId(99)).is_none());
    }

    /// B4: identity exists, but only Playoff rows resident. Per the
    /// docstring contract, `career_regular` returns `Some(empty_iter)`
    /// — not `None` (None means unknown identity).
    #[test]
    fn l0_hart2_career_regular_empty_when_only_playoff_rows() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Playoff, "EDM"))
            .unwrap();

        let reg: Vec<_> = r.career_regular(PlayerId(8478402)).unwrap().collect();
        let pla: Vec<_> = r.career_playoff(PlayerId(8478402)).unwrap().collect();
        assert!(reg.is_empty(), "Some(empty), not None");
        assert_eq!(pla.len(), 1);
    }

    // ── League / skaters / goalies ──────────────────────────────────────────

    fn goalie_stats(pid: u32, season: u32, t: SeasonType, team: &str) -> SeasonStats {
        let stint = TeamStint {
            team: TeamAbbr(team.into()),
            started: Some("2022-10-15".into()),
            ended: Some("2023-04-13".into()),
            gp: 50,
            goals: 0,
            assists: 0,
            points: 0,
            goalie: None,
        };
        SeasonStatsBuilder::new(PlayerId(pid), Season(season), t, Position::Goalie)
            .add_team_stint(stint)
            .with_totals(StatTotals {
                gp: 50,
                ..Default::default()
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
            .build()
    }

    #[test]
    fn l0_hart2_goalies_filter_separates_from_skaters() {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(8478402).build())
            .unwrap();
        r.upsert_identity(fixtures::identity(8478406).build())
            .unwrap();

        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(goalie_stats(8478406, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();

        let skaters: Vec<_> = r.skaters(Season(20222023), SeasonType::Regular).collect();
        let goalies: Vec<_> = r.goalies(Season(20222023), SeasonType::Regular).collect();
        let league: Vec<_> = r.league(Season(20222023), SeasonType::Regular).collect();

        assert_eq!(skaters.len(), 1);
        assert_eq!(skaters[0].id(), PlayerId(8478402));
        assert_eq!(goalies.len(), 1);
        assert_eq!(goalies[0].id(), PlayerId(8478406));
        assert!(goalies[0].is_goalie());
        assert_eq!(league.len(), 2);
    }

    // ── Roster indexes — last_stint vs all_stints ───────────────────────────

    fn traded_skater_stats(pid: u32, season: u32, from: &str, to: &str) -> SeasonStats {
        let s1 = TeamStint {
            team: TeamAbbr(from.into()),
            started: Some("2022-10-15".into()),
            ended: Some("2023-02-09".into()),
            gp: 38,
            goals: 10,
            assists: 11,
            points: 21,
            goalie: None,
        };
        let s2 = TeamStint {
            team: TeamAbbr(to.into()),
            started: Some("2023-02-10".into()),
            ended: Some("2023-04-13".into()),
            gp: 31,
            goals: 8,
            assists: 13,
            points: 21,
            goalie: None,
        };
        SeasonStatsBuilder::new(
            PlayerId(pid),
            Season(season),
            SeasonType::Regular,
            Position::RightWing,
        )
        .add_team_stint(s1)
        .add_team_stint(s2)
        .with_totals(StatTotals {
            gp: 69,
            goals: 18,
            assists: 24,
            points: 42,
            ..Default::default()
        })
        .build()
    }

    #[test]
    fn l0_hart2_team_roster_last_stint_vs_all_stints_for_traded_player() {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(8475765).build())
            .unwrap();
        r.upsert_stats(traded_skater_stats(8475765, 20222023, "STL", "NYR"))
            .unwrap();

        let stl_last = r.team_roster(
            &TeamAbbr("STL".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr_last = r.team_roster(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let stl_all = r.team_roster_all_stints(
            &TeamAbbr("STL".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr_all = r.team_roster_all_stints(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );

        assert_eq!(
            stl_last.len(),
            0,
            "last-stint excludes the traded-from team"
        );
        assert_eq!(
            nyr_last.len(),
            1,
            "last-stint includes only the traded-to team"
        );
        assert_eq!(stl_all.len(), 1, "all-stints includes the traded-from team");
        assert_eq!(nyr_all.len(), 1, "all-stints includes the traded-to team");
    }

    #[test]
    fn l0_hart2_view_was_traded_in_window_flag() {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(8475765).build())
            .unwrap();
        r.upsert_stats(traded_skater_stats(8475765, 20222023, "STL", "NYR"))
            .unwrap();
        let v = r
            .view(PlayerId(8475765), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert!(v.was_traded_in_window());
        assert_eq!(v.team_display(), "NYR");
    }

    #[test]
    fn l0_hart2_replacing_stats_rebuilds_roster_indexes() {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(8475765).build())
            .unwrap();
        r.upsert_stats(traded_skater_stats(8475765, 20222023, "STL", "NYR"))
            .unwrap();
        // Now the player gets traded again to BOS, replacing the prior row.
        r.upsert_stats(traded_skater_stats(8475765, 20222023, "NYR", "BOS"))
            .unwrap();

        let stl_all = r.team_roster_all_stints(
            &TeamAbbr("STL".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr_all = r.team_roster_all_stints(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let bos_all = r.team_roster_all_stints(
            &TeamAbbr("BOS".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(stl_all.len(), 0, "old STL roster entry removed");
        assert_eq!(
            nyr_all.len(),
            1,
            "NYR is still in the all-stints index (now via the new row's first stint)"
        );
        assert_eq!(bos_all.len(), 1, "BOS gets the new last-stint");
    }

    /// EDGE+TAPE: a re-acquired-mid-window player has stints
    /// `[STL, NYR, STL]`. Without index dedup, STL would appear twice
    /// in `team_roster_all_stints("STL", ...)` and Hart.4's query layer
    /// would double-count goals/points for STL. Pins the dedup fix in
    /// `index_rosters`.
    #[test]
    fn l0_hart2_same_team_twice_in_stints_dedups_in_all_stints_index() {
        let s1 = TeamStint {
            team: TeamAbbr("STL".into()),
            started: Some("2022-10-15".into()),
            ended: Some("2023-01-15".into()),
            gp: 30,
            goals: 8,
            assists: 10,
            points: 18,
            goalie: None,
        };
        let s2 = TeamStint {
            team: TeamAbbr("NYR".into()),
            started: Some("2023-01-16".into()),
            ended: Some("2023-02-28".into()),
            gp: 15,
            goals: 4,
            assists: 5,
            points: 9,
            goalie: None,
        };
        let s3 = TeamStint {
            team: TeamAbbr("STL".into()),
            started: Some("2023-03-01".into()),
            ended: Some("2023-04-13".into()),
            gp: 24,
            goals: 6,
            assists: 9,
            points: 15,
            goalie: None,
        };
        let stats = SeasonStatsBuilder::new(
            PlayerId(8475765),
            Season(20222023),
            SeasonType::Regular,
            Position::RightWing,
        )
        .add_team_stint(s1)
        .add_team_stint(s2)
        .add_team_stint(s3)
        .with_totals(StatTotals {
            gp: 69,
            goals: 18,
            assists: 24,
            points: 42,
            ..Default::default()
        })
        .build();

        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(8475765).build())
            .unwrap();
        r.upsert_stats(stats).unwrap();

        let stl = r.team_roster_all_stints(
            &TeamAbbr("STL".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr = r.team_roster_all_stints(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(
            stl.len(),
            1,
            "STL appears once even though stints contain it twice"
        );
        assert_eq!(nyr.len(), 1);

        // Last-stint roster: chronologically last stint is the second STL.
        let stl_last = r.team_roster(
            &TeamAbbr("STL".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(stl_last.len(), 1, "last-stint = the STL re-acquisition");
    }

    /// TAPE: a goalie traded mid-(season, type) must have both teams in
    /// `rosters_all_stints` and the destination team in `rosters_last_stint`.
    /// Repo logic is goalie-agnostic for indexing; this is a regression
    /// fence around the Hart.1 invariant chain.
    #[test]
    fn l0_hart2_goalie_mid_trade_roster_indexed_for_both_teams() {
        let stint_a = TeamStint {
            team: TeamAbbr("BOS".into()),
            started: Some("2024-04-22".into()),
            ended: Some("2024-04-30".into()),
            gp: 3,
            goals: 0,
            assists: 0,
            points: 0,
            goalie: Some(crate::season_stats::GoalieStintStats {
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
            goalie: Some(crate::season_stats::GoalieStintStats {
                games_started: 7,
                wins: 4,
                losses: 3,
                ot_losses: Some(0),
            }),
        };
        let stats = SeasonStatsBuilder::new(
            PlayerId(9000001),
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
        .with_goalie(GoalieSeasonStats {
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
        })
        .build();

        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(9000001).build())
            .unwrap();
        r.upsert_stats(stats).unwrap();

        let bos_all = r.team_roster_all_stints(
            &TeamAbbr("BOS".into()),
            Season(20232024),
            SeasonType::Playoff,
        );
        let fla_all = r.team_roster_all_stints(
            &TeamAbbr("FLA".into()),
            Season(20232024),
            SeasonType::Playoff,
        );
        let bos_last = r.team_roster(
            &TeamAbbr("BOS".into()),
            Season(20232024),
            SeasonType::Playoff,
        );
        let fla_last = r.team_roster(
            &TeamAbbr("FLA".into()),
            Season(20232024),
            SeasonType::Playoff,
        );

        assert_eq!(bos_all.len(), 1, "BOS appears in all-stints (origin team)");
        assert_eq!(
            fla_all.len(),
            1,
            "FLA appears in all-stints (destination team)"
        );
        assert_eq!(
            bos_last.len(),
            0,
            "BOS NOT in last-stint (player ended on FLA)"
        );
        assert_eq!(fla_last.len(), 1, "FLA in last-stint");

        let v = fla_last.into_iter().next().unwrap();
        assert!(v.is_goalie());
        assert!(v.was_traded_in_window());
    }

    // ── LRU eviction ────────────────────────────────────────────────────────

    /// BENCH-mandated: cap=2; load A, B, C (evicts A); load A again;
    /// assert A reloads cleanly AND B is now LRU.
    #[test]
    fn l0_hart2_lru_evicted_window_reloads_correctly() {
        let mut r = StatsRepository::with_lru_cap(2);
        r.upsert_identity(fixtures::identity(1).build()).unwrap();
        r.upsert_identity(fixtures::identity(2).build()).unwrap();
        r.upsert_identity(fixtures::identity(3).build()).unwrap();

        // A: (20212022, Regular)
        r.upsert_stats(skater_stats(1, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        // B: (20222023, Regular)
        r.upsert_stats(skater_stats(2, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        // C: (20232024, Regular) — evicts A
        r.upsert_stats(skater_stats(3, 20232024, SeasonType::Regular, "EDM"))
            .unwrap();

        assert!(
            !r.has_window(Season(20212022), SeasonType::Regular),
            "A evicted"
        );
        assert!(
            r.has_window(Season(20222023), SeasonType::Regular),
            "B resident"
        );
        assert!(
            r.has_window(Season(20232024), SeasonType::Regular),
            "C resident"
        );
        assert!(
            r.season(PlayerId(1), Season(20212022), SeasonType::Regular)
                .is_none(),
            "A's stats also evicted"
        );

        // Re-load A — evicts B (which is now LRU since C is MRU and A just-loaded → MRU).
        r.upsert_stats(skater_stats(1, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();

        assert!(
            r.has_window(Season(20212022), SeasonType::Regular),
            "A back"
        );
        assert!(
            !r.has_window(Season(20222023), SeasonType::Regular),
            "B now evicted"
        );
        assert!(
            r.has_window(Season(20232024), SeasonType::Regular),
            "C still resident"
        );
        assert!(
            r.season(PlayerId(1), Season(20212022), SeasonType::Regular)
                .is_some(),
            "A's stats reloaded cleanly"
        );
        assert_eq!(
            r.resident_windows(),
            2,
            "deque must still respect cap=2 after eviction-then-reload"
        );
    }

    #[test]
    fn l0_hart2_lru_does_not_evict_within_cap() {
        let mut r = StatsRepository::with_lru_cap(4);
        for i in 1..=4u32 {
            r.upsert_identity(fixtures::identity(i).build()).unwrap();
            r.upsert_stats(skater_stats(i, 20222023, SeasonType::Regular, "EDM"))
                .unwrap();
        }
        assert_eq!(
            r.resident_windows(),
            1,
            "all four ids share the same window"
        );

        // Now four different windows.
        let mut r = StatsRepository::with_lru_cap(4);
        r.upsert_identity(fixtures::identity(1).build()).unwrap();
        r.upsert_stats(skater_stats(1, 20202021, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(1, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(1, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(1, 20232024, SeasonType::Regular, "EDM"))
            .unwrap();
        assert_eq!(r.resident_windows(), 4);
        assert!(r.has_window(Season(20202021), SeasonType::Regular));
        assert!(r.has_window(Season(20232024), SeasonType::Regular));
    }

    #[test]
    fn l0_hart2_lru_touch_promotes_existing_window_to_mru() {
        let mut r = StatsRepository::with_lru_cap(2);
        r.upsert_identity(fixtures::identity(1).build()).unwrap();
        r.upsert_identity(fixtures::identity(2).build()).unwrap();

        // A then B
        r.upsert_stats(skater_stats(1, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(2, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        // Touch A again (re-upsert another player into A's window).
        r.upsert_identity(fixtures::identity(99).build()).unwrap();
        r.upsert_stats(skater_stats(99, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        // Now load C — should evict B (B is now LRU since A was just touched).
        r.upsert_identity(fixtures::identity(3).build()).unwrap();
        r.upsert_stats(skater_stats(3, 20232024, SeasonType::Regular, "EDM"))
            .unwrap();

        assert!(
            r.has_window(Season(20212022), SeasonType::Regular),
            "A still resident (touched)"
        );
        assert!(
            !r.has_window(Season(20222023), SeasonType::Regular),
            "B evicted"
        );
        assert!(
            r.has_window(Season(20232024), SeasonType::Regular),
            "C resident"
        );
    }

    /// B12: cap=4, load 6 windows. The first 2 must be evicted; the
    /// last 4 (in order: oldest-resident-of-the-survivors → MRU) remain.
    /// Catches a "pop_front called once per call instead of in a loop"
    /// regression — though touch_window_lru is currently single-pop and
    /// each upsert can only push one window, so eviction of multiple
    /// windows happens across multiple upserts. This test pins that
    /// multiple sequential upserts each evict their own LRU.
    #[test]
    fn l0_hart2_lru_multi_eviction_cap_4_load_6() {
        let mut r = StatsRepository::with_lru_cap(4);
        let windows: [(u32, SeasonType); 6] = [
            (20192020, SeasonType::Regular),
            (20202021, SeasonType::Regular),
            (20212022, SeasonType::Regular),
            (20222023, SeasonType::Regular),
            (20232024, SeasonType::Regular),
            (20242025, SeasonType::Regular),
        ];
        for (i, &(season, t)) in windows.iter().enumerate() {
            let pid = (i + 1) as u32;
            r.upsert_identity(fixtures::identity(pid).build()).unwrap();
            r.upsert_stats(skater_stats(pid, season, t, "EDM")).unwrap();
        }

        assert_eq!(r.resident_windows(), 4, "cap honored");
        assert!(
            !r.has_window(Season(20192020), SeasonType::Regular),
            "first evicted"
        );
        assert!(
            !r.has_window(Season(20202021), SeasonType::Regular),
            "second evicted"
        );
        assert!(r.has_window(Season(20212022), SeasonType::Regular));
        assert!(r.has_window(Season(20222023), SeasonType::Regular));
        assert!(r.has_window(Season(20232024), SeasonType::Regular));
        assert!(r.has_window(Season(20242025), SeasonType::Regular));

        // Verify the evicted windows' stats are gone.
        assert!(r
            .season(PlayerId(1), Season(20192020), SeasonType::Regular)
            .is_none());
        assert!(r
            .season(PlayerId(2), Season(20202021), SeasonType::Regular)
            .is_none());
        // Verify the survivors' stats are reachable.
        assert!(r
            .season(PlayerId(3), Season(20212022), SeasonType::Regular)
            .is_some());
        assert!(r
            .season(PlayerId(6), Season(20242025), SeasonType::Regular)
            .is_some());
    }

    // ── iter accessors (lockdown sanity) ────────────────────────────────────

    #[test]
    fn l0_hart3_upsert_contract_idempotent_and_view_carries_it() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        let c = crate::contract::PlayerContract {
            expiry_year: Some(2026),
            expiry_type: Some("UFA".into()),
            salary: Some(12_500_000),
        };
        r.upsert_contract(PlayerId(8478402), c.clone());

        // Direct lookup.
        assert_eq!(r.contract(PlayerId(8478402)), Some(&c));
        assert_eq!(r.contracts_len(), 1);

        // PlayerView carries it.
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert_eq!(v.contract_expiry_year(), Some(2026));
        assert_eq!(v.contract_expiry_type(), Some("UFA"));
        assert_eq!(v.contract_salary(), Some(12_500_000));

        // Re-upsert is idempotent.
        r.upsert_contract(PlayerId(8478402), c.clone());
        assert_eq!(r.contracts_len(), 1);
    }

    #[test]
    fn l0_hart3_view_contract_none_when_unset() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert!(v.contract.is_none());
        assert_eq!(v.contract_expiry_year(), None);
    }

    #[test]
    fn l0_hart3_contract_survives_window_eviction() {
        // Contracts are window-independent — evicting (s, t) drops stats
        // and roster indexes but must NOT drop contracts. The next
        // upsert into a fresh window can still see the contract.
        let mut r = StatsRepository::with_lru_cap(2);
        r.upsert_identity(fixtures::identity(1).build()).unwrap();
        r.upsert_contract(
            PlayerId(1),
            crate::contract::PlayerContract {
                expiry_year: Some(2026),
                ..Default::default()
            },
        );
        r.upsert_identity(fixtures::identity(2).build()).unwrap();
        r.upsert_identity(fixtures::identity(3).build()).unwrap();

        // Fill cap then push a third window — evicts the first.
        r.upsert_stats(skater_stats(1, 20212022, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(2, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(3, 20232024, SeasonType::Regular, "EDM"))
            .unwrap();

        // Player 1's stats are gone (window evicted) — but the contract
        // must still be reachable.
        assert!(r
            .season(PlayerId(1), Season(20212022), SeasonType::Regular)
            .is_none());
        assert!(r.contract(PlayerId(1)).is_some());
        assert_eq!(r.contract(PlayerId(1)).unwrap().expiry_year, Some(2026));
    }

    #[test]
    fn l0_hart2_iter_identities_and_stats_yield_expected_counts() {
        let mut r = StatsRepository::new();
        r.upsert_identity(fixtures::identity(1).build()).unwrap();
        r.upsert_identity(fixtures::identity(2).build()).unwrap();
        r.upsert_stats(skater_stats(1, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        r.upsert_stats(skater_stats(1, 20222023, SeasonType::Playoff, "EDM"))
            .unwrap();

        assert_eq!(r.iter_identities().count(), 2);
        assert_eq!(r.identities_len(), 2);
        assert_eq!(r.iter_stats().count(), 2);
        assert_eq!(r.stats_len(), 2);
    }

    // ── repo_swap ──────────────────────────────────────────────────────────

    #[test]
    fn l0_hart2_repo_swap_returns_old_repo() {
        let mut r1 = make_repo_with_player(8478402);
        r1.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();

        let r2 = make_repo_with_player(8475765);

        let old = r1.repo_swap(r2);

        // r1 now holds r2's state; the returned `old` holds r1's prior state.
        assert!(r1.identity(PlayerId(8475765)).is_some());
        assert!(r1.identity(PlayerId(8478402)).is_none());
        assert!(old.identity(PlayerId(8478402)).is_some());
        assert!(old
            .season(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .is_some());
    }

    // ── Hart.4.1 v0.2 — invariant proptests ────────────────────────────────
    //
    // Two BENCH-deferred proptests from Hart.2.1 review land here:
    //
    // - LRU invariant proptest (Gap D): asserts the bidirectional
    //   bijection between `repo.stats` and `repo.window_lru` after any
    //   sequence of upserts, plus the strict cap bound (TAPE #7).
    //
    // - Roster sum invariant proptest (Gap E): asserts that for any
    //   set of player stints (including raw possibly-duplicate stint
    //   teams — Hart.2.1 dedup regression fence), the sum of
    //   `team_roster_all_stints(team).len()` over teams equals the
    //   count of distinct (player, team) pairs (BENCH #2).

    use proptest::prelude::*;

    /// Build a small alphabet of (Season, SeasonType) windows. Six
    /// distinct values (3 seasons × 2 types) gives ~30% birthday-paradox
    /// repeat rate at length 50, exercising the touch-then-promote LRU
    /// path consistently.
    fn lru_window_strategy() -> impl Strategy<Value = (Season, SeasonType)> {
        (
            0u32..3,
            prop_oneof![Just(SeasonType::Regular), Just(SeasonType::Playoff)],
        )
            .prop_map(|(idx, t)| (Season(20212022 + idx * 10001), t))
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            // Override via `PROPTEST_CASES` env if CI runtime regresses.
            ..ProptestConfig::default()
        })]

        /// Gap D — LRU bidirectional bijection invariants.
        /// Asserts after ANY sequence of upserts:
        ///   1. resident_windows() <= cap
        ///   2. ∀ (pid, s, t) ∈ stats → has_window(s, t)
        ///   3. ∀ (s, t) ∈ window_lru → ∃ stats row keyed there
        ///   4. The most-recently-upserted window is always resident
        #[test]
        fn lru_invariant_proptest(
            cap in 1usize..=10,
            ops in prop::collection::vec(lru_window_strategy(), 0..50),
        ) {
            let mut repo = StatsRepository::with_lru_cap(cap);

            for (next_pid, (s, t)) in (1_u32..).zip(ops.iter()) {
                let pid = PlayerId(next_pid);
                repo.upsert_identity(crate::fixtures::identity(pid.0).build()).unwrap();
                let stats = crate::fixtures::stats(pid.0, s.0, "EDM")
                    .season_type(*t)
                    .build();
                repo.upsert_stats(stats).unwrap();
                let last_window = (*s, *t);

                // 1. cap bound
                prop_assert!(
                    repo.resident_windows() <= cap,
                    "resident={} > cap={}", repo.resident_windows(), cap
                );

                // 2. stats → window_lru
                for &(_, ks, kt) in repo.stats.keys() {
                    prop_assert!(
                        repo.has_window(ks, kt),
                        "stats row at ({ks:?}, {kt:?}) but window not in LRU"
                    );
                }

                // 3. window_lru → stats
                for &(ws, wt) in &repo.window_lru {
                    let any_match = repo
                        .stats
                        .keys()
                        .any(|&(_, sk, tk)| sk == ws && tk == wt);
                    prop_assert!(
                        any_match,
                        "window ({ws:?}, {wt:?}) in LRU but no stats row"
                    );
                }

                // 4. last upsert always resident
                let (lw_s, lw_t) = last_window;
                prop_assert!(
                    repo.has_window(lw_s, lw_t),
                    "most-recent upsert ({lw_s:?}, {lw_t:?}) not resident"
                );
            }
        }

        /// Gap E — roster sum invariant. Strategy feeds raw
        /// possibly-duplicate Vec<TeamAbbr> directly (no pre-dedup) so
        /// the same-team-twice regression Hart.2.1 fixed surfaces here
        /// (BENCH #2).
        #[test]
        fn roster_sum_invariant_proptest(
            players in prop::collection::vec(
                (1u32..1000, prop::collection::vec("[A-Z]{3}", 1..=4)),
                1..=20,
            )
        ) {
            use std::collections::HashSet;

            let season = Season(20232024);
            let stype = SeasonType::Regular;

            // Reference: distinct (player, team) pairs across all input.
            let expected_distinct_pairs: HashSet<(u32, String)> = players
                .iter()
                .flat_map(|(pid, teams)| {
                    teams.iter().map(move |t| (*pid, t.clone()))
                })
                .collect();

            let mut repo = StatsRepository::new();
            // Dedup player_ids on input (one stats row per pid is the
            // primary-key invariant; the proptest may emit duplicates).
            let mut seen_pids: HashSet<u32> = HashSet::new();
            let dedup_players: Vec<_> = players
                .iter()
                .filter(|(pid, _)| seen_pids.insert(*pid))
                .collect();

            for (pid, teams) in &dedup_players {
                repo.upsert_identity(crate::fixtures::identity(*pid).build()).unwrap();
                let stints: Vec<TeamStint> = teams
                    .iter()
                    .map(|t| TeamStint {
                        team: TeamAbbr(t.clone()),
                        started: None,
                        ended: None,
                        gp: 1,
                        goals: 0,
                        assists: 0,
                        points: 0,
                        goalie: None,
                    })
                    .collect();
                let stats =
                    crate::season_stats::SeasonStatsBuilder::new(
                        PlayerId(*pid),
                        season,
                        stype,
                        Position::Center,
                    )
                    .with_totals(crate::season_stats::StatTotals {
                        gp: stints.len() as u32,
                        ..Default::default()
                    })
                    .replace_team_stints(stints)
                    .build();
                repo.upsert_stats(stats).unwrap();
            }

            // Now compute: distinct (pid, team) pairs from the DEDUPED
            // players (the proptest may emit duplicate pids; only the
            // first is in the repo).
            let distinct_dedup: HashSet<(u32, String)> = dedup_players
                .iter()
                .flat_map(|(pid, teams)| {
                    teams.iter().map(move |t| (*pid, t.clone()))
                })
                .collect();

            // All-stints sum: each distinct pair contributes one entry.
            let all_teams: HashSet<String> =
                distinct_dedup.iter().map(|(_, t)| t.clone()).collect();
            let total_all_stints: usize = all_teams
                .iter()
                .map(|t| {
                    repo.team_roster_all_stints(&TeamAbbr(t.clone()), season, stype)
                        .len()
                })
                .sum();
            prop_assert_eq!(
                total_all_stints,
                distinct_dedup.len(),
                "all-stints roster sum != distinct (pid, team) pair count",
            );

            // Last-stint sum: each player has exactly one last-stint
            // team, so total == count of dedup players (TAPE #8).
            let total_last_stint: usize = all_teams
                .iter()
                .map(|t| repo.team_roster(&TeamAbbr(t.clone()), season, stype).len())
                .sum();
            prop_assert_eq!(
                total_last_stint,
                dedup_players.len(),
                "last-stint roster sum != count of distinct players",
            );

            // Avoid unused-warning on the input-level distinct count.
            let _ = expected_distinct_pairs;
        }
    }

    /// Gap D companion — explicit case for `cap = 1` pure-churn path
    /// (BENCH #3). Every upsert evicts the prior; the deque never
    /// holds more than one window.
    #[test]
    fn l0_hart4_1_lru_cap_one_pure_churn() {
        let mut repo = StatsRepository::with_lru_cap(1);
        for i in 1..=5u32 {
            let pid = PlayerId(i);
            repo.upsert_identity(crate::fixtures::identity(i).build())
                .unwrap();
            let stats = crate::fixtures::stats(i, 20212022 + i * 10001, "EDM").build();
            repo.upsert_stats(stats).unwrap();

            assert_eq!(
                repo.resident_windows(),
                1,
                "cap=1 must hold exactly 1 window"
            );
            // The most-recent upsert's window is the only resident one.
            assert!(repo.has_window(Season(20212022 + i * 10001), SeasonType::Regular));
            // Earlier windows are gone.
            if i > 1 {
                assert!(!repo.has_window(Season(20212022 + (i - 1) * 10001), SeasonType::Regular));
                assert!(repo
                    .season(
                        PlayerId(i - 1),
                        Season(20212022 + (i - 1) * 10001),
                        SeasonType::Regular
                    )
                    .is_none());
            }
            let _ = pid;
        }
    }

    /// Gap E companion — replace-path coverage (TAPE #9). The proptest
    /// only fires `index_rosters`. This test fires `unindex_rosters_for`
    /// by replacing a stats row with different team_stints.
    /// Hart.5b2-prep: PlayerView derived helpers mirror the legacy
    /// Player methods. Catches sign/scale errors in the per-82 helpers
    /// and locks the gp=0 → None semantic.
    #[test]
    fn l0_hart5_view_derived_helpers_match_legacy_player_semantics() {
        let mut r = make_repo_with_player(8478402);
        r.upsert_stats(skater_stats(8478402, 20222023, SeasonType::Regular, "EDM"))
            .unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();

        // is_rankable: pace_score is Some on the fixture.
        assert!(v.is_rankable());

        // pp_assists = pp_points - pp_goals = 28 - 10 = 18 (fixture defaults).
        assert_eq!(v.pp_assists(), 18);

        // Per-82 helpers: fixture has gp=70, pp_goals=10, pp_points=28,
        // gwg=5, shots=220.
        let pp_pts_82 = v.pp_points_per_82().unwrap();
        let want_pp_pts_82 = 28.0 / 70.0 * 82.0;
        assert!((pp_pts_82 - want_pp_pts_82).abs() < 1e-6);

        let pp_g_82 = v.pp_goals_per_82().unwrap();
        let want_pp_g_82 = 10.0 / 70.0 * 82.0;
        assert!((pp_g_82 - want_pp_g_82).abs() < 1e-6);

        let gwg_82 = v.gwg_per_82().unwrap();
        let want_gwg_82 = 5.0 / 70.0 * 82.0;
        assert!((gwg_82 - want_gwg_82).abs() < 1e-6);

        // pace_82 / goals_per_82 from the seeded PaceScore.
        assert!((v.pace_82().unwrap() - 93.7).abs() < 1e-6);
        assert!((v.goals_per_82().unwrap() - 35.1).abs() < 1e-6);
        // pace_sort_key combines them (per PaceScore::sort_key).
        let sk = v.pace_sort_key();
        assert!(sk > 93.7);
        assert!(sk < 100.0);

        // toi_mmss: fixture sets toi_per_game_sec = 20*60 = 1200 → "20:00".
        assert_eq!(v.toi_mmss().as_deref(), Some("20:00"));

        // Cold-start realtime is None: hits_per_82() and
        // blocked_shots_per_82() return None (None-arm).
        assert!(v.hits_per_82().is_none());
        assert!(v.blocked_shots_per_82().is_none());
    }

    /// Hart.5c.7.12 — gap-fill for the per-82 helpers whose happy-path
    /// branches were not exercised after the legacy `Player` test mod was
    /// deleted. Default fixture has gp=70, shots=220, sh_goals=0; adds a
    /// realtime payload (hits=100, blocks=50) and overrides sh_goals to a
    /// non-zero value so every per-82 helper that production code calls
    /// has at least one Some-arm assertion.
    #[test]
    fn l0_hart5c7_view_per_82_happy_path_for_every_production_helper() {
        let mut r = make_repo_with_player(8478402);
        let mut stats = fixtures::stats(8478402, 20222023, "EDM")
            .realtime(100, 50, 40, 20)
            .build();
        // Override sh_goals on the totals so sh_goals_per_82 has a non-zero
        // numerator. Default fixture is sh_goals=0, which produces 0.0 and
        // hides any sign/scale bug.
        stats.totals.sh_goals = 4;
        r.upsert_stats(stats).unwrap();

        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();

        // gp=70 baseline — hard-coded so the test fails loudly if the fixture
        // shifts under it.
        assert_eq!(v.gp(), 70);

        let want_shots_82 = 220.0 / 70.0 * 82.0; // ≈ 257.71
        assert!((v.shots_per_82().unwrap() - want_shots_82).abs() < 1e-6);

        let want_sh_g_82 = 4.0 / 70.0 * 82.0; // ≈ 4.69
        assert!((v.sh_goals_per_82().unwrap() - want_sh_g_82).abs() < 1e-6);

        let want_hits_82 = 100.0 / 70.0 * 82.0; // ≈ 117.14
        assert!((v.hits_per_82().unwrap() - want_hits_82).abs() < 1e-6);

        let want_blocks_82 = 50.0 / 70.0 * 82.0; // ≈ 58.57
        assert!((v.blocked_shots_per_82().unwrap() - want_blocks_82).abs() < 1e-6);
    }

    /// Hart.5c.7.12 — `toi_mmss` edge cases that the legacy
    /// `l0_player_toi_mmss_*` tests covered before deletion: None
    /// passthrough, "00:00" zero, exact-minute boundary, sub-minute
    /// rounding via integer division.
    #[test]
    fn l0_hart5c7_view_toi_mmss_edge_cases() {
        let mut r = make_repo_with_player(8478402);
        let mut stats = fixtures::stats(8478402, 20222023, "EDM").build();

        // 1: toi_per_game_sec = None → toi_mmss returns None.
        stats.totals.toi_per_game_sec = None;
        r.upsert_stats(stats.clone()).unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert!(v.toi_mmss().is_none());
        let _ = v;

        // 2: 0 seconds → "00:00".
        stats.totals.toi_per_game_sec = Some(0);
        r.upsert_stats(stats.clone()).unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert_eq!(v.toi_mmss().as_deref(), Some("00:00"));
        let _ = v;

        // 3: exactly 60s → "01:00".
        stats.totals.toi_per_game_sec = Some(60);
        r.upsert_stats(stats.clone()).unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert_eq!(v.toi_mmss().as_deref(), Some("01:00"));
        let _ = v;

        // 4: 1259s = 20:59 — exercises the minute/second split when seconds
        // are not zero. Catches off-by-one bugs in the formatter.
        stats.totals.toi_per_game_sec = Some(1259);
        r.upsert_stats(stats).unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert_eq!(v.toi_mmss().as_deref(), Some("20:59"));
    }

    /// Hart.5c.7.12 — `pp_assists` saturating-sub edge case. Production
    /// data should never have pp_goals > pp_points, but the helper uses
    /// `saturating_sub` defensively. Locks the contract: never underflow.
    #[test]
    fn l0_hart5c7_view_pp_assists_saturating_sub() {
        let mut r = make_repo_with_player(8478402);
        let mut stats = fixtures::stats(8478402, 20222023, "EDM").build();
        // Force the pathological case: pp_goals exceeds pp_points.
        stats.totals.pp_goals = 5;
        stats.totals.pp_points = 3;
        r.upsert_stats(stats).unwrap();
        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        // 3 - 5 saturates to 0 — must NOT panic or wrap.
        assert_eq!(v.pp_assists(), 0);
    }

    /// Hart.5b2-prep: the gp=0 edge case for per-82 helpers — must
    /// return None (not divide-by-zero or 0.0).
    #[test]
    fn l0_hart5_view_per_82_helpers_none_when_gp_zero() {
        let mut r = make_repo_with_player(8478402);
        let stats = crate::fixtures::stats(8478402, 20222023, "EDM")
            .build();
        // Manually craft a stats row with gp=0 by going through the builder.
        let mut zero_gp = stats.clone();
        zero_gp.totals.gp = 0;
        zero_gp.totals.pp_goals = 0;
        zero_gp.totals.pp_points = 0;
        zero_gp.totals.gwg = 0;
        zero_gp.totals.shots = 0;
        zero_gp.totals.pace_score = None;
        r.upsert_stats(zero_gp).unwrap();

        let v = r
            .view(PlayerId(8478402), Season(20222023), SeasonType::Regular)
            .unwrap();
        assert_eq!(v.gp(), 0);
        assert!(v.pp_points_per_82().is_none());
        assert!(v.pp_goals_per_82().is_none());
        assert!(v.gwg_per_82().is_none());
        assert!(v.shots_per_82().is_none());
        assert!(!v.is_rankable());
        assert_eq!(v.pace_sort_key(), 0.0);
    }

    #[test]
    fn l0_hart4_1_replace_stats_unindexes_old_roster_entries() {
        let mut repo = StatsRepository::new();
        repo.upsert_identity(crate::fixtures::identity(8475765).build())
            .unwrap();

        // V1: stints [BOS, NYR]
        let s1 = crate::fixtures::traded_skater(
            8475765,
            20222023,
            TeamAbbr("BOS".into()),
            TeamAbbr("NYR".into()),
        )
        .build();
        repo.upsert_stats(s1).unwrap();
        let bos = repo.team_roster_all_stints(
            &TeamAbbr("BOS".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr = repo.team_roster_all_stints(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(bos.len(), 1, "BOS in all-stints after v1");
        assert_eq!(nyr.len(), 1, "NYR in all-stints after v1");

        // V2: stints [TOR] only — replace
        let s2 = crate::fixtures::stats(8475765, 20222023, "TOR").build();
        repo.upsert_stats(s2).unwrap();

        let bos = repo.team_roster_all_stints(
            &TeamAbbr("BOS".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let nyr = repo.team_roster_all_stints(
            &TeamAbbr("NYR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        let tor = repo.team_roster_all_stints(
            &TeamAbbr("TOR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(bos.len(), 0, "BOS unindexed after v2");
        assert_eq!(nyr.len(), 0, "NYR unindexed after v2");
        assert_eq!(tor.len(), 1, "TOR indexed by v2");

        // Last-stint also reflects the replacement.
        let tor_last = repo.team_roster(
            &TeamAbbr("TOR".into()),
            Season(20222023),
            SeasonType::Regular,
        );
        assert_eq!(tor_last.len(), 1);
    }
}
