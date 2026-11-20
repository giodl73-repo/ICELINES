use icelines_core::{
    fixtures, identity::PlayerId, model::Season, season_stats::SeasonType,
    stats_repository::StatsRepository, view_model::rinkcraft_achievements::from_league,
};
#[test]
fn native_feat_reference_rejects_a_different_year_or_playoffs() {
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(fixtures::identity(1).build()).unwrap();
    let row = fixtures::stats(1, 20242025, "AAA").build();
    repo.upsert_stats(row.clone()).unwrap();
    let view = || {
        vec![repo
            .view(PlayerId(1), Season(20242025), SeasonType::Regular)
            .unwrap()]
    };
    assert!(from_league(view(), 20242025, 1).unwrap().is_some());
    assert!(from_league(view(), 20252026, 1)
        .unwrap_err()
        .contains("window"));
    let mut playoff = row;
    playoff.season_type = SeasonType::Playoff;
    repo.upsert_stats(playoff).unwrap();
    assert!(from_league(
        vec![repo
            .view(PlayerId(1), Season(20242025), SeasonType::Playoff)
            .unwrap()],
        20242025,
        1
    )
    .unwrap_err()
    .contains("window"));
}
