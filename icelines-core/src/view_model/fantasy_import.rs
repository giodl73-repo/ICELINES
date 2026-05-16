use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::Season;
use crate::season_stats::SeasonType;
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
    ViewWindow,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyImportView {
    pub context: ViewContext,
    pub league: String,
    pub mode: FantasyImportMode,
    pub mode_label: String,
    pub summary: FantasyImportSummary,
    pub teams: Vec<FantasyImportTeamRow>,
    pub rows: Vec<FantasyImportPlayerRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyImportSummary {
    pub teams_seen: usize,
    pub teams_created: usize,
    pub teams_updated: usize,
    pub teams_unchanged: usize,
    pub teams_error: usize,
    pub players_seen: usize,
    pub players_imported: usize,
    pub players_skipped: usize,
    pub players_unresolved: usize,
    pub players_duplicate: usize,
    pub players_error: usize,
    pub diagnostic_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyImportTeamRow {
    pub rank: usize,
    pub team: String,
    pub owner: Option<String>,
    pub is_user_team: bool,
    pub status: FantasyImportTeamStatus,
    pub imported_players: u16,
    pub skipped_rows: u16,
    pub error_rows: u16,
    pub rostered_players_after: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyImportPlayerRow {
    pub row_number: u32,
    pub player_name: String,
    pub normalized_name: Option<String>,
    pub fantasy_team: Option<String>,
    pub owner: Option<String>,
    pub nhl_team_hint: Option<String>,
    pub position_hint: Option<String>,
    pub status: FantasyImportRowStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyImportTeamInput {
    pub team: String,
    pub owner: Option<String>,
    pub is_user_team: bool,
    pub status: FantasyImportTeamStatus,
    pub rostered_players_after: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyImportRowInput {
    pub row_number: u32,
    pub player_name: String,
    pub normalized_name: Option<String>,
    pub fantasy_team: Option<String>,
    pub owner: Option<String>,
    pub nhl_team_hint: Option<String>,
    pub position_hint: Option<String>,
    pub status: FantasyImportRowStatus,
    pub message: Option<String>,
}

pub struct FantasyImportViewInput {
    pub season: Season,
    pub season_type: SeasonType,
    pub league: String,
    pub mode: FantasyImportMode,
    pub teams: Vec<FantasyImportTeamInput>,
    pub rows: Vec<FantasyImportRowInput>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyImportMode {
    DryRun,
    Apply,
}

impl FantasyImportMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyImportTeamStatus {
    Created,
    Updated,
    Unchanged,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyImportRowStatus {
    Imported,
    Skipped,
    Unresolved,
    Duplicate,
    Error,
}

impl FantasyImportView {
    pub fn from_input(input: FantasyImportViewInput) -> Self {
        let mut source_state = input.source_state;
        if source_state.is_empty() {
            source_state.push(SourceState::complete(SourceKind::FantasyImport));
        }

        let mut rows = input
            .rows
            .into_iter()
            .map(|row| FantasyImportPlayerRow {
                row_number: row.row_number,
                player_name: row.player_name,
                normalized_name: row.normalized_name,
                fantasy_team: row.fantasy_team,
                owner: row.owner,
                nhl_team_hint: row.nhl_team_hint,
                position_hint: row.position_hint,
                status: row.status,
                message: row.message,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            a.row_number
                .cmp(&b.row_number)
                .then_with(|| a.player_name.cmp(&b.player_name))
                .then_with(|| a.fantasy_team.cmp(&b.fantasy_team))
        });

        let row_counts = row_counts_by_team(&rows);
        let mut teams = input
            .teams
            .into_iter()
            .map(|team| {
                let counts = row_counts.get(&team.team);
                FantasyImportTeamRow {
                    rank: 0,
                    team: team.team,
                    owner: team.owner,
                    is_user_team: team.is_user_team,
                    status: team.status,
                    imported_players: counts.map(|counts| counts.imported).unwrap_or_default(),
                    skipped_rows: counts.map(|counts| counts.skipped).unwrap_or_default(),
                    error_rows: counts.map(|counts| counts.error).unwrap_or_default(),
                    rostered_players_after: team.rostered_players_after,
                }
            })
            .collect::<Vec<_>>();
        teams.sort_by(|a, b| a.team.cmp(&b.team));
        for (idx, team) in teams.iter_mut().enumerate() {
            team.rank = idx + 1;
        }

        let summary = FantasyImportSummary::from_rows(&teams, &rows);
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No fantasy roster import rows".to_string(),
                detail: Some(
                    "Provide a Yahoo roster CSV with player and fantasy-team columns.".to_string(),
                ),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        let mut context = ViewContext::new(ViewWindow::new(input.season, input.season_type));
        context.source_state = source_state.clone();
        context.completeness = if empty_state.is_some() {
            Completeness::Unavailable
        } else if summary.diagnostic_rows > 0 || !input.warnings.is_empty() {
            Completeness::Partial
        } else {
            Completeness::Complete
        };

        Self {
            context,
            league: input.league,
            mode: input.mode,
            mode_label: input.mode.label().to_string(),
            summary,
            teams,
            rows,
            source_state,
            warnings: input.warnings,
            empty_state,
        }
    }
}

impl FantasyImportSummary {
    fn from_rows(teams: &[FantasyImportTeamRow], rows: &[FantasyImportPlayerRow]) -> Self {
        Self {
            teams_seen: teams.len(),
            teams_created: teams
                .iter()
                .filter(|team| team.status == FantasyImportTeamStatus::Created)
                .count(),
            teams_updated: teams
                .iter()
                .filter(|team| team.status == FantasyImportTeamStatus::Updated)
                .count(),
            teams_unchanged: teams
                .iter()
                .filter(|team| team.status == FantasyImportTeamStatus::Unchanged)
                .count(),
            teams_error: teams
                .iter()
                .filter(|team| team.status == FantasyImportTeamStatus::Error)
                .count(),
            players_seen: rows.len(),
            players_imported: rows
                .iter()
                .filter(|row| row.status == FantasyImportRowStatus::Imported)
                .count(),
            players_skipped: rows
                .iter()
                .filter(|row| row.status == FantasyImportRowStatus::Skipped)
                .count(),
            players_unresolved: rows
                .iter()
                .filter(|row| row.status == FantasyImportRowStatus::Unresolved)
                .count(),
            players_duplicate: rows
                .iter()
                .filter(|row| row.status == FantasyImportRowStatus::Duplicate)
                .count(),
            players_error: rows
                .iter()
                .filter(|row| row.status == FantasyImportRowStatus::Error)
                .count(),
            diagnostic_rows: rows
                .iter()
                .filter(|row| row.status != FantasyImportRowStatus::Imported)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TeamRowCounts {
    imported: u16,
    skipped: u16,
    error: u16,
}

fn row_counts_by_team(rows: &[FantasyImportPlayerRow]) -> BTreeMap<String, TeamRowCounts> {
    let mut counts = BTreeMap::<String, TeamRowCounts>::new();
    for row in rows {
        let Some(team) = row.fantasy_team.as_ref() else {
            continue;
        };
        let entry = counts.entry(team.clone()).or_default();
        match row.status {
            FantasyImportRowStatus::Imported => {
                entry.imported = entry.imported.saturating_add(1);
            }
            FantasyImportRowStatus::Error => {
                entry.error = entry.error.saturating_add(1);
            }
            FantasyImportRowStatus::Skipped
            | FantasyImportRowStatus::Unresolved
            | FantasyImportRowStatus::Duplicate => {
                entry.skipped = entry.skipped.saturating_add(1);
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::context::WarningKind;

    fn team(name: &str, status: FantasyImportTeamStatus) -> FantasyImportTeamInput {
        FantasyImportTeamInput {
            team: name.to_string(),
            owner: Some(format!("{name} Owner")),
            is_user_team: name == "Alpha",
            status,
            rostered_players_after: Some(2),
        }
    }

    fn row(
        row_number: u32,
        player: &str,
        team: Option<&str>,
        status: FantasyImportRowStatus,
    ) -> FantasyImportRowInput {
        FantasyImportRowInput {
            row_number,
            player_name: player.to_string(),
            normalized_name: Some(crate::name::normalize_name(player)),
            fantasy_team: team.map(str::to_string),
            owner: team.map(|team| format!("{team} Owner")),
            nhl_team_hint: Some("EDM".to_string()),
            position_hint: Some("C".to_string()),
            status,
            message: (status != FantasyImportRowStatus::Imported)
                .then(|| format!("{status:?} test diagnostic")),
        }
    }

    fn input(
        teams: Vec<FantasyImportTeamInput>,
        rows: Vec<FantasyImportRowInput>,
    ) -> FantasyImportViewInput {
        FantasyImportViewInput {
            season: Season(20252026),
            season_type: SeasonType::Regular,
            league: "Office League".to_string(),
            mode: FantasyImportMode::DryRun,
            teams,
            rows,
            source_state: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn l0_fantasy_import_summary_counts_row_and_team_statuses() {
        let view = FantasyImportView::from_input(input(
            vec![
                team("Bravo", FantasyImportTeamStatus::Updated),
                team("Alpha", FantasyImportTeamStatus::Created),
                team("Charlie", FantasyImportTeamStatus::Unchanged),
            ],
            vec![
                row(3, "Skip Me", Some("Alpha"), FantasyImportRowStatus::Skipped),
                row(
                    1,
                    "Connor McDavid",
                    Some("Alpha"),
                    FantasyImportRowStatus::Imported,
                ),
                row(
                    2,
                    "Duplicate Player",
                    Some("Bravo"),
                    FantasyImportRowStatus::Duplicate,
                ),
                row(
                    4,
                    "Missing Player",
                    Some("Charlie"),
                    FantasyImportRowStatus::Unresolved,
                ),
                row(5, "Bad Row", None, FantasyImportRowStatus::Error),
            ],
        ));

        assert_eq!(view.summary.teams_seen, 3);
        assert_eq!(view.summary.teams_created, 1);
        assert_eq!(view.summary.teams_updated, 1);
        assert_eq!(view.summary.teams_unchanged, 1);
        assert_eq!(view.summary.players_seen, 5);
        assert_eq!(view.summary.players_imported, 1);
        assert_eq!(view.summary.players_skipped, 1);
        assert_eq!(view.summary.players_duplicate, 1);
        assert_eq!(view.summary.players_unresolved, 1);
        assert_eq!(view.summary.players_error, 1);
        assert_eq!(view.summary.diagnostic_rows, 4);
        assert_eq!(view.context.completeness, Completeness::Partial);
    }

    #[test]
    fn l0_fantasy_import_orders_teams_by_name_and_rows_by_csv_row_number() {
        let view = FantasyImportView::from_input(input(
            vec![
                team("Zulu", FantasyImportTeamStatus::Updated),
                team("Alpha", FantasyImportTeamStatus::Created),
            ],
            vec![
                row(
                    20,
                    "Late Player",
                    Some("Zulu"),
                    FantasyImportRowStatus::Imported,
                ),
                row(
                    10,
                    "Early Player",
                    Some("Alpha"),
                    FantasyImportRowStatus::Imported,
                ),
            ],
        ));

        assert_eq!(
            view.teams
                .iter()
                .map(|team| (team.rank, team.team.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "Alpha"), (2, "Zulu")]
        );
        assert_eq!(
            view.rows
                .iter()
                .map(|row| row.player_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Early Player", "Late Player"]
        );
    }

    #[test]
    fn l0_fantasy_import_team_summaries_count_imported_skipped_and_error_rows() {
        let view = FantasyImportView::from_input(input(
            vec![team("Alpha", FantasyImportTeamStatus::Created)],
            vec![
                row(
                    1,
                    "Imported",
                    Some("Alpha"),
                    FantasyImportRowStatus::Imported,
                ),
                row(2, "Skipped", Some("Alpha"), FantasyImportRowStatus::Skipped),
                row(
                    3,
                    "Unresolved",
                    Some("Alpha"),
                    FantasyImportRowStatus::Unresolved,
                ),
                row(4, "Error", Some("Alpha"), FantasyImportRowStatus::Error),
            ],
        ));

        let alpha = &view.teams[0];
        assert_eq!(alpha.imported_players, 1);
        assert_eq!(alpha.skipped_rows, 2);
        assert_eq!(alpha.error_rows, 1);
    }

    #[test]
    fn l0_fantasy_import_mode_labels_are_stable_for_surfaces() {
        assert_eq!(FantasyImportMode::DryRun.label(), "dry-run");
        assert_eq!(FantasyImportMode::Apply.label(), "apply");

        let mut apply_input = input(
            vec![team("Alpha", FantasyImportTeamStatus::Updated)],
            vec![row(
                1,
                "Connor McDavid",
                Some("Alpha"),
                FantasyImportRowStatus::Imported,
            )],
        );
        apply_input.mode = FantasyImportMode::Apply;
        let view = FantasyImportView::from_input(apply_input);

        assert_eq!(view.mode, FantasyImportMode::Apply);
        assert_eq!(view.mode_label, "apply");
        assert_eq!(view.context.completeness, Completeness::Complete);
    }

    #[test]
    fn l0_fantasy_import_empty_rows_surface_empty_state() {
        let view = FantasyImportView::from_input(input(Vec::new(), Vec::new()));

        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(
            view.empty_state.as_ref().map(|empty| empty.kind),
            Some(EmptyKind::NoRows)
        );
        assert!(view
            .source_state
            .iter()
            .any(|source| source.source == SourceKind::FantasyImport
                && source.state == Completeness::Complete));
    }

    #[test]
    fn l0_fantasy_import_warnings_make_context_partial_without_row_errors() {
        let mut import_input = input(
            vec![team("Alpha", FantasyImportTeamStatus::Updated)],
            vec![row(
                1,
                "Connor McDavid",
                Some("Alpha"),
                FantasyImportRowStatus::Imported,
            )],
        );
        import_input.warnings.push(ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::FantasyImport),
            message: "eligibility column was not present".to_string(),
            recovery: Vec::new(),
        });

        let view = FantasyImportView::from_input(import_input);

        assert_eq!(view.context.completeness, Completeness::Partial);
        assert_eq!(view.warnings.len(), 1);
        assert_eq!(view.summary.diagnostic_rows, 0);
    }
}
