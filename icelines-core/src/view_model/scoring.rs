use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::view_model::{SourceKind, SourceState, ViewContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShotEventKind {
    Goal,
    ShotOnGoal,
    MissedShot,
    BlockedShot,
}

impl ShotEventKind {
    pub fn counts_as_goal(self) -> bool {
        matches!(self, Self::Goal)
    }

    pub fn counts_as_shot_on_goal(self) -> bool {
        matches!(self, Self::Goal | Self::ShotOnGoal)
    }

    pub fn counts_as_unblocked_attempt(self) -> bool {
        matches!(self, Self::Goal | Self::ShotOnGoal | Self::MissedShot)
    }

    pub fn counts_as_attempt(self) -> bool {
        matches!(
            self,
            Self::Goal | Self::ShotOnGoal | Self::MissedShot | Self::BlockedShot
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShotLocation {
    pub x_coord: Option<i16>,
    pub y_coord: Option<i16>,
    pub zone_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringEventInput {
    pub game_id: u64,
    pub event_id: u32,
    pub date: Option<String>,
    pub kind: ShotEventKind,
    pub period: u8,
    pub period_type: String,
    pub time_in_period: String,
    pub situation_code: Option<String>,
    pub event_owner_team_id: Option<u32>,
    pub event_owner_team_abbrev: Option<String>,
    pub shooting_player_id: Option<u32>,
    pub scoring_player_id: Option<u32>,
    pub blocking_player_id: Option<u32>,
    pub goalie_in_net_id: Option<u32>,
    pub location: ShotLocation,
    pub shot_type: Option<String>,
    pub reason: Option<String>,
    pub home_team_defending_side: Option<String>,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringEventSummary {
    pub goals: u32,
    pub shots_on_goal: u32,
    pub missed_shots: u32,
    pub blocked_shots: u32,
    pub shot_attempts: u32,
    pub unblocked_attempts: u32,
}

impl ScoringEventSummary {
    pub fn from_events(events: &[ScoringEventInput]) -> Self {
        let mut summary = Self::default();
        for event in events {
            match event.kind {
                ShotEventKind::Goal => summary.goals += 1,
                ShotEventKind::ShotOnGoal => {}
                ShotEventKind::MissedShot => summary.missed_shots += 1,
                ShotEventKind::BlockedShot => summary.blocked_shots += 1,
            }
            if event.kind.counts_as_shot_on_goal() {
                summary.shots_on_goal += 1;
            }
            if event.kind.counts_as_unblocked_attempt() {
                summary.unblocked_attempts += 1;
            }
            if event.kind.counts_as_attempt() {
                summary.shot_attempts += 1;
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringSplitSummary {
    pub label: String,
    pub summary: ScoringEventSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringShooterSummary {
    pub player_id: u32,
    pub summary: ScoringEventSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScoringReportView {
    pub context: ViewContext,
    pub game_id: u64,
    pub events: Vec<ScoringEventInput>,
    pub summary: ScoringEventSummary,
    pub team_summaries: Vec<ScoringSplitSummary>,
    pub period_summaries: Vec<ScoringSplitSummary>,
    pub situation_summaries: Vec<ScoringSplitSummary>,
    pub top_shooters: Vec<ScoringShooterSummary>,
}

impl GameScoringReportView {
    pub fn from_events(context: ViewContext, game_id: u64, events: Vec<ScoringEventInput>) -> Self {
        Self::from_source_events(context, game_id, !events.is_empty(), events)
    }

    pub fn from_source_events(
        mut context: ViewContext,
        game_id: u64,
        source_loaded: bool,
        events: Vec<ScoringEventInput>,
    ) -> Self {
        context
            .source_state
            .push(play_by_play_source_state(source_loaded));
        let summary = ScoringEventSummary::from_events(&events);
        let team_summaries = split_summaries(&events, team_label);
        let period_summaries = split_summaries(&events, period_label);
        let situation_summaries = split_summaries(&events, situation_label);
        let top_shooters = top_shooter_summaries(&events);
        Self {
            context,
            game_id,
            events,
            summary,
            team_summaries,
            period_summaries,
            situation_summaries,
            top_shooters,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamScoringProfileView {
    pub context: ViewContext,
    pub team: String,
    pub events: Vec<ScoringEventInput>,
    pub summary: ScoringEventSummary,
    pub period_summaries: Vec<ScoringSplitSummary>,
    pub situation_summaries: Vec<ScoringSplitSummary>,
    pub top_shooters: Vec<ScoringShooterSummary>,
}

impl TeamScoringProfileView {
    pub fn from_events(
        context: ViewContext,
        team: impl Into<String>,
        events: Vec<ScoringEventInput>,
    ) -> Self {
        Self::from_source_events(context, team, !events.is_empty(), events)
    }

    pub fn from_source_events(
        mut context: ViewContext,
        team: impl Into<String>,
        source_loaded: bool,
        events: Vec<ScoringEventInput>,
    ) -> Self {
        context
            .source_state
            .push(play_by_play_source_state(source_loaded));
        let summary = ScoringEventSummary::from_events(&events);
        let period_summaries = split_summaries(&events, period_label);
        let situation_summaries = split_summaries(&events, situation_label);
        let top_shooters = top_shooter_summaries(&events);
        Self {
            context,
            team: team.into().to_ascii_uppercase(),
            events,
            summary,
            period_summaries,
            situation_summaries,
            top_shooters,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerScoringProfileView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub events: Vec<ScoringEventInput>,
    pub summary: ScoringEventSummary,
    pub period_summaries: Vec<ScoringSplitSummary>,
    pub situation_summaries: Vec<ScoringSplitSummary>,
}

impl PlayerScoringProfileView {
    pub fn from_events(
        context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        events: Vec<ScoringEventInput>,
    ) -> Self {
        Self::from_source_events(context, player_id, player_name, !events.is_empty(), events)
    }

    pub fn from_source_events(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        source_loaded: bool,
        events: Vec<ScoringEventInput>,
    ) -> Self {
        context
            .source_state
            .push(play_by_play_source_state(source_loaded));
        let summary = ScoringEventSummary::from_events(&events);
        let period_summaries = split_summaries(&events, period_label);
        let situation_summaries = split_summaries(&events, situation_label);
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            events,
            summary,
            period_summaries,
            situation_summaries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TonightScoringIntelView {
    pub context: ViewContext,
    pub date: String,
    pub games_loaded: usize,
    pub events_loaded: usize,
    pub summary: ScoringEventSummary,
    pub favorite_teams: Vec<TonightFavoriteTeamScoringRow>,
    pub favorite_players: Vec<TonightFavoritePlayerScoringRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TonightFavoriteTeamScoringRow {
    pub team: String,
    pub events_loaded: usize,
    pub summary: ScoringEventSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TonightFavoritePlayerScoringRow {
    pub player_key: String,
    pub player_id: Option<u32>,
    pub events_loaded: usize,
    pub summary: ScoringEventSummary,
}

impl TonightScoringIntelView {
    pub fn from_events(
        context: ViewContext,
        date: impl Into<String>,
        games_loaded: usize,
        events: &[ScoringEventInput],
    ) -> Self {
        Self::from_source_events(context, date, games_loaded, !events.is_empty(), events)
    }

    pub fn from_source_events(
        context: ViewContext,
        date: impl Into<String>,
        games_loaded: usize,
        source_loaded: bool,
        events: &[ScoringEventInput],
    ) -> Self {
        Self::from_favorites(
            context,
            date,
            games_loaded,
            source_loaded,
            events,
            Vec::new(),
            Vec::new(),
        )
    }

    pub fn from_favorites(
        mut context: ViewContext,
        date: impl Into<String>,
        games_loaded: usize,
        source_loaded: bool,
        events: &[ScoringEventInput],
        favorite_teams: Vec<TonightFavoriteTeamScoringRow>,
        favorite_players: Vec<TonightFavoritePlayerScoringRow>,
    ) -> Self {
        context
            .source_state
            .push(play_by_play_source_state(source_loaded));
        Self {
            context,
            date: date.into(),
            games_loaded,
            events_loaded: events.len(),
            summary: ScoringEventSummary::from_events(events),
            favorite_teams,
            favorite_players,
        }
    }
}

fn play_by_play_source_state(source_loaded: bool) -> SourceState {
    if source_loaded {
        SourceState::complete(SourceKind::PlayByPlay)
    } else {
        SourceState::missing(SourceKind::PlayByPlay)
    }
}

fn split_summaries(
    events: &[ScoringEventInput],
    label_fn: fn(&ScoringEventInput) -> String,
) -> Vec<ScoringSplitSummary> {
    let mut groups: BTreeMap<String, Vec<ScoringEventInput>> = BTreeMap::new();
    for event in events {
        groups
            .entry(label_fn(event))
            .or_default()
            .push(event.clone());
    }
    groups
        .into_iter()
        .map(|(label, events)| ScoringSplitSummary {
            label,
            summary: ScoringEventSummary::from_events(&events),
        })
        .collect()
}

fn top_shooter_summaries(events: &[ScoringEventInput]) -> Vec<ScoringShooterSummary> {
    let mut groups: BTreeMap<u32, Vec<ScoringEventInput>> = BTreeMap::new();
    for event in events {
        if let Some(player_id) = event.shooting_player_id.or(event.scoring_player_id) {
            groups.entry(player_id).or_default().push(event.clone());
        }
    }
    let mut rows: Vec<_> = groups
        .into_iter()
        .map(|(player_id, events)| ScoringShooterSummary {
            player_id,
            summary: ScoringEventSummary::from_events(&events),
        })
        .collect();
    rows.sort_by(|a, b| {
        b.summary
            .shot_attempts
            .cmp(&a.summary.shot_attempts)
            .then(b.summary.shots_on_goal.cmp(&a.summary.shots_on_goal))
            .then(b.summary.goals.cmp(&a.summary.goals))
            .then(a.player_id.cmp(&b.player_id))
    });
    rows
}

fn team_label(event: &ScoringEventInput) -> String {
    event
        .event_owner_team_abbrev
        .clone()
        .or_else(|| event.event_owner_team_id.map(|id| id.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn period_label(event: &ScoringEventInput) -> String {
    if event.period_type == "REG" {
        format!("P{}", event.period)
    } else {
        format!("{}{}", event.period_type, event.period)
    }
}

fn situation_label(event: &ScoringEventInput) -> String {
    event
        .situation_code
        .clone()
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    fn event(kind: ShotEventKind) -> ScoringEventInput {
        ScoringEventInput {
            game_id: 2025020001,
            event_id: 1,
            date: Some("2025-10-07".to_string()),
            kind,
            period: 1,
            period_type: "REG".to_string(),
            time_in_period: "01:00".to_string(),
            situation_code: Some("1551".to_string()),
            event_owner_team_id: Some(16),
            event_owner_team_abbrev: Some("CHI".to_string()),
            shooting_player_id: Some(8483493),
            scoring_player_id: if kind == ShotEventKind::Goal {
                Some(8483493)
            } else {
                None
            },
            blocking_player_id: None,
            goalie_in_net_id: Some(8475683),
            location: ShotLocation {
                x_coord: Some(66),
                y_coord: Some(-1),
                zone_code: Some("O".to_string()),
            },
            shot_type: Some("snap".to_string()),
            reason: None,
            home_team_defending_side: Some("right".to_string()),
            away_score: None,
            home_score: None,
        }
    }

    #[test]
    fn l0_scoring_summary_counts_nhl_attempt_families() {
        // NHL shot taxonomy: Corsi = all attempts (4), Fenwick = unblocked (3),
        // SOG = goals + saves (2), goals = goals only (1).
        let events = vec![
            event(ShotEventKind::Goal),
            event(ShotEventKind::ShotOnGoal),
            event(ShotEventKind::MissedShot),
            event(ShotEventKind::BlockedShot),
        ];

        let summary = ScoringEventSummary::from_events(&events);

        assert_eq!(summary.goals, 1);
        assert_eq!(summary.shots_on_goal, 2);
        assert_eq!(summary.missed_shots, 1);
        assert_eq!(summary.blocked_shots, 1);
        assert_eq!(summary.unblocked_attempts, 3);
        assert_eq!(summary.shot_attempts, 4);
    }

    #[test]
    fn l0_game_scoring_view_projects_split_summaries() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let mut goal = event(ShotEventKind::Goal);
        goal.event_owner_team_abbrev = Some("EDM".to_string());
        goal.period = 1;
        goal.situation_code = Some("1551".to_string());
        let mut shot = event(ShotEventKind::ShotOnGoal);
        shot.event_owner_team_abbrev = Some("LAK".to_string());
        shot.period = 2;
        shot.situation_code = Some("1451".to_string());
        let view =
            GameScoringReportView::from_source_events(context, 2025020001, true, vec![goal, shot]);

        assert_eq!(view.team_summaries.len(), 2);
        assert_eq!(view.team_summaries[0].label, "EDM");
        assert_eq!(view.period_summaries[0].label, "P1");
        assert_eq!(view.situation_summaries[0].label, "1451");
        assert_eq!(view.top_shooters[0].summary.shot_attempts, 2);
    }

    #[test]
    fn l0_game_scoring_view_marks_missing_play_by_play_when_empty() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = GameScoringReportView::from_events(context, 2025020001, Vec::new());

        assert_eq!(view.summary.shot_attempts, 0);
        assert_eq!(view.context.source_state[0].source, SourceKind::PlayByPlay);
        assert_eq!(
            view.context.source_state[0].state,
            crate::view_model::Completeness::Unavailable
        );
    }

    #[test]
    fn l0_game_scoring_view_distinguishes_loaded_zero_event_source() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = GameScoringReportView::from_source_events(context, 2025020001, true, Vec::new());

        assert_eq!(view.summary.shot_attempts, 0);
        assert_eq!(view.context.source_state[0].source, SourceKind::PlayByPlay);
        assert_eq!(
            view.context.source_state[0].state,
            crate::view_model::Completeness::Complete
        );
    }
}
