//! Start-of-season professional-game evidence for AHL development rules.
//!
//! The official rule defines the threshold, but does not expose a machine-
//! readable league catalog. This adapter therefore requires a reviewed,
//! versioned treatment for every professional league observed in the career
//! source. Unknown professional leagues stay visible and fail closed.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use icelines_core::{CareerGameType, LeagueTier, Position};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::{
        AhlFeedError, AhlIdentityCrosswalkView, AhlIdentityLeagueCrosswalkView,
        AhlIdentityReviewStatus, AhlProjectionPlayerFacts, AHL_IDENTITY_CROSSWALK_SCHEMA,
        AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA,
    },
    career_landing::CareerHistoryStore,
};

pub const AHL_PROFESSIONAL_GAME_POLICY_SCHEMA: &str = "ahl_professional_game_policy.v2";
pub const AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA: &str = "ahl_professional_game_ledger.v2";
pub const AHL_PROFESSIONAL_GAME_FACTS_SCHEMA: &str = "ahl_professional_game_facts_application.v2";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlProfessionalGamePolicyAuthority {
    #[default]
    Draft,
    Provisional,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlProfessionalLeagueTreatmentKind {
    Included,
    EuropeanElite,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProfessionalLeagueTreatment {
    /// Exact, case-insensitive league abbreviation from NHL career history.
    pub league: String,
    pub kind: AhlProfessionalLeagueTreatmentKind,
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProfessionalGamePolicy {
    pub schema: String,
    pub policy_id: String,
    #[serde(default)]
    pub authority_status: AhlProfessionalGamePolicyAuthority,
    pub target_season: u32,
    /// Season for which the base dressed-skater threshold authority is effective.
    pub development_rule_effective_season: u32,
    pub as_of: String,
    pub threshold: u32,
    pub source_urls: Vec<String>,
    pub league_treatments: Vec<AhlProfessionalLeagueTreatment>,
    #[serde(default)]
    pub age_qualification: Option<AhlDevelopmentAgeQualification>,
    #[serde(default)]
    pub european_elite_youth_exemption: Option<AhlEuropeanEliteYouthExemption>,
    pub methodology: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlDevelopmentAgeQualification {
    /// Season for which this exception authority is effective.
    pub effective_season: u32,
    /// ISO date on which age is tested for the target season.
    pub cutoff_date: String,
    /// A player younger than this age on the cutoff automatically qualifies.
    pub automatically_qualifies_under_age: u8,
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlEuropeanEliteYouthExemption {
    /// Season for which this exception authority is effective.
    pub effective_season: u32,
    /// Maximum age during the season's opening calendar year whose European
    /// elite games are exempt. This excludes the CHL over-age year.
    pub maximum_non_overage_age: u8,
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProfessionalGameLeagueTotal {
    pub league: String,
    pub games: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProfessionalGamePlayerRow {
    pub nhl_player_id: u32,
    pub display_name: String,
    pub affiliate_appearances: usize,
    /// Official primary position from the same NHL landing payload as the
    /// career history. This can fill an otherwise generic AHL `F` position,
    /// but never establishes assignment or multi-position eligibility.
    #[serde(default)]
    pub official_position: Option<Position>,
    pub professional_games_at_season_start: Option<u32>,
    /// Game-count test only. Age and European youth-season exemptions remain
    /// separate rule facts before a final development classification.
    pub within_game_threshold: Option<bool>,
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub age_at_policy_cutoff: Option<u8>,
    #[serde(default)]
    pub automatically_age_qualified: Option<bool>,
    #[serde(default)]
    pub development_rule_qualified: Option<bool>,
    pub included_leagues: Vec<AhlProfessionalGameLeagueTotal>,
    #[serde(default)]
    pub exempted_european_elite_leagues: Vec<AhlProfessionalGameLeagueTotal>,
    pub excluded_leagues: Vec<AhlProfessionalGameLeagueTotal>,
    pub unresolved_professional_leagues: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlProfessionalGameLedgerView {
    pub schema: String,
    pub policy_id: String,
    pub policy_authority_status: AhlProfessionalGamePolicyAuthority,
    pub prior_season: u32,
    pub target_season: u32,
    pub development_rule_effective_season: u32,
    #[serde(default)]
    pub age_qualification_effective_season: Option<u32>,
    #[serde(default)]
    pub european_elite_youth_exemption_effective_season: Option<u32>,
    pub as_of: String,
    pub threshold: u32,
    pub career_store_fetched_at: String,
    pub source_fingerprint: String,
    pub canonical_players: usize,
    pub complete_players: usize,
    pub missing_histories: usize,
    pub unresolved_players: usize,
    pub players: Vec<AhlProfessionalGamePlayerRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProfessionalGameFactsApplicationView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub nhl_team: String,
    pub ahl_team: String,
    pub policy_id: String,
    pub ledger_fingerprint: String,
    pub players_applied: usize,
    pub facts: Vec<AhlProjectionPlayerFacts>,
    pub disclosures: Vec<String>,
}

/// Apply a final, age/exemption-aware ledger to existing team projection facts.
/// Every other fact remains authored by its own authority; this adapter only
/// supplies and verifies professional-game totals and final rule qualification.
pub fn apply_ahl_professional_game_ledger_to_facts(
    crosswalk: &AhlIdentityCrosswalkView,
    ledger: &AhlProfessionalGameLedgerView,
    nhl_team: &str,
    ahl_team: &str,
    facts: &[AhlProjectionPlayerFacts],
) -> Result<AhlProfessionalGameFactsApplicationView, AhlFeedError> {
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != ledger.prior_season
        || crosswalk.ahl_team != ahl_team
        || crosswalk
            .nhl_affiliate
            .as_deref()
            .is_some_and(|affiliate| affiliate != nhl_team)
        || ledger.schema != AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA
        || ledger.policy_authority_status != AhlProfessionalGamePolicyAuthority::Final
        || ledger.development_rule_effective_season != ledger.target_season
        || ledger.age_qualification_effective_season != Some(ledger.target_season)
        || ledger.european_elite_youth_exemption_effective_season != Some(ledger.target_season)
        || ledger.source_fingerprint.trim().is_empty()
        || facts.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "professional-game facts application requires matching team identity and final ledger authority"
                .to_owned(),
        ));
    }
    let identity_by_provider = crosswalk
        .rows
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let ledger_by_player = ledger
        .players
        .iter()
        .map(|row| (row.nhl_player_id, row))
        .collect::<BTreeMap<_, _>>();
    if ledger_by_player.len() != ledger.players.len() {
        return Err(AhlFeedError::Validation(
            "professional-game ledger contains duplicate canonical players".to_owned(),
        ));
    }
    let mut provider_ids = BTreeSet::new();
    let mut applied = Vec::with_capacity(facts.len());
    for fact in facts {
        if !provider_ids.insert(fact.provider_player_id.as_str()) {
            return Err(AhlFeedError::Validation(format!(
                "duplicate projection facts for provider player {}",
                fact.provider_player_id
            )));
        }
        let identity = identity_by_provider
            .get(fact.provider_player_id.as_str())
            .copied()
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "projection facts reference provider player {} absent from team identity",
                    fact.provider_player_id
                ))
            })?;
        let player_id = identity
            .nhl_player_id
            .filter(|_| identity.review_status == AhlIdentityReviewStatus::Reviewed)
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "projection facts reference provider player {} without reviewed canonical identity",
                    fact.provider_player_id
                ))
            })?;
        let evidence = ledger_by_player.get(&player_id).copied().ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "final professional-game ledger has no player {player_id}"
            ))
        })?;
        let games = evidence.professional_games_at_season_start.ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "final professional-game ledger has no total for player {player_id}"
            ))
        })?;
        let qualified = evidence.development_rule_qualified.ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "final professional-game ledger has no qualification for player {player_id}"
            ))
        })?;
        if fact
            .professional_games_at_season_start
            .is_some_and(|existing| existing != games)
            || fact
                .development_rule_qualified
                .is_some_and(|existing| existing != qualified)
        {
            return Err(AhlFeedError::Validation(format!(
                "projection facts conflict with final professional-game evidence for player {player_id}"
            )));
        }
        let mut enriched = fact.clone();
        enriched.professional_games_at_season_start = Some(games);
        enriched.development_rule_qualified = Some(qualified);
        applied.push(enriched);
    }
    applied.sort_by(|left, right| left.provider_player_id.cmp(&right.provider_player_id));
    Ok(AhlProfessionalGameFactsApplicationView {
        schema: AHL_PROFESSIONAL_GAME_FACTS_SCHEMA.to_owned(),
        prior_season: ledger.prior_season,
        target_season: ledger.target_season,
        nhl_team: nhl_team.to_owned(),
        ahl_team: ahl_team.to_owned(),
        policy_id: ledger.policy_id.clone(),
        ledger_fingerprint: ledger.source_fingerprint.clone(),
        players_applied: applied.len(),
        facts: applied,
        disclosures: vec![
            "Only professional-game totals and final development-rule qualification come from the bound ledger; projection, role, prospect, assignment, recall, and waiver facts retain their separate authorities.".to_owned(),
            "A draft or provisional policy cannot be applied to affiliate projection facts.".to_owned(),
        ],
    })
}

