use icelines_core::{
    fixtures,
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    stats_repository::StatsRepository,
    view_model::{
        rinkcraft_deployment::from_stat_profile,
        rinkcraft_profile::{
            attach_stat_attribute_catalog, build_rinkcraft_stat_profile, RinkcraftProfileRequest,
        },
        rinkcraft_skater_context::Context,
        rinkcraft_stat_scores::{self as scores, Catalog, Player},
    },
};
use std::collections::BTreeMap;
fn catalog() -> Catalog {
    let mut profiles = scores::build(
        20262027,
        "all strengths",
        &[Player {
            player_id: 1,
            group: "Forwards".into(),
            games: 20,
            minutes: 400.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap();
    profiles[0].skater_context = Some(Context::new(
        BTreeMap::from([("pp-share".into(), 0.1), ("sh-seconds".into(), 150.)]),
        "Synthetic NHL".into(),
        vec![],
    ));
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn recovers_exact_strength_seconds_and_keeps_zero_distinct_from_missing() {
    let mut c = catalog();
    let p = &mut c.profiles[0];
    let usage = from_stat_profile(p).unwrap();
    assert_eq!(
        usage.rows.iter().map(|r| r.seconds).collect::<Vec<_>>(),
        vec![24000, 18600, 2400, 3000]
    );
    assert_eq!(usage.rows[2].per_game_display, "2:00");
    assert_eq!(usage.rows[3].per_game_display, "2:30");
    assert_eq!(usage.rows[3].share, Some(0.125));
    let v = &mut p.skater_context.as_mut().unwrap().values;
    v.insert("pp-share".into(), 0.);
    v.insert("sh-seconds".into(), 0.);
    let usage = from_stat_profile(p).unwrap();
    assert_eq!(usage.rows[2].per_game_display, "0:00");
    assert_eq!(usage.rows[3].share, Some(0.));
    p.skater_context.as_mut().unwrap().values.remove("pp-share");
    assert!(from_stat_profile(p).is_none());
}
#[test]
fn refuses_fractional_reconstruction_invalid_sums_and_inactive_or_goalie_windows() {
    for change in 0..6 {
        let mut c = catalog();
        let p = &mut c.profiles[0];
        match change {
            0 => p.minutes += 0.01,
            1 => {
                p.skater_context
                    .as_mut()
                    .unwrap()
                    .values
                    .insert("sh-seconds".into(), 150.01);
            }
            2 => {
                p.skater_context
                    .as_mut()
                    .unwrap()
                    .values
                    .insert("pp-share".into(), 1.1);
            }
            3 => {
                p.skater_context
                    .as_mut()
                    .unwrap()
                    .values
                    .insert("sh-seconds".into(), 1200.);
            }
            4 => p.games = 0,
            _ => p.group = "Goalies".into(),
        };
        assert!(from_stat_profile(p).is_none(), "case {change}");
    }
}
#[test]
fn attachment_uses_admitted_window_and_preserves_matching_direct_usage() {
    let c = catalog();
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(fixtures::identity(1).build()).unwrap();
    let mut row = fixtures::stats(1, 20262027, "AAA").build();
    row.totals.gp = 20;
    repo.upsert_stats(row).unwrap();
    let mut p = build_rinkcraft_stat_profile(
        &repo,
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: Some(Season(20262027)),
            last_season: Some(Season(20262027)),
        },
    )
    .unwrap();
    p.deployment_evidence = None;
    attach_stat_attribute_catalog(&mut p, &c).unwrap();
    assert_eq!(
        p.deployment_evidence.as_ref().unwrap().rows[3].per_game_display,
        "2:30"
    );
    p.deployment_evidence.as_mut().unwrap().source_label = "Preferred direct source".into();
    attach_stat_attribute_catalog(&mut p, &c).unwrap();
    assert_eq!(
        p.deployment_evidence.as_ref().unwrap().source_label,
        "Preferred direct source"
    );
    p.deployment_evidence.as_mut().unwrap().games = 19;
    attach_stat_attribute_catalog(&mut p, &c).unwrap();
    assert_eq!(p.deployment_evidence.as_ref().unwrap().games, 20);
    p.deployment_evidence = None;
    p.timeline[0].games = 19;
    attach_stat_attribute_catalog(&mut p, &c).unwrap();
    assert!(p.deployment_evidence.is_none());
    assert!(p.stat_attribute_seasons.is_empty());
}
