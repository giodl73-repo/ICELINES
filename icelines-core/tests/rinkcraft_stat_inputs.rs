use icelines_core::{
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    view_model::{
        rinkcraft_achievements::{self as feats, AchievementProfile},
        rinkcraft_deployment::{self as deployment, DeploymentEvidence, UsageInput},
        rinkcraft_edge::EdgeEvidence,
        rinkcraft_stat_scores::observed_player,
    },
};
use std::collections::BTreeMap;

fn sources() -> (AchievementProfile, DeploymentEvidence, EdgeEvidence) {
    let record = feats::evaluate(
        &feats::Input {
            season: 20262027,
            source_urls: vec![],
            source_sha256: "synthetic".into(),
            players: vec![feats::PlayerInput {
                player_id: 1,
                name: "Synthetic skater".into(),
                group: "Forwards".into(),
                games: 20,
                counts: BTreeMap::from([
                    ("goals".into(), 10.),
                    ("shots".into(), 100.),
                    ("assists".into(), 20.),
                    ("giveaways".into(), 0.),
                    ("drawn".into(), 5.),
                    ("taken".into(), 2.),
                    ("takeaways".into(), 4.),
                    ("blocks".into(), 8.),
                ]),
            }],
        },
        &feats::default_rules(),
    )
    .unwrap()
    .profiles
    .remove(0);
    let usage = deployment::build(UsageInput {
        player_id: PlayerId(1),
        season: Season(20262027),
        season_type: SeasonType::Regular,
        games: 20,
        total: 12000,
        skater_strengths: Some([9000, 2000, 1000]),
        goalie_starts: None,
    })
    .unwrap();
    let edge = serde_json::from_value(serde_json::json!({
        "schema":"icelines.rinkcraft-edge.v1", "player_id":1, "season":20262027,
        "season_type":"regular", "group":"Forwards", "games":20, "captured_at":"synthetic",
        "strength_scope":"all strengths", "source":{"url":"https://example.test/edge","sha256":"a".repeat(64),"bytes":1},
        "locations":[],"notes":[],"measures":[
            {"key":"max-skating-speed","label":"Top speed","value":24.,"unit":"Mph","source_field":"speed","interpretation":"Synthetic"},
            {"key":"speed-bursts-over-20","label":"Bursts","value":80.,"unit":"Count","source_field":"bursts","interpretation":"Synthetic"}
        ]
    })).unwrap();
    (record, usage, edge)
}

#[test]
fn all_strength_counts_use_total_toi_and_preserve_measured_zero() {
    let (record, usage, edge) = sources();
    let p = observed_player(&record, &usage, Some(&edge)).unwrap();
    assert_eq!(p.minutes, 200.);
    assert_eq!(p.metrics["goals"], 3.);
    assert_eq!(p.metrics["shots"], 30.);
    assert_eq!(p.metrics["assists"], 6.);
    assert_eq!(p.metrics["giveaways"], 0.);
    assert_eq!(p.metrics["peak-speed"], 24.);
    assert_eq!(p.metrics["bursts"], 24.);
    let missing = observed_player(&record, &usage, None).unwrap();
    assert!(!missing.metrics.contains_key("peak-speed"));
    assert!(!missing.metrics.contains_key("bursts"));
}

#[test]
fn mismatched_actor_window_games_or_strength_cannot_be_joined() {
    let (record, usage, edge) = sources();
    for change in 0..5 {
        let mut other = usage.clone();
        match change {
            0 => other.player_id = PlayerId(2),
            1 => other.season = Season(20252026),
            2 => other.games = 19,
            3 => other.season_type = SeasonType::Playoff,
            _ => other.goalie = true,
        }
        assert!(observed_player(&record, &other, None).is_err());
    }
    for change in 0..6 {
        let mut other = edge.clone();
        match change {
            0 => other.player_id = 2,
            1 => other.season = 20252026,
            2 => other.games = 19,
            3 => other.strength_scope = "5on5".into(),
            4 => other.group = "Defense".into(),
            _ => other.season_type = "playoff".into(),
        }
        assert!(observed_player(&record, &usage, Some(&other)).is_err());
    }
}

