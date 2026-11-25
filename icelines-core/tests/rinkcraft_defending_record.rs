use icelines_core::view_model::{
    rinkcraft_defending_record as record,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = (0..21)
        .map(|i| Player {
            player_id: i + 1,
            group: "Defense".into(),
            games: 30,
            minutes: 600.,
            metrics: BTreeMap::from([
                ("blocks".into(), 3.),
                ("takeaways".into(), 1.),
                ("taken".into(), 0.5),
            ]),
        })
        .collect::<Vec<_>>();
    let mut profiles = scores::build(20262027, "Synthetic all-strength rates", &players).unwrap();
    for (i, p) in profiles.iter_mut().enumerate() {
        p.skater_context = Some(Context::new(
            BTreeMap::from([
                ("blocks".into(), 3.),
                ("takeaways".into(), 1.),
                ("taken".into(), 0.5),
                ("def-5v5-seconds".into(), 24000.),
                ("def-5v5-xga60".into(), i as f64),
                ("def-5v5-fa60".into(), i as f64 * 10.),
                ("def-5v5-off-xga60".into(), 10.),
            ]),
            "NHL / MoneyPuck.com synthetic fixture".into(),
            vec![],
        ));
    }
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn separates_activity_and_chances_and_preserves_existing_grades() {
    let mut c = fixture();
    let before = c
        .profiles
        .iter()
        .map(|p| serde_json::to_value(&p.scores).unwrap())
        .collect::<Vec<_>>();
    record::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .defending
        .as_ref()
        .unwrap();
    assert_eq!(r.activity, "3.00 blocks · 1.00 takeaways /60");
    assert_eq!(r.chances, "0.00 xGA/60");
    assert_eq!(r.on_off, "10.00 fewer xGA/60 on ice");
    assert!(r.rank.unwrap() > 95.);
    assert_eq!(r.peers, 21);
    let last = c.profiles[20]
        .skater_context
        .as_ref()
        .unwrap()
        .defending
        .as_ref()
        .unwrap();
    assert!(last.on_off.contains("more"));
    assert!(last.rank.unwrap() < 5.);
    assert_eq!(
        before,
        c.profiles
            .iter()
            .map(|p| serde_json::to_value(&p.scores).unwrap())
            .collect::<Vec<_>>()
    );
    let encoded = serde_json::to_string(&c).unwrap();
    let restored = Catalog::from_json(&encoded).unwrap();
    assert_eq!(
        restored.profiles[0].skater_context,
        c.profiles[0].skater_context
    );
}
#[test]
fn unknown_exposure_has_no_rank_and_records_are_bound_to_the_profile_window() {
    let mut c = fixture();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .retain(|key, _| !key.starts_with("def-5v5-"));
    record::attach(&mut c.profiles, 20262027);
    c.validate().unwrap();
    let r = c.profiles[0]
        .skater_context
        .as_ref()
        .unwrap()
        .defending
        .as_ref()
        .unwrap();
    assert_eq!(r.minutes_5v5, None);
    assert_eq!(r.rank, None);
    assert_eq!(r.chances, "Unavailable");
    assert!(r.activity.contains("3.00"));
    assert!(r.note.contains("not a poor defensive result"));
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .defending
        .as_mut()
        .unwrap()
        .games += 1;
    assert!(c.validate().is_err());
}
