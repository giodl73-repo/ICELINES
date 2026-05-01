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
         {season:?}/{season_type:?}"
    )]
    StatsWithoutIdentity {
        id: PlayerId,
        season: Season,
        season_type: SeasonType,
    },
}

pub struct StatsRepository {
    pub identities: HashMap<PlayerId, PlayerIdentity>,
    pub stats: HashMap<(PlayerId, Season, SeasonType), SeasonStats>,

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

    pub fn view(&self, id: PlayerId, s: Season, t: SeasonType) -> Option<PlayerView<'_>> {
        let identity = self.identities.get(&id)?;
        let stats = self.stats.get(&(id, s, t))?;
        Some(PlayerView { identity, stats })
    }

    // ── Career iterators (TAPE: typed, never mixed) ─────────────────────────

    /// Regular-season rows for one player, ordered ascending by season.
    /// Returns `None` only if the player has zero rows of any kind;
    /// `Some(empty_iter)` if they have only playoff rows.
    pub fn career_regular(&self, id: PlayerId) -> Option<impl Iterator<Item = &SeasonStats>> {
        if !self.identities.contains_key(&id) {
            return None;
        }
        Some(self.career_filtered(id, Some(SeasonType::Regular)))
    }

    pub fn career_playoff(&self, id: PlayerId) -> Option<impl Iterator<Item = &SeasonStats>> {
        if !self.identities.contains_key(&id) {
            return None;
        }
        Some(self.career_filtered(id, Some(SeasonType::Playoff)))
    }

    pub fn career_all(&self, id: PlayerId) -> Option<impl Iterator<Item = &SeasonStats>> {
        if !self.identities.contains_key(&id) {
            return None;
        }
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
    /// should `.collect::<Vec>()` and sort.
    pub fn league(&self, s: Season, t: SeasonType) -> impl Iterator<Item = PlayerView<'_>> {
        self.stats
            .iter()
            .filter_map(move |(&(pid, ks, kt), stats)| {
                if ks == s && kt == t {
                    let identity = self.identities.get(&pid)?;
                    Some(PlayerView { identity, stats })
                } else {
                    None
                }
            })
    }

    pub fn skaters(&self, s: Season, t: SeasonType) -> impl Iterator<Item = PlayerView<'_>> {
        self.league(s, t).filter(|v| !v.is_goalie())
    }

    pub fn goalies(&self, s: Season, t: SeasonType) -> impl Iterator<Item = PlayerView<'_>> {
        self.league(s, t).filter(|v| v.is_goalie())
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

        for stint in &stats.team_stints {
            let key = (s, t, stint.team.clone());
            self.rosters_all_stints.entry(key).or_default().push(pid);
        }

        if let Some(last) = stats.team_stints.last() {
            let key = (s, t, last.team.clone());
            self.rosters_last_stint.entry(key).or_default().push(pid);
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
    /// drop(view);
    /// ```
    pub fn repo_swap(&mut self, new_repo: StatsRepository) -> StatsRepository {
        std::mem::replace(self, new_repo)
    }
}

/// Borrowed projection over `(PlayerIdentity, SeasonStats)`. Render code
/// never sees raw structs — accesses go through these accessors. Lifetime
/// is render-frame-scoped: a `PlayerView` MUST NOT outlive a frame, and
/// MUST NOT be a field of any struct whose lifetime exceeds one render
/// pass. (Hart.0 surveyed for current Player-storage patterns; none port
/// to view-storage.)
#[derive(Debug, Clone, Copy)]
pub struct PlayerView<'a> {
    pub identity: &'a PlayerIdentity,
    pub stats: &'a SeasonStats,
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
    }

    #[test]
    fn l0_hart2_view_returns_none_for_missing_stats() {
        let r = make_repo_with_player(8478402);
        let v = r.view(PlayerId(8478402), Season(20222023), SeasonType::Regular);
        assert!(v.is_none(), "no stats yet — view must be None");
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
}
