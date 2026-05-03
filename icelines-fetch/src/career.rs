//! Multi-season career stats assembly from bundled and snapshot data.

use crate::{bundled, snapshot::SnapshotStore};
use icelines_core::{
    history::{CareerSummary, SeasonLine},
    name::normalize_name,
};

/// Build a CareerSummary for a player by searching bundled season data.
///
/// Searches across all bundled seasons (newest first) for matching player.
/// Returns None if player not found in any season.
pub fn load_career(
    player_name: &str,
    n_seasons: usize,
    _store: &SnapshotStore, // reserved for future snapshot career data
) -> Option<CareerSummary> {
    let norm = normalize_name(player_name);
    let mut player_id: Option<u32> = None;
    let mut full_name = player_name.to_owned();
    let mut lines: Vec<SeasonLine> = Vec::new();

    let seasons = bundled::BUNDLED_SEASONS.iter().take(n_seasons);

    for &season in seasons {
        let bios = match bundled::get_bios(season) {
            Some(b) => b,
            None => continue,
        };
        let stats_opt = bundled::get_stats(season);

        // Find this player in the season — skip seasons they didn't play in
        let bio = match bios
            .iter()
            .find(|b| normalize_name(&b.skater_full_name).contains(&norm))
        {
            Some(b) => b,
            None => continue,
        };

        if player_id.is_none() {
            player_id = Some(bio.player_id);
            full_name = bio.skater_full_name.clone();
        }

        // Look up their stats for this season
        let (goals, assists, gp) = if let Some(ref stats) = stats_opt {
            if let Some(s) = stats.iter().find(|s| s.player_id == bio.player_id) {
                (s.goals, s.assists, s.games_played)
            } else {
                (bio.goals, bio.assists, bio.games_played)
            }
        } else {
            (bio.goals, bio.assists, bio.games_played)
        };

        let team = bio.current_team_abbrev.as_deref().unwrap_or("—");
        lines.push(SeasonLine::new(season, team, gp, goals, assists));
    }

    if lines.is_empty() {
        return None;
    }

    Some(CareerSummary::from_seasons(player_id, &full_name, lines))
}
