use icelines_core::view_model::rinkcraft_achievements::raw_thresholds;
use icelines_core::view_model::rinkcraft_stat_scores::{estimate_record, RecordMetric};
#[test]
fn season_pace_respects_lengths_limits_rates_and_zero_games() {
    let metrics = vec![
        RecordMetric {
            key: "points".into(),
            label: "Points / GP".into(),
            unit: "PerGame".into(),
            value: Some(2.),
        },
        RecordMetric {
            key: "goals".into(),
            label: "Goals".into(),
            unit: "Count".into(),
            value: Some(10.),
        },
        RecordMetric {
            key: "sv".into(),
            label: "Save percentage".into(),
            unit: "Pct".into(),
            value: Some(0.91),
        },
    ];
    let current = estimate_record(20262027, 20, 200., None, &metrics);
    assert_eq!(current.target_games, 84);
    assert_eq!(current.metrics[0].value, Some(168.));
    assert_eq!(current.metrics[0].per_game, Some(2.));
    assert_eq!(current.metrics[1].value, Some(42.));
    assert_eq!(current.metrics[2].value, Some(0.91));
    assert!(!current.metrics[2].projected_total);
    assert_eq!(
        estimate_record(20252026, 20, 200., None, &metrics).metrics[0].value,
        Some(164.)
    );
    assert_eq!(
        estimate_record(20262027, 20, 200., Some(70), &metrics).metrics[0].value,
        Some(140.)
    );
    assert_eq!(
        estimate_record(20262027, 0, 0., None, &metrics).metrics[0].value,
        None
    );
}
#[test]
fn displayed_raw_boundaries_match_existing_percentile_rules_with_and_without_ties() {
    for peers in [
        (0..40).map(|i| i as f64 / 7.).collect::<Vec<_>>(),
        vec![0.; 40],
        (0..40).map(|i| (i / 8) as f64).collect(),
    ] {
        let cutoffs = raw_thresholds(&peers);
        for target in [60., 75., 90.] {
            let cutoff = &cutoffs[&format!("{target:.0}")];
            for value in peers.iter().copied().chain([-1., 0.1, 0.5, 2.1, 10., 100.]) {
                let rank = 100.
                    * (peers.iter().filter(|v| **v < value).count() as f64
                        + 0.5 * peers.iter().filter(|v| **v == value).count() as f64)
                    / peers.len() as f64;
                let raw_pass = if cutoff.inclusive {
                    value >= cutoff.value
                } else {
                    value > cutoff.value
                };
                assert_eq!(raw_pass, rank >= target, "value={value} target={target}");
            }
        }
    }
    assert!(raw_thresholds(&[0.; 19]).is_empty());
}
#[test]
fn per_sixty_record_uses_observed_ice_time_per_game() {
    let metric = RecordMetric {
        key: "shots".into(),
        label: "Shots / 60".into(),
        unit: "Per60".into(),
        value: Some(3.),
    };
    let record = estimate_record(20262027, 20, 200., None, &[metric]);
    assert_eq!(record.metrics[0].label, "Shots");
    assert_eq!(record.metrics[0].per_game, Some(0.5));
    assert_eq!(record.metrics[0].value, Some(42.));
}

#[test]
fn ten_game_feats_unlock_and_saved_profiles_match_fresh_evaluation() {
    use icelines_core::view_model::rinkcraft_achievements::{
        default_rules, evaluate, refresh_profiles, Input, PlayerInput,
    };
    use std::collections::BTreeMap;
    let mut input = Input {
        season: 20262027,
        source_urls: vec![],
        source_sha256: "fixture".into(),
        players: (1..=41)
            .map(|id| PlayerInput {
                player_id: id,
                name: format!("Player {id}"),
                group: "Forwards".into(),
                games: if id == 41 { 18 } else { 20 },
                counts: BTreeMap::from([
                    ("hits".into(), id as f64),
                    ("blocks".into(), id as f64),
                    ("points".into(), id as f64),
                ]),
            })
            .collect(),
    };
    let result = evaluate(&input, &default_rules()).unwrap();
    let feat = |p: &icelines_core::view_model::rinkcraft_achievements::AchievementProfile| {
        p.feats
            .iter()
            .find(|f| f.id == "shot-shield")
            .unwrap()
            .clone()
    };
    assert_eq!(feat(&result.profiles[40]).state, "earned");
    assert_eq!(feat(&result.profiles[40]).tier.as_deref(), Some("Gold"));
    assert_eq!(feat(&result.profiles[40]).conditions[0].peers, 41);
    assert!(result.profiles[40].earned > 0);
    assert_eq!(result.profiles[40].minimum_games, 10);
    let mut saved = result.profiles.clone();
    for f in &mut saved[40].feats {
        f.state = "ineligible".into();
        f.tier = None;
        for c in &mut f.conditions {
            c.percentile = None;
            c.qualifies = None;
        }
    }
    refresh_profiles(&mut saved);
    assert_eq!(saved, result.profiles);
    input.players[40].games = 9;
    assert_eq!(
        feat(&evaluate(&input, &default_rules()).unwrap().profiles[40]).state,
        "ineligible"
    );
    input.players[40].games = 10;
    assert_eq!(
        feat(&evaluate(&input, &default_rules()).unwrap().profiles[40]).state,
        "earned"
    );
    input.players[40].counts.remove("blocks");
    assert_eq!(
        feat(&evaluate(&input, &default_rules()).unwrap().profiles[40]).state,
        "unavailable"
    );
    input.players[40].counts.insert("blocks".into(), 0.);
    assert_eq!(
        feat(&evaluate(&input, &default_rules()).unwrap().profiles[40]).state,
        "not_earned"
    );
    input.players[40].counts.insert("blocks".into(), 41.);
    input.players[40].games = 20;
    assert_eq!(
        feat(&evaluate(&input, &default_rules()).unwrap().profiles[40]).state,
        "earned"
    );
}

#[test]
fn goalies_use_the_same_ten_game_minimum() {
    use icelines_core::view_model::rinkcraft_achievements::{evaluate, Input, PlayerInput, Rule};
    use std::collections::BTreeMap;
    let mut input = Input {
        season: 20262027,
        source_urls: vec![],
        source_sha256: "fixture".into(),
        players: (1..=21)
            .map(|id| PlayerInput {
                player_id: id,
                name: format!("Goalie {id}"),
                group: "Goalies".into(),
                games: if id == 21 { 5 } else { 15 },
                counts: BTreeMap::from([("shutouts".into(), if id == 21 { 1. } else { 0. })]),
            })
            .collect(),
    };
    let rules = vec![Rule {
        id: "clean-sheet".into(),
        name: "Clean sheet".into(),
        branch: "goalie".into(),
        metrics: vec!["shutouts".into()],
        placements: vec!["task_skills".into()],
        note: None,
    }];
    for (gp, state) in [
        (9, "ineligible"),
        (10, "earned"),
        (14, "earned"),
        (15, "earned"),
    ] {
        input.players[20].games = gp;
        assert_eq!(
            evaluate(&input, &rules).unwrap().profiles[20].feats[0].state,
            state
        );
    }
}
