#[allow(dead_code)]
#[path = "support/player_read_cases.rs"]
mod cases;
use icelines_core::{
    contract::PlayerContract,
    identity::PlayerId,
    model::{Position, Season},
    season_stats::*,
    SeasonStatsRead,
};
use icelines_query::{
    BioConstraint, BioField, Constraint, DataProvider, FetchError, FetchEvent, PlanRequirement,
    PlayerRead, Predicate, ScalarOp, ScalarValue,
};
use serde_json::Value;
use std::cell::Cell;

// No SeasonStats owner or whole-row reference is available from this layout.
macro_rules! split_reader {
    ($($field:ident:$ty:ty),+ $(,)?) => {
        struct Split {
            player_id:PlayerId,season:Season,season_type:SeasonType,position:Position,
            sweater_number:Option<u32>,totals:StatTotals,team_stints:Vec<TeamStint>,
            $($field:Option<Box<$ty>>,)+
        }
        impl From<SeasonStats> for Split {
            fn from(row:SeasonStats)->Self {
                let SeasonStats {player_id,season,season_type,position,sweater_number,totals,team_stints,$($field,)+}=row;
                Self {player_id,season,season_type,position,sweater_number,totals,team_stints,$($field:$field.map(Box::new),)+}
            }
        }
        impl SeasonStatsRead for Split {
            fn player_id(&self)->PlayerId{self.player_id}
            fn season(&self)->Season{self.season}
            fn season_type(&self)->SeasonType{self.season_type}
            fn position(&self)->Position{self.position}
            fn sweater_number(&self)->Option<u32>{self.sweater_number}
            fn totals(&self)->&StatTotals{&self.totals}
            fn team_stints(&self)->&[TeamStint]{&self.team_stints}
            $(fn $field(&self)->Option<&$ty>{self.$field.as_deref()})+
        }
    }
}
split_reader! {
    realtime:RealtimeStats, advanced:AdvancedStats, goalie:GoalieSeasonStats,
    time_on_ice:TimeOnIceStats, goals_for_against:GoalsForAgainstStats,
    goalie_advanced:GoalieAdvancedStats, goalie_saves_by_strength:GoalieSavesByStrengthStats, goalie_bios:GoalieBios,
}

#[test]
fn full_evaluator_matches_frozen_original_results_for_inline_split_and_dynamic_reads() {
    let golden: Value =
        serde_json::from_str(include_str!("fixtures/player-read-legacy.json")).unwrap();
    assert_eq!(
        cases::observations(),
        golden,
        "existing API must retain the frozen pre-migration results"
    );
    let rows = cases::cases();
    let provider = cases::Provider;
    let ctx = cases::ctx(&provider);
    assert_eq!(rows.len(), 288);
    let constraints = cases::constraints();
    assert_eq!(constraints.len(), 813);
    let split: Vec<_> = rows
        .iter()
        .map(|(_, row)| Split::from(row.clone()))
        .collect();
    for (i, constraint) in constraints.iter().enumerate() {
        assert_eq!(
            golden["constraints"][i]["constraint"],
            format!("{constraint:?}")
        );
        for (j, (identity, row)) in rows.iter().enumerate() {
            let expected = golden["constraints"][i]["matches"][j].as_bool().unwrap();
            let contract = PlayerContract {
                cap_hit: Some(12_345_678),
                ..Default::default()
            };
            let inline = PlayerRead {
                identity,
                stats: row,
                contract: Some(&contract),
            };
            let separated = PlayerRead {
                identity,
                stats: &split[j],
                contract: Some(&contract),
            };
            let dynamic = PlayerRead {
                identity,
                stats: &split[j] as &dyn SeasonStatsRead,
                contract: Some(&contract),
            };
            assert!(std::ptr::eq(separated.contract.unwrap(), &contract));
            assert_eq!(
                constraint.matches_read(&inline, &ctx),
                expected,
                "inline constraint {i} row {j}"
            );
            assert_eq!(
                constraint.matches_read(&separated, &ctx),
                expected,
                "split constraint {i} row {j}"
            );
            assert_eq!(
                constraint.matches_read(&dynamic, &ctx),
                expected,
                "dynamic constraint {i} row {j}"
            );
        }
    }
}

