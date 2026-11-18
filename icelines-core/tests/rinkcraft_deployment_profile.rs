use icelines_core::{
    fixtures,
    identity::PlayerId,
    model::{Position, Season},
    season_stats::{GoalieSeasonStats, SeasonType, TimeOnIceStats},
    stats_repository::StatsRepository,
    view_model::rinkcraft_profile::{build_rinkcraft_stat_profile, RinkcraftProfileRequest},
};

fn profile(
    row: icelines_core::season_stats::SeasonStats,
) -> icelines_core::view_model::rinkcraft_profile::RinkcraftStatProfile {
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(
        fixtures::identity(1)
            .name("Example Player", "example player")
            .build(),
    )
    .unwrap();
    repo.upsert_stats(row).unwrap();
    build_rinkcraft_stat_profile(
        &repo,
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: Some(Season(20242025)),
            last_season: Some(Season(20242025)),
        },
    )
    .unwrap()
}
#[test]
fn canonical_profile_uses_report_totals_not_rounded_summary_rates() {
    let mut row = fixtures::stats(1, 20242025, "AAA").build();
    row.totals.gp = 2;
    row.totals.toi_per_game_sec = Some(999);
    row.time_on_ice = Some(TimeOnIceStats {
        time_on_ice_sec: 1200,
        ev_time_on_ice_sec: 1000,
        pp_time_on_ice_sec: 200,
        sh_time_on_ice_sec: 0,
        ot_time_on_ice_sec: Some(100),
        ..Default::default()
    });
    let d = profile(row).deployment_evidence.unwrap();
    assert_eq!(d.rows[0].seconds_per_game, 600.0);
    assert_eq!(d.rows[3].seconds, 0);
    assert_eq!(d.source_sha256, None);
}
#[test]
fn goalie_relief_uses_appearances_rather_than_starts() {
    let mut row = fixtures::stats(1, 20242025, "AAA").build();
    row.totals.gp = 2;
    row.position = Position::Goalie;
    row.goalie = Some(GoalieSeasonStats {
        games_started: 1,
        time_on_ice_sec: 5400,
        ..Default::default()
    });
    let d = profile(row).deployment_evidence.unwrap();
    assert_eq!(d.rows.len(), 1);
    assert_eq!(d.rows[0].per_game_display, "45:00");
    assert_eq!(d.starts, Some(1));
}
#[test]
fn missing_or_incoherent_optional_report_cannot_invent_workload() {
    let mut row = fixtures::stats(1, 20242025, "AAA").build();
    assert!(profile(row.clone()).deployment_evidence.is_none());
    row.time_on_ice = Some(TimeOnIceStats {
        time_on_ice_sec: 1200,
        ev_time_on_ice_sec: 1300,
        ..Default::default()
    });
    assert!(profile(row).deployment_evidence.is_none());
}
