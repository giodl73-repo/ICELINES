//! Typed presentation boundary for IceLines surfaces.
//!
//! ViewModels carry hockey semantics, source context, identity, warnings, and
//! display policy. CLI, TUI, web, JSON, and reports render these shapes without
//! recomputing hockey logic.

pub mod context;
pub mod goalies;
pub mod leaders;
pub mod poach;
pub mod team_depth;
pub mod tokens;

pub use context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, FilterKey, FilterOp, RecoveryAction,
    ReportContext, ReportKind, ReportSectionRef, SortDirection, SortKey, SortState, SourceKind,
    SourceProvenance, SourceState, ViewContext, ViewWarning, ViewWindow, WarningKind,
};
pub use goalies::{GoalieRoleFilter, GoalieRoleSignal, GoalieRow, GoaliesView};
pub use leaders::{LeaderKind, LeaderRow, LeadersView};
pub use poach::{
    default_watch_rules_view, poach_report_context, AvailabilityState, ComponentStatus,
    ConfidenceSummary, DeploymentSignal, ExplanationImpact, PoachAvailabilityFilter,
    PoachBoardView, PoachCandidateKind, PoachComponentKind, PoachConfidence, PoachExplanation,
    PoachPlayerRow, PoachQuery, PoachReportSection, PoachReportView, PoachScheduleFilter,
    PoachScore, PoachScoreComponent, PoachWindow, RecommendationKind, ScoreRange, WatchRule,
    WatchRuleTrigger, WatchRulesView,
};
pub use team_depth::{
    DeploymentEvidence, DepthGoalieSlot, DepthLine, DepthPair, DepthPlayerSlot, DepthSlotKind,
    DepthSummary, TeamDepthView,
};
pub use tokens::{MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision};

#[cfg(test)]
mod tests {
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{
        Completeness, EmptyKind, LeaderKind, LeadersView, MetricCell, MetricUnit, MetricValue,
        SemanticToken, SourceKind, SourceProvenance, SourceState, StatKey, ValuePrecision,
        ViewContext, ViewWindow,
    };

    #[test]
    fn context_source_state_survives_json_projection() {
        let mut context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        context.completeness = Completeness::Partial;
        context.data_generation = Some("fixture-generation-1".to_string());
        context.source_state.push(SourceState {
            source: SourceKind::Roster,
            state: Completeness::Partial,
            provenance: Some(SourceProvenance::Snapshot {
                id: "snapshot-2026-05-09".to_string(),
            }),
            fetched_at: None,
            stale_reason: None,
            message: Some("missing shifts".to_string()),
        });

        let view = LeadersView::new(context, LeaderKind::Skaters);
        let json = serde_json::to_string(&view).expect("serialize leaders view");

        assert!(json.contains("\"season\":20252026"));
        assert!(json.contains("\"season_type\":\"regular\""));
        assert!(json.contains("\"completeness\":\"partial\""));
        assert!(json.contains("\"source\":\"roster\""));
        assert!(json.contains("\"data_generation\":\"fixture-generation-1\""));
    }

    #[test]
    fn metric_cell_carries_precision_and_semantic_token() {
        let cell = MetricCell {
            key: StatKey::from("points_per_game"),
            label: "PPG".to_string(),
            value: MetricValue::Decimal(1.27),
            unit: MetricUnit::PerGame,
            precision: ValuePrecision::TwoDecimals,
            token: Some(SemanticToken::DecisionHighlight),
        };

        let json = serde_json::to_value(&cell).expect("serialize metric cell");

        assert_eq!(json["key"], "points_per_game");
        assert_eq!(json["unit"], "per_game");
        assert_eq!(json["precision"], "two_decimals");
        assert_eq!(json["token"], "decision_highlight");
    }

    #[test]
    fn report_context_reuses_view_context() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let report = crate::view_model::ReportContext {
            kind: crate::view_model::ReportKind::Scouting,
            view_context: context,
            report_id: "scouting-8478402".to_string(),
            title: "Scouting Report".to_string(),
            sections: vec![crate::view_model::ReportSectionRef {
                id: "summary".to_string(),
                title: "Summary".to_string(),
            }],
        };

        let json = serde_json::to_value(&report).expect("serialize report context");