struct Routing(Cell<Option<u32>>);
impl DataProvider for Routing {
    fn ensure(&self, _: &PlanRequirement, _: &mut dyn FnMut(FetchEvent)) -> Result<(), FetchError> {
        Ok(())
    }
    fn fetch_game_lines(
        &self,
        id: u32,
        season: u32,
    ) -> Vec<icelines_query::sliding_window::GameStatLine> {
        self.0.set(Some(id));
        cases::Provider.fetch_game_lines(id, season)
    }
}
#[test]
fn provider_ids_and_historical_identity_borrows_are_preserved() {
    let (mut identity, row) = cases::cases().remove(0);
    identity.id = PlayerId(999999); // deliberately differs from the row ID
    let split = Split::from(row);
    let provider = Routing(Cell::new(None));
    let ctx = cases::ctx(&provider);
    let view = PlayerRead {
        identity: &identity,
        stats: &split,
        contract: None,
    };
    let constraint = cases::constraints()
        .into_iter()
        .find(|c| matches!(c, Constraint::SlidingWindow(_)))
        .unwrap();
    constraint.matches_read(&view, &ctx);
    assert_eq!(provider.0.get(), Some(identity.id.0));
    let mut historical = identity.clone();
    historical.bio.birth_country = Some("USA".into());
    historical.bio.nationality_code = Some("USA".into());
    let historical_view = PlayerRead {
        identity: &historical,
        stats: &split,
        contract: None,
    };
    let country = Constraint::Bio(BioConstraint {
        field: BioField::Country,
        predicate: Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text("can".into())),
    });
    assert!(country.matches_read(&view, &ctx));
    assert!(!country.matches_read(&historical_view, &ctx));
    provider.0.set(None);
    assert!(!Constraint::All(vec![country.clone(), constraint.clone()])
        .matches_read(&historical_view, &ctx));
    assert_eq!(
        provider.0.get(),
        None,
        "false All must short-circuit provider access"
    );
    assert!(Constraint::Any(vec![country, constraint]).matches_read(&view, &ctx));
    assert_eq!(
        provider.0.get(),
        None,
        "true Any must short-circuit provider access"
    );
}

#[test]
fn separate_goalie_bios_preserve_fallback_precedence_and_absence() {
    let (mut identity, mut row) = cases::cases()
        .into_iter()
        .find(|(_, r)| r.goalie_bios.is_some())
        .unwrap();
    identity.bio = Default::default();
    row.goalie_bios.as_mut().unwrap().birth_country_code = Some("CAN".into());
    let mut split = Split::from(row);
    let provider = cases::Provider;
    let ctx = cases::ctx(&provider);
    let country = Constraint::Bio(BioConstraint {
        field: BioField::Country,
        predicate: Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text("can".into())),
    });
    assert!(country.matches_read(
        &PlayerRead {
            identity: &identity,
            stats: &split,
            contract: None
        },
        &ctx
    ));
    identity.bio.birth_country = Some("USA".into());
    assert!(!country.matches_read(
        &PlayerRead {
            identity: &identity,
            stats: &split,
            contract: None
        },
        &ctx
    ));
    identity.bio = Default::default();
    split.goalie_bios = None;
    assert!(!country.matches_read(
        &PlayerRead {
            identity: &identity,
            stats: &split,
            contract: None
        },
        &ctx
    ));
    split.team_stints.clear();
    assert!(PlayerRead {
        identity: &identity,
        stats: &split,
        contract: None
    }
    .team()
    .is_none());
}

#[test]
fn team_any_matches_aggregate_season_codes_without_changing_totals_or_current_team() {
    let identity = icelines_core::fixtures::identity(1).build();
    let provider = cases::Provider;
    let ctx = cases::ctx(&provider);
    for code in ["STL/NYI", " STL, NYI ", "STL/NYI/NYI", "STL//NYI"] {
        let row = icelines_core::fixtures::stats(1, 20252026, code).build();
        let before = row.clone();
        let read = PlayerRead {
            identity: &identity,
            stats: &row,
            contract: None,
        };
        for (query, expected) in [
            ("team.any=NYI", true),
            ("team.any=STL", true),
            ("team.any IN (NYI, SEA)", true),
            ("team.any=NY", false),
            ("team.any=SEA", false),
            ("team=NYI", false),
            ("NOT (team.any=NYI)", false),
        ] {
            let plan = icelines_query::parse_query(icelines_query::FilterInput::Form(query.into()))
                .unwrap();
            assert_eq!(
                plan.root.matches_read(&read, &ctx),
                expected,
                "{code}: {query}"
            );
        }
        assert_eq!(
            serde_json::to_value(&row).unwrap(),
            serde_json::to_value(before).unwrap()
        );
    }
    let row = icelines_core::fixtures::stats(1, 20252026, "NYI").build();
    let read = PlayerRead {
        identity: &identity,
        stats: &row,
        contract: None,
    };
    let plan =
        icelines_query::parse_query(icelines_query::FilterInput::Form("team.any=NYI".into()))
            .unwrap();
    assert!(plan.root.matches_read(&read, &ctx));
}
