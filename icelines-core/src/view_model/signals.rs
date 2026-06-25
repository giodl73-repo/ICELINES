use serde::{Deserialize, Serialize};

use crate::model::{Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::signal_metrics::{
    SignalEvidenceTier, SignalInput, SignalMetricId, SignalMetricUnit, SignalPolarity,
};
use crate::stats_repository::PlayerView;
use crate::view_model::{ViewContext, ViewWindow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSignalsView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub team: String,
    pub position: String,
    pub games_played: u32,
    pub rows: Vec<PlayerSignalRow>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
    pub source_authority: SignalsSourceAuthority,
}

impl PlayerSignalsView {
    pub fn from_player(context: ViewContext, player: &PlayerView<'_>) -> Self {
        Self {
            context,
            player_id: player.id().0,
            player_name: player.full_name().to_string(),
            team: player.team_display().to_string(),
            position: player.position().abbreviation().to_string(),
            games_played: player.gp(),
            rows: SignalMetricId::all()
                .iter()
                .copied()
                .map(|metric| PlayerSignalRow::from_metric(metric, player))
                .collect(),
            disclosures: vec![
                "Signals are descriptive derived metrics built from existing stat inputs."
                    .to_string(),
                "Unavailable Signals mean required evidence is missing or below threshold, not zero value truth."
                    .to_string(),
            ],
            non_claims: vec![
                "Not a prediction, betting edge, injury signal, deployment recommendation, or autonomous coaching decision."
                    .to_string(),
            ],
            source_authority: SignalsSourceAuthority::default(),
        }
    }

    pub fn context_for_player(player: &PlayerView<'_>) -> ViewContext {
        ViewContext::new(ViewWindow::new(player.season(), player.season_type()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalsSourceAuthority {
    pub source: String,
    pub coverage_state: String,
    pub covered_inputs: Vec<String>,
    pub covered_metrics: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub limitations: Vec<String>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalRosterEvidenceFilter {
    All,
    Full,
    Partial,
    Missing,
}

impl SignalRosterEvidenceFilter {
    pub fn matches(self, view: &PlayerSignalsView) -> bool {
        match self {
            Self::All => true,
            Self::Full => view
                .rows
                .iter()
                .all(|row| row.evidence_tier == SignalEvidenceTier::Full),
            Self::Partial => view
                .rows
                .iter()
                .any(|row| row.evidence_tier == SignalEvidenceTier::Partial),
            Self::Missing => view
                .rows
                .iter()
                .any(|row| row.evidence_tier == SignalEvidenceTier::Missing),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Full => "full",
            Self::Partial => "partial",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalsRosterView {
    pub schema_note: String,
    pub team: String,
    pub season: u32,
    pub season_type: String,
    pub rows: Vec<PlayerSignalsView>,
    pub evidence_filter: SignalRosterEvidenceFilter,
    pub total_player_count: usize,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
    pub source_authority: SignalsSourceAuthority,
}

impl SignalsRosterView {
    pub fn from_players<'a>(
        team: TeamAbbr,
        season: Season,
        season_type: SeasonType,
        players: impl IntoIterator<Item = PlayerView<'a>>,
        evidence_filter: SignalRosterEvidenceFilter,
    ) -> Self {
        let all_rows: Vec<PlayerSignalsView> = players
            .into_iter()
            .filter(|player| player.team_display() == team.0)
            .map(|player| {
                PlayerSignalsView::from_player(
                    PlayerSignalsView::context_for_player(&player),
                    &player,
                )
            })
            .collect();
        let total_player_count = all_rows.len();
        let mut rows: Vec<PlayerSignalsView> = all_rows
            .into_iter()
            .filter(|row| evidence_filter.matches(row))
            .collect();
        rows.sort_by(|a, b| {
            a.player_name
                .cmp(&b.player_name)
                .then_with(|| a.player_id.cmp(&b.player_id))
        });

        Self {
            schema_note: "Team-scoped Signals discovery matrix; not a leaderboard.".to_string(),
            team: team.0,
            season: season.0,
            season_type: season_type.label().to_string(),
            rows,
            evidence_filter,
            total_player_count,
            disclosures: vec![
                "Signals are descriptive derived metrics built from existing stat inputs."
                    .to_string(),
                "Unavailable Signals mean required evidence is missing or below threshold, not zero value truth."
                    .to_string(),
                "This matrix is an inspection aid; open a player Signals card or Markdown packet for full methodology and limitations."
                    .to_string(),
            ],
            non_claims: vec![
                "Not a prediction, betting edge, injury signal, deployment recommendation, player-quality grade, or autonomous coaching decision."
                    .to_string(),
                "Not a Signal leaderboard, StatId promotion, filter key, or analytics-cache metric family."
                    .to_string(),
            ],
            source_authority: SignalsSourceAuthority::default(),
        }
    }

    pub fn filtered_out_count(&self) -> usize {
        self.total_player_count.saturating_sub(self.rows.len())
    }
}

impl Default for SignalsSourceAuthority {
    fn default() -> Self {
        Self {
            source: "PlayerSignalsView stat inputs".to_string(),
            coverage_state: "descriptive_derived".to_string(),
            covered_inputs: vec![
                "season_stat_summary".to_string(),
                "skater_realtime_when_loaded".to_string(),
                "time_on_ice_when_loaded".to_string(),
                "minimum_games_threshold".to_string(),
            ],
            covered_metrics: vec![
                "physical_engagement_rate".to_string(),
                "puck_management_differential".to_string(),
                "penalty_drag_rate".to_string(),
            ],
            blocked_claims: vec![
                "prediction".to_string(),
                "betting_edge".to_string(),
                "injury_signal".to_string(),
                "deployment_recommendation".to_string(),
                "player_quality_grade".to_string(),
                "autonomous_coaching_decision".to_string(),
                "stat_catalog_promotion".to_string(),
                "leaderboard_ranking".to_string(),
            ],
            limitations: vec![
                "missing_inputs_render_unavailable_not_zero".to_string(),
                "realtime_stats_carry_rink_scorer_bias".to_string(),
                "penalty_minutes_mix_penalty_types".to_string(),
                "no_teammate_zone_or_matchup_isolation".to_string(),
            ],
            label: "Signals authority: descriptive derived metrics from loaded stat inputs; not predictions, betting, injury, deployment, player-grade, leaderboard, or autonomous coaching claims."
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSignalRow {
    pub id: SignalMetricId,
    pub label: String,
    pub short_label: String,
    pub cli_key: String,
    pub unit: SignalMetricUnit,
    pub polarity: SignalPolarity,
    pub value: Option<f64>,
    pub evidence_tier: SignalEvidenceTier,
    pub missing_inputs: Vec<SignalInput>,
    pub methodology: String,
    pub limitations: String,
}

impl PlayerSignalRow {
    pub fn from_metric(metric: SignalMetricId, player: &PlayerView<'_>) -> Self {
        let descriptor = metric.descriptor();
        let evidence = metric.evidence(player);
        Self {
            id: metric,
            label: descriptor.label.to_string(),
            short_label: descriptor.short_label.to_string(),
            cli_key: descriptor.cli_key.to_string(),
            unit: descriptor.unit,
            polarity: descriptor.polarity,
            value: metric.read(player),
            evidence_tier: evidence.tier,
            missing_inputs: evidence.missing_inputs,
            methodology: descriptor.methodology.to_string(),
            limitations: descriptor.limitations.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{identity, stats, test_repo_with};
    use crate::identity::PlayerId;
    use crate::model::Season;
    use crate::season_stats::SeasonType;

    fn signal_player(realtime: bool) -> crate::stats_repository::StatsRepository {
        let identity = identity(8478402).build();
        let mut stats = stats(8478402, 20252026, "EDM");
        if realtime {
            stats = stats.realtime(140, 70, 35, 21);
        }
        test_repo_with(identity, stats.build())
    }

    fn row(view: &PlayerSignalsView, id: SignalMetricId) -> &PlayerSignalRow {
        view.rows
            .iter()
            .find(|row| row.id == id)
            .expect("signal row")
    }

    #[test]
    fn l0_player_signals_view_preserves_signal_descriptors_and_values() {
        let repo = signal_player(true);
        let player = repo
            .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
            .expect("player view");
        let view =
            PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player);

        assert_eq!(view.player_name, "Connor McDavid");
        assert_eq!(view.team, "EDM");
        assert_eq!(view.position, "C");
        assert_eq!(view.games_played, 70);
        assert_eq!(view.rows.len(), SignalMetricId::all().len());

        let physical = row(&view, SignalMetricId::PhysicalEngagementRate);
        assert_eq!(physical.cli_key, "physical-engagement-rate");
        assert_eq!(physical.short_label, "Phys/60");
        assert_eq!(physical.evidence_tier, SignalEvidenceTier::Full);
        assert_eq!(physical.value, Some(9.0));
        assert!(physical.limitations.contains("scorer bias"));

        let puck_management = row(&view, SignalMetricId::PuckManagementDifferential);
        assert_eq!(puck_management.polarity, SignalPolarity::HigherIsBetter);
        assert_eq!(puck_management.value, Some(0.6));

        let penalty_drag = row(&view, SignalMetricId::PenaltyDragRate);
        assert_eq!(penalty_drag.polarity, SignalPolarity::LowerIsBetter);
        assert!(penalty_drag.value.unwrap() > 0.0);
    }

    #[test]
    fn l0_player_signals_view_preserves_unavailable_evidence_without_zero_fill() {
        let repo = signal_player(false);
        let player = repo
            .view(PlayerId(8478402), Season(20252026), SeasonType::Regular)
            .expect("player view");
        let view =
            PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player);

        let physical = row(&view, SignalMetricId::PhysicalEngagementRate);
        assert_eq!(physical.value, None);
        assert_eq!(physical.evidence_tier, SignalEvidenceTier::Partial);
        assert_eq!(physical.missing_inputs, vec![SignalInput::Realtime]);

        let puck_management = row(&view, SignalMetricId::PuckManagementDifferential);
        assert_eq!(puck_management.value, None);
        assert_eq!(puck_management.missing_inputs, vec![SignalInput::Realtime]);

        let penalty_drag = row(&view, SignalMetricId::PenaltyDragRate);
        assert!(penalty_drag.value.is_some());
        assert_eq!(penalty_drag.missing_inputs, Vec::<SignalInput>::new());
        assert!(view.non_claims[0].contains("Not a prediction"));
        assert!(view.disclosures[1].contains("not zero value truth"));
        assert_eq!(view.source_authority.coverage_state, "descriptive_derived");
        assert!(view
            .source_authority
            .covered_metrics
            .contains(&"physical_engagement_rate".to_string()));
        assert!(view
            .source_authority
            .blocked_claims
            .contains(&"deployment_recommendation".to_string()));
        assert!(view
            .source_authority
            .limitations
            .contains(&"missing_inputs_render_unavailable_not_zero".to_string()));
    }
}
