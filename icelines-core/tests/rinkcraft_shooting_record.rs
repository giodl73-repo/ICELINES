use icelines_core::view_model::{
    rinkcraft_shooting_record as shooting,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = vec![Player {
        player_id: 1,
        group: "Forwards".into(),
        games: 20,
        minutes: 400.,
        metrics: BTreeMap::from([("goals".into(), 1.5), ("shots".into(), 10.)]),
    }];
    let mut profiles = scores::build(20262027, "Synthetic", &players).unwrap();
    profiles[0].skater_context = Some(Context::new(
        BTreeMap::from([
            ("goals".into(), 1.5),
            ("shots".into(), 10.),
            ("quality-attempts".into(), 100.),
            ("quality-goals".into(), 8.),
            ("ixg".into(), 6.),
            ("finishing".into(), 2.),
            ("shot-quality".into(), 0.06),
            ("shot-distance".into(), 30.),
            ("edge-shot-speed".into(), 90.),
        ]),
        "Synthetic".into(),
        vec!["MoneyPuck.com shot archive captured synthetic; independent window".into()],
    ));
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn separates_official_volume_from_archive_finishing_and_preserves_scores() {
    let mut c = fixture();
    let before = c.profiles[0].scores.clone();
    shooting::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .shooting
        .as_ref()
        .unwrap();
    assert_eq!(r.shots, "10.00 /60");
    assert_eq!(r.conversion, "15.0%");
    assert_eq!(r.rows[4].value, "8 goals");
    assert_eq!(r.rows[6].value, "+2.00 goals");
    assert_eq!(r.rows[7].value, "0.060 xG/attempt");
    assert_eq!(r.source_notes.len(), 1);
    assert!(r.formula.contains("never NHL minutes"));
    assert!(r.note.contains("not established finishing talent"));
    assert_eq!(before, c.profiles[0].scores);
}
#[test]
fn zero_misses_missing_and_no_shots_are_distinct() {
    let mut c = fixture();
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("goals".into(), 0.);
    v.insert("quality-goals".into(), 0.);
    v.insert("ixg".into(), 0.);
    shooting::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .shooting
        .as_ref()
        .unwrap();
    assert_eq!(r.conversion, "0.0%");
    assert_eq!(r.rows[6].value, "+0.00 goals");
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("shots".into(), 0.);
    v.remove("ixg");
    v.insert("quality-attempts".into(), 0.);
    shooting::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .shooting
        .as_ref()
        .unwrap();
    assert_eq!(r.shots, "0.00 /60");
    assert_eq!(r.conversion, "Unavailable");
    assert_eq!(r.rows[5].value, "Unavailable");
    assert_eq!(r.rows[6].value, "Unavailable");
    assert_eq!(r.rows[7].value, "Unavailable");
}
#[test]
fn isolates_seasons_and_checks_actor_exposure() {
    let mut c = fixture();
    let mut previous = c.profiles[0].clone();
    previous.season = 20252026;
    c.profiles.push(previous.clone());
    c.profiles[0].minutes = 0.;
    c.profiles[0].games = 0;
    shooting::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    assert_eq!(c.profiles[1], previous);
    let r = c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .shooting
        .as_mut()
        .unwrap();
    assert_eq!(r.shots, "Unavailable");
    assert_eq!(r.conversion, "Unavailable");
    r.player_id = 2;
    assert!(c.validate().is_err());
}
