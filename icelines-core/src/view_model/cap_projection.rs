use crate::model::{Position, Season};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub const CAP_PROJECTION_SCHEMA: &str = "cap_projection.v1";
pub const CAP_PROJECTION_METHOD: &str = "current-roster market-cost scenario";
pub const CAP_LIMIT_SOURCE_URL: &str = "https://www.nhl.com/news/topic/catching-up-with/nhl-nhlpa-announce-team-payroll-ranges-for-next-3-seasons-through-2027-28";
pub const CARLSSON_ANCHOR_URL: &str =
    "https://www.nhl.com/ducks/news/ducks-match-five-year-offer-sheet-for-carlsson";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapLimitAuthority {
    Official,
    AnnouncedProjection,
    Scenario,
}

impl CapLimitAuthority {
    pub fn label(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::AnnouncedProjection => "announced projection",
            Self::Scenario => "scenario",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapPressure {
    Flexible,
    Tight,
    Critical,
    MarketGap,
}

impl CapPressure {
    pub fn from_share(cap_share_pct: f64) -> Self {
        if cap_share_pct >= 100.0 {
            Self::MarketGap
        } else if cap_share_pct >= 95.0 {
            Self::Critical
        } else if cap_share_pct >= 85.0 {
            Self::Tight
        } else {
            Self::Flexible
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Flexible => "flexible",
            Self::Tight => "tight",
            Self::Critical => "critical",
            Self::MarketGap => "market-gap",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SalaryBasis {
    Confirmed,
    Modeled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapProjectionRole {
    FranchiseForward,
    EliteForward,
    FirstLineForward,
    TopSixForward,
    ThirdLineForward,
    FourthLineForward,
    TopDefense,
    SecondPairDefense,
    ThirdPairDefense,
    StartingGoalie,
    TandemGoalie,
    DepthGoalie,
}

impl CapProjectionRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::FranchiseForward => "franchise F",
            Self::EliteForward => "elite F",
            Self::FirstLineForward => "first-line F",
            Self::TopSixForward => "top-six F",
            Self::ThirdLineForward => "third-line F",
            Self::FourthLineForward => "fourth-line F",
            Self::TopDefense => "top D",
            Self::SecondPairDefense => "second-pair D",
            Self::ThirdPairDefense => "third-pair D",
            Self::StartingGoalie => "starting G",
            Self::TandemGoalie => "tandem G",
            Self::DepthGoalie => "depth G",
        }
    }

    pub fn market_cap_share_pct(self) -> f64 {
        match self {
            // Carlsson's $18M on a $104M cap is the explicit young-star anchor.
            Self::FranchiseForward => 17.3,
            Self::EliteForward => 13.0,
            Self::FirstLineForward => 11.5,
            Self::TopSixForward => 7.0,
            Self::ThirdLineForward => 3.85,
            Self::FourthLineForward => 1.6,
            Self::TopDefense => 10.5,
            Self::SecondPairDefense => 6.0,
            Self::ThirdPairDefense => 2.75,
            Self::StartingGoalie => 8.0,
            Self::TandemGoalie => 4.5,
            Self::DepthGoalie => 1.8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapProjectionContractInput {
    pub valuation_season: Option<u32>,
    pub expiry_year: Option<u16>,
    pub cap_hit: Option<u64>,
    pub aav: Option<u64>,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapProjectionPlayerInput {
    pub player_id: u32,
    pub player: String,
    pub team: String,
    pub position: Position,
    pub age: u8,
    pub games_played: u32,
    pub points_per_82: Option<f64>,
    pub contract: Option<CapProjectionContractInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapProjectionAssumptions {
    pub base_season: u32,
    pub years: u8,
    pub modeled_growth_pct: f64,
    pub cap_limit_source_url: String,
    pub market_anchor: String,
    pub market_anchor_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapLimitProjection {
    pub season: u32,
    pub upper_limit: u64,
    pub authority: CapLimitAuthority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerCapProjection {
    pub player_id: u32,
    pub player: String,
    pub team: String,
    pub position: Position,
    pub age: u8,
    pub points_per_82: Option<f64>,
    pub role: CapProjectionRole,
    pub salary_basis: SalaryBasis,
    pub projected_cap_hit_low: u64,
    pub projected_cap_hit: u64,
    pub projected_cap_hit_high: u64,
    pub cap_share_pct: f64,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonCapProjection {
    pub season: u32,
    pub upper_limit: u64,
    pub cap_limit_authority: CapLimitAuthority,
    pub roster_players: u32,
    pub source_roster_players: u32,
    pub excluded_depth_players: u32,
    pub confirmed_players: u32,
    pub modeled_players: u32,
    pub projected_cap_hit_low: u64,
    pub projected_cap_hit: u64,
    pub projected_cap_hit_high: u64,
    pub cap_share_pct: f64,
    pub pressure: CapPressure,
    pub players: Vec<PlayerCapProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCapProjection {
    pub team: String,
    pub seasons: Vec<TeamSeasonCapProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapProjectionView {
    pub schema: String,
    pub method: String,
    pub assumptions: CapProjectionAssumptions,
    pub cap_limits: Vec<CapLimitProjection>,
    pub teams: Vec<TeamCapProjection>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CapProjectionError {
    #[error("projection years must be between 1 and 10; got {0}")]
    InvalidYears(u8),
    #[error("modeled cap growth must be finite and between -10% and 25%; got {0}")]
    InvalidGrowth(f64),
    #[error("base projection season must be 2025-26 or later; got {0}")]
    InvalidBaseSeason(u32),
    #[error("projection input has no current-roster players")]
    EmptyRoster,
}

pub fn build_cap_projection(
    players: Vec<CapProjectionPlayerInput>,
    base_season: Season,
    years: u8,
    modeled_growth_pct: f64,
) -> Result<CapProjectionView, CapProjectionError> {
    if !(1..=10).contains(&years) {
        return Err(CapProjectionError::InvalidYears(years));
    }
    if !modeled_growth_pct.is_finite() || !(-10.0..=25.0).contains(&modeled_growth_pct) {
        return Err(CapProjectionError::InvalidGrowth(modeled_growth_pct));
    }
    if base_season.0 < 20_252_026 {
        return Err(CapProjectionError::InvalidBaseSeason(base_season.0));
    }
    if players.is_empty() {
        return Err(CapProjectionError::EmptyRoster);
    }

    let cap_limits = project_cap_limits(base_season, years, modeled_growth_pct);
    let mut by_team: BTreeMap<String, Vec<CapProjectionPlayerInput>> = BTreeMap::new();
    for player in players {
        by_team.entry(player.team.clone()).or_default().push(player);
    }

    let teams = by_team
        .into_iter()
        .map(|(team, mut roster)| {
            roster.sort_by(|a, b| a.player.cmp(&b.player));
            let source_roster_players = roster.len() as u32;
            let active_roster = select_active_roster(&roster);
            let seasons = cap_limits
                .iter()
                .map(|cap| {
                    project_team_season(
                        &team,
                        &active_roster,
                        source_roster_players,
                        base_season,
                        cap,
                    )
                })
                .collect();
            TeamCapProjection { team, seasons }
        })
        .collect();

    Ok(CapProjectionView {
        schema: CAP_PROJECTION_SCHEMA.to_owned(),
        method: CAP_PROJECTION_METHOD.to_owned(),
        assumptions: CapProjectionAssumptions {
            base_season: base_season.0,
            years,
            modeled_growth_pct,
            cap_limit_source_url: CAP_LIMIT_SOURCE_URL.to_owned(),
            market_anchor: "Leo Carlsson: $18M AAV beginning 2026-27".to_owned(),
            market_anchor_url: CARLSSON_ANCHOR_URL.to_owned(),
        },
        cap_limits,
        teams,
        disclosures: vec![
            "2026-27 uses the official $104M upper limit; 2027-28 uses the announced $113.5M projection, which remains subject to adjustment.".to_owned(),
            "Later cap limits compound the selected scenario growth rate and are not NHL/NHLPA forecasts.".to_owned(),
            "Confirmed cap hits are carried through a known expiry year; all other values are role-market estimates tied to that season's cap.".to_owned(),
            "Low/high modeled values are an 85%-115% scenario band around the role midpoint.".to_owned(),
            "Team totals use a deterministic 23-player active-roster scenario: up to 14 forwards, 7 defensemen, and 2 goalies, ranked by games played and production.".to_owned(),
            "Modeled market values apply a 5% age discount at 30-31 and a 15% discount at 32 or older.".to_owned(),
            "Player production and role are held at the source-season level while age advances in each forecast season.".to_owned(),
        ],
        non_claims: vec![
            "This report does not predict trades, waivers, buyouts, retention, injuries, roster turnover, term, clauses, or signing bonuses.".to_owned(),
            "Current-roster market cost is not the same as a team's legally committed cap payroll.".to_owned(),
            "Missing or low-sample production is classified conservatively and never represented as a confirmed salary.".to_owned(),
        ],
    })
}

pub fn classify_cap_role(player: &CapProjectionPlayerInput) -> CapProjectionRole {
    let pace = player.points_per_82.unwrap_or(0.0);
    match player.position {
        Position::Goalie => {
            if player.games_played >= 40 {
                CapProjectionRole::StartingGoalie
            } else if player.games_played >= 20 {
                CapProjectionRole::TandemGoalie
            } else {
                CapProjectionRole::DepthGoalie
            }
        }
        Position::Defense => {
            if pace >= 45.0 || (player.age <= 23 && pace >= 35.0) {
                CapProjectionRole::TopDefense
            } else if pace >= 25.0 {
                CapProjectionRole::SecondPairDefense
            } else {
                CapProjectionRole::ThirdPairDefense
            }
        }
        Position::Center | Position::LeftWing | Position::RightWing => {
            if pace >= 90.0 || (player.age <= 24 && pace >= 65.0) {
                CapProjectionRole::FranchiseForward
            } else if pace >= 75.0 {
                CapProjectionRole::EliteForward
            } else if pace >= 60.0 {
                CapProjectionRole::FirstLineForward
            } else if pace >= 40.0 {
                CapProjectionRole::TopSixForward
            } else if pace >= 25.0 {
                CapProjectionRole::ThirdLineForward
            } else {
                CapProjectionRole::FourthLineForward
            }
        }
    }
}

fn project_cap_limits(
    base_season: Season,
    years: u8,
    modeled_growth_pct: f64,
) -> Vec<CapLimitProjection> {
    let mut rows = Vec::with_capacity(years as usize);
    for offset in 0..years {
        let season = add_seasons(base_season, offset as u32);
        let (upper_limit, authority) = known_cap_limit(season.0).unwrap_or_else(|| {
            let prior = rows
                .last()
                .map(|row: &CapLimitProjection| row.upper_limit)
                .unwrap_or(113_500_000);
            let projected = ((prior as f64 * (1.0 + modeled_growth_pct / 100.0)) / 100_000.0)
                .round() as u64
                * 100_000;
            (projected, CapLimitAuthority::Scenario)
        });
        rows.push(CapLimitProjection {
            season: season.0,
            upper_limit,
            authority,
        });
    }
    rows
}

fn known_cap_limit(season: u32) -> Option<(u64, CapLimitAuthority)> {
    match season {
        20_252_026 => Some((95_500_000, CapLimitAuthority::Official)),
        20_262_027 => Some((104_000_000, CapLimitAuthority::Official)),
        20_272_028 => Some((113_500_000, CapLimitAuthority::AnnouncedProjection)),
        _ => None,
    }
}

fn project_team_season(
    team: &str,
    roster: &[CapProjectionPlayerInput],
    source_roster_players: u32,
    base_season: Season,
    cap: &CapLimitProjection,
) -> TeamSeasonCapProjection {
    let mut players: Vec<_> = roster
        .iter()
        .map(|player| project_player(team, player, base_season, cap))
        .collect();
    players.sort_by(|a, b| {
        b.projected_cap_hit
            .cmp(&a.projected_cap_hit)
            .then_with(|| a.player.cmp(&b.player))
    });
    let confirmed_players = players
        .iter()
        .filter(|row| row.salary_basis == SalaryBasis::Confirmed)
        .count() as u32;
    let projected_cap_hit_low = players.iter().map(|row| row.projected_cap_hit_low).sum();
    let projected_cap_hit = players.iter().map(|row| row.projected_cap_hit).sum();
    let projected_cap_hit_high = players.iter().map(|row| row.projected_cap_hit_high).sum();
    let cap_share_pct = percentage(projected_cap_hit, cap.upper_limit);

    TeamSeasonCapProjection {
        season: cap.season,
        upper_limit: cap.upper_limit,
        cap_limit_authority: cap.authority,
        roster_players: players.len() as u32,
        source_roster_players,
        excluded_depth_players: source_roster_players.saturating_sub(players.len() as u32),
        confirmed_players,
        modeled_players: players.len() as u32 - confirmed_players,
        projected_cap_hit_low,
        projected_cap_hit,
        projected_cap_hit_high,
        cap_share_pct,
        pressure: CapPressure::from_share(cap_share_pct),
        players,
    }
}

fn project_player(
    team: &str,
    player: &CapProjectionPlayerInput,
    base_season: Season,
    cap: &CapLimitProjection,
) -> PlayerCapProjection {
    let role = classify_cap_role(player);
    let projected_age = player.age.saturating_add(
        Season(cap.season)
            .start_year()
            .saturating_sub(base_season.start_year()) as u8,
    );
    let confirmed = player
        .contract
        .as_ref()
        .and_then(|contract| confirmed_cap_hit(contract, cap.season));
    let (salary_basis, low, midpoint, high, source, source_url) = match confirmed {
        Some(value) => (
            SalaryBasis::Confirmed,
            value,
            value,
            value,
            player.contract.as_ref().and_then(|row| row.source.clone()),
            player
                .contract
                .as_ref()
                .and_then(|row| row.source_url.clone()),
        ),
        None => {
            let midpoint = round_to_50k(
                cap.upper_limit as f64
                    * role.market_cap_share_pct()
                    * age_market_factor(projected_age)
                    / 100.0,
            );
            (
                SalaryBasis::Modeled,
                round_to_50k(midpoint as f64 * 0.85),
                midpoint,
                round_to_50k(midpoint as f64 * 1.15),
                Some("icelines-role-market-scenario".to_owned()),
                Some(CARLSSON_ANCHOR_URL.to_owned()),
            )
        }
    };

    PlayerCapProjection {
        player_id: player.player_id,
        player: player.player.clone(),
        team: team.to_owned(),
        position: player.position,
        age: projected_age,
        points_per_82: player.points_per_82,
        role,
        salary_basis,
        projected_cap_hit_low: low,
        projected_cap_hit: midpoint,
        projected_cap_hit_high: high,
        cap_share_pct: percentage(midpoint, cap.upper_limit),
        source,
        source_url,
    }
}

fn select_active_roster(roster: &[CapProjectionPlayerInput]) -> Vec<CapProjectionPlayerInput> {
    let mut forwards: Vec<_> = roster
        .iter()
        .filter(|player| player.position.is_forward())
        .cloned()
        .collect();
    let mut defense: Vec<_> = roster
        .iter()
        .filter(|player| player.position.is_defense())
        .cloned()
        .collect();
    let mut goalies: Vec<_> = roster
        .iter()
        .filter(|player| player.position == Position::Goalie)
        .cloned()
        .collect();
    sort_roster_priority(&mut forwards);
    sort_roster_priority(&mut defense);
    sort_roster_priority(&mut goalies);

    let mut selected = Vec::new();
    selected.extend(forwards.iter().take(14).cloned());
    selected.extend(defense.iter().take(7).cloned());
    selected.extend(goalies.iter().take(2).cloned());
    if selected.len() < 23 {
        let mut leftovers: Vec<_> = forwards
            .into_iter()
            .skip(14)
            .chain(defense.into_iter().skip(7))
            .chain(goalies.into_iter().skip(2))
            .collect();
        sort_roster_priority(&mut leftovers);
        selected.extend(leftovers.into_iter().take(23 - selected.len()));
    }
    selected
}

fn sort_roster_priority(players: &mut [CapProjectionPlayerInput]) {
    players.sort_by(|a, b| {
        b.games_played
            .cmp(&a.games_played)
            .then_with(|| {
                b.points_per_82
                    .unwrap_or(0.0)
                    .partial_cmp(&a.points_per_82.unwrap_or(0.0))
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| a.player.cmp(&b.player))
    });
}

fn age_market_factor(age: u8) -> f64 {
    if age >= 32 {
        0.85
    } else if age >= 30 {
        0.95
    } else {
        1.0
    }
}

fn confirmed_cap_hit(contract: &CapProjectionContractInput, target_season: u32) -> Option<u64> {
    let value = contract.cap_hit.or(contract.aav)?;
    let valuation_season = contract.valuation_season?;
    if target_season < valuation_season {
        return None;
    }
    match contract.expiry_year {
        Some(expiry_year) if season_end_year(target_season) <= expiry_year => Some(value),
        Some(_) => None,
        None if target_season == valuation_season => Some(value),
        None => None,
    }
}

fn add_seasons(season: Season, offset: u32) -> Season {
    let start = season.start_year() as u32 + offset;
    Season(start * 10_000 + start + 1)
}

fn season_end_year(season: u32) -> u16 {
    (season % 10_000) as u16
}

fn percentage(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

fn round_to_50k(value: f64) -> u64 {
    ((value / 50_000.0).round() as u64) * 50_000
}

pub fn sort_team_seasons_by_pressure(rows: &mut [&TeamSeasonCapProjection]) {
    rows.sort_by(|a, b| {
        b.cap_share_pct
            .partial_cmp(&a.cap_share_pct)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.season.cmp(&b.season))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(
        id: u32,
        team: &str,
        position: Position,
        age: u8,
        gp: u32,
        pace: Option<f64>,
    ) -> CapProjectionPlayerInput {
        CapProjectionPlayerInput {
            player_id: id,
            player: format!("Player {id}"),
            team: team.to_owned(),
            position,
            age,
            games_played: gp,
            points_per_82: pace,
            contract: None,
        }
    }

    #[test]
    fn l0_cap_projection_uses_known_limits_then_scenario_growth() {
        let view = build_cap_projection(
            vec![player(1, "ANA", Position::Center, 21, 70, Some(67.0))],
            Season(20_262_027),
            5,
            5.0,
        )
        .unwrap();
        assert_eq!(view.cap_limits[0].upper_limit, 104_000_000);
        assert_eq!(view.cap_limits[0].authority, CapLimitAuthority::Official);
        assert_eq!(view.cap_limits[1].upper_limit, 113_500_000);
        assert_eq!(
            view.cap_limits[1].authority,
            CapLimitAuthority::AnnouncedProjection
        );
        assert_eq!(view.cap_limits[2].upper_limit, 119_200_000);
        assert_eq!(view.cap_limits[2].authority, CapLimitAuthority::Scenario);
    }

    #[test]
    fn l0_cap_projection_maps_requested_role_spectrum() {
        assert_eq!(
            classify_cap_role(&player(1, "CHI", Position::Center, 20, 70, Some(80.0))),
            CapProjectionRole::FranchiseForward
        );
        assert_eq!(
            classify_cap_role(&player(2, "NYR", Position::LeftWing, 25, 70, Some(32.0))),
            CapProjectionRole::ThirdLineForward
        );
        assert_eq!(
            classify_cap_role(&player(3, "COL", Position::Defense, 25, 70, Some(52.0))),
            CapProjectionRole::TopDefense
        );
        assert_eq!(
            classify_cap_role(&player(4, "SEA", Position::Defense, 28, 70, Some(18.0))),
            CapProjectionRole::ThirdPairDefense
        );
        assert_eq!(
            classify_cap_role(&player(5, "WPG", Position::Goalie, 30, 55, None)),
            CapProjectionRole::StartingGoalie
        );
    }

    #[test]
    fn l0_cap_projection_carries_confirmed_contract_through_expiry_then_models() {
        let mut carlsson = player(1, "ANA", Position::Center, 21, 70, Some(67.0));
        carlsson.contract = Some(CapProjectionContractInput {
            valuation_season: Some(20_262_027),
            expiry_year: Some(2031),
            cap_hit: Some(18_000_000),
            aav: Some(18_000_000),
            source: Some("csv".to_owned()),
            source_url: Some(CARLSSON_ANCHOR_URL.to_owned()),
        });
        let view = build_cap_projection(vec![carlsson], Season(20_262_027), 6, 5.0).unwrap();
        for season in &view.teams[0].seasons[..5] {
            assert_eq!(season.players[0].salary_basis, SalaryBasis::Confirmed);
            assert_eq!(season.players[0].projected_cap_hit, 18_000_000);
        }
        assert_eq!(
            view.teams[0].seasons[5].players[0].salary_basis,
            SalaryBasis::Modeled
        );
    }

    #[test]
    fn l0_cap_projection_third_line_anchor_is_about_four_million() {
        let view = build_cap_projection(
            vec![player(1, "NYR", Position::LeftWing, 25, 70, Some(32.0))],
            Season(20_262_027),
            1,
            5.0,
        )
        .unwrap();
        let row = &view.teams[0].seasons[0].players[0];
        assert_eq!(row.role, CapProjectionRole::ThirdLineForward);
        assert_eq!(row.projected_cap_hit, 4_000_000);
    }

    #[test]
    fn l0_cap_projection_separates_veteran_elite_from_young_franchise_anchor() {
        let view = build_cap_projection(
            vec![player(1, "NYR", Position::Center, 33, 70, Some(79.0))],
            Season(20_262_027),
            1,
            5.0,
        )
        .unwrap();
        let row = &view.teams[0].seasons[0].players[0];
        assert_eq!(row.role, CapProjectionRole::EliteForward);
        assert_eq!(row.projected_cap_hit, 11_500_000);
    }

    #[test]
    fn l0_cap_projection_selects_active_23_and_discloses_depth_exclusions() {
        let mut roster = Vec::new();
        for id in 1..=16 {
            roster.push(player(id, "NYR", Position::Center, 25, id, Some(id as f64)));
        }
        for id in 17..=25 {
            roster.push(player(
                id,
                "NYR",
                Position::Defense,
                25,
                id,
                Some(id as f64),
            ));
        }
        for id in 26..=28 {
            roster.push(player(id, "NYR", Position::Goalie, 28, id, None));
        }
        let view = build_cap_projection(roster, Season(20_262_027), 1, 5.0).unwrap();
        let row = &view.teams[0].seasons[0];
        assert_eq!(row.roster_players, 23);
        assert_eq!(row.source_roster_players, 28);
        assert_eq!(row.excluded_depth_players, 5);
        assert_eq!(
            row.players
                .iter()
                .filter(|player| player.position == Position::Goalie)
                .count(),
            2
        );
    }

    #[test]
    fn l0_cap_projection_advances_age_across_forecast_seasons() {
        let view = build_cap_projection(
            vec![player(1, "NYR", Position::LeftWing, 29, 70, Some(32.0))],
            Season(20_262_027),
            3,
            5.0,
        )
        .unwrap();
        assert_eq!(view.teams[0].seasons[0].players[0].age, 29);
        assert_eq!(view.teams[0].seasons[1].players[0].age, 30);
        assert_eq!(view.teams[0].seasons[2].players[0].age, 31);
    }

    #[test]
    fn l0_cap_projection_rejects_invalid_assumptions() {
        let roster = vec![player(1, "NYR", Position::Center, 25, 70, Some(40.0))];
        assert!(matches!(
            build_cap_projection(roster.clone(), Season(20_262_027), 0, 5.0),
            Err(CapProjectionError::InvalidYears(0))
        ));
        assert!(matches!(
            build_cap_projection(roster, Season(20_262_027), 5, f64::NAN),
            Err(CapProjectionError::InvalidGrowth(_))
        ));
    }
}