        assert_eq!(json["kind"], "scouting");
        assert_eq!(json["view_context"]["window"]["season"], 20252026);
        assert_eq!(json["sections"][0]["id"], "summary");
    }

    #[test]
    fn first_viewmodel_builders_read_from_repository() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let leaders = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        assert_eq!(leaders.rows.len(), 1);
        assert_eq!(
            leaders.rows[0].primary.key,
            crate::view_model::StatKey::from("pace_82")
        );
        assert_eq!(leaders.rows[0].rank, 1);

        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );
        assert_eq!(depth.forward_lines.len(), 4);
        assert!(depth.forward_lines.iter().any(|line| line
            .center
            .as_ref()
            .is_some_and(|slot| slot.display_name == "Connor McDavid")));
    }

    #[test]
    fn leaders_viewmodel_contract_fixture_serializes_surface_fields() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut view = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        view.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize leaders contract fixture");
        let row = &json["rows"][0];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["kind"], "skaters");
        assert_eq!(json["sort"]["key"], "pace_82");
        assert_eq!(json["sort"]["direction"], "desc");

        assert_eq!(row["rank"], 1);
        assert_eq!(row["player_id"], 8478402);
        assert_eq!(row["display_name"], "Connor McDavid");
        assert_eq!(row["team"], "EDM");
        assert_eq!(row["position"], "Center");
        assert_eq!(row["primary"]["key"], "pace_82");
        assert_eq!(row["primary"]["value"]["decimal"], 130.0);
        assert_eq!(row["primary"]["unit"], "per82");
        assert_eq!(row["primary"]["precision"], "one_decimal");
        assert_eq!(row["primary"]["token"], "decision_highlight");

        assert_eq!(row["secondary"][0]["key"], "age");
        assert_eq!(row["secondary"][0]["value"]["integer"], 29);
        assert_eq!(row["secondary"][1]["key"], "gp");
        assert_eq!(row["secondary"][1]["value"]["integer"], 82);
        assert_eq!(row["secondary"][4]["key"], "points");
        assert_eq!(row["secondary"][4]["value"]["integer"], 130);
        assert_eq!(row["tokens"][0], "supporting_evidence");
    }

    #[test]
    fn goalie_viewmodel_builder_preserves_role_evidence() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let goalies = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );

        assert_eq!(goalies.rows.len(), 1);
        assert_eq!(goalies.rows[0].role_signal.label, "starter");
        assert_eq!(
            goalies.rows[0].role_signal.evidence,
            crate::view_model::DeploymentEvidence::Actual
        );
    }

    #[test]
    fn goalie_viewmodel_contract_fixture_serializes_role_and_metrics() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let mut view = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(20242025),
            SeasonType::Regular,
        );
        view.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&view).expect("serialize goalies contract fixture");
        let row = &json["rows"][0];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["sort"]["key"], "save_pct");
        assert_eq!(json["sort"]["direction"], "desc");

        assert_eq!(row["player_id"], 8476434);
        assert_eq!(row["display_name"], "Connor McDavid");
        assert_eq!(row["team"], "FLA");
        assert_eq!(row["role_signal"]["label"], "starter");
        assert_eq!(row["role_signal"]["evidence"], "actual");
        assert_eq!(row["metrics"][0]["key"], "gp");
        assert_eq!(row["metrics"][0]["unit"], "games");
        assert_eq!(row["metrics"][0]["precision"], "integer");
        assert_eq!(row["metrics"][5]["key"], "save_pct");
        assert_eq!(row["metrics"][5]["unit"], "percentage");
        assert_eq!(row["metrics"][5]["precision"], "three_decimals");
        assert_eq!(row["tokens"][0], "supporting_evidence");
    }

    #[test]
    fn missing_windows_are_not_marked_complete() {
        let repo = crate::stats_repository::StatsRepository::new();

        let leaders = crate::view_model::LeadersView::skater_pace(
            &repo,
            Season(19991998),
            SeasonType::Regular,
        );
        let goalies = crate::view_model::GoaliesView::from_repository(
            &repo,
            Season(19991998),
            SeasonType::Regular,
        );
        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            crate::model::TeamAbbr("EDM".to_string()),
            Season(19991998),
            SeasonType::Regular,
        );

        assert_eq!(leaders.context.completeness, Completeness::Unavailable);
        assert_eq!(
            leaders.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
        assert_eq!(goalies.context.completeness, Completeness::Unavailable);
        assert_eq!(
            goalies.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
        assert_eq!(depth.context.completeness, Completeness::Unavailable);
        assert_eq!(depth.context.source_state[0].source, SourceKind::Roster);
    }

    #[test]
    fn team_depth_preserves_goalie_section() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::goalie();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with_goalie(identity, stats);

        let depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );

        assert_eq!(depth.goalies.len(), 1);
        assert_eq!(depth.goalies[0].role, "starter");
        assert!(
            depth.extras.is_empty(),
            "goalies must not be rendered as extras"
        );
    }

    #[test]
    fn team_depth_contract_fixture_serializes_slots_and_goalie_section() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let team = stats.team_stints[0].team.clone();
        let repo = crate::fixtures::test_repo_with(identity, stats);

        let mut depth = crate::view_model::TeamDepthView::from_repository(
            &repo,
            team,
            Season(20242025),
            SeasonType::Regular,
        );
        depth.context.data_generation = Some("campbell-contract-fixture-v1".to_string());

        let json = serde_json::to_value(&depth).expect("serialize team depth contract fixture");
        let center = &json["forward_lines"][0]["center"];

        assert_eq!(json["context"]["window"]["season"], 20242025);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["context"]["completeness"], "complete");
        assert_eq!(
            json["context"]["data_generation"],
            "campbell-contract-fixture-v1"
        );
        assert_eq!(json["team"], "EDM");
        assert_eq!(json["summary"]["metrics"][0]["key"], "rostered");
        assert_eq!(json["summary"]["metrics"][0]["value"]["integer"], 1);
        assert_eq!(json["summary"]["tokens"][0], "supporting_evidence");

        assert_eq!(center["player_id"], 8478402);
        assert_eq!(center["display_name"], "Connor McDavid");
        assert_eq!(center["team"], "EDM");
        assert_eq!(center["slot"]["forward"]["line"], 1);
        assert_eq!(center["slot"]["forward"]["slot"], "center");
        assert_eq!(center["position"], "Center");
        assert_eq!(center["evidence"], "estimated");
        assert_eq!(center["metrics"][0]["key"], "pace_82");
        assert_eq!(center["metrics"][0]["unit"], "per82");
        assert_eq!(center["metrics"][0]["precision"], "one_decimal");
    }
}
