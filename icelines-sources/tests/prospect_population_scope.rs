use icelines_core::source_facts::{OrganizationId, SourceObjectState};
use icelines_sources::prospect_population::{
    PopulationObjectResult, ProspectPopulationScope, ProspectPopulationSourceFamily,
};
use std::collections::BTreeMap;

const TEAMS_2026_27: [&str; 32] = [
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

fn scope() -> ProspectPopulationScope {
    ProspectPopulationScope::new(
        TEAMS_2026_27
            .iter()
            .map(|team| OrganizationId::try_new(*team).unwrap())
            .collect(),
        vec![
            ProspectPopulationSourceFamily::Draft,
            ProspectPopulationSourceFamily::CampPublication,
            ProspectPopulationSourceFamily::ContractPublication,
            ProspectPopulationSourceFamily::TransactionPublication,
            ProspectPopulationSourceFamily::CurrentNhlAssignment,
            ProspectPopulationSourceFamily::CurrentAhlAssignment,
        ],
        "prospect-population-sources.2026-27.v1",
    )
    .unwrap()
}

#[test]
fn all_32_scope_keeps_every_team_and_source_family_visible() {
    let manifest = scope().build_manifest(&BTreeMap::new()).unwrap();
    assert_eq!(manifest.objects.len(), 32 * 6);
    assert!(!manifest.complete);
    assert_eq!(
        manifest
            .objects
            .iter()
            .filter(|object| matches!(object.state, SourceObjectState::Failed { .. }))
            .count(),
        32 * 6
    );
    for team in TEAMS_2026_27 {
        assert_eq!(
            manifest
                .objects
                .iter()
                .filter(|object| object.organization.as_ref().unwrap().as_str() == team)
                .count(),
            6
        );
    }
}

#[test]
fn acquired_zero_is_distinct_from_missing_and_can_be_authoritative() {
    let scope = ProspectPopulationScope::new(
        vec![OrganizationId::try_new("SEA").unwrap()],
        vec![ProspectPopulationSourceFamily::CampPublication],
        "test.v1",
    )
    .unwrap();
    let key = ProspectPopulationScope::object_id(
        &OrganizationId::try_new("SEA").unwrap(),
        ProspectPopulationSourceFamily::CampPublication.key(),
    );
    let results = BTreeMap::from([(
        key,
        PopulationObjectResult {
            terminal_pagination: true,
            state: SourceObjectState::Acquired { records: 0 },
        },
    )]);
    let manifest = scope.build_manifest(&results).unwrap();
    assert!(manifest.complete);
    assert!(matches!(
        manifest.objects[0].state,
        SourceObjectState::Acquired { records: 0 }
    ));
}

#[test]
fn scope_rejects_team_specific_results_outside_the_catalog() {
    let results = BTreeMap::from([(
        "ATL:nhl_draft".to_owned(),
        PopulationObjectResult {
            terminal_pagination: true,
            state: SourceObjectState::Acquired { records: 1 },
        },
    )]);
    assert!(scope().build_manifest(&results).is_err());
}