#[test]
fn conflicting_counts_and_ambiguous_or_absent_workload_are_rejected() {
    let (mut record, mut usage, edge) = sources();
    let mut shots = record
        .feats
        .iter()
        .flat_map(|f| &f.conditions)
        .find(|c| c.key == "shots")
        .unwrap()
        .clone();
    shots.numerator = Some(101.);
    shots.value = Some(101. / 20.);
    record.feats[0].conditions.push(shots);
    assert!(observed_player(&record, &usage, Some(&edge))
        .unwrap_err()
        .contains("Conflicting"));
    let (record, _, _) = sources();
    usage.rows.push(usage.rows[0].clone());
    assert!(observed_player(&record, &usage, None).is_err());
    usage.rows.retain(|r| r.key != "total");
    assert!(observed_player(&record, &usage, None).is_err());
}

#[test]
fn duplicate_edge_measure_and_wrong_units_cannot_silently_overwrite() {
    let (record, usage, mut edge) = sources();
    edge.measures[0].unit = "Kph".into();
    assert!(observed_player(&record, &usage, Some(&edge)).is_err());
    edge.measures[0].unit = "Mph".into();
    edge.measures.push(edge.measures[0].clone());
    assert!(observed_player(&record, &usage, Some(&edge)).is_err());
}

#[test]
fn conversion_noise_does_not_split_tied_stat_measurements() {
    use icelines_core::view_model::rinkcraft_stat_scores::{build, Player};
    let (record, usage, edge) = sources();
    let row = observed_player(&record, &usage, Some(&edge)).unwrap();
    let mut input = (1..=20)
        .map(|id| Player {
            player_id: id,
            ..row.clone()
        })
        .collect::<Vec<_>>();
    input[0].metrics.insert("assists".into(), 6. + 1e-13);
    let scores = build(20262027, "all strengths", &input).unwrap();
    assert!(scores
        .iter()
        .all(|p| p.scores.iter().all(|s| s.score == Some(10))));
    assert_eq!(scores[0].scores[2].parts[0].percentile, Some(50.));
    assert_eq!(scores[0].scores[2].parts[0].value, Some(6. + 1e-13));
    input[0].metrics.insert("assists".into(), 6. + 1e-6);
    let scores = build(20262027, "all strengths", &input).unwrap();
    assert_eq!(scores[0].scores[2].score, Some(20));
}

