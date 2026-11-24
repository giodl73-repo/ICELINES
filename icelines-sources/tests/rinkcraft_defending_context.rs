use icelines_core::view_model::rinkcraft_stat_scores::Player;
use icelines_sources::rinkcraft_defending_context as source;
use std::collections::BTreeMap;
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
