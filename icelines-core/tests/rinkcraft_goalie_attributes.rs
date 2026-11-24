use icelines_core::view_model::rinkcraft_stat_scores::{attribute_labels, build, Catalog, Player};
use std::collections::BTreeMap;
#[test]
fn goalie_recipes_rank_only_goalies_with_separate_labels_and_reverse_rebounds() {
    let mut players = (1..=20)
        .map(|id| Player {
            player_id: id,
            group: "Goalies".into(),
            games: 20,
            minutes: 1000.,
            metrics: BTreeMap::from([
                ("goalie-save-fraction".into(), 0.9),
                ("goalie-low-save".into(), 0.97),
                ("goalie-high-save".into(), 0.7),
                ("goalie-cross-save".into(), 0.6),
                ("goalie-rebound-ratio".into(), id as f64 / 10.),
                ("goalie-rebound-save".into(), 0.8),
                ("goalie-assists".into(), 0.),
            ]),
        })
        .collect::<Vec<_>>();
    players.push(Player {
        player_id: 30,
        group: "Forwards".into(),
        games: 20,
        minutes: 1000.,
        metrics: players[0].metrics.clone(),
    });
    let output = build(20252026, "all strengths", &players).unwrap();
    let p = &output[0];
    assert_eq!(
        p.scores
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>(),
        attribute_labels("Goalies")
    );
    assert!(p.scores.iter().all(|s| s.score.is_some()));
    assert_eq!(p.scores[0].score, Some(10));
    assert_eq!(p.scores[3].score, Some(20));
    assert_eq!(output[19].scores[3].score, Some(1));
    assert!(p
        .scores
        .iter()
        .flat_map(|s| &s.parts)
        .all(|t| t.peers == 20));
    assert_eq!(output[20].scores[0].label, "Skating");
    assert!(output[20].scores.iter().all(|s| s.score.is_none()));
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: output,
    }
    .validate()
    .unwrap();
    let mut small = players[0].clone();
    small.player_id = 31;
    small.games = 1;
    small.minutes = 20.;
    let mut with_small = players.clone();
    with_small.push(small);
    let result = build(20252026, "all strengths", &with_small).unwrap();
    assert!(result.last().unwrap().small_sample);
    assert!(result
        .last()
        .unwrap()
        .scores
        .iter()
        .all(|s| s.score.is_some()));
    players[0].games = 1;
    let output = build(20252026, "all strengths", &players).unwrap();
    assert!(output[0].small_sample);
    assert!(output[0].scores.iter().all(|s| s.score.is_none())); // Only 19 eligible peers remain.
}
