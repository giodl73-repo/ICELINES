use icelines_core::{
    fixtures,
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    stats_repository::StatsRepository,
    view_model::{
        rinkcraft_profile::{build_rinkcraft_stat_profile, RinkcraftProfileRequest},
        rinkcraft_stat_scores::{self as scores, Catalog, Player},
    },
};
use std::collections::BTreeMap;

#[test]
fn ordinary_file_loader_attaches_only_the_matching_snapshot() {
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(fixtures::identity(1).build()).unwrap();
    let mut row = fixtures::stats(1, 20262027, "AAA").build();
    row.totals.gp = 20;
    repo.upsert_stats(row).unwrap();
    let mut profile = build_rinkcraft_stat_profile(
        &repo,
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: Some(Season(20262027)),
            last_season: Some(Season(20262027)),
        },
    )
    .unwrap();
    let players = (1..=20)
        .map(|id| Player {
            player_id: id,
            group: "Forwards".into(),
            games: 20,
            minutes: 200.,
            metrics: BTreeMap::from([("assists".into(), 3.)]),
        })
        .collect::<Vec<_>>();
    let catalog = Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: scores::build(20262027, "all strengths", &players).unwrap(),
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("attributes.json");
    std::fs::write(&path, serde_json::to_vec(&catalog).unwrap()).unwrap();
    icelines_fetch::rinkcraft_profile::attach_stat_attribute_catalog(&mut profile, &path).unwrap();
    assert_eq!(profile.stat_attribute_seasons.len(), 1);
    assert_eq!(profile.stat_attribute_seasons[0].scores[2].score, Some(10));
    assert!(profile.numerical_evidence.is_none());
    profile.timeline[0].games = 21;
    icelines_fetch::rinkcraft_profile::attach_stat_attribute_catalog(&mut profile, &path).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
    std::fs::write(&path, b"{}").unwrap();
    assert!(
        icelines_fetch::rinkcraft_profile::attach_stat_attribute_catalog(&mut profile, &path)
            .is_err()
    );
}