/// Build a deterministic ledger for every canonical identity in a reviewed
/// league crosswalk. Only preceding regular seasons count. Playoffs and the
/// target season are outside this start-of-season measure.
pub fn build_ahl_professional_game_ledger(
    crosswalk: &AhlIdentityLeagueCrosswalkView,
    career_store: &CareerHistoryStore,
    policy: &AhlProfessionalGamePolicy,
) -> Result<AhlProfessionalGameLedgerView, AhlFeedError> {
    validate_policy(crosswalk, career_store, policy)?;

    let mut identities = BTreeMap::<u32, (String, usize)>::new();
    for team in &crosswalk.crosswalks {
        for row in &team.rows {
            if row.review_status != AhlIdentityReviewStatus::Reviewed {
                continue;
            }
            let Some(player_id) = row.nhl_player_id else {
                continue;
            };
            let display_name = row
                .nhl_display_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(&row.ahl_display_name)
                .trim()
                .to_owned();
            match identities.get_mut(&player_id) {
                Some((known_name, appearances)) => {
                    if !known_name.eq_ignore_ascii_case(&display_name) {
                        return Err(AhlFeedError::Validation(format!(
                            "canonical player {player_id} has conflicting reviewed names"
                        )));
                    }
                    *appearances += 1;
                }
                None => {
                    identities.insert(player_id, (display_name, 1));
                }
            }
        }
    }

    let treatments = policy
        .league_treatments
        .iter()
        .map(|treatment| (treatment.league.to_ascii_uppercase(), treatment.kind))
        .collect::<BTreeMap<_, _>>();
    let mut players = Vec::with_capacity(identities.len());
    let mut missing_histories = 0usize;
    let mut unresolved_players = 0usize;

    for (player_id, (display_name, affiliate_appearances)) in identities {
        let mut row = AhlProfessionalGamePlayerRow {
            nhl_player_id: player_id,
            display_name,
            affiliate_appearances,
            official_position: career_store
                .position(player_id)
                .and_then(parse_official_position),
            professional_games_at_season_start: None,
            within_game_threshold: None,
            birth_date: career_store.birth_date(player_id).map(str::to_owned),
            age_at_policy_cutoff: None,
            automatically_age_qualified: None,
            development_rule_qualified: None,
            included_leagues: Vec::new(),
            exempted_european_elite_leagues: Vec::new(),
            excluded_leagues: Vec::new(),
            unresolved_professional_leagues: Vec::new(),
            blockers: Vec::new(),
        };
        let Some(history) = career_store.get(player_id) else {
            missing_histories += 1;
            unresolved_players += 1;
            row.blockers.push("missing_career_history".to_owned());
            players.push(row);
            continue;
        };
        if history.player_id != player_id {
            return Err(AhlFeedError::Validation(format!(
                "career history key {player_id} contains player {}",
                history.player_id
            )));
        }

        let mut included = BTreeMap::<String, u32>::new();
        let mut exempted_european_elite = BTreeMap::<String, u32>::new();
        let mut excluded = BTreeMap::<String, u32>::new();
        let mut unresolved = BTreeSet::new();
        for stint in history.stints.iter().filter(|stint| {
            stint.season.0 < policy.target_season
                && stint.game_type == CareerGameType::Regular
                && stint.league.tier() == LeagueTier::Pro
        }) {
            let league = stint.league.as_str().to_ascii_uppercase();
            match treatments.get(&league) {
                Some(AhlProfessionalLeagueTreatmentKind::Included) => {
                    checked_add_games(&mut included, &league, stint.gp, player_id)?;
                }
                Some(AhlProfessionalLeagueTreatmentKind::EuropeanElite) => {
                    let Some(exemption) = &policy.european_elite_youth_exemption else {
                        unresolved.insert(league);
                        continue;
                    };
                    let Some(birth_year) = row.birth_date.as_deref().and_then(parse_birth_year)
                    else {
                        row.blockers
                            .push("missing_birth_date_for_european_exemption".to_owned());
                        unresolved.insert(league);
                        continue;
                    };
                    let season_start_year = stint.season.0 / 10_000;
                    let age_in_opening_year = season_start_year.saturating_sub(birth_year);
                    if age_in_opening_year <= u32::from(exemption.maximum_non_overage_age) {
                        checked_add_games(
                            &mut exempted_european_elite,
                            &league,
                            stint.gp,
                            player_id,
                        )?;
                    } else {
                        checked_add_games(&mut included, &league, stint.gp, player_id)?;
                    }
                }
                Some(AhlProfessionalLeagueTreatmentKind::Excluded) => {
                    checked_add_games(&mut excluded, &league, stint.gp, player_id)?;
                }
                None => {
                    unresolved.insert(league);
                }
            }
        }
        row.included_leagues = league_totals(included);
        row.exempted_european_elite_leagues = league_totals(exempted_european_elite);
        row.excluded_leagues = league_totals(excluded);
        row.unresolved_professional_leagues = unresolved.into_iter().collect();
        if row.unresolved_professional_leagues.is_empty() {
            let total = row
                .included_leagues
                .iter()
                .try_fold(0u32, |total, league| total.checked_add(league.games))
                .ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "professional-game total overflows for player {player_id}"
                    ))
                })?;
            row.professional_games_at_season_start = Some(total);
            row.within_game_threshold = Some(total <= policy.threshold);
            if let Some(age_rule) = &policy.age_qualification {
                row.age_at_policy_cutoff = row
                    .birth_date
                    .as_deref()
                    .and_then(|birth_date| age_on_date(birth_date, &age_rule.cutoff_date));
                row.automatically_age_qualified = row
                    .age_at_policy_cutoff
                    .map(|age| age < age_rule.automatically_qualifies_under_age);
            }
            row.development_rule_qualified = match (
                row.within_game_threshold,
                row.automatically_age_qualified,
                policy.authority_status == AhlProfessionalGamePolicyAuthority::Final,
            ) {
                (Some(true), _, true) => Some(true),
                (Some(false), Some(age_qualified), true) => Some(age_qualified),
                _ => None,
            };
        } else {
            unresolved_players += 1;
            row.blockers
                .push("unreviewed_professional_league".to_owned());
        }
        players.push(row);
    }

    let complete_players = players
        .iter()
        .filter(|player| player.professional_games_at_season_start.is_some())
        .count();
    let source_fingerprint = fingerprint(crosswalk, career_store, policy, &players)?;
    Ok(AhlProfessionalGameLedgerView {
        schema: AHL_PROFESSIONAL_GAME_LEDGER_SCHEMA.to_owned(),
        policy_id: policy.policy_id.clone(),
        policy_authority_status: policy.authority_status,
        prior_season: crosswalk.season,
        target_season: policy.target_season,
        development_rule_effective_season: policy.development_rule_effective_season,
        age_qualification_effective_season: policy
            .age_qualification
            .as_ref()
            .map(|authority| authority.effective_season),
        european_elite_youth_exemption_effective_season: policy
            .european_elite_youth_exemption
            .as_ref()
            .map(|authority| authority.effective_season),
        as_of: policy.as_of.clone(),
        threshold: policy.threshold,
        career_store_fetched_at: career_store.fetched_at.clone().unwrap_or_default(),
        source_fingerprint,
        canonical_players: players.len(),
        complete_players,
        missing_histories,
        unresolved_players,
        players,
        disclosures: vec![
            "Totals include only preceding regular-season stints in leagues explicitly included by the reviewed policy.".to_owned(),
            "A known professional league without an explicit policy treatment withholds that player's total; playoff and target-season games never count.".to_owned(),
            "This ledger classifies development-rule game experience only. It does not establish contract, assignment, waiver, recall, or lineup authority.".to_owned(),
            "Final development-rule qualification is emitted only when the policy supplies the age rule and every applicable European elite youth exemption can be evaluated.".to_owned(),
        ],
    })
}

