//! Synthetic executor inputs shared by the pre-change oracle and new-reader tests.
#[allow(dead_code)]
#[path = "../../../icelines-core/tests/support/season_read_cases.rs"]
mod stats_cases;
use chrono::NaiveDate;
use icelines_core::{
    identity::PlayerIdentity, season_stats::SeasonStats, stats_catalog::StatId,
    stats_repository::PlayerView,
};
use icelines_query::{sliding_window::GameStatLine, *};
use serde_json::{json, Value};

pub fn cases() -> Vec<(PlayerIdentity, SeasonStats)> {
    let mut rows = stats_cases::cases();
    for (identity, row) in &mut rows {
        if !row.player_id.0.is_multiple_of(3) {
            identity.bio.birth_date = Some("1995-02-01".into());
            identity.bio.birth_country = Some("CAN".into());
            identity.bio.nationality_code = Some("CAN".into());
            identity.bio.shoots_catches = Some("L".into());
            identity.bio.draft_year = Some(2013);
            identity.bio.draft_round = Some(1);
            identity.bio.draft_overall = Some(5);
            identity.bio.birth_city = Some("Montréal".into());
            identity.bio.birth_state_province = Some("QC".into());
            identity.bio.rookie_season = Some("20152016".into());
        }
    }
    // Provider routing must retain identity-based IDs, even for legacy incoherent views.
    rows[0].0.id = icelines_core::identity::PlayerId(900001);
    rows
}
pub struct Provider;
impl DataProvider for Provider {
    fn ensure(&self, _: &PlanRequirement, _: &mut dyn FnMut(FetchEvent)) -> Result<(), FetchError> {
        Ok(())
    }
    fn fetch_game_lines(&self, player_id: u32, _: u32) -> Vec<GameStatLine> {
        if player_id.is_multiple_of(3) {
            return Vec::new();
        }
        [(2005, 1, 1), (2024, 12, 1), (2025, 12, 1), (2026, 1, 1)]
            .into_iter()
            .enumerate()
            .map(|(i, (y, m, d))| GameStatLine {
                player_id,
                date: NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                game_id: i as u64,
                team_abbrev: if i < 2 { "BOS" } else { "EDM" }.into(),
                goals: i as u32,
                assists: 1,
                plus_minus: -1,
                sog: 3,
                hits: 2,
                blocked_shots: 1,
                takeaways: 0,
                giveaways: 1,
                pim: 2,
                toi_seconds: 300,
            })
            .collect()
    }
    fn fetch_career_history(
        &self,
        player_id: u32,
    ) -> Option<icelines_core::career_history::CareerHistory> {
        if player_id.is_multiple_of(3) {
            return None;
        }
        Some(serde_json::from_value(json!({"player_id":player_id,"stints":[{
            "season":20122013,"league":"OHL","team":"TEST","game_type":"Regular","sequence":0,"gp":10,
            "goals":4,"assists":5,"points":9,"shots_against":10,"goals_against":2
        }]})).unwrap())
    }
}
pub fn ctx(provider: &dyn DataProvider) -> EvalCtx<'_> {
    EvalCtx::new(
        provider,
        StrictMode::default(),
        true,
        NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
        20252026,
    )
}
pub fn constraints() -> Vec<Constraint> {
    let mut out = Vec::new();
    for stat in StatId::all() {
        for op in [
            ScalarOp::Eq,
            ScalarOp::Ne,
            ScalarOp::Lt,
            ScalarOp::Le,
            ScalarOp::Gt,
            ScalarOp::Ge,
        ] {
            out.push(Constraint::SeasonStat(SeasonStatConstraint {
                stat: *stat,
                predicate: Predicate::Scalar(op, ScalarValue::Number(1.0)),
                axis: SeasonAxis::Regular,
            }));
        }
        out.push(Constraint::SeasonStat(SeasonStatConstraint {
            stat: *stat,
            predicate: Predicate::Range(NumericRange {
                min: 0.0,
                max: 10.0,
            }),
            axis: SeasonAxis::Playoff,
        }));
    }
    for field in [
        BioField::Age,
        BioField::DraftYear,
        BioField::DraftRound,
        BioField::DraftOverall,
        BioField::Height,
        BioField::Weight,
        BioField::Country,
        BioField::Nationality,
        BioField::Shoots,
        BioField::Position,
        BioField::Team,
        BioField::TeamAny,
        BioField::TeamCareer,
        BioField::BirthCity,
        BioField::BirthState,
        BioField::RookieSeason,
    ] {
        let predicates = if field.is_numeric() {
            vec![
                Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(1.0)),
                Predicate::Range(NumericRange {
                    min: 20.0,
                    max: 35.0,
                }),
            ]
        } else {
            vec![
                Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text("can".into())),
                Predicate::Member(
                    MemberOp::In,
                    vec![
                        ScalarValue::Text("edm".into()),
                        ScalarValue::Text("c".into()),
                    ],
                ),
                Predicate::Pattern(PatternOp::Like, GlobPattern::parse("*o*")),
            ]
        };
        for predicate in predicates {
            out.push(Constraint::Bio(BioConstraint { field, predicate }));
        }
    }
    for window in [
        SlidingWindow::LastN_Days(30),
        SlidingWindow::LastN_Weeks(4),
        SlidingWindow::LastN_Months(2),
        SlidingWindow::LastN_GP {
            n: 2,
            scope: WindowScope::CurrentTeamCurrentSeason,
            policy: WindowPolicy::AllowPartialAbove(1),
        },
    ] {
        out.push(Constraint::SlidingWindow(SlidingWindowConstraint {
            stat: StatId::Points,
            window,
            predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(1.0)),
            axis: SeasonAxis::Regular,
        }));
    }
    for aggregator in [
        CareerAggregator::LifetimeSum,
        CareerAggregator::AnyWindow(2),
        CareerAggregator::LongestStreak,
        CareerAggregator::SeasonsWith,
    ] {
        out.push(Constraint::CareerAggregate(CareerAggrConstraint {
            stat: StatId::Points,
            aggregator,
            predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(1.0)),
            at_age: Some(AgeBound {
                min: Some(20),
                max: Some(35),
            }),
            axis: SeasonAxis::Regular,
        }));
    }
    for league in [
        LeagueAtom::Code("OHL".into()),
        LeagueAtom::InSet(vec!["OHL".into(), "WHL".into()]),
        LeagueAtom::Tier(LeagueTier::Junior),
    ] {
        out.push(Constraint::CareerLeague(CareerLeagueConstraint {
            league: league.clone(),
            stat: None,
            aggregator: None,
            predicate: None,
        }));
        out.push(Constraint::CareerLeague(CareerLeagueConstraint {
            league,
            stat: Some(StatId::Points),
            aggregator: Some(CareerAggregator::LifetimeSum),
            predicate: Some(Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(1.0))),
        }));
    }
    out.push(Constraint::All(vec![
        out[0].clone(),
        Constraint::Not(Box::new(out[1].clone())),
    ]));
    out.push(Constraint::Any(vec![out[2].clone(), out[3].clone()]));
    out
}
pub fn observations() -> Value {
    let rows = cases();
    let provider = Provider;
    let ctx = ctx(&provider);
    let results:Vec<_>=constraints().iter().map(|c| json!({"constraint":format!("{c:?}"),"matches":rows.iter().map(|(identity,stats)| c.matches(&PlayerView{identity,stats,contract:None},&ctx)).collect::<Vec<_>>()})).collect();
    json!({"scope":"synthetic-legacy-executor-oracle","rows":rows.len(),"constraints":results})
}
