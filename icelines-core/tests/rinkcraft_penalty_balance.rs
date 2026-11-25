use icelines_core::view_model::{
    rinkcraft_penalty_balance as balance, rinkcraft_skater_context::Context,
    rinkcraft_stat_scores as scores,
};
use std::collections::BTreeMap;

fn player(drawn: Option<f64>, taken: Option<f64>) -> scores::Profile {
    let mut p = scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: "Forwards".into(),
            games: 4,
            minutes: 80.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0);
    p.skater_context = Some(Context::new(
        drawn
            .map(|v| ("drawn".into(), v))
            .into_iter()
            .chain(taken.map(|v| ("taken".into(), v)))
            .collect(),
        "Neutral fixture".into(),
        vec![],
    ));
    p
}

#[test]
fn balance_uses_unrounded_rates_without_ten_game_gate_or_mutation() {
    let p = player(Some(1.234), Some(0.786));
    let before = p.clone();
    let r = balance::build(&p).unwrap();
    assert_eq!(
        (&r.drawn, &r.taken, &r.balance),
        (&"1.23 /60".into(), &"0.79 /60".into(), &"+0.45 /60".into())
    );
    assert_eq!(p, before);
    balance::validate(&p, &r).unwrap();
    assert_eq!(
        balance::build(&player(Some(0.), Some(2.))).unwrap().balance,
        "-2.00 /60"
    );
    assert_eq!(
        balance::build(&player(Some(0.), Some(0.))).unwrap().balance,
        "+0.00 /60"
    );
    assert_eq!(
        balance::build(&player(Some(0.1), Some(0.101)))
            .unwrap()
            .balance,
        "+0.00 /60"
    );
}

#[test]
fn partial_missing_invalid_and_wrong_scope_do_not_fabricate_rates() {
    let r = balance::build(&player(Some(0.), None)).unwrap();
    assert_eq!(r.drawn, "0.00 /60");
    assert_eq!(r.taken, "Unavailable");
    assert_eq!(r.balance, "Unavailable");
    assert!(balance::build(&player(None, None)).is_none());
    assert!(balance::build(&player(Some(-1.), Some(f64::NAN))).is_none());
    for edit in 0..5 {
        let mut p = player(Some(1.), Some(2.));
        match edit {
            0 => p.group = "Goalies".into(),
            1 => p.scope = "5v5".into(),
            2 => p.games = 0,
            3 => p.minutes = 0.,
            _ => p.minutes = f64::INFINITY,
        };
        assert!(balance::build(&p).is_none());
    }
}

#[test]
fn stale_or_altered_record_is_rejected_and_optional_catalog_roundtrips() {
    let mut p = player(Some(1.), Some(2.));
    p.penalty_balance = balance::build(&p);
    let cat = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: vec![p.clone()],
    };
    scores::Catalog::from_json(&serde_json::to_string(&cat).unwrap()).unwrap();
    for edit in 0..4 {
        let mut r = p.penalty_balance.clone().unwrap();
        match edit {
            0 => r.player_id = 2,
            1 => r.season = 20252026,
            2 => r.minutes = 81.,
            _ => r.balance = "+10.00 /60".into(),
        };
        assert!(balance::validate(&p, &r).is_err());
    }
    p.penalty_balance = None;
    assert!(!serde_json::to_string(&p)
        .unwrap()
        .contains("penalty_balance"));
}
