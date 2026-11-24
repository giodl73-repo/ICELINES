use icelines_core::view_model::{
    rinkcraft_passing_record as passing,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = (1..=2)
        .map(|id| Player {
            player_id: id,
            group: "Forwards".into(),
            games: 30,
            minutes: 600.,
            metrics: BTreeMap::from([("assists".into(), 2.)]),
        })
        .collect::<Vec<_>>();
    let mut profiles = scores::build(20262027, "Synthetic", &players).unwrap();
    for (i, p) in profiles.iter_mut().enumerate() {
        p.skater_context = Some(Context::new(
            BTreeMap::from([
                ("assists".into(), 2.),
                ("primary".into(), if i == 0 { 13. } else { 7. }),
                ("secondary".into(), if i == 0 { 7. } else { 13. }),
                ("ev-assists".into(), 1.2),
                ("pp-assists".into(), 6.),
                ("pp-share".into(), 1. / 6.),
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
fn distinguishes_equal_totals_and_preserves_strength_denominators_and_grades() {
    let mut c = fixture();
    let before =
        serde_json::to_value(c.profiles.iter().map(|p| &p.scores).collect::<Vec<_>>()).unwrap();
    passing::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let a = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .passing
        .as_ref()
        .unwrap();
    let b = c.profiles[1]
        .skater_context
        .as_ref()
        .unwrap()
        .passing
        .as_ref()
        .unwrap();
    assert_eq!(a.rows[0], b.rows[0]);
    assert_eq!(a.primary, "1.30 /60");
    assert_eq!(b.primary, "0.70 /60");
    assert_eq!(a.mix, "65.0% primary credits");
    assert_eq!(b.mix, "35.0% primary credits");
    assert_eq!(a.rows[4].value, "1.20 /60 EV");
    assert_eq!(a.rows[5].value, "6.00 /60 PP");
    assert!(a.note.contains("not pass attempts"));
    assert!(a.note.contains("PP unit"));
    assert_eq!(
        before,
        serde_json::to_value(c.profiles.iter().map(|p| &p.scores).collect::<Vec<_>>()).unwrap()
    );
}
#[test]
fn zero_credits_and_missing_detail_do_not_create_a_zero_share_or_strength_rate() {
    let mut c = fixture();
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("primary".into(), 0.);
    v.insert("secondary".into(), 0.);
    v.insert("assists".into(), 0.);
    v.remove("pp-assists");
    passing::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .passing
        .as_ref()
        .unwrap();
    assert_eq!(r.primary, "0.00 /60");
    assert_eq!(r.mix, "No credited assists");
    assert_eq!(r.rows[5].value, "Unavailable");
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .remove("secondary");
    passing::attach(&mut c.profiles, 20262027);
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .passing
        .as_ref()
        .unwrap();
    assert_eq!(r.secondary, "Unavailable");
    assert_eq!(r.mix, "Assist mix unavailable");
}
#[test]
fn inactive_window_and_actor_binding_are_enforced_without_changing_other_seasons() {
    let mut c = fixture();
    c.profiles[1].season = 20252026;
    let old = serde_json::to_value(&c.profiles[1]).unwrap();
    c.profiles[0].games = 0;
    c.profiles[0].minutes = 0.;
    passing::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    assert_eq!(old, serde_json::to_value(&c.profiles[1]).unwrap());
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .passing
        .as_ref()
        .unwrap();
    assert_eq!(r.primary, "Unavailable");
    assert_eq!(r.secondary, "Unavailable");
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .passing
        .as_mut()
        .unwrap()
        .games += 1;
    assert!(c.validate().is_err());
}
