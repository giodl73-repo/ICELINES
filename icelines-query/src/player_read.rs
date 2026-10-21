//! Identity, contract and season facts borrowed from independent owners.
use icelines_core::{
    contract::PlayerContract,
    identity::PlayerIdentity,
    model::{Position, TeamAbbr},
    season_stats::SeasonStats,
    SeasonStatsRead,
};

/// Additive query view that does not require an assembled `SeasonStats`.
///
/// Identity/contract references retain their original ownership and historical
/// context. As with `PlayerView`, callers supply a coherent player/window. Provider
/// lookups use identity.id; position, goalie applicability and teams use season
/// facts. Contract facts are carried unchanged; the current evaluator has no
/// contract predicates. Creating this view does not allocate or clone any data.
///
/// ```compile_fail,E0506
/// use icelines_core::{fixtures, SeasonStatsRead};
/// use icelines_query::PlayerRead;
/// let identity = fixtures::identity(1).build();
/// let mut row = fixtures::stats(1, 20252026, "EDM").build();
/// let read = PlayerRead { identity: &identity, stats: &row, contract: None };
/// let totals = read.stats.totals();
/// row.totals.points += 1;
/// std::hint::black_box(totals.points);
/// ```
pub struct PlayerRead<'a, R: SeasonStatsRead + ?Sized = SeasonStats> {
    pub identity: &'a PlayerIdentity,
    pub stats: &'a R,
    pub contract: Option<&'a PlayerContract>,
}

impl<R: SeasonStatsRead + ?Sized> PlayerRead<'_, R> {
    pub fn position(&self) -> Position {
        self.stats.position()
    }
    pub fn is_goalie(&self) -> bool {
        self.stats.goalie().is_some()
    }
    /// Same last-stint semantics as the existing PlayerView.
    pub fn team(&self) -> Option<&TeamAbbr> {
        self.stats.team_stints().last().map(|s| &s.team)
    }
}
