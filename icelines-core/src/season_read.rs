//! Borrowed season-stat access independent of repository storage layout.
//!
//! This additive interface does not replace `SeasonStats`, `PlayerView::stats`,
//! repository ownership, or serialization. Implementors return coherent references
//! for one player/season/type and preserve every optional payload's missingness.
use crate::{
    identity::PlayerId,
    model::{PaceScore, Position, Season},
    season_stats::*,
    stats_repository::PlayerView,
};

/// Read-only access to one coherent season row, without requiring `&SeasonStats`.
///
/// References borrow their owner; callers cannot retain them across mutation.
/// Implementors may resolve separately stored typed payloads lazily. No conversion,
/// allocation, width narrowing or fabricated whole-row reference is required.
/// PlayerView retains its identity-based id semantics; valid repository views have
/// the same id in their identity and season row. Core computations are shared
/// with the existing PlayerView methods and the stat catalog.
///
/// ```compile_fail
/// use icelines_core::{fixtures, SeasonStatsRead};
/// let mut row = fixtures::stats(1, 20252026, "EDM").build();
/// let read: &dyn SeasonStatsRead = &row;
/// let totals = read.totals();
/// row.totals.points += 1;
/// std::hint::black_box(totals.points);
/// ```
pub trait SeasonStatsRead {
    fn player_id(&self) -> PlayerId;
    fn season(&self) -> Season;
    fn season_type(&self) -> SeasonType;
    fn position(&self) -> Position;
    fn sweater_number(&self) -> Option<u32>;
    fn totals(&self) -> &StatTotals;
    fn team_stints(&self) -> &[TeamStint];
    fn realtime(&self) -> Option<&RealtimeStats>;
    fn advanced(&self) -> Option<&AdvancedStats>;
    fn goalie(&self) -> Option<&GoalieSeasonStats>;
    fn time_on_ice(&self) -> Option<&TimeOnIceStats>;
    fn goals_for_against(&self) -> Option<&GoalsForAgainstStats>;
    fn goalie_advanced(&self) -> Option<&GoalieAdvancedStats>;
    fn goalie_saves_by_strength(&self) -> Option<&GoalieSavesByStrengthStats>;
    fn goalie_bios(&self) -> Option<&GoalieBios>;
}

// Blanket, crate-private extension: downstream storage implementations provide
// data access only and cannot replace catalog/PlayerView computations.
pub(crate) trait SeasonStatsReadExt: SeasonStatsRead {
    fn is_goalie(&self) -> bool {
        self.goalie().is_some()
    }

    fn was_traded_in_window(&self) -> bool {
        self.team_stints().len() > 1
    }

    fn goals(&self) -> u32 {
        self.totals().goals
    }

    fn assists(&self) -> u32 {
        self.totals().assists
    }

    fn points(&self) -> u32 {
        self.totals().points
    }

    fn gp(&self) -> u32 {
        self.totals().gp
    }

    fn shots(&self) -> u32 {
        self.totals().shots
    }

    fn pace_score(&self) -> Option<&PaceScore> {
        self.totals().pace_score.as_ref()
    }

    fn hits(&self) -> Option<u32> {
        self.realtime().map(|r| r.hits)
    }

    fn blocked_shots(&self) -> Option<u32> {
        self.realtime().map(|r| r.blocked_shots)
    }

    fn takeaways(&self) -> Option<u32> {
        self.realtime().map(|r| r.takeaways)
    }

    fn giveaways(&self) -> Option<u32> {
        self.realtime().map(|r| r.giveaways)
    }

