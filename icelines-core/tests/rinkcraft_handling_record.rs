use icelines_core::view_model::{
    rinkcraft_handling_record as handling,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = (1..=2)
        .map(|id| Player {
            player_id: id,
            group: "Forwards".into(),
            games: 20,
            minutes: 400.,
            metrics: BTreeMap::new(),
        })
        .collect::<Vec<_>>();
    let mut profiles = scores::build(20262027, "Synthetic all strengths", &players).unwrap();
    for (i, p) in profiles.iter_mut().enumerate() {
        p.skater_context = Some(Context::new(
            BTreeMap::from([
                ("giveaways".into(), 1.),
                ("drawn".into(), 0.8),
                ("taken".into(), 0.3),
                ("shots".into(), if i == 0 { 2. } else { 9. }),
                ("assists".into(), if i == 0 { 1. } else { 4. }),
            ]),
            "NHL synthetic fixture".into(),
            vec![],
        ));
    }
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn separates_recorded_losses_from_drawing_and_exposes_the_production_denominator() {
    let mut c = fixture();
    let grades =
        serde_json::to_value(c.profiles.iter().map(|p| &p.scores).collect::<Vec<_>>()).unwrap();
    handling::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let a = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .handling
        .as_ref()
        .unwrap();
    let b = c.profiles[1]
        .skater_context
        .as_ref()
        .unwrap()
        .handling
        .as_ref()
        .unwrap();
    assert_eq!(a.giveaways, b.giveaways);
    assert_eq!(a.drawn, b.drawn);
    assert_eq!(a.rows[3].value, "+0.50 /60");
    assert_eq!(a.rows[4].value, "25.0%");
    assert_eq!(b.rows[4].value, "7.1%");
    assert!(a.note.contains("not possession opportunities"));
    assert_eq!(
        grades,
        serde_json::to_value(c.profiles.iter().map(|p| &p.scores).collect::<Vec<_>>()).unwrap()
    );
}
#[test]
fn zero_missing_and_no_opportunity_are_distinct() {
    let mut c = fixture();
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("giveaways".into(), 0.);
    v.insert("drawn".into(), 0.);
    handling::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .handling
        .as_ref()
        .unwrap();
    assert_eq!(r.giveaways, "0.00 /60");
    assert_eq!(r.drawn, "0.00 /60");
    assert_eq!(r.rows[4].value, "0.0%");
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.remove("drawn");
    v.insert("shots".into(), 0.);
    v.insert("assists".into(), 0.);
    handling::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .handling
        .as_ref()
        .unwrap();
    assert_eq!(r.giveaways, "0.00 /60");
    assert_eq!(r.drawn, "Unavailable");
    assert_eq!(r.rows[4].value, "Unavailable");
}
#[test]
fn only_selected_season_is_changed_and_records_are_window_bound() {
    let mut c = fixture();
    c.profiles[1].season = 20252026;
    let before = serde_json::to_value(&c.profiles[1]).unwrap();
    handling::attach(&mut c.profiles, 20262027);
    assert_eq!(before, serde_json::to_value(&c.profiles[1]).unwrap());
    c.validate().unwrap();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .handling
        .as_mut()
        .unwrap()
        .games += 1;
    assert!(c.validate().is_err());
}
