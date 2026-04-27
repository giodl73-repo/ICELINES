use crate::moneypuck::MoneyPuckStats;
use crate::schema::{PlayerContract, RosterPlayer, SkaterBio, SkaterRealtime, SkaterStats};
use icelines_core::{
    model::{GpStatus, Player, Season},
    name::normalize_name,
    scoring::compute_pace_score,
    Position, TeamAbbr,
};
use std::collections::HashMap;

/// All inputs required to build a single Player.
pub struct BuildInputs<'a> {
    pub nhl_id: u32,
    pub full_name: String,
    pub team: &'a TeamAbbr,
    pub position: Position,
    pub headshot_url: Option<String>,
    pub sweater_number: Option<u32>,
    pub bio: Option<&'a SkaterBio>,
    pub stats: Option<&'a SkaterStats>,
    pub realtime: Option<&'a SkaterRealtime>,
    pub mp: Option<&'a MoneyPuckStats>,
    pub contract: Option<&'a PlayerContract>,
    pub fallback_gp: u32,
}

/// Build a Vec<Player> by joining roster, bio, and stats on player_id.
///
/// roster    — from /v1/roster/{TEAM}/{SEASON}
/// bios      — from /stats/rest/en/skater/bios (indexed by player_id)
/// stats     — from /stats/rest/en/skater/summary (indexed by player_id)
/// realtime  — from /stats/rest/en/skater/realtime (indexed by player_id)
/// mp        — MoneyPuck stats (indexed by player_id; empty map = not fetched)
/// contracts — contract data (indexed by player_id; empty map = not fetched)
/// season    — the Season this data belongs to
/// team      — the team abbreviation
#[allow(clippy::too_many_arguments)]
pub fn build_players(
    roster: &[RosterPlayer],
    bios: &HashMap<u32, &SkaterBio>,
    stats: &HashMap<u32, &SkaterStats>,
    realtime: &HashMap<u32, SkaterRealtime>,
    mp: &HashMap<u32, MoneyPuckStats>,
    contracts: &HashMap<u32, PlayerContract>,
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
                realtime.get(&rp.id),
                mp.get(&rp.id),
                contracts.get(&rp.id),
                team,
            )
        })
        .collect()
}

/// Shared Player construction from resolved inputs.
pub fn make_player(inputs: BuildInputs<'_>) -> Player {
    let BuildInputs {
        nhl_id,
        full_name,
        team,
        position,
        headshot_url,
        sweater_number,
        bio,
        stats,
        realtime,
        mp,
        contract,
        fallback_gp,
    } = inputs;

    let goals = stats
        .map(|s| s.goals)
        .or_else(|| bio.map(|b| b.goals))
        .unwrap_or(0);
    let assists = stats
        .map(|s| s.assists)
        .or_else(|| bio.map(|b| b.assists))
        .unwrap_or(0);
    let gp = stats.map(|s| s.games_played).unwrap_or(fallback_gp);

    Player {
        nhl_id: Some(nhl_id),
        full_name: full_name.clone(),
        name_normalized: normalize_name(&full_name),
        team: team.clone(),
        position,
        eligible_pos: vec![position],
        gp_status: GpStatus::from_gp(gp),
        season_goals: goals,
        season_assists: assists,
        season_points: goals + assists,
        pace_score: compute_pace_score(goals, assists, gp),

        // Power play
        pp_goals: stats.map(|s| s.pp_goals).unwrap_or(0),
        pp_points: stats.map(|s| s.pp_points).unwrap_or(0),

        // Shorthanded
        sh_goals: stats.map(|s| s.sh_goals).unwrap_or(0),
        sh_points: stats.map(|s| s.sh_points).unwrap_or(0),

        // Other scoring
        gwg: stats.map(|s| s.game_winning_goals).unwrap_or(0),
        ot_goals: stats.map(|s| s.ot_goals).unwrap_or(0),

        // Shot metrics
        shots: stats.map(|s| s.shots).unwrap_or(0),
        shooting_pct: stats.and_then(|s| s.shooting_pctg),

        // Two-way / ice time
        plus_minus: stats.map(|s| s.plus_minus).unwrap_or(0),
        toi_per_game_sec: stats.and_then(|s| s.time_on_ice_per_game),
        faceoff_win_pct: stats.and_then(|s| s.faceoff_win_pct),

        // Physical / two-way stats (NHL realtime API)
        hits: realtime.map(|r| r.hits).unwrap_or(0),
        blocked_shots: realtime.map(|r| r.blocked_shots).unwrap_or(0),
        missed_shots: realtime.map(|r| r.missed_shots).unwrap_or(0),
        giveaways: realtime.map(|r| r.giveaways).unwrap_or(0),
        takeaways: realtime.map(|r| r.takeaways).unwrap_or(0),
        pim: realtime.map(|r| r.pim).unwrap_or(0),

        // MoneyPuck advanced metrics
        xg: mp.map(|m| m.xg_all),
        xg_per_60: mp.map(|m| m.xg_per_60),
        cf_pct_5v5: mp.map(|m| m.cf_pct_5v5),
        ff_pct_5v5: mp.map(|m| m.ff_pct_5v5),
        xgf_pct_5v5: mp.map(|m| m.xgf_pct_5v5),

        // Headshot / display
        headshot_url,
        sweater_number,

        // Bio / demographics
        birth_date: bio.and_then(|b| b.birth_date.clone()),
        birth_country: bio.and_then(|b| b.birth_country.clone()),
        nationality_code: bio.and_then(|b| b.nationality_code.clone()),
        birth_city: bio.and_then(|b| b.birth_city.clone()),
        birth_state_province: bio.and_then(|b| b.birth_state_province_code.clone()),
        shoots_catches: bio.and_then(|b| b.shoots_catches.clone()),
        height_in_inches: bio.and_then(|b| b.height),
        weight_lbs: bio.and_then(|b| b.weight),

        // Draft / career
        draft_year: bio.and_then(|b| b.draft_year.map(|v| v as u16)),
        draft_round: bio.and_then(|b| b.draft_round.map(|v| v as u8)),
        draft_overall: bio.and_then(|b| b.draft_overall.map(|v| v as u16)),
        rookie_season: bio.and_then(|b| b.first_season_for_game_type),

        // Contract (from NHL landing API — None if not fetched)
        contract_expiry_year: contract.and_then(|c| c.expiry_year),
        expiry_type: contract.and_then(|c| c.expiry_type.clone()),
        salary: contract.and_then(|c| c.salary),
    }
}