    fn xg(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.xg)
    }

    fn xg_per_60(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.xg_per_60)
    }

    fn cf_pct(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.cf_pct)
    }

    fn ff_pct(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.ff_pct)
    }

    fn on_ice_xg_for(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.on_ice_xg_for)
    }

    fn on_ice_xg_against(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.on_ice_xg_against)
    }

    fn xgf_pct(&self) -> Option<f64> {
        self.advanced().and_then(|a| a.xgf_pct)
    }

    fn pp_assists(&self) -> u32 {
        self.totals()
            .pp_points
            .saturating_sub(self.totals().pp_goals)
    }

    fn pace_82(&self) -> Option<f64> {
        self.pace_score().map(|s| s.pace_82)
    }

    fn goals_per_82(&self) -> Option<f64> {
        self.pace_score().map(|s| s.goals_per_82)
    }

    fn pace_sort_key(&self) -> f64 {
        self.pace_score().map(|s| s.sort_key()).unwrap_or(0.0)
    }
}

impl<R: SeasonStatsRead + ?Sized> SeasonStatsReadExt for R {}

impl SeasonStatsRead for SeasonStats {
    fn player_id(&self) -> PlayerId {
        self.player_id
    }
    fn season(&self) -> Season {
        self.season
    }
    fn season_type(&self) -> SeasonType {
        self.season_type
    }
    fn position(&self) -> Position {
        self.position
    }
    fn sweater_number(&self) -> Option<u32> {
        self.sweater_number
    }
    fn totals(&self) -> &StatTotals {
        &self.totals
    }
    fn team_stints(&self) -> &[TeamStint] {
        &self.team_stints
    }
    fn realtime(&self) -> Option<&RealtimeStats> {
        self.realtime.as_ref()
    }
    fn advanced(&self) -> Option<&AdvancedStats> {
        self.advanced.as_ref()
    }
    fn goalie(&self) -> Option<&GoalieSeasonStats> {
        self.goalie.as_ref()
    }
    fn time_on_ice(&self) -> Option<&TimeOnIceStats> {
        self.time_on_ice.as_ref()
    }
    fn goals_for_against(&self) -> Option<&GoalsForAgainstStats> {
        self.goals_for_against.as_ref()
    }
    fn goalie_advanced(&self) -> Option<&GoalieAdvancedStats> {
        self.goalie_advanced.as_ref()
    }
    fn goalie_saves_by_strength(&self) -> Option<&GoalieSavesByStrengthStats> {
        self.goalie_saves_by_strength.as_ref()
    }
    fn goalie_bios(&self) -> Option<&GoalieBios> {
        self.goalie_bios.as_ref()
    }
}

impl SeasonStatsRead for PlayerView<'_> {
    fn player_id(&self) -> PlayerId {
        self.identity.id
    }
    fn season(&self) -> Season {
        self.stats.season
    }
    fn season_type(&self) -> SeasonType {
        self.stats.season_type
    }
    fn position(&self) -> Position {
        self.stats.position
    }
    fn sweater_number(&self) -> Option<u32> {
        self.stats.sweater_number
    }
    fn totals(&self) -> &StatTotals {
        &self.stats.totals
    }
    fn team_stints(&self) -> &[TeamStint] {
        &self.stats.team_stints
    }
    fn realtime(&self) -> Option<&RealtimeStats> {
        self.stats.realtime.as_ref()
    }
    fn advanced(&self) -> Option<&AdvancedStats> {
        self.stats.advanced.as_ref()
    }
    fn goalie(&self) -> Option<&GoalieSeasonStats> {
        self.stats.goalie.as_ref()
    }
    fn time_on_ice(&self) -> Option<&TimeOnIceStats> {
        self.stats.time_on_ice.as_ref()
    }
    fn goals_for_against(&self) -> Option<&GoalsForAgainstStats> {
        self.stats.goals_for_against.as_ref()
    }
    fn goalie_advanced(&self) -> Option<&GoalieAdvancedStats> {
        self.stats.goalie_advanced.as_ref()
    }
    fn goalie_saves_by_strength(&self) -> Option<&GoalieSavesByStrengthStats> {
        self.stats.goalie_saves_by_strength.as_ref()
    }
    fn goalie_bios(&self) -> Option<&GoalieBios> {
        self.stats.goalie_bios.as_ref()
    }
}