fn estimate_catalog() -> icelines_core::view_model::rinkcraft_stat_scores::Catalog {
    use icelines_core::view_model::rinkcraft_stat_scores::{build, Catalog, Player};
    let (record, usage, edge) = sources();
    let row = observed_player(&record, &usage, Some(&edge)).unwrap();
    let rows = (1..=20)
        .map(|id| Player {
            player_id: id,
            ..row.clone()
        })
        .collect::<Vec<_>>();
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: build(20262027, "all strengths", &rows).unwrap(),
    }
}
fn ordinary_profile() -> icelines_core::view_model::rinkcraft_profile::RinkcraftStatProfile {
    use icelines_core::{
        fixtures,
        stats_repository::StatsRepository,
        view_model::rinkcraft_profile::{build_rinkcraft_stat_profile, RinkcraftProfileRequest},
    };
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(fixtures::identity(1).build()).unwrap();
    let mut row = fixtures::stats(1, 20262027, "AAA").build();
    row.totals.gp = 20;
    repo.upsert_stats(row).unwrap();
    build_rinkcraft_stat_profile(
        &repo,
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: Some(Season(20262027)),
            last_season: Some(Season(20262027)),
        },
    )
    .unwrap()
}
#[test]
fn estimate_catalog_selector_requires_exact_actor_season_and_gp() {
    use icelines_core::view_model::rinkcraft_stat_scores::{catalog_profile_json, Catalog};
    let source = serde_json::to_string(&estimate_catalog()).unwrap();
    let catalog = Catalog::from_json(&source).unwrap();
    let profile = catalog.profile(1, 20262027, 20).unwrap();
    assert_eq!(profile.games, 20);
    assert_eq!(profile.minutes, 200.);
    assert!(profile.scores.iter().all(|s| s.score == Some(10)));
    for (id, season, gp) in [(999, 20262027, 20), (1, 20252026, 20), (1, 20262027, 21)] {
        assert_eq!(
            catalog_profile_json(&source, id, season, gp).unwrap(),
            "null"
        );
    }
    let mut invalid = estimate_catalog();
    invalid.profiles.push(invalid.profiles[0].clone());
    assert!(invalid.validate().is_err());
    let mut invalid = estimate_catalog();
    invalid.profiles[0].scores[0].score = Some(21);
    assert!(invalid.validate().is_err());
    let mut invalid = estimate_catalog();
    invalid.profiles[0].games = 0;
    assert!(invalid.validate().is_err());
}
#[test]
fn ordinary_profiles_receive_estimates_without_a_numerical_dossier() {
    use icelines_core::view_model::rinkcraft_profile::attach_stat_attribute_catalog;
    let mut profile = ordinary_profile();
    assert!(profile.numerical_evidence.is_none());
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert_eq!(profile.stat_attribute_seasons.len(), 1);
    assert!(profile.stat_attribute_seasons[0]
        .scores
        .iter()
        .all(|s| s.score == Some(10)));
    let json = serde_json::to_value(&profile).unwrap();
    assert_eq!(json["stat_attribute_seasons"][0]["games"], 20);
    profile.timeline[0].games = 21;
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
    assert!(profile.limitations.iter().any(|n| n.contains("withheld")));
    profile.timeline[0].games = 20;
    profile.request.season_type = SeasonType::Playoff;
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
}
#[test]
fn canonical_position_cannot_receive_a_different_groups_estimates() {
    use icelines_core::{
        model::Position, view_model::rinkcraft_profile::attach_stat_attribute_catalog,
    };
    let mut profile = ordinary_profile();
    for position in [Position::Defense, Position::Goalie] {
        profile.timeline[0].position = position;
        attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
        assert!(profile.stat_attribute_seasons.is_empty());
    }
}

#[test]
fn supplied_numeric_snapshot_controls_gp_without_bypassing_position_or_window() {
    use icelines_core::{
        model::Position,
        view_model::{rinkcraft_numbers as n, rinkcraft_profile::attach_stat_attribute_catalog},
    };
    let catalog = n::NumericalCatalog {
        schema: n::SCHEMA.into(),
        protocol: n::PROTOCOL.into(),
        season: 20262027,
        competition: "regular".into(),
        source_credit: "Synthetic".into(),
        sources: vec![],
        missing_players: vec![],
        limitations: vec![],
        profiles: vec![n::NumericalProfile {
            style_evidence: None,
            player_id: 1,
            name: "Synthetic".into(),
            group: "Forwards".into(),
            situation: "5on5".into(),
            games: 20,
            minutes: 200.,
            minimum_minutes: 100.,
            eligible_peers: 20,
            metrics: vec![],
        }],
    };
    let mut profile = ordinary_profile();
    profile.timeline[0].games = 21;
    profile.numerical_evidence = catalog.card(1, None).unwrap();
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert_eq!(profile.stat_attribute_seasons[0].games, 20);
    profile.numerical_evidence.as_mut().unwrap().games = 19;
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
    profile.numerical_evidence.as_mut().unwrap().games = 20;
    profile.timeline[0].position = Position::Goalie;
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
    profile.timeline[0].position = Position::Center;
    profile.request.last_season = Some(Season(20252026));
    attach_stat_attribute_catalog(&mut profile, &estimate_catalog()).unwrap();
    assert!(profile.stat_attribute_seasons.is_empty());
}