fn build_one(
    rp: &RosterPlayer,
    bio: Option<&SkaterBio>,
    stats: Option<&SkaterStats>,
    realtime: Option<&SkaterRealtime>,
    mp: Option<&MoneyPuckStats>,
    contract: Option<&PlayerContract>,
    team: &TeamAbbr,
) -> Option<Player> {
    let position = Position::from_api_code(&rp.position_code)?;
    let full_name = format!("{} {}", rp.first_name.as_str(), rp.last_name.as_str());
    let fallback_gp = bio.map(|b| b.games_played).unwrap_or(0);

    let mut p = make_player(BuildInputs {
        nhl_id: rp.id,
        full_name,
        team,
        position,
        headshot_url: rp.headshot.clone(),
        sweater_number: rp.sweater_number,
        bio,
        stats,
        realtime,
        mp,
        contract,
        fallback_gp,
    });

    // Roster has height/weight when bio doesn't
    if p.height_in_inches.is_none() {
        p.height_in_inches = rp.height_in_inches;
    }
    if p.weight_lbs.is_none() {
        p.weight_lbs = rp.weight_in_pounds;
    }

    Some(p)
}

/// Build players directly from bios (no roster required) — cold-start path.
///
/// The NHL bios API returns multiple rows per player when they were traded
/// mid-season. We deduplicate by player_id, keeping the last occurrence
/// (most recent team / most accumulated stats in the ordered API response).
pub fn build_players_from_bios(
    bios: &[SkaterBio],
    stats_idx: &HashMap<u32, &SkaterStats>,
    realtime: &HashMap<u32, SkaterRealtime>,
    mp: &HashMap<u32, MoneyPuckStats>,
    contracts: &HashMap<u32, PlayerContract>,
    _season: icelines_core::model::Season,
) -> Vec<Player> {
    // For traded players the API emits one row per team stint; keep the last
    // (current-team) row for each player_id so they don't appear twice.
    let mut seen_ids: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let deduped: Vec<&SkaterBio> = bios.iter().rev()
        .filter(|bio| seen_ids.insert(bio.player_id))
        .collect::<Vec<_>>()
        .into_iter().rev().collect();

    deduped.iter()
        .filter_map(|bio| {
            let position = Position::from_api_code(&bio.position_code)?;
            if !position.is_forward() && !position.is_defense() {
                return None;
            }
            let team_str = bio.current_team_abbrev.as_deref().unwrap_or("");
            if team_str.is_empty() {
                return None;
            }
            let stats = stats_idx.get(&bio.player_id).copied();
            let fallback_gp = bio.games_played;
            let team = TeamAbbr(team_str.to_owned());
            Some(make_player(BuildInputs {
                nhl_id: bio.player_id,
                full_name: bio.skater_full_name.clone(),
                team: &team,
                position,
                headshot_url: None,
                sweater_number: None,
                bio: Some(bio),
                stats,
                realtime: realtime.get(&bio.player_id),
                mp: mp.get(&bio.player_id),
                contract: contracts.get(&bio.player_id),
                fallback_gp,
            }))
        })
        .collect()
}

/// Index a slice by player_id for O(1) lookups.
pub fn index_bios(bios: &[SkaterBio]) -> HashMap<u32, &SkaterBio> {
    bios.iter().map(|b| (b.player_id, b)).collect()
}

pub fn index_stats(stats: &[SkaterStats]) -> HashMap<u32, &SkaterStats> {
    stats.iter().map(|s| (s.player_id, s)).collect()
}

pub fn index_realtime(rt: &[SkaterRealtime]) -> HashMap<u32, SkaterRealtime> {
    rt.iter().map(|r| (r.player_id, r.clone())).collect()
}

pub fn index_contracts(contracts: &[PlayerContract]) -> HashMap<u32, PlayerContract> {
    contracts.iter().map(|c| (c.player_id, c.clone())).collect()
}
