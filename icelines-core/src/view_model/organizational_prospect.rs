//! Operational organizational-prospect status.
//!
//! This population rule is used by IceLines reserve-system views. It is not
//! NHL rookie eligibility, contract status, waiver status, or a scouting grade.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

pub const ORGANIZATIONAL_PROSPECT_POLICY_SCHEMA: &str = "organizational_prospect_policy.v1";
pub const ORGANIZATIONAL_PROSPECT_METHOD: &str = "age_and_nhl_workload.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationalProspectPolicy {
    pub schema: String,
    pub method_version: String,
    pub as_of_date: String,
    pub maximum_age: u8,
    pub maximum_nhl_regular_season_games: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationalProspectBasis {
    Eligible,
    AgeGraduated,
    WorkloadGraduated,
    AgeAndWorkloadGraduated,
    InsufficientEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationalProspectStatusView {
    pub method_version: String,
    pub as_of_date: String,
    pub age: Option<u8>,
    pub nhl_regular_season_games: Option<u32>,
    pub prospect: Option<bool>,
    pub basis: OrganizationalProspectBasis,
}

pub fn classify_organizational_prospect(
    policy: &OrganizationalProspectPolicy,
    birth_date: &str,
    nhl_regular_season_games: u32,
) -> Result<OrganizationalProspectStatusView, String> {
    evaluate_organizational_prospect(policy, Some(birth_date), Some(nhl_regular_season_games))
}

pub fn evaluate_organizational_prospect(
    policy: &OrganizationalProspectPolicy,
    birth_date: Option<&str>,
    nhl_regular_season_games: Option<u32>,
) -> Result<OrganizationalProspectStatusView, String> {
    if policy.schema != ORGANIZATIONAL_PROSPECT_POLICY_SCHEMA
        || policy.method_version != ORGANIZATIONAL_PROSPECT_METHOD
        || policy.maximum_age == 0
    {
        return Err("invalid organizational prospect policy".to_owned());
    }
    let as_of = NaiveDate::parse_from_str(&policy.as_of_date, "%Y-%m-%d")
        .map_err(|_| "organizational prospect policy has invalid as-of date".to_owned())?;
    let age = birth_date
        .map(|birth_date| {
            let birth = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d").map_err(|_| {
                "organizational prospect identity has invalid birth date".to_owned()
            })?;
            if birth > as_of {
                return Err("organizational prospect birth date follows as-of date".to_owned());
            }
            let mut age = as_of.year() - birth.year();
            if (as_of.month(), as_of.day()) < (birth.month(), birth.day()) {
                age -= 1;
            }
            u8::try_from(age)
                .map_err(|_| "organizational prospect age is outside supported range".to_owned())
        })
        .transpose()?;
    let age_graduated = age.map(|age| age > policy.maximum_age);
    let workload_graduated =
        nhl_regular_season_games.map(|games| games > policy.maximum_nhl_regular_season_games);
    let basis = match (age_graduated, workload_graduated) {
        (Some(false), Some(false)) => OrganizationalProspectBasis::Eligible,
        (Some(true), Some(true)) => OrganizationalProspectBasis::AgeAndWorkloadGraduated,
        (Some(true), _) => OrganizationalProspectBasis::AgeGraduated,
        (_, Some(true)) => OrganizationalProspectBasis::WorkloadGraduated,
        _ => OrganizationalProspectBasis::InsufficientEvidence,
    };
    let prospect = match basis {
        OrganizationalProspectBasis::Eligible => Some(true),
        OrganizationalProspectBasis::AgeGraduated
        | OrganizationalProspectBasis::WorkloadGraduated
        | OrganizationalProspectBasis::AgeAndWorkloadGraduated => Some(false),
        OrganizationalProspectBasis::InsufficientEvidence => None,
    };
    Ok(OrganizationalProspectStatusView {
        method_version: policy.method_version.clone(),
        as_of_date: policy.as_of_date.clone(),
        age,
        nhl_regular_season_games,
        prospect,
        basis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> OrganizationalProspectPolicy {
        OrganizationalProspectPolicy {
            schema: ORGANIZATIONAL_PROSPECT_POLICY_SCHEMA.to_owned(),
            method_version: ORGANIZATIONAL_PROSPECT_METHOD.to_owned(),
            as_of_date: "2026-09-15".to_owned(),
            maximum_age: 24,
            maximum_nhl_regular_season_games: 50,
        }
    }

    #[test]
    fn exact_age_and_workload_boundaries_remain_eligible() {
        let status = classify_organizational_prospect(&policy(), "2002-09-15", 50).unwrap();
        assert_eq!(status.prospect, Some(true));
        assert_eq!(status.age, Some(24));
        assert_eq!(status.basis, OrganizationalProspectBasis::Eligible);
    }

    #[test]
    fn birthday_after_as_of_has_not_happened_yet() {
        let status = classify_organizational_prospect(&policy(), "2001-09-16", 0).unwrap();
        assert_eq!(status.age, Some(24));
        assert_eq!(status.prospect, Some(true));
    }

    #[test]
    fn age_and_workload_graduation_are_independent() {
        let age = classify_organizational_prospect(&policy(), "2000-01-01", 10).unwrap();
        let workload = classify_organizational_prospect(&policy(), "2004-01-01", 51).unwrap();
        assert_eq!(age.basis, OrganizationalProspectBasis::AgeGraduated);
        assert_eq!(
            workload.basis,
            OrganizationalProspectBasis::WorkloadGraduated
        );
        assert_eq!(age.prospect, Some(false));
        assert_eq!(workload.prospect, Some(false));
    }

    #[test]
    fn future_birth_date_fails_closed() {
        assert!(classify_organizational_prospect(&policy(), "2027-01-01", 0).is_err());
    }

    #[test]
    fn one_observed_graduation_axis_is_decisive_but_eligibility_needs_both() {
        let age = evaluate_organizational_prospect(&policy(), Some("2000-01-01"), None).unwrap();
        let workload = evaluate_organizational_prospect(&policy(), None, Some(51)).unwrap();
        let incomplete = evaluate_organizational_prospect(&policy(), None, Some(0)).unwrap();
        assert_eq!(age.prospect, Some(false));
        assert_eq!(workload.prospect, Some(false));
        assert_eq!(incomplete.prospect, None);
        assert_eq!(
            incomplete.basis,
            OrganizationalProspectBasis::InsufficientEvidence
        );
    }
}
