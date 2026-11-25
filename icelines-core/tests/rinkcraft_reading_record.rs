use icelines_core::view_model::{
    rinkcraft_reading_record as reading,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = [Player {
        player_id: 1,
        group: "Forwards".into(),
        games: 20,
        minutes: 400.,
        metrics: BTreeMap::from([
            ("takeaways".into(), 2.),
            ("assists".into(), 3.),
            ("taken".into(), 0.5),
        ]),
    }];
    let mut profiles = scores::build(20262027, "Synthetic", &players).unwrap();
    let mut values = players[0].metrics.clone();
    values.extend(BTreeMap::from([
        ("primary-rate".into(), 1.8),
        ("secondary".into(), 8.),
    ]));
    profiles[0].skater_context = Some(Context::new(values, "NHL synthetic fixture".into(), vec![]));
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn shows_assist_split_and_events_without_assigning_anticipation_or_changing_scores() {
    let mut c = fixture();
    let before = serde_json::to_value(&c.profiles[0].scores).unwrap();
    reading::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .reading
        .as_ref()
        .unwrap();
    assert_eq!(r.takeaways, "2.00 /60");
    assert_eq!(r.assists, "3.00 /60");
    assert_eq!(r.taken, "0.50 /60");
    assert_eq!(r.rows[3].value, "1.80 /60");
    assert_eq!(r.rows[4].value, "1.20 /60");
    assert!(r
        .note
        .contains("Anticipation and assigned coverage remain unmeasured"));
    assert!(r.note.contains("Assists also inform Passing"));
    assert_eq!(before, serde_json::to_value(&c.profiles[0].scores).unwrap());
}
#[test]
fn recorded_zero_missing_detail_and_absent_exposure_remain_distinct() {
    let mut c = fixture();
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("takeaways".into(), 0.);
    v.remove("primary-rate");
    v.remove("secondary");
    reading::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .reading
        .as_ref()
        .unwrap();
    assert_eq!(r.takeaways, "0.00 /60");
    assert_eq!(r.assists, "3.00 /60");
    assert_eq!(r.rows[3].value, "Unavailable");
    assert_eq!(r.rows[4].value, "Unavailable");
    c.profiles[0].minutes = 0.;
    c.profiles[0].games = 0;
    reading::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .reading
        .as_ref()
        .unwrap();
    assert_eq!(r.takeaways, "Unavailable");
    assert_eq!(r.assists, "Unavailable");
}
#[test]
fn selected_season_isolated_and_window_binding_enforced() {
    let mut c = fixture();
    let mut old = c.profiles[0].clone();
    old.season = 20252026;
    c.profiles.push(old);
    let before = serde_json::to_value(&c.profiles[1]).unwrap();
    reading::attach(&mut c.profiles, 20262027);
    assert_eq!(before, serde_json::to_value(&c.profiles[1]).unwrap());
    c.validate().unwrap();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .reading
        .as_mut()
        .unwrap()
        .games += 1;
    assert!(c.validate().is_err());
}