fn parse_official_position(value: &str) -> Option<Position> {
    match value.trim().to_ascii_uppercase().as_str() {
        "C" => Some(Position::Center),
        "L" | "LW" => Some(Position::LeftWing),
        "R" | "RW" => Some(Position::RightWing),
        "D" => Some(Position::Defense),
        "G" => Some(Position::Goalie),
        _ => None,
    }
}

fn validate_policy(
    crosswalk: &AhlIdentityLeagueCrosswalkView,
    career_store: &CareerHistoryStore,
    policy: &AhlProfessionalGamePolicy,
) -> Result<(), AhlFeedError> {
    if crosswalk.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || policy.schema != AHL_PROFESSIONAL_GAME_POLICY_SCHEMA
        || policy.policy_id.trim().is_empty()
        || policy.target_season <= crosswalk.season
        || policy.development_rule_effective_season == 0
        || policy.development_rule_effective_season > policy.target_season
        || policy.as_of.trim().is_empty()
        || policy.threshold == 0
        || policy.source_urls.is_empty()
        || policy.source_urls.iter().any(|url| !absolute_url(url))
        || policy.methodology.trim().is_empty()
        || career_store.schema_version == 0
        || career_store
            .fetched_at
            .as_deref()
            .is_none_or(|fetched_at| fetched_at.trim().is_empty())
    {
        return Err(AhlFeedError::Validation(
            "professional-game ledger requires reviewed identity, career, season, and policy authorities"
                .to_owned(),
        ));
    }
    if let Some(age) = &policy.age_qualification {
        if age.effective_season == 0
            || age.effective_season > policy.target_season
            || age.automatically_qualifies_under_age == 0
            || parse_date(&age.cutoff_date).is_none()
            || age.note.trim().is_empty()
            || age.evidence_urls.is_empty()
            || age.evidence_urls.iter().any(|url| !absolute_url(url))
        {
            return Err(AhlFeedError::Validation(
                "professional-game policy has an invalid age qualification".to_owned(),
            ));
        }
    }
    if policy.authority_status == AhlProfessionalGamePolicyAuthority::Final
        && (policy.development_rule_effective_season != policy.target_season
            || policy
                .age_qualification
                .as_ref()
                .is_none_or(|authority| authority.effective_season != policy.target_season)
            || policy
                .european_elite_youth_exemption
                .as_ref()
                .is_none_or(|authority| authority.effective_season != policy.target_season))
    {
        return Err(AhlFeedError::Validation(
            "final professional-game policy requires target-season base-rule, age, and European elite exemption authority".to_owned(),
        ));
    }
    if let Some(exemption) = &policy.european_elite_youth_exemption {
        if exemption.effective_season == 0
            || exemption.effective_season > policy.target_season
            || exemption.maximum_non_overage_age == 0
            || exemption.note.trim().is_empty()
            || exemption.evidence_urls.is_empty()
            || exemption.evidence_urls.iter().any(|url| !absolute_url(url))
        {
            return Err(AhlFeedError::Validation(
                "professional-game policy has an invalid European elite youth exemption".to_owned(),
            ));
        }
    }
    let mut leagues = BTreeSet::new();
    for treatment in &policy.league_treatments {
        let league = treatment.league.trim().to_ascii_uppercase();
        if league.is_empty()
            || !leagues.insert(league)
            || treatment.note.trim().is_empty()
            || treatment.evidence_urls.is_empty()
            || treatment.evidence_urls.iter().any(|url| !absolute_url(url))
        {
            return Err(AhlFeedError::Validation(
                "professional-game policy has an empty, duplicate, or unsourced league treatment"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn parse_birth_year(value: &str) -> Option<u32> {
    u32::try_from(NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?.year()).ok()
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn age_on_date(birth_date: &str, cutoff_date: &str) -> Option<u8> {
    let birth = parse_date(birth_date)?;
    let cutoff = parse_date(cutoff_date)?;
    if cutoff < birth {
        return None;
    }
    let mut age = cutoff.year() - birth.year();
    if (cutoff.month(), cutoff.day()) < (birth.month(), birth.day()) {
        age -= 1;
    }
    u8::try_from(age).ok()
}

fn checked_add_games(
    totals: &mut BTreeMap<String, u32>,
    league: &str,
    games: u32,
    player_id: u32,
) -> Result<(), AhlFeedError> {
    let total = totals.entry(league.to_owned()).or_default();
    *total = total.checked_add(games).ok_or_else(|| {
        AhlFeedError::Validation(format!(
            "professional-game league total overflows for player {player_id}"
        ))
    })?;
    Ok(())
}

fn league_totals(totals: BTreeMap<String, u32>) -> Vec<AhlProfessionalGameLeagueTotal> {
    totals
        .into_iter()
        .map(|(league, games)| AhlProfessionalGameLeagueTotal { league, games })
        .collect()
}

fn fingerprint(
    crosswalk: &AhlIdentityLeagueCrosswalkView,
    career_store: &CareerHistoryStore,
    policy: &AhlProfessionalGamePolicy,
    players: &[AhlProfessionalGamePlayerRow],
) -> Result<String, AhlFeedError> {
    #[derive(Serialize)]
    struct FingerprintInput<'a> {
        crosswalk: &'a AhlIdentityLeagueCrosswalkView,
        career_store_fetched_at: &'a Option<String>,
        policy: &'a AhlProfessionalGamePolicy,
        players: &'a [AhlProfessionalGamePlayerRow],
    }
    let bytes = serde_json::to_vec(&FingerprintInput {
        crosswalk,
        career_store_fetched_at: &career_store.fetched_at,
        policy,
        players,
    })
    .map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn absolute_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{CareerHistory, CareerStint, LeagueAbbrev, Season};

    use crate::ahl::{
        AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow, AhlIdentityCrosswalkView,
        AhlIdentityMatchBasis,
    };

    fn stint(season: u32, league: &str, game_type: CareerGameType, gp: u32) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "Club".to_owned(),
            game_type,
            sequence: 1,
            gp,
            goals: None,
            assists: None,
            points: None,
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    fn crosswalk() -> AhlIdentityLeagueCrosswalkView {
        let row = AhlIdentityCrosswalkRow {
            provider_player_id: "p1".to_owned(),
            ahl_display_name: "Player One".to_owned(),
            ahl_birth_date: "2000-01-01".to_owned(),
            match_basis: AhlIdentityMatchBasis::ExactNameAndBirthDate,
            review_status: AhlIdentityReviewStatus::Reviewed,
            nhl_player_id: Some(1),
            nhl_display_name: Some("Player One".to_owned()),
            nhl_birth_date: Some("2000-01-01".to_owned()),
            evidence_urls: vec!["https://example.com/player".to_owned()],
            note: "reviewed".to_owned(),
        };
        AhlIdentityLeagueCrosswalkView {
            schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.to_owned(),
            season: 20252026,
            provider: "ahl_hockeytech".to_owned(),
            roster_fetched_at: "2026-07-26T00:00:00Z".to_owned(),
            candidates_checked_at: "2026-07-27T00:00:00Z".to_owned(),
            teams: 1,
            roster_appearances: 1,
            unique_provider_players: 1,
            crosswalks: vec![AhlIdentityCrosswalkView {
                schema: crate::ahl::AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
                season: 20252026,
                provider: "ahl_hockeytech".to_owned(),
                ahl_team: "Hartford Wolf Pack".to_owned(),
                nhl_affiliate: Some("NYR".to_owned()),
                roster_fetched_at: "2026-07-26T00:00:00Z".to_owned(),
                candidates_checked_at: "2026-07-27T00:00:00Z".to_owned(),
                counts: AhlIdentityCrosswalkCounts {
                    roster_players: 1,
                    exact_name_and_birth_date: 1,
                    surname_and_birth_date: 0,
                    exact_name_only: 0,
                    ambiguous: 0,
                    conflicts: 0,
                    unmatched: 0,
                    reviewed: 1,
                },
                rows: vec![row],
                disclosures: Vec::new(),
            }],
            disclosures: Vec::new(),
        }
    }

    fn policy() -> AhlProfessionalGamePolicy {
        AhlProfessionalGamePolicy {
            schema: AHL_PROFESSIONAL_GAME_POLICY_SCHEMA.to_owned(),
            policy_id: "ahl-development-2026-27.reviewed.v2".to_owned(),
            authority_status: AhlProfessionalGamePolicyAuthority::Draft,
            target_season: 20262027,
            development_rule_effective_season: 20262027,
            as_of: "2026-07-28".to_owned(),
            threshold: 260,
            source_urls: vec!["https://theahl.com/faq".to_owned()],
            league_treatments: vec!["NHL", "AHL"]
                .into_iter()
                .map(|league| AhlProfessionalLeagueTreatment {
                    league: league.to_owned(),
                    kind: AhlProfessionalLeagueTreatmentKind::Included,
                    evidence_urls: vec!["https://theahl.com/faq".to_owned()],
                    note: "Named by the official development rule.".to_owned(),
                })
                .collect(),
            age_qualification: None,
            european_elite_youth_exemption: None,
            methodology: "Count preceding regular seasons only.".to_owned(),
        }
    }

    #[test]
    fn ledger_counts_reviewed_leagues_and_excludes_playoffs_and_target_season() {
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20242025, "AHL", CareerGameType::Regular, 200),
                stint(20252026, "NHL", CareerGameType::Regular, 61),
                stint(20252026, "NHL", CareerGameType::Playoff, 20),
                stint(20262027, "NHL", CareerGameType::Regular, 5),
                stint(20232024, "OHL", CareerGameType::Regular, 60),
            ],
        });
        store.upsert_position(1, "C");
        let ledger =
            build_ahl_professional_game_ledger(&crosswalk(), &store, &policy()).expect("ledger");
        assert_eq!(ledger.complete_players, 1);
        assert_eq!(
            ledger.players[0].professional_games_at_season_start,
            Some(261)
        );
        assert_eq!(ledger.players[0].within_game_threshold, Some(false));
        assert_eq!(ledger.players[0].official_position, Some(Position::Center));
        assert!(ledger.players[0].blockers.is_empty());
    }

    #[test]
    fn unknown_professional_league_and_missing_history_fail_closed_per_player() {
        let mut league_crosswalk = crosswalk();
        let mut second = league_crosswalk.crosswalks[0].rows[0].clone();
        second.provider_player_id = "p2".to_owned();
        second.nhl_player_id = Some(2);
        second.ahl_display_name = "Player Two".to_owned();
        second.nhl_display_name = Some("Player Two".to_owned());
        league_crosswalk.crosswalks[0].rows.push(second);
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![stint(20252026, "SHL", CareerGameType::Regular, 40)],
        });
        let ledger = build_ahl_professional_game_ledger(&league_crosswalk, &store, &policy())
            .expect("partial ledger");
        assert_eq!(ledger.complete_players, 0);
        assert_eq!(ledger.missing_histories, 1);
        assert_eq!(ledger.unresolved_players, 2);
        assert_eq!(ledger.players[0].unresolved_professional_leagues, ["SHL"]);
        assert_eq!(ledger.players[1].blockers, ["missing_career_history"]);
    }

    #[test]
    fn explicit_exclusion_is_auditable_and_allows_completion() {
        let mut reviewed_policy = policy();
        reviewed_policy
            .league_treatments
            .push(AhlProfessionalLeagueTreatment {
                league: "ECHL".to_owned(),
                kind: AhlProfessionalLeagueTreatmentKind::Excluded,
                evidence_urls: vec!["https://example.com/review".to_owned()],
                note: "Reviewed exclusion for this policy version.".to_owned(),
            });
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20242025, "AHL", CareerGameType::Regular, 20),
                stint(20252026, "ECHL", CareerGameType::Regular, 50),
            ],
        });
        let ledger =
            build_ahl_professional_game_ledger(&crosswalk(), &store, &reviewed_policy).unwrap();
        assert_eq!(
            ledger.players[0].professional_games_at_season_start,
            Some(20)
        );
        assert_eq!(ledger.players[0].excluded_leagues[0].games, 50);
    }

    #[test]
    fn european_youth_games_are_exempt_and_age_can_qualify_over_threshold_player() {
        let mut reviewed_policy = policy();
        reviewed_policy
            .league_treatments
            .push(AhlProfessionalLeagueTreatment {
                league: "SHL".to_owned(),
                kind: AhlProfessionalLeagueTreatmentKind::EuropeanElite,
                evidence_urls: vec!["https://example.com/rulebook".to_owned()],
                note: "Reviewed European elite treatment.".to_owned(),
            });
        reviewed_policy.age_qualification = Some(AhlDevelopmentAgeQualification {
            effective_season: 20262027,
            cutoff_date: "2026-07-01".to_owned(),
            automatically_qualifies_under_age: 25,
            evidence_urls: vec!["https://example.com/rulebook".to_owned()],
            note: "Players under 25 at the cutoff qualify.".to_owned(),
        });
        reviewed_policy.european_elite_youth_exemption = Some(AhlEuropeanEliteYouthExemption {
            effective_season: 20262027,
            maximum_non_overage_age: 19,
            evidence_urls: vec!["https://example.com/rulebook".to_owned()],
            note: "European elite games while CHL-eligible, excluding the over-age year."
                .to_owned(),
        });
        reviewed_policy.authority_status = AhlProfessionalGamePolicyAuthority::Final;
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert_birth_date(1, "2002-08-01");
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20202021, "SHL", CareerGameType::Regular, 30),
                stint(20222023, "SHL", CareerGameType::Regular, 10),
                stint(20252026, "NHL", CareerGameType::Regular, 270),
            ],
        });
        let ledger =
            build_ahl_professional_game_ledger(&crosswalk(), &store, &reviewed_policy).unwrap();
        let player = &ledger.players[0];
        assert_eq!(player.exempted_european_elite_leagues[0].games, 30);
        assert_eq!(player.professional_games_at_season_start, Some(280));
        assert_eq!(player.within_game_threshold, Some(false));
        assert_eq!(player.age_at_policy_cutoff, Some(23));
        assert_eq!(player.automatically_age_qualified, Some(true));
        assert_eq!(player.development_rule_qualified, Some(true));

        reviewed_policy.authority_status = AhlProfessionalGamePolicyAuthority::Provisional;
        let provisional =
            build_ahl_professional_game_ledger(&crosswalk(), &store, &reviewed_policy).unwrap();
        assert_eq!(
            provisional.players[0].development_rule_qualified, None,
            "provisional policy may calculate facts but cannot certify qualification"
        );
    }

    #[test]
    fn missing_birth_date_withholds_european_elite_total() {
        let mut reviewed_policy = policy();
        reviewed_policy
            .league_treatments
            .push(AhlProfessionalLeagueTreatment {
                league: "LIIGA".to_owned(),
                kind: AhlProfessionalLeagueTreatmentKind::EuropeanElite,
                evidence_urls: vec!["https://example.com/rulebook".to_owned()],
                note: "Reviewed European elite treatment.".to_owned(),
            });
        reviewed_policy.european_elite_youth_exemption = Some(AhlEuropeanEliteYouthExemption {
            effective_season: 20262027,
            maximum_non_overage_age: 19,
            evidence_urls: vec!["https://example.com/rulebook".to_owned()],
            note: "European youth exemption.".to_owned(),
        });
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![stint(20252026, "Liiga", CareerGameType::Regular, 40)],
        });
        let ledger =
            build_ahl_professional_game_ledger(&crosswalk(), &store, &reviewed_policy).unwrap();
        assert_eq!(ledger.players[0].professional_games_at_season_start, None);
        assert!(ledger.players[0]
            .blockers
            .iter()
            .any(|blocker| blocker == "missing_birth_date_for_european_exemption"));
    }

    #[test]
    fn final_ledger_applies_only_rule_facts_and_provisional_ledger_is_rejected() {
        let mut final_policy = policy();
        final_policy.authority_status = AhlProfessionalGamePolicyAuthority::Final;
        final_policy.age_qualification = Some(AhlDevelopmentAgeQualification {
            effective_season: 20262027,
            cutoff_date: "2026-07-01".to_owned(),
            automatically_qualifies_under_age: 25,
            evidence_urls: vec!["https://example.com/rulebook".to_owned()],
            note: "Age rule.".to_owned(),
        });
        final_policy.european_elite_youth_exemption = Some(AhlEuropeanEliteYouthExemption {
            effective_season: 20262027,
            maximum_non_overage_age: 19,
            evidence_urls: vec!["https://example.com/rulebook".to_owned()],
            note: "Youth exemption.".to_owned(),
        });
        let league_crosswalk = crosswalk();
        let team_crosswalk = &league_crosswalk.crosswalks[0];
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-28T00:00:00Z".to_owned());
        store.upsert_birth_date(1, "1990-01-01");
        store.upsert(CareerHistory {
            player_id: 1,
            stints: vec![stint(20252026, "AHL", CareerGameType::Regular, 261)],
        });
        let mut stale_policy = final_policy.clone();
        stale_policy
            .age_qualification
            .as_mut()
            .expect("age authority")
            .effective_season = 20252026;
        assert!(
            build_ahl_professional_game_ledger(&league_crosswalk, &store, &stale_policy).is_err()
        );
        let ledger =
            build_ahl_professional_game_ledger(&league_crosswalk, &store, &final_policy).unwrap();
        let facts = vec![AhlProjectionPlayerFacts {
            provider_player_id: "p1".to_owned(),
            primary_position: icelines_core::Position::Center,
            eligible_positions: vec![icelines_core::Position::Center],
            projected_score: 42.0,
            prospect: false,
            recall_readiness: Some(0.6),
            professional_games_at_season_start: None,
            development_rule_qualified: None,
            assigned_to_affiliate: true,
            waiver_required: false,
        }];
        let applied = apply_ahl_professional_game_ledger_to_facts(
            team_crosswalk,
            &ledger,
            "NYR",
            "Hartford Wolf Pack",
            &facts,
        )
        .unwrap();
        assert_eq!(applied.players_applied, 1);
        assert_eq!(applied.facts[0].projected_score, 42.0);
        assert_eq!(
            applied.facts[0].professional_games_at_season_start,
            Some(261)
        );
        assert_eq!(applied.facts[0].development_rule_qualified, Some(false));

        let mut provisional = ledger;
        provisional.policy_authority_status = AhlProfessionalGamePolicyAuthority::Provisional;
        assert!(apply_ahl_professional_game_ledger_to_facts(
            team_crosswalk,
            &provisional,
            "NYR",
            "Hartford Wolf Pack",
            &facts,
        )
        .is_err());
    }
}
