use serde::{Deserialize, Serialize};

use crate::career_history::{CareerStint, LeagueTier};
use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_catalog::{StatId, StatUnit};
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
    pub pre_nhl_career: Vec<PlayerPreNhlCareerRow>,
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
                    .map(|stats| {
                        let row_view = PlayerView {
                            identity,
                            stats,
                            contract: None,
                        };
                        player_career_summary(&row_view)
                    })
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
            pre_nhl_career: Vec::new(),
            warnings: Vec::new(),
            empty_state,
        })
    }

    pub fn with_pre_nhl_stints(mut self, stints: &[CareerStint]) -> Self {
        self.pre_nhl_career = Self::pre_nhl_rows(stints);
        self
    }

    pub fn pre_nhl_rows(stints: &[CareerStint]) -> Vec<PlayerPreNhlCareerRow> {
        let mut rows: Vec<PlayerPreNhlCareerRow> = stints.iter().map(pre_nhl_row).collect();
        rows.sort_by(|a, b| b.season.cmp(&a.season).then(a.sequence.cmp(&b.sequence)));
        rows
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
    pub catalog_metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerPreNhlCareerRow {
    pub season: Season,
    pub season_label: String,
    pub league: String,
    pub league_tier: String,
    pub team: String,
    pub sequence: u8,
    pub games: u32,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub points_per_game: Option<f32>,
}

fn pre_nhl_row(stint: &CareerStint) -> PlayerPreNhlCareerRow {
    PlayerPreNhlCareerRow {
        season: stint.season,
        season_label: pretty_season(stint.season),
        league: stint.league.0.clone(),
        league_tier: match stint.league.tier() {
            LeagueTier::Pro => "pro",
            LeagueTier::Junior => "junior",
            LeagueTier::College => "college",
            LeagueTier::International => "international",
            LeagueTier::Other => "other",
        }
        .to_string(),
        team: stint.team.clone(),
        sequence: stint.sequence,
        games: stint.gp,
        goals: stint.goals,
        assists: stint.assists,
        points: stint.points,
        points_per_game: stint.points_per_game(),
    }
}

fn pretty_season(season: Season) -> String {
    let value = season.0.to_string();
    if value.len() == 8 {
        format!("{}-{}", &value[2..4], &value[6..8])
    } else {
        value
    }
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

fn player_career_summary(view: &PlayerView<'_>) -> PlayerCareerSummary {
    let stats = view.stats;
    let totals = &view.stats.totals;
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
        catalog_metrics: catalog_metrics(view),
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn catalog_metrics(view: &PlayerView<'_>) -> Vec<MetricCell> {
    StatId::all()
        .iter()
        .map(|sid| {
            let value = sid
                .read(view)
                .map(metric_value_for_stat_unit(sid.unit()))
                .unwrap_or(MetricValue::Missing);
            MetricCell {
                key: StatKey::from(sid.cli_key()),
                label: sid.short_label().to_string(),
                value,
                unit: metric_unit_for_stat_unit(sid.unit()),
                precision: precision_for_stat_unit(sid.unit()),
                token: None,
            }
        })
        .collect()
}

fn metric_value_for_stat_unit(unit: StatUnit) -> impl Fn(f64) -> MetricValue {
    move |value| match unit {
        StatUnit::Count | StatUnit::Seconds => MetricValue::Integer(value as i64),
        StatUnit::Pct | StatUnit::Per60 | StatUnit::Rate | StatUnit::Inverted => {
            MetricValue::Decimal(value)
        }
    }
}

fn metric_unit_for_stat_unit(unit: StatUnit) -> MetricUnit {
    match unit {
        StatUnit::Count => MetricUnit::Count,
        StatUnit::Pct => MetricUnit::Percentage,
        StatUnit::Per60 => MetricUnit::PerGame,
        StatUnit::Seconds => MetricUnit::Seconds,
        StatUnit::Rate | StatUnit::Inverted => MetricUnit::Score,
    }
}

fn precision_for_stat_unit(unit: StatUnit) -> ValuePrecision {
    match unit {
        StatUnit::Count | StatUnit::Seconds => ValuePrecision::Integer,
        StatUnit::Pct => ValuePrecision::PercentOneDecimal,
        StatUnit::Per60 | StatUnit::Rate | StatUnit::Inverted => ValuePrecision::TwoDecimals,
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
