use icelines_core::view_model::{rinkcraft_stat_scores as scores, rinkcraft_unit_links as links};
use std::collections::BTreeMap;
fn player(group: &str) -> scores::Profile {
    scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: group.into(),
            games: 20,
            minutes: 400.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0)
}
fn member(id: u32, pos: &str) -> links::Member {
    links::Member {
        player_id: id,
        name: format!("Player {id}"),
        position: pos.into(),
    }
}
fn game(day: u32, partner: u32, seconds: u32) -> links::Game {
    let u = links::Unit {
        team_id: 3,
        team: "NYR".into(),
        members: vec![member(1, "D"), member(partner, "D")],
        seconds,
    };
    links::Game {
        game_id: 2026020000 + u64::from(day),
        date: format!("2026-10-{day:02}"),
        group: "Defense".into(),
        states: BTreeMap::from([("5v5".into(), seconds)]),
        unresolved_seconds: 0,
        units: BTreeMap::from([("5v5".into(), vec![u])]),
        partners: vec![links::Shared {
            team_id: 3,
            team: "NYR".into(),
            member: member(partner, "D"),
            seconds,
        }],
    }
}
fn build(p: &scores::Profile, g: Vec<links::Game>) -> links::Record {
    links::build(p, g, "a".repeat(64), "b".repeat(64)).unwrap()
}
#[test]
fn shared_groups_use_own_state_time_and_last_five_is_an_explicit_subset() {
    let p = player("Defense");
    let games = (1..=7)
        .map(|day| {
            game(
                day,
                if day <= 2 { 2 } else { 3 },
                if day <= 2 { 500 } else { 100 },
            )
        })
        .collect::<Vec<_>>();
    let r = build(&p, games.clone());
    assert_eq!(r.contexts[0].seconds, 1500);
    assert_eq!(r.contexts[0].groups[0].share_display, "66.7%");
    assert_eq!(r.contexts[0].groups[0].partners_display, "Player 2");
    assert_eq!(r.recent.game_ids.len(), 5);
    assert_eq!(
        r.recent.leading.as_ref().unwrap().partners_display,
        "Player 3"
    );
    assert_eq!(r.recent.different_from_full, Some(true));
    assert_eq!(r.contexts[1].groups.len(), 0);
    assert_eq!(r.contexts[1].time_display, "0:00");
    let mut shuffled = games;
    shuffled.reverse();
    assert_eq!(r, build(&p, shuffled));
    let restored: links::Record =
        serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(r, restored);
}
#[test]
fn ties_remain_ambiguous_and_wrong_parent_cannot_survive_refresh() {
    let p = player("Defense");
    let r = build(&p, vec![game(1, 2, 100), game(2, 3, 100)]);
    assert_eq!(r.contexts[0].tied_leaders, 2);
    assert!(r.recent.leading.is_none());
    assert_eq!(r.recent.different_from_full, None);
    assert!(links::rebind(&p, &r, &"c".repeat(64)).is_none());
    assert_eq!(links::rebind(&p, &r, &"b".repeat(64)), Some(r.clone()));
    for edit in 0..5 {
        let mut bad = r.clone();
        match edit {
            0 => bad.player_id = 2,
            1 => bad.contexts[0].groups[0].share_display = "99%".into(),
            2 => bad.contexts[0].groups[0].seconds = 201,
            3 => bad.recent.game_ids = vec![2026020001, 2026020001],
            _ => bad.contexts[0].groups[0].members[0].player_id = 9,
        };
        assert!(links::validate(&p, &bad).is_err());
    }
    assert!(links::build(
        &p,
        vec![game(1, 2, 100), game(1, 2, 100)],
        "a".repeat(64),
        "b".repeat(64)
    )
    .is_err());
}
#[test]
fn goalies_link_to_defenders_in_front_and_trades_keep_separate_groups() {
    let p = player("Goalies");
    let mut a = game(1, 2, 100);
    a.group = "Goalies".into();
    a.units.get_mut("5v5").unwrap()[0].members = vec![member(2, "D"), member(3, "D")];
    let r = build(&p, vec![a]);
    assert_eq!(r.contexts[0].label, "5v5 defense in front");
    assert_eq!(
        r.contexts[0].groups[0].partners_display,
        "Player 2 / Player 3"
    );
    let p = player("Defense");
    let a = game(1, 2, 100);
    let mut b = game(2, 2, 120);
    b.units.get_mut("5v5").unwrap()[0].team_id = 55;
    b.units.get_mut("5v5").unwrap()[0].team = "SEA".into();
    let r = build(&p, vec![a, b]);
    assert_eq!(r.contexts[0].total_groups, 2);
    assert_eq!(r.contexts[0].groups[0].team, "SEA");
}
