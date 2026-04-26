use crate::schema::{RosterPlayer, SkaterBio, SkaterStats};
use icelines_core::{
    model::{GpStatus, Player, Season},
    name::normalize_name,
    scoring::compute_pace_score,
    Position, TeamAbbr,
};
use std::collections::HashMap;

/// Build a Vec<Player> by joining roster, bio, and stats on player_id.
///
/// roster  — from /v1/roster/{TEAM}/{SEASON}
/// bios    — from /stats/rest/en/skater/bios (indexed by player_id)
/// stats   — from /stats/rest/en/skater/summary (indexed by player_id)
/// season  — the Season this data belongs to
/// team    — the team abbreviation
pub fn build_players(
    roster: &[RosterPlayer],
    bios: &HashMap<u32, &SkaterBio>,
    stats: &HashMap<u32, &SkaterStats>,
    _season: Season,
    team: &TeamAbbr,
) -> Vec<Player> {
    roster
        .iter()
        .filter_map(|rp| {
            build_one(
                rp,
                bios.get(&rp.id).copied(),
                stats.get(&rp.id).copied(),
                team,
            )
        })
        .collect()
}

fn build_one(
    rp: &RosterPlayer,
    bio: Option<&SkaterBio>,
    stats: Option<&SkaterStats>,
    team: &TeamAbbr,
) -> Option<Player> {
    let position = Position::from_api_code(&rp.position_code)?;

    let first = rp.first_name.as_str();
    let last = rp.last_name.as_str();
    let full_name = format!("{first} {last}");

    // Goals and assists come from stats; fallback to bio totals if stats missing
    let goals = stats
        .map(|s| s.goals)
        .or_else(|| bio.map(|b| b.goals))
        .unwrap_or(0);
    let assists = stats
        .map(|s| s.assists)
        .or_else(|| bio.map(|b| b.assists))
        .unwrap_or(0);
    let gp = stats
        .map(|s| s.games_played)
        .or_else(|| bio.map(|b| b.games_played))
        .unwrap_or(0);

    let gp_status = GpStatus::from_gp(gp);
    let pace_score = compute_pace_score(goals, assists, gp);

    // Eligible positions: from API positionCode only (Yahoo CSV is optional overlay)
    let eligible_pos = match position {
        Position::Center
        | Position::LeftWing
        | Position::RightWing
        | Position::Defense
        | Position::Goalie => vec![position],
    };

    Some(Player {
        nhl_id: Some(rp.id),
        full_name: full_name.clone(),
        name_normalized: normalize_name(&full_name),
        team: team.clone(),
        position,
        eligible_pos,
        gp_status,
        season_goals: goals,
        season_assists: assists,
        season_points: goals + assists,
        pace_score,
        headshot_url: rp.headshot.clone(),
        birth_date: bio.and_then(|b| b.birth_date.clone()),
        birth_country: bio.and_then(|b| b.birth_country.clone()),
        nationality_code: bio.and_then(|b| b.nationality_code.clone()),
        shoots_catches: bio.and_then(|b| b.shoots_catches.clone()),
        draft_year: bio.and_then(|b| b.draft_year.map(|v| v as u16)),
        draft_round: bio.and_then(|b| b.draft_round.map(|v| v as u8)),
        draft_overall: bio.and_then(|b| b.draft_overall.map(|v| v as u16)),
        rookie_season: bio.and_then(|b| b.first_season_for_game_type),
    })
}

/// Index a slice by player_id for O(1) lookups.
pub fn index_bios(bios: &[SkaterBio]) -> HashMap<u32, &SkaterBio> {
    bios.iter().map(|b| (b.player_id, b)).collect()
}

pub fn index_stats(stats: &[SkaterStats]) -> HashMap<u32, &SkaterStats> {
    stats.iter().map(|s| (s.player_id, s)).collect()
}
