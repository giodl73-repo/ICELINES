use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::view_model::{Completeness, SourceKind, SourceState, ViewContext};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InsideShotBucket {
    Crease,
    Inside,
    Slot,
    Outside,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InsideShotProxy {
    pub bucket: InsideShotBucket,
    pub distance_ft: Option<f64>,
}

impl InsideShotProxy {
    pub fn from_location(location: &ShotLocation) -> Self {
        let Some(x_coord) = location.x_coord else {
            return Self::unknown();
        };
        let Some(y_coord) = location.y_coord else {
            return Self::unknown();
        };

        let x_distance = 89.0 - f64::from(x_coord).abs();
        let y_distance = f64::from(y_coord);
        let distance_ft = (x_distance.powi(2) + y_distance.powi(2)).sqrt();
        Self {
            bucket: InsideShotBucket::from_distance_ft(distance_ft),
            distance_ft: Some(distance_ft),
        }
    }

    fn unknown() -> Self {
        Self {
            bucket: InsideShotBucket::Unknown,
            distance_ft: None,
        }
    }
}

impl InsideShotBucket {
    pub fn from_distance_ft(distance_ft: f64) -> Self {
        if distance_ft <= 10.0 {
            Self::Crease
        } else if distance_ft <= 25.0 {
            Self::Inside
        } else if distance_ft <= 40.0 {
            Self::Slot
        } else {
            Self::Outside
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Crease => "crease",
            Self::Inside => "inside",
            Self::Slot => "slot",
            Self::Outside => "outside",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsideShotBucketCounts {
    pub crease: u32,
    pub inside: u32,
    pub slot: u32,
    pub outside: u32,
    pub unknown: u32,
}

impl InsideShotBucketCounts {
    fn record(&mut self, bucket: InsideShotBucket) {
        match bucket {
            InsideShotBucket::Crease => self.crease += 1,
            InsideShotBucket::Inside => self.inside += 1,
            InsideShotBucket::Slot => self.slot += 1,
            InsideShotBucket::Outside => self.outside += 1,
            InsideShotBucket::Unknown => self.unknown += 1,
        }
    }
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

impl ScoringEventInput {
    pub fn inside_shot_proxy(&self) -> InsideShotProxy {
        InsideShotProxy::from_location(&self.location)
    }

    pub fn scoring_attempt_player_id(&self) -> Option<u32> {
        match self.kind {
            ShotEventKind::Goal => self.scoring_player_id,
            ShotEventKind::ShotOnGoal | ShotEventKind::MissedShot | ShotEventKind::BlockedShot => {
                self.shooting_player_id
            }
        }
    }

    pub fn matches_scoring_attempt_player(&self, player_id: u32) -> bool {
        self.scoring_attempt_player_id() == Some(player_id)
    }
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
            summary.record_event(event);
        }
        summary
    }

    pub fn record_event(&mut self, event: &ScoringEventInput) {
        match event.kind {
            ShotEventKind::Goal => self.goals += 1,
            ShotEventKind::ShotOnGoal => {}
            ShotEventKind::MissedShot => self.missed_shots += 1,
            ShotEventKind::BlockedShot => self.blocked_shots += 1,
        }
        if event.kind.counts_as_shot_on_goal() {
            self.shots_on_goal += 1;
        }
        if event.kind.counts_as_unblocked_attempt() {
            self.unblocked_attempts += 1;
        }
        if event.kind.counts_as_attempt() {
            self.shot_attempts += 1;
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlayerScoringTrendWindow {
    Last3Games,
    Last5Games,
    Last10Games,
    SeasonLoaded,
}

impl PlayerScoringTrendWindow {
    fn max_games(self) -> Option<usize> {
        match self {
            Self::Last3Games => Some(3),
            Self::Last5Games => Some(5),
            Self::Last10Games => Some(10),
            Self::SeasonLoaded => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Last3Games => "recent volume - last 3 games",
            Self::Last5Games => "recent volume - last 5 games",
            Self::Last10Games => "recent volume - last 10 games",
            Self::SeasonLoaded => "season loaded",
        }
    }
}

const PLAYER_SCORING_TREND_WINDOWS: [PlayerScoringTrendWindow; 4] = [
    PlayerScoringTrendWindow::Last3Games,
    PlayerScoringTrendWindow::Last5Games,
    PlayerScoringTrendWindow::Last10Games,
    PlayerScoringTrendWindow::SeasonLoaded,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScoringTrendRow {
    pub window: PlayerScoringTrendWindow,
    pub label: String,
    pub games_loaded: usize,
    pub events_loaded: usize,
    pub source_loaded: bool,
    pub source_partial: bool,
    pub summary: ScoringEventSummary,
    pub shot_pct: Option<f64>,
    pub bucket_counts: InsideShotBucketCounts,
}

impl PlayerScoringTrendRow {
    fn from_games(
        window: PlayerScoringTrendWindow,
        source_loaded: bool,
        source_partial: bool,
        games: &[Vec<&ScoringEventInput>],
    ) -> Self {
        let mut summary = ScoringEventSummary::default();
        let mut bucket_counts = InsideShotBucketCounts::default();
        let mut events_loaded = 0;
        for game in games {
            for event in game {
                events_loaded += 1;
                summary.record_event(event);
                bucket_counts.record(event.inside_shot_proxy().bucket);
            }
        }
        Self {
            window,
            label: window.label().to_string(),
            games_loaded: games.len(),
            events_loaded,
            source_loaded,
            source_partial,
            summary,
            shot_pct: (summary.shots_on_goal > 0)
                .then_some(summary.goals as f64 / summary.shots_on_goal as f64),
            bucket_counts,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScoringProfileView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub events: Vec<ScoringEventInput>,
    pub summary: ScoringEventSummary,
    pub trends: Vec<PlayerScoringTrendRow>,
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
        let trends = player_scoring_trend_rows(
            player_id,
            source_loaded,
            play_by_play_source_partial(&context),
            &events,
        );
        let period_summaries = split_summaries(&events, period_label);
        let situation_summaries = split_summaries(&events, situation_label);
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            events,
            summary,
            trends,
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

fn play_by_play_source_partial(context: &ViewContext) -> bool {
    context.completeness == Completeness::Partial
        || context.source_state.iter().any(|state| {
            state.source == SourceKind::PlayByPlay && state.state == Completeness::Partial
        })
}

fn player_scoring_trend_rows(
    player_id: u32,
    source_loaded: bool,
    source_partial: bool,
    events: &[ScoringEventInput],
) -> Vec<PlayerScoringTrendRow> {
    let mut games: BTreeMap<(Option<String>, u64), Vec<&ScoringEventInput>> = BTreeMap::new();
    for event in events
        .iter()
        .filter(|event| event.matches_scoring_attempt_player(player_id))
    {
        games
            .entry((event.date.clone(), event.game_id))
            .or_default()
            .push(event);
    }
    let games: Vec<Vec<&ScoringEventInput>> = games.into_values().collect();
    PLAYER_SCORING_TREND_WINDOWS
        .into_iter()
        .map(|window| {
            let start = window
                .max_games()
                .map_or(0, |max_games| games.len().saturating_sub(max_games));
            PlayerScoringTrendRow::from_games(
                window,
                source_loaded,
                source_partial,
                &games[start..],
            )
        })
        .collect()
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

    fn location(x_coord: Option<i16>, y_coord: Option<i16>) -> ShotLocation {
        ShotLocation {
            x_coord,
            y_coord,
            zone_code: None,
        }
    }

    fn player_event(
        kind: ShotEventKind,
        game_number: u64,
        x_coord: Option<i16>,
        y_coord: Option<i16>,
    ) -> ScoringEventInput {
        let mut event = event(kind);
        event.game_id = 2025020000 + game_number;
        event.event_id = game_number as u32;
        event.date = Some(format!("2025-10-{game_number:02}"));
        event.location = location(x_coord, y_coord);
        event
    }

    fn trend(
        view: &PlayerScoringProfileView,
        window: PlayerScoringTrendWindow,
    ) -> &PlayerScoringTrendRow {
        view.trends
            .iter()
            .find(|row| row.window == window)
            .expect("trend window")
    }

    #[test]
    fn l0_inside_shot_proxy_buckets_known_distances() {
        let cases = [
            // distance = sqrt((89 - abs(89))^2 + 0^2) = 0 ft.
            (Some(89), Some(0), 0.0, InsideShotBucket::Crease),
            // distance = sqrt((89 - abs(79))^2 + 0^2) = 10 ft.
            (Some(79), Some(0), 10.0, InsideShotBucket::Crease),
            // distance = sqrt((89 - abs(69))^2 + 0^2) = 20 ft.
            (Some(69), Some(0), 20.0, InsideShotBucket::Inside),
            // distance = sqrt((89 - abs(54))^2 + 0^2) = 35 ft.
            (Some(54), Some(0), 35.0, InsideShotBucket::Slot),
            // distance = sqrt((89 - abs(0))^2 + 0^2) = 89 ft.
            (Some(0), Some(0), 89.0, InsideShotBucket::Outside),
        ];

        for (x_coord, y_coord, distance_ft, bucket) in cases {
            let proxy = InsideShotProxy::from_location(&location(x_coord, y_coord));

            assert_eq!(proxy.distance_ft, Some(distance_ft));
            assert_eq!(proxy.bucket, bucket);
        }
    }

    #[test]
    fn l0_inside_shot_proxy_missing_coordinates_are_unknown() {
        for (x_coord, y_coord) in [(None, Some(0)), (Some(89), None), (None, None)] {
            let proxy = InsideShotProxy::from_location(&location(x_coord, y_coord));

            assert_eq!(proxy.distance_ft, None);
            assert_eq!(proxy.bucket, InsideShotBucket::Unknown);
        }
    }

    #[test]
    fn l0_inside_shot_proxy_is_symmetric_across_rink_ends() {
        let positive = InsideShotProxy::from_location(&location(Some(69), Some(0)));
        let negative = InsideShotProxy::from_location(&location(Some(-69), Some(0)));

        assert_eq!(positive.distance_ft, Some(20.0));
        assert_eq!(negative.distance_ft, Some(20.0));
        assert_eq!(positive.bucket, negative.bucket);
    }

    #[test]
    fn l0_scoring_event_projects_inside_shot_proxy_from_location() {
        let mut event = event(ShotEventKind::ShotOnGoal);
        event.location = location(Some(54), Some(0));

        let proxy = event.inside_shot_proxy();

        assert_eq!(proxy.distance_ft, Some(35.0));
        assert_eq!(proxy.bucket, InsideShotBucket::Slot);
    }

    #[test]
    fn l0_scoring_attempt_player_id_matches_kind_specific_source() {
        let mut goal = event(ShotEventKind::Goal);
        goal.scoring_player_id = Some(1);
        goal.shooting_player_id = Some(2);
        let mut shot = event(ShotEventKind::ShotOnGoal);
        shot.scoring_player_id = Some(1);
        shot.shooting_player_id = Some(2);

        assert_eq!(goal.scoring_attempt_player_id(), Some(1));
        assert!(goal.matches_scoring_attempt_player(1));
        assert!(!goal.matches_scoring_attempt_player(2));
        assert_eq!(shot.scoring_attempt_player_id(), Some(2));
        assert!(shot.matches_scoring_attempt_player(2));
        assert!(!shot.matches_scoring_attempt_player(1));
    }

    #[test]
    fn l0_scoring_attempt_player_id_does_not_guess_missing_ids() {
        let mut goal = event(ShotEventKind::Goal);
        goal.scoring_player_id = None;
        goal.shooting_player_id = Some(2);
        let mut shot = event(ShotEventKind::MissedShot);
        shot.scoring_player_id = Some(1);
        shot.shooting_player_id = None;

        assert_eq!(goal.scoring_attempt_player_id(), None);
        assert!(!goal.matches_scoring_attempt_player(2));
        assert_eq!(shot.scoring_attempt_player_id(), None);
        assert!(!shot.matches_scoring_attempt_player(1));
    }

    #[test]
    fn l0_player_scoring_trends_select_recent_windows() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let events = vec![
            // Game 1: one goal, distance = 0 ft -> crease.
            player_event(ShotEventKind::Goal, 1, Some(89), Some(0)),
            // Game 2: one saved shot, distance = 20 ft -> inside.
            player_event(ShotEventKind::ShotOnGoal, 2, Some(69), Some(0)),
            // Game 3: one miss, missing x -> unknown location.
            player_event(ShotEventKind::MissedShot, 3, None, Some(0)),
            // Game 4: one block, distance = 89 ft -> outside.
            player_event(ShotEventKind::BlockedShot, 4, Some(0), Some(0)),
        ];

        let view = PlayerScoringProfileView::from_source_events(
            context,
            8483493,
            "Connor Bedard",
            true,
            events,
        );
        let last_three = trend(&view, PlayerScoringTrendWindow::Last3Games);
        let season = trend(&view, PlayerScoringTrendWindow::SeasonLoaded);

        assert_eq!(view.trends.len(), 4);
        assert_eq!(last_three.label, "recent volume - last 3 games");
        assert_eq!(last_three.games_loaded, 3);
        assert_eq!(last_three.events_loaded, 3);
        assert_eq!(last_three.summary.goals, 0);
        assert_eq!(last_three.summary.shots_on_goal, 1);
        assert_eq!(last_three.summary.unblocked_attempts, 2);
        assert_eq!(last_three.summary.shot_attempts, 3);
        assert_eq!(season.games_loaded, 4);
        assert_eq!(season.events_loaded, 4);
        assert_eq!(season.summary.goals, 1);
        assert_eq!(season.summary.shots_on_goal, 2);
        assert_eq!(season.shot_pct, Some(0.5));
    }

    #[test]
    fn l0_player_scoring_trends_match_goal_and_shot_player_ids() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let mut matching_goal = player_event(ShotEventKind::Goal, 1, Some(89), Some(0));
        matching_goal.scoring_player_id = Some(1);
        matching_goal.shooting_player_id = Some(2);
        let mut matching_shot = player_event(ShotEventKind::ShotOnGoal, 2, Some(69), Some(0));
        matching_shot.scoring_player_id = Some(2);
        matching_shot.shooting_player_id = Some(1);
        let mut non_matching_goal = player_event(ShotEventKind::Goal, 3, Some(54), Some(0));
        non_matching_goal.scoring_player_id = Some(2);
        non_matching_goal.shooting_player_id = Some(1);
        let mut non_matching_shot = player_event(ShotEventKind::MissedShot, 4, Some(0), Some(0));
        non_matching_shot.scoring_player_id = Some(1);
        non_matching_shot.shooting_player_id = Some(2);

        let view = PlayerScoringProfileView::from_source_events(
            context,
            1,
            "Player One",
            true,
            vec![
                matching_goal,
                matching_shot,
                non_matching_goal,
                non_matching_shot,
            ],
        );
        let season = trend(&view, PlayerScoringTrendWindow::SeasonLoaded);

        assert_eq!(season.games_loaded, 2);
        assert_eq!(season.events_loaded, 2);
        assert_eq!(season.summary.goals, 1);
        assert_eq!(season.summary.shots_on_goal, 2);
        assert_eq!(season.summary.shot_attempts, 2);
    }

    #[test]
    fn l0_player_scoring_trends_keep_zero_shot_conversion_null() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = PlayerScoringProfileView::from_source_events(
            context,
            8483493,
            "Connor Bedard",
            true,
            vec![
                player_event(ShotEventKind::MissedShot, 1, Some(54), Some(0)),
                player_event(ShotEventKind::BlockedShot, 2, Some(0), Some(0)),
            ],
        );
        let season = trend(&view, PlayerScoringTrendWindow::SeasonLoaded);

        assert_eq!(season.summary.shots_on_goal, 0);
        assert_eq!(season.summary.shot_attempts, 2);
        assert_eq!(season.shot_pct, None);
    }

    #[test]
    fn l0_player_scoring_trends_count_inside_proxy_buckets_and_unknown_locations() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = PlayerScoringProfileView::from_source_events(
            context,
            8483493,
            "Connor Bedard",
            true,
            vec![
                // Hand-calculated buckets from Pulse 02: 0, 20, 35, 89, unknown.
                player_event(ShotEventKind::Goal, 1, Some(89), Some(0)),
                player_event(ShotEventKind::ShotOnGoal, 2, Some(69), Some(0)),
                player_event(ShotEventKind::MissedShot, 3, Some(54), Some(0)),
                player_event(ShotEventKind::BlockedShot, 4, Some(0), Some(0)),
                player_event(ShotEventKind::ShotOnGoal, 5, None, Some(0)),
            ],
        );
        let season = trend(&view, PlayerScoringTrendWindow::SeasonLoaded);

        assert_eq!(season.bucket_counts.crease, 1);
        assert_eq!(season.bucket_counts.inside, 1);
        assert_eq!(season.bucket_counts.slot, 1);
        assert_eq!(season.bucket_counts.outside, 1);
        assert_eq!(season.bucket_counts.unknown, 1);
    }

    #[test]
    fn l0_player_scoring_trends_distinguish_missing_from_loaded_empty_source() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let missing = PlayerScoringProfileView::from_events(
            context.clone(),
            8483493,
            "Connor Bedard",
            vec![],
        );
        let loaded_empty = PlayerScoringProfileView::from_source_events(
            context,
            8483493,
            "Connor Bedard",
            true,
            vec![],
        );

        let missing_season = trend(&missing, PlayerScoringTrendWindow::SeasonLoaded);
        let loaded_season = trend(&loaded_empty, PlayerScoringTrendWindow::SeasonLoaded);
        assert!(!missing_season.source_loaded);
        assert!(loaded_season.source_loaded);
        assert_eq!(missing_season.events_loaded, 0);
        assert_eq!(loaded_season.events_loaded, 0);
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
