use icelines_core::view_model::rinkcraft_stat_scores::Player;
use icelines_sources::rinkcraft_defending_context as source;
use std::collections::BTreeMap;
#[test]
fn enrichment_replaces_stale_context_and_preserves_grades_and_other_seasons() {
    use icelines_core::view_model::{
        rinkcraft_skater_context::Context, rinkcraft_stat_scores as scores,
    };
    let mut profiles = scores::build(20232024, "Synthetic", &[player()]).unwrap();
    profiles[0].skater_context = Some(Context::new(
        BTreeMap::from([("def-5v5-xga60".into(), 99.), ("blocks".into(), 3.)]),
        "NHL".into(),
        vec![],
    ));
    let mut other = profiles[0].clone();
    other.season = 20242025;
    profiles.push(other);
    let before = serde_json::to_value(&profiles[1]).unwrap();
    let grades = serde_json::to_value(&profiles[0].scores).unwrap();
    assert_eq!(
        source::enrich(&mut profiles, 20232024, CSV.as_bytes(), "fixture capture").unwrap(),
        1
    );
    assert_eq!(
        profiles[0].skater_context.as_ref().unwrap().values["def-5v5-xga60"],
        2.
    );
    assert_eq!(grades, serde_json::to_value(&profiles[0].scores).unwrap());
    assert_eq!(before, serde_json::to_value(&profiles[1]).unwrap());
    let mismatched = CSV.replace(",5on5,60,", ",5on5,59,");
    assert_eq!(
        source::enrich(
            &mut profiles,
            20232024,
            mismatched.as_bytes(),
            "new capture"
        )
        .unwrap(),
        0
    );
    let c = profiles[0].skater_context.as_ref().unwrap();
    assert!(!c.values.contains_key("def-5v5-xga60"));
    assert_eq!(c.defending.as_ref().unwrap().chances, "Unavailable");
}
fn player() -> Player {
    Player {
        player_id: 1,
        group: "Defense".into(),
        games: 60,
        minutes: 1000.,
        metrics: BTreeMap::new(),
    }
}
const CSV:&str="playerId,season,situation,games_played,icetime,timeOnBench,OnIce_A_xGoals,OffIce_A_xGoals,OnIce_A_unblockedShotAttempts,I_F_dZoneShiftStarts,I_F_oZoneShiftStarts\n1,2023,all,60,60000,120000,90,150,1000,200,100\n1,2023,5on5,60,36000,72000,20,60,400,100,100\n";
#[test]
fn uses_same_strength_exposure_and_requires_matching_gp() {
    let out = source::parse(CSV.as_bytes(), 20232024, &[player()]).unwrap();
    let v = &out[&1];
    assert_eq!(v["def-5v5-xga60"], 2.);
    assert_eq!(v["def-5v5-off-xga60"], 3.);
    assert_eq!(v["def-5v5-fa60"], 40.);
    assert_eq!(v["def-5v5-dz-share"], 0.5);
    let mut p = player();
    p.games = 59;
    assert!(source::parse(CSV.as_bytes(), 20232024, &[p])
        .unwrap()
        .is_empty());
    assert!(source::parse(
        CSV.replace(",2023,", ",2024,").as_bytes(),
        20232024,
        &[player()]
    )
    .is_err());
    assert!(source::parse(
        format!("{CSV}1,2023,5on5,60,36000,72000,20,60,400,100,100\n").as_bytes(),
        20232024,
        &[player()]
    )
    .is_err());
}
#[test]
fn retains_true_zero_but_never_divides_by_missing_exposure() {
    let csv = CSV.replace("36000,72000,20,60,400,100,100", "36000,0,0,0,0,0,0");
    let out = source::parse(csv.as_bytes(), 20232024, &[player()]).unwrap();
    let v = &out[&1];
    assert_eq!(v["def-5v5-xga60"], 0.);
    assert_eq!(v["def-5v5-fa60"], 0.);
    assert!(!v.contains_key("def-5v5-off-xga60"));
    assert!(!v.contains_key("def-5v5-dz-share"));
    assert!(source::parse(
        CSV.replace("36000,72000", "0,72000").as_bytes(),
        20232024,
        &[player()]
    )
    .is_err());
    assert!(source::parse(
        CSV.replace("36000,72000", "61000,72000").as_bytes(),
        20232024,
        &[player()]
    )
    .is_err());
    assert!(source::parse(
        CSV.replace("20,60,400", "20,60,400.5").as_bytes(),
        20232024,
        &[player()]
    )
    .is_err());
}
