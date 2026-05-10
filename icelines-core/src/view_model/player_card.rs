use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::{SeasonStats, SeasonType};
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
    ViewWindow,
};
use crate::view_model::tokens::{
    MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerCardView {
    pub context: ViewContext,
    pub player_id: PlayerId,
    pub display_name: String,
    pub headshot_url: Option<String>,
    pub active: Option<PlayerSeasonSummary>,
    pub career: Vec<PlayerCareerSummary>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl PlayerCardView {
    pub fn from_repository(
        repo: &StatsRepository,
        player_id: PlayerId,
        season: Season,
        season_type: SeasonType,
    ) -> Option<Self> {
        let identity = repo.identity(player_id)?;
        let has_window = repo.has_window(season, season_type);
        let active_view = repo.view(player_id, season, season_type);
        let active = active_view.as_ref().map(player_season_summary);
        let mut career: Vec<PlayerCareerSummary> = repo
            .career_all(player_id)
            .map(|iter| {
                iter.filter(|stats| stats.totals.gp > 0)
                    .map(player_career_summary)
                    .collect()
            })
            .unwrap_or_default();
        career.sort_by(|a, b| {
            b.season
                .cmp(&a.season)
                .then(a.season_type.cmp(&b.season_type))
        });

        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        if !has_window {
            context.completeness = Completeness::Unavailable;
            context
                .source_state
                .push(SourceState::missing(SourceKind::Roster));
        }

        let empty_state = if active.is_none() {
            Some(EmptyState {
                kind: if has_window {
                    EmptyKind::NoRows
                } else {
                    EmptyKind::MissingSource
                },
                title: "No active-season row".to_string(),
                detail: Some("This player has no row in the requested season/type.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Some(Self {
            context,
            player_id,
            display_name: identity.full_name.clone(),
            headshot_url: identity.headshot_canonical_url.clone(),
            active,
            career,
            warnings: Vec::new(),
            empty_state,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSeasonSummary {
    pub season: Season,
    pub season_type: SeasonType,
    pub position: Position,
    pub team: TeamAbbr,
    pub team_display: String,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerCareerSummary {
    pub season: Season,
    pub season_type: SeasonType,
    pub team: TeamAbbr,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

fn player_season_summary(view: &PlayerView<'_>) -> PlayerSeasonSummary {
    PlayerSeasonSummary {
        season: view.season(),
        season_type: view.season_type(),
        position: view.position(),
        team: view
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        team_display: view.team_display().to_string(),
        metrics: skater_metrics(
            view.gp(),
            view.goals(),
            view.assists(),
            view.points(),
            if view.gp() > 0 {
                Some(view.points() as f64 / view.gp() as f64)
            } else {
                None
            },
        )
        .into_iter()
        .chain(active_detail_metrics(view))
        .collect(),
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn player_career_summary(stats: &SeasonStats) -> PlayerCareerSummary {
    let totals = &stats.totals;
    PlayerCareerSummary {
        season: stats.season,
        season_type: stats.season_type,
        team: stats
            .team_stints
            .last()
            .map(|stint| stint.team.clone())
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        metrics: skater_metrics(
            totals.gp,
            totals.goals,
            totals.assists,
            totals.points,
            if totals.gp > 0 {
                Some(totals.points as f64 / totals.gp as f64)
            } else {
                None
            },
        ),
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn skater_metrics(
    gp: u32,
    goals: u32,
    assists: u32,
    points: u32,
    points_per_game: Option<f64>,
) -> Vec<MetricCell> {
    vec![
        metric_int("gp", "GP", gp, MetricUnit::Games),
        metric_int("goals", "G", goals, MetricUnit::Goals),
        metric_int("assists", "A", assists, MetricUnit::Assists),
        metric_int("points", "PTS", points, MetricUnit::Points),
        MetricCell {
            key: StatKey::from("points_per_game"),
            label: "PPG".to_string(),
            value: points_per_game
                .map(MetricValue::Decimal)
                .unwrap_or(MetricValue::Missing),
            unit: MetricUnit::PerGame,
            precision: ValuePrecision::TwoDecimals,
            token: None,
        },
    ]
}

fn metric_int(key: &str, label: &str, value: u32, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: MetricValue::Integer(value as i64),
        unit,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn active_detail_metrics(view: &PlayerView<'_>) -> Vec<MetricCell> {
    let totals = &view.stats.totals;
    vec![
        metric_signed_int("plus_minus", "+/-", view.plus_minus(), MetricUnit::Count),
        metric_int("pim", "PIM", totals.pim, MetricUnit::Minutes),
        metric_int("shots", "SOG", totals.shots, MetricUnit::Count),
        metric_optional_decimal(
            "shooting_pct",
            "S%",
            totals.shooting_pct.map(f64::from),
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        metric_optional_int("hits", "Hits", view.hits(), MetricUnit::Count),
        metric_optional_int("blocks", "Blocks", view.blocked_shots(), MetricUnit::Count),
        metric_optional_int(
            "takeaways",
            "Takeaways",
            view.takeaways(),
            MetricUnit::Count,
        ),
        metric_optional_int(
            "giveaways",
            "Giveaways",
            view.giveaways(),
            MetricUnit::Count,
        ),
        metric_optional_decimal(
            "faceoff_win_pct",
            "FO%",
            totals.faceoff_win_pct.map(f64::from),
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        metric_int("pp_goals", "PPG", totals.pp_goals, MetricUnit::Goals),
        metric_int("pp_points", "PPP", totals.pp_points, MetricUnit::Points),
        metric_int("sh_goals", "SHG", totals.sh_goals, MetricUnit::Goals),
        metric_int("gwg", "GWG", totals.gwg, MetricUnit::Goals),
        metric_optional_int(
            "toi_per_game_sec",
            "TOI/GP",
            totals.toi_per_game_sec,
            MetricUnit::Seconds,
        ),
    ]
}

fn metric_signed_int(key: &str, label: &str, value: i32, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: MetricValue::Integer(value as i64),
        unit,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn metric_optional_int(key: &str, label: &str, value: Option<u32>, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(|value| MetricValue::Integer(value as i64))
            .unwrap_or(MetricValue::Missing),
        unit,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn metric_optional_decimal(
    key: &str,
    label: &str,
    value: Option<f64>,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(MetricValue::Decimal)
            .unwrap_or(MetricValue::Missing),
        unit,
        precision,
        token: None,
    }
}
