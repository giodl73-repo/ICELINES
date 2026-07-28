use crate::config::Config;
use anyhow::Context;
use chrono::{DateTime, Utc};
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_cap_projection, build_team_ceiling, build_team_lineup_projection,
    build_team_prognosis_card, team_ceiling_player_lens_score, CapProjectionContractInput,
    CapProjectionPlayerInput, CapProjectionView, Completeness, LineupAssignmentEvidence,
    SalaryBasis, SourceKind, SourceState, TeamCeilingLens, TeamCeilingPlayerInput, TeamCeilingView,
    TeamLineupPlayerInput, TeamLineupPlayerView, TeamLineupProjectionView, TeamPrognosisCardInput,
    ViewContext, ViewWindow, CANONICAL_TEAMS,
};
use icelines_fetch::schema::{RosterPlayer, RosterResponse};
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::snapshot::SnapshotTier;
use icelines_fetch::stats_loader::load_into_repo;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct ReportCatalogEntry {
    name: &'static str,
    status: &'static str,
    canonical: &'static str,
    formats: &'static str,
    screens: &'static str,
    notes: &'static str,
}

const REPORT_CATALOG: &[ReportCatalogEntry] = &[
    ReportCatalogEntry {
        name: "team-card",
        status: "available",
        canonical: "icelines report team-card --team NYR --scenario-id nyr-development-variance [--json]",
        formats: "text,json",
        screens: "CLI, web/TUI canonical source document",
        notes: "Two-page Depth Chart and Insider prognosis card with paired isolated impacts.",
    },
    ReportCatalogEntry {
        name: "team-lineup",
        status: "available",
        canonical: "icelines report team-lineup --team NYR [--json]",
        formats: "text,json",
        screens: "CLI, card/web/TUI source document",
        notes: "Four lines, three defense pairs, goalies, extras, portraits, and one IceLines display score.",
    },
    ReportCatalogEntry {
        name: "team-ceiling",
        status: "available",
        canonical: "icelines report team-ceiling [--team NYR] [--json]",
        formats: "text,json",
        screens: "CLI durable report",
        notes: "2026-27 roster ceiling, multi-lens player ratings, and year-over-year delta.",
    },
    ReportCatalogEntry {
        name: "cap-forecast",
        status: "available",
        canonical: "icelines report cap-forecast [--team NYR] [--json]",
        formats: "text,json",
        screens: "CLI durable report",
        notes: "Five-year current-roster market-cost scenario with confirmed/modelled values.",
    },
    ReportCatalogEntry {
        name: "leaderboards",
        status: "available",
        canonical: "icelines query leaders | icelines x leaders | icelines export md leaders",
        formats: "table,json,csv,markdown",
        screens: "TUI Stats, web /leaders",
        notes: "Use query for filters; x for quick CSV/JSON; export md for durable docs.",
    },
    ReportCatalogEntry {
        name: "goalies",
        status: "available",
        canonical: "icelines query goalies | icelines x goalies",
        formats: "table,json,csv",
        screens: "TUI Goalies",
        notes: "Goalie filters route through query goalies.",
    },
    ReportCatalogEntry {
        name: "player",
        status: "available",
        canonical: "icelines query player <name> | icelines history <name> | icelines x history",
        formats: "table,json,csv",
        screens: "TUI/Web player card",
        notes: "Player card is the screen; history is the exportable season log.",
    },
    ReportCatalogEntry {
        name: "compare",
        status: "available",
        canonical:
            "icelines query compare <a> <b> | icelines x compare | icelines export md compare",
        formats: "table,json,csv,markdown",
        screens: "TUI handoff, web compare",
        notes: "Use query compare for interactive output; export md for report packets.",
    },
    ReportCatalogEntry {
        name: "team",
        status: "available",
        canonical: "icelines team <ABBR> | icelines export md team",
        formats: "table,markdown",
        screens: "TUI Team/Depth, web team",
        notes: "Team depth remains the roster/depth view.",
    },
    ReportCatalogEntry {
        name: "team-season",
        status: "available",
        canonical: "icelines team-season <ABBR> | icelines export md team-season",
        formats: "table,json,markdown",
        screens: "TUI/Web team season",
        notes: "Season record, splits, form, remaining schedule, and opponent context.",
    },
    ReportCatalogEntry {
        name: "fantasy-poach",
        status: "available",
        canonical: "icelines poach | icelines report poach | icelines export md fantasy",
        formats: "table,json,markdown",
        screens: "TUI Poach, web /poach",
        notes: "Report poach emits a durable PoachReportView document.",
    },
    ReportCatalogEntry {
        name: "weekly-fantasy",
        status: "available",
        canonical: "icelines report weekly",
        formats: "markdown,json",
        screens: "web /reports/weekly",
        notes: "Weekly prep report over the same poach ViewModel plus watch context.",
    },
    ReportCatalogEntry {
        name: "draft-class",
        status: "available",
        canonical: "icelines class <year> | icelines x class",
        formats: "table,json,csv",
        screens: "TUI command handoff",
        notes: "Draft-year cohort ranking.",
    },
    ReportCatalogEntry {
        name: "peers",
        status: "available",
        canonical: "icelines peers <name> | icelines x peers",
        formats: "table,json,csv",
        screens: "TUI command handoff",
        notes: "Statistical similarity cohort.",
    },
    ReportCatalogEntry {
        name: "transactions",
        status: "available",
        canonical: "icelines transactions | icelines x transactions",
        formats: "table,json,csv",
        screens: "TUI Transactions, web /transactions",
        notes: "League/team/player transaction feed.",
    },
    ReportCatalogEntry {
        name: "records",
        status: "available",
        canonical: "icelines records player <name> | icelines records team <ABBR>",
        formats: "table,json,csv",
        screens: "future Player Records / Team Records",
        notes:
            "Available: teams/goalies scored against plus fight opponents from cached play-by-play.",
    },
    ReportCatalogEntry {
        name: "stathead-packs",
        status: "available",
        canonical:
            "icelines stathead | icelines stathead --markdown | icelines stathead --commands --read-only | icelines stathead --commands --writes-only",
        formats: "text,json,markdown,commands",
        screens: "CLI docs/report discovery",
        notes: "Curated editorial query recipes; use --commands --read-only or --writes-only to filter by command effect.",
    },
];

#[derive(Debug, Clone)]
pub struct CapForecastArgs {
    pub season: String,
    pub years: u8,
    pub growth_pct: f64,
    pub team: Option<String>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct TeamCeilingArgs {
    pub roster_season: String,
    pub stats_season: String,
    pub team: Option<String>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct TeamLineupArgs {
    pub roster_season: String,
    pub stats_season: String,
    pub team: String,
    pub json: bool,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct TeamCardArgs {
    pub roster_season: String,
    pub stats_season: String,
    pub team: String,
    pub scenario_id: String,
    pub scenario_comparison_key: Option<String>,
    pub trials: u32,
    pub seed: u64,
    pub generated_at: Option<String>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub async fn run_team_card(args: TeamCardArgs) -> anyhow::Result<()> {
    let roster_season: Season = args.roster_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid roster season '{}': {error}", args.roster_season)
    })?;
    let stats_season: Season = args.stats_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid stats season '{}': {error}", args.stats_season)
    })?;
    let team = TeamAbbr::parse(&args.team).map_err(|error| anyhow::anyhow!(error))?;
    let lineup = load_team_lineup_view(roster_season, stats_season, &team)?;
    let generated_at = args
        .generated_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("--generated-at must be RFC 3339, for example 2026-07-22T12:00:00Z")?
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let season_args = super::icecast::IceCastSeasonArgs {
        season: roster_season.0,
        stats_season: stats_season.0,
        teams: vec![team.as_str().to_string()],
        trials: args.trials,
        seed: args.seed,
        scenario: None,
        scenario_id: Some(args.scenario_id.clone()),
        isolated_impacts: true,
        auto_personnel: false,
        trade_mode: "off".to_string(),
        replay_mode: "frozen".to_string(),
        ignore_replay_personnel_after: None,
        through: None,
        retrospective_opening_lineups: false,
        all_games: false,
        refresh: false,
        json: true,
        out: None,
        game_forecast_out: None,
    };
    let (mut forecast, _, calendar_fingerprint) =
        super::icecast::build_season_view(&season_args).await?;
    let isolated_impact = forecast
        .isolated_impact
        .take()
        .context("IceCast did not return requested isolated impacts")?;
    let mut view = ViewContext::new(ViewWindow::new(roster_season, SeasonType::Regular));
    view.generated_at = Some(generated_at);
    view.completeness = if lineup.warnings.is_empty() {
        Completeness::Complete
    } else {
        Completeness::Partial
    };
    view.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::complete(SourceKind::Schedule),
        SourceState::complete(SourceKind::Snapshot),
    ];
    let card = build_team_prognosis_card(TeamPrognosisCardInput {
        team_name: team_display_name(team.as_str()).to_string(),
        team_abbreviation: team.as_str().to_string(),
        lineup,
        forecast,
        isolated_impact,
        view,
        evidence_at: Some(generated_at),
        roster_snapshot_id: None,
        calendar_fingerprint: Some(calendar_fingerprint),
        scenario_id: Some(args.scenario_id),
        scenario_comparison_key: args.scenario_comparison_key,
        event_projections: Vec::new(),
    })
    .map_err(anyhow::Error::new)?;
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&card)?)
    } else {
        super::card_renderer::render_team_card(&card)
    };
    emit_report(&output, args.out.as_ref())
}

pub fn run_team_lineup(args: TeamLineupArgs) -> anyhow::Result<()> {
    let roster_season: Season = args.roster_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid roster season '{}': {error}", args.roster_season)
    })?;
    let stats_season: Season = args.stats_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid stats season '{}': {error}", args.stats_season)
    })?;
    let team = TeamAbbr::parse(&args.team).map_err(|error| anyhow::anyhow!(error))?;
    let view = load_team_lineup_view(roster_season, stats_season, &team)?;
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_team_lineup(&view)
    };
    emit_report(&output, args.out.as_ref())
}

pub(crate) fn load_team_lineup_view(
    roster_season: Season,
    stats_season: Season,
    team: &TeamAbbr,
) -> anyhow::Result<TeamLineupProjectionView> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(stats_season, SeasonType::Regular, &store).map_err(|error| {
        anyhow::anyhow!(
            "{error}\n  Try: icelines fetch all --season {}",
            stats_season.0
        )
    })?;
    load_team_lineup_view_from_store(roster_season, stats_season, team, &store, &outcome.repo)
}

pub(crate) fn load_league_team_lineup_views(
    roster_season: Season,
    stats_season: Season,
) -> anyhow::Result<Vec<TeamLineupProjectionView>> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(stats_season, SeasonType::Regular, &store).map_err(|error| {
        anyhow::anyhow!(
            "{error}\n  Try: icelines fetch all --season {}",
            stats_season.0
        )
    })?;
    CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| {
            let team = TeamAbbr::parse(team).map_err(|error| anyhow::anyhow!(error))?;
            load_team_lineup_view_from_store(
                roster_season,
                stats_season,
                &team,
                &store,
                &outcome.repo,
            )
        })
        .collect()
}

fn load_team_lineup_view_from_store(
    roster_season: Season,
    stats_season: Season,
    team: &TeamAbbr,
    store: &SnapshotStore,
    repo: &icelines_core::stats_repository::StatsRepository,
) -> anyhow::Result<TeamLineupProjectionView> {
    let roster = store
        .read_tier_file_any_for_season::<RosterResponse>(
            &SnapshotTier::Rosters,
            &format!("{}.json", team.as_str()),
            &roster_season.as_str(),
        )
        .with_context(|| {
            format!(
                "reading {} roster for {} — run `icelines fetch rosters --season {} --refresh`",
                roster_season.0,
                team.as_str(),
                roster_season.0
            )
        })?;
    let players = roster
        .forwards
        .iter()
        .chain(&roster.defensemen)
        .chain(&roster.goalies)
        .filter_map(|player| lineup_input(player, team.as_str(), roster_season, stats_season, repo))
        .collect();
    build_team_lineup_projection(team.as_str(), roster_season.0, players)
        .map_err(anyhow::Error::new)
}

fn lineup_input(
    player: &RosterPlayer,
    team: &str,
    roster_season: Season,
    stats_season: Season,
    repo: &icelines_core::stats_repository::StatsRepository,
) -> Option<TeamLineupPlayerInput> {
    let ceiling = roster_input(player, team, roster_season, stats_season, repo)?;
    let deployment = repo
        .view(
            icelines_core::identity::PlayerId(player.id),
            stats_season,
            SeasonType::Regular,
        )
        .and_then(|view| {
            view.stats.time_on_ice.as_ref().map(|time| {
                (
                    time.pp_time_on_ice_per_game_sec as f64,
                    time.sh_time_on_ice_per_game_sec as f64,
                )
            })
        });
    Some(TeamLineupPlayerInput {
        player_id: ceiling.player_id,
        display_name: ceiling.player.clone(),
        team: ceiling.team.clone(),
        prior_team: ceiling.prior_team.clone(),
        primary_position: ceiling.position,
        eligible_positions: vec![ceiling.position],
        headshot_canonical_url: player.headshot.clone(),
        games_played: ceiling.games_played,
        lens_scores: TeamCeilingLens::ALL
            .into_iter()
            .map(|lens| (lens, team_ceiling_player_lens_score(&ceiling, lens)))
            .collect(),
        score_evidence: icelines_core::EvidenceLabel::Confirmed,
        power_play_role_score: deployment.map(|value| value.0),
        penalty_kill_role_score: deployment.map(|value| value.1),
        special_teams_evidence: deployment.map(|_| icelines_core::EvidenceLabel::Confirmed),
        requested_slot: None,
        assignment_evidence: LineupAssignmentEvidence::Estimated,
    })
}

pub fn run_team_ceiling(args: TeamCeilingArgs) -> anyhow::Result<()> {
    let roster_season: Season = args.roster_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid roster season '{}': {error}", args.roster_season)
    })?;
    let stats_season: Season = args.stats_season.parse().map_err(|error| {
        anyhow::anyhow!("invalid stats season '{}': {error}", args.stats_season)
    })?;
    let selected_team = args
        .team
        .as_deref()
        .map(TeamAbbr::parse)
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?;
    let mut view = load_team_ceiling_view(roster_season, stats_season)?;
    if let Some(team) = selected_team.as_ref() {
        view.teams.retain(|row| row.team == team.as_str());
        if view.teams.is_empty() {
            anyhow::bail!("team {} is absent from the ceiling report", team.as_str());
        }
    }
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_team_ceiling(&view)
    };
    emit_report(&output, args.out.as_ref())
}

pub(crate) fn load_team_ceiling_view(
    roster_season: Season,
    stats_season: Season,
) -> anyhow::Result<TeamCeilingView> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(stats_season, SeasonType::Regular, &store).map_err(|error| {
        anyhow::anyhow!(
            "{error}\n  Try: icelines fetch all --season {}",
            stats_season.0
        )
    })?;
    let mut current = Vec::new();
    for team in TeamAbbr::all() {
        let current_roster = store
            .read_tier_file_any_for_season::<RosterResponse>(
                &SnapshotTier::Rosters,
                &format!("{}.json", team.as_str()),
                &roster_season.as_str(),
            )
            .with_context(|| {
                format!(
                    "reading {} roster for {} — run `icelines fetch rosters --season {} --refresh`",
                    roster_season.0,
                    team.as_str(),
                    roster_season.0
                )
            })?;
        current.extend(roster_inputs(
            &current_roster,
            team.as_str(),
            roster_season,
            stats_season,
            &outcome.repo,
        ));
    }
    let previous = prior_season_roster_inputs(&outcome.repo, stats_season);
    Ok(build_team_ceiling(
        current,
        previous,
        roster_season.0,
        stats_season.0,
    )?)
}

fn roster_inputs(
    roster: &RosterResponse,
    team: &str,
    age_season: Season,
    stats_season: Season,
    repo: &icelines_core::stats_repository::StatsRepository,
) -> Vec<TeamCeilingPlayerInput> {
    roster
        .forwards
        .iter()
        .chain(&roster.defensemen)
        .chain(&roster.goalies)
        .filter_map(|player| roster_input(player, team, age_season, stats_season, repo))
        .collect()
}

fn roster_input(
    player: &RosterPlayer,
    team: &str,
    age_season: Season,
    stats_season: Season,
    repo: &icelines_core::stats_repository::StatsRepository,
) -> Option<TeamCeilingPlayerInput> {
    let position = icelines_core::model::Position::from_api_code(&player.position_code)?;
    let view = repo.view(
        icelines_core::identity::PlayerId(player.id),
        stats_season,
        SeasonType::Regular,
    );
    let gp = view.as_ref().map_or(0, |value| value.gp());
    let fantasy_per_82 = view.as_ref().and_then(|value| {
        (value.gp() > 0).then(|| {
            icelines_core::cross_team::fantasy_score_view(value) / value.gp() as f64 * 82.0
        })
    });
    let goalie_quality = view.as_ref().and_then(|value| {
        value.stats.goalie.as_ref().and_then(|goalie| {
            goalie.save_pct.map(|save_pct| {
                (50.0 + (save_pct as f64 - 0.900) * 1000.0 + value.gp().min(50) as f64 * 0.25)
                    .clamp(0.0, 100.0)
            })
        })
    });
    Some(TeamCeilingPlayerInput {
        player_id: player.id,
        player: format!(
            "{} {}",
            player.first_name.as_str(),
            player.last_name.as_str()
        ),
        team: team.to_owned(),
        prior_team: view
            .as_ref()
            .and_then(|value| value.team())
            .and_then(|value| canonical_last_team(value.as_str())),
        position,
        age: age_at_season_start(player.birth_date.as_deref(), age_season),
        games_played: gp,
        points_per_82: view.as_ref().and_then(|value| value.pace_82()),
        goals_per_82: view.as_ref().and_then(|value| value.goals_per_82()),
        shots_per_82: view.as_ref().and_then(|value| value.shots_per_82()),
        fantasy_per_82,
        goalie_quality,
    })
}

fn prior_season_roster_inputs(
    repo: &icelines_core::stats_repository::StatsRepository,
    stats_season: Season,
) -> Vec<TeamCeilingPlayerInput> {
    let mut by_team: std::collections::BTreeMap<String, Vec<TeamCeilingPlayerInput>> =
        std::collections::BTreeMap::new();
    for view in repo.league(stats_season, SeasonType::Regular) {
        let Some(team) = view
            .team()
            .and_then(|value| canonical_last_team(value.as_str()))
        else {
            continue;
        };
        by_team
            .entry(team.clone())
            .or_default()
            .push(view_roster_input(view, &team, stats_season));
    }

    let mut selected = Vec::new();
    for mut players in by_team.into_values() {
        players.sort_by(|a, b| {
            b.games_played
                .cmp(&a.games_played)
                .then_with(|| {
                    b.points_per_82
                        .unwrap_or(0.0)
                        .partial_cmp(&a.points_per_82.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.player.cmp(&b.player))
        });
        let mut forwards = players
            .iter()
            .filter(|player| player.position.is_forward())
            .take(14)
            .cloned()
            .collect::<Vec<_>>();
        let mut defense = players
            .iter()
            .filter(|player| player.position.is_defense())
            .take(7)
            .cloned()
            .collect::<Vec<_>>();
        let mut goalies = players
            .iter()
            .filter(|player| player.position == icelines_core::model::Position::Goalie)
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        selected.append(&mut forwards);
        selected.append(&mut defense);
        selected.append(&mut goalies);
    }
    selected
}

fn canonical_last_team(value: &str) -> Option<String> {
    let team = value
        .rsplit('/')
        .find(|part| !part.trim().is_empty())?
        .trim()
        .to_ascii_uppercase();
    TeamAbbr::parse(&team)
        .ok()
        .map(|team| team.as_str().to_owned())
}

fn view_roster_input(
    view: icelines_core::stats_repository::PlayerView<'_>,
    team: &str,
    age_season: Season,
) -> TeamCeilingPlayerInput {
    let fantasy_per_82 = (view.gp() > 0)
        .then(|| icelines_core::cross_team::fantasy_score_view(&view) / view.gp() as f64 * 82.0);
    let goalie_quality = view.stats.goalie.as_ref().and_then(|goalie| {
        goalie.save_pct.map(|save_pct| {
            (50.0 + (save_pct as f64 - 0.900) * 1000.0 + view.gp().min(50) as f64 * 0.25)
                .clamp(0.0, 100.0)
        })
    });
    TeamCeilingPlayerInput {
        player_id: view.id().0,
        player: view.full_name().to_owned(),
        team: team.to_owned(),
        prior_team: Some(team.to_owned()),
        position: view.position(),
        age: age_at_season_start(view.identity.bio.birth_date.as_deref(), age_season),
        games_played: view.gp(),
        points_per_82: view.pace_82(),
        goals_per_82: view.goals_per_82(),
        shots_per_82: view.shots_per_82(),
        fantasy_per_82,
        goalie_quality,
    }
}

fn render_team_lineup(view: &TeamLineupProjectionView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} PROJECTED LINEUP", view.team);
    let _ = writeln!(
        out,
        "Roster {} · Score {} ({})",
        view.roster_season, view.score_schema, view.score_method
    );
    for line in &view.forward_lines {
        let _ = writeln!(
            out,
            "L{}  {:<28} {:<28} {:<28}",
            line.line,
            lineup_player_text(line.left_wing.as_ref()),
            lineup_player_text(line.center.as_ref()),
            lineup_player_text(line.right_wing.as_ref())
        );
    }
    for pair in &view.defense_pairs {
        let _ = writeln!(
            out,
            "D{}  {:<42} {}",
            pair.pair,
            lineup_player_text(pair.left.as_ref()),
            lineup_player_text(pair.right.as_ref())
        );
    }
    let _ = writeln!(
        out,
        "G   starter: {} · backup: {}",
        lineup_player_text(view.goalies.starter.as_ref()),
        lineup_player_text(view.goalies.backup.as_ref())
    );
    if !view.extras.is_empty() {
        let _ = writeln!(
            out,
            "Extras: {}",
            view.extras
                .iter()
                .map(|player| lineup_player_text(Some(player)))
                .collect::<Vec<_>>()
                .join(" · ")
        );
    }
    for warning in &view.warnings {
        let _ = writeln!(out, "WARN {}: {}", warning.code, warning.message);
    }
    out
}

fn lineup_player_text(player: Option<&TeamLineupPlayerView>) -> String {
    player.map_or_else(
        || "—".to_string(),
        |player| format!("{} [{}]", player.display_name, player.score.display),
    )
}

fn team_display_name(team: &str) -> &str {
    match team {
        "NYR" => "New York Rangers",
        "SEA" => "Seattle Kraken",
        _ => team,
    }
}

fn render_team_ceiling(view: &TeamCeilingView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "2026-27 TEAM CEILING AND ROSTER DELTA");
    let _ = writeln!(out, "Schema: {}", view.schema);
    let _ = writeln!(
        out,
        "Roster: {}  Production: {}",
        view.roster_season, view.stats_season
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "RK  TEAM  SCORE  DELTA  CEILING  PLAYOFF RANGE  COVERAGE  NEW/OUT"
    );
    for row in &view.teams {
        let _ = writeln!(
            out,
            "{:>2}  {:<4}  {:>5.1}  {:+5.1}  {:>7.1}  {:>5.1}-{:>4.1}%    {:>5.1}%    {}/{}",
            row.rank,
            row.team,
            row.ensemble_score,
            row.delta,
            row.ceiling_score,
            row.playoff_chance_low_pct,
            row.playoff_chance_high_pct,
            row.coverage_pct,
            row.newcomers.len(),
            row.departures.len(),
        );
        if view.teams.len() == 1 {
            for lens in &row.lenses {
                let _ = writeln!(
                    out,
                    "    {:<22} {:>5.1} ({:+.1}, #{})",
                    lens.label, lens.score, lens.delta, lens.rank
                );
            }
            let _ = writeln!(out, "    New: {}", join_names(&row.newcomers));
            let _ = writeln!(out, "    Out: {}", join_names(&row.departures));
        }
    }
    let _ = writeln!(out);
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

fn join_names(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

pub fn run_cap_forecast(args: CapForecastArgs) -> anyhow::Result<()> {
    let base_season: Season = args
        .season
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid forecast season '{}': {error}", args.season))?;
    let selected_team = args
        .team
        .as_deref()
        .map(TeamAbbr::parse)
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?;

    let (outcome, stats_season, _) =
        crate::commands::players::load_repo_for_season(None, Some(SeasonType::Regular))?;

    let mut players = Vec::new();
    let teams: Vec<_> = match selected_team.as_ref() {
        Some(team) => vec![team.clone()],
        None => TeamAbbr::all().collect(),
    };
    for team in teams {
        for view in outcome
            .repo
            .team_roster(&team, stats_season, SeasonType::Regular)
        {
            let age = age_at_season_start(view.identity.bio.birth_date.as_deref(), base_season);
            players.push(CapProjectionPlayerInput {
                player_id: view.id().0,
                player: view.full_name().to_owned(),
                team: team.as_str().to_owned(),
                position: view.position(),
                age,
                games_played: view.gp(),
                points_per_82: view.pace_score().map(|score| score.pace_82),
                contract: view.contract.map(|contract| CapProjectionContractInput {
                    valuation_season: contract
                        .valuation_season
                        .as_deref()
                        .and_then(|value| value.parse().ok()),
                    expiry_year: contract.expiry_year,
                    cap_hit: contract.cap_hit,
                    aav: contract.aav,
                    source: contract.source.clone(),
                    source_url: contract.source_url.clone(),
                }),
            });
        }
    }
    if players.is_empty() {
        let suffix = selected_team
            .as_ref()
            .map(|team| format!(" for {}", team.as_str()))
            .unwrap_or_default();
        anyhow::bail!("no current-roster players found{suffix} — run `icelines fetch all`");
    }

    let view = build_cap_projection(players, base_season, args.years, args.growth_pct)?;
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_cap_forecast(&view, selected_team.as_ref().map(TeamAbbr::as_str))
    };
    emit_report(&output, args.out.as_ref())
}

fn age_at_season_start(birth_date: Option<&str>, season: Season) -> u8 {
    birth_date
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u16>().ok())
        .map(|year| season.start_year().saturating_sub(year).min(99) as u8)
        .unwrap_or(27)
}

fn emit_report(output: &str, path: Option<&PathBuf>) -> anyhow::Result<()> {
    match path.and_then(|path| path.to_str()) {
        None | Some("-") => print!("{output}"),
        Some(_) => {
            let path = path.expect("path checked above");
            std::fs::write(path, output)
                .with_context(|| format!("writing report to {}", path.display()))?;
            println!("Wrote report to {}", path.display());
        }
    }
    Ok(())
}

fn render_cap_forecast(view: &CapProjectionView, selected_team: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "FIVE-YEAR ROSTER MARKET-COST FORECAST");
    let _ = writeln!(out, "Schema: {}", view.schema);
    let _ = writeln!(out, "Method: {}", view.method);
    let _ = writeln!(
        out,
        "Scenario growth after announced limits: {:.1}%",
        view.assumptions.modeled_growth_pct
    );
    let _ = writeln!(out, "Market anchor: {}", view.assumptions.market_anchor);
    let _ = writeln!(
        out,
        "Cap-limit source: {}",
        view.assumptions.cap_limit_source_url
    );
    let _ = writeln!(
        out,
        "Market-anchor source: {}",
        view.assumptions.market_anchor_url
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<9} {:<4} {:>7} {:>6} {:>3} {:>4} {:>4} {:>11} {:>7}  Pressure",
        "Season", "Team", "Cap $M", "Active", "Out", "Conf", "Model", "Spend $M", "Share"
    );
    let _ = writeln!(out, "{}", "-".repeat(88));

    let mut summaries: Vec<_> = view
        .teams
        .iter()
        .flat_map(|team| team.seasons.iter().map(move |season| (&team.team, season)))
        .collect();
    summaries.sort_by(|(team_a, a), (team_b, b)| {
        a.season
            .cmp(&b.season)
            .then_with(|| {
                b.cap_share_pct
                    .partial_cmp(&a.cap_share_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| team_a.cmp(team_b))
    });
    for (team, row) in summaries {
        let _ = writeln!(
            out,
            "{:<9} {:<4} {:>7.1} {:>6} {:>3} {:>4} {:>4} {:>11.1} {:>6.1}%  {}",
            season_label(row.season),
            team,
            millions(row.upper_limit),
            row.roster_players,
            row.excluded_depth_players,
            row.confirmed_players,
            row.modeled_players,
            millions(row.projected_cap_hit),
            row.cap_share_pct,
            row.pressure.label()
        );
    }

    if let Some(team) = selected_team {
        if let Some(team_view) = view.teams.iter().find(|row| row.team == team) {
            if let Some(first) = team_view.seasons.first() {
                let _ = writeln!(out);
                let _ = writeln!(
                    out,
                    "{} PLAYER MARKET — {}",
                    team,
                    season_label(first.season)
                );
                let _ = writeln!(
                    out,
                    "{:<24} {:<4} {:<16} {:<9} {:>8} {:>17}",
                    "Player", "Pos", "Role", "Basis", "Mid $M", "Low-high $M"
                );
                let _ = writeln!(out, "{}", "-".repeat(84));
                for player in &first.players {
                    let basis = match player.salary_basis {
                        SalaryBasis::Confirmed => "confirmed",
                        SalaryBasis::Modeled => "modeled",
                    };
                    let _ = writeln!(
                        out,
                        "{:<24} {:<4} {:<16} {:<9} {:>8.2} {:>8.2}-{:>8.2}",
                        truncate(&player.player, 24),
                        player.position.abbreviation(),
                        player.role.label(),
                        basis,
                        millions(player.projected_cap_hit),
                        millions(player.projected_cap_hit_low),
                        millions(player.projected_cap_hit_high)
                    );
                }
            }
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "DISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    let _ = writeln!(out, "NON-CLAIMS");
    for non_claim in &view.non_claims {
        let _ = writeln!(out, "- {non_claim}");
    }
    out
}

fn millions(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

fn season_label(season: u32) -> String {
    let text = season.to_string();
    if text.len() == 8 {
        format!("{}-{}", &text[..4], &text[6..])
    } else {
        text
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_owned()
    } else {
        let mut output: String = value.chars().take(width.saturating_sub(1)).collect();
        output.push('…');
        output
    }
}

pub fn run_list(json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(REPORT_CATALOG)?);
        return Ok(());
    }

    println!("IceLines report surface");
    println!();
    println!("Use `query` when you are asking a question, `x` when you want CSV/JSON,");
    println!("`export md` when you want a markdown packet, `report` for durable decision");
    println!("reports, and `stathead` for curated editorial query recipes.");
    println!();
    println!(
        "{:<16} {:<10} {:<26} Canonical command",
        "Report", "Status", "Formats"
    );
    println!("{:-<16} {:-<10} {:-<26} {:-<1}", "", "", "", "");
    for entry in REPORT_CATALOG {
        println!(
            "{:<16} {:<10} {:<26} {}",
            entry.name, entry.status, entry.formats, entry.canonical
        );
    }
    println!();
    println!("Available records examples:");
    println!("  icelines records player \"Andre Burakovsky\" --metric teams-scored-against");
    println!("  icelines records player \"Andre Burakovsky\" --metric goalies-scored-against");
    println!("  icelines records player \"Andre Burakovsky\" --metric fight-opponents");
    println!("  icelines records team SEA --metric players-scored-against-team");
    println!("  icelines records team SEA --metric goalies-beaten-by-team");
    println!("  icelines records team SEA --metric fight-opponents-by-team");
    println!();
    println!("Stathead starter examples:");
    println!("  icelines stathead");
    println!("  icelines stathead --markdown --out stathead-packs.md");
    Ok(())
}

#[cfg(test)]
mod team_ceiling_tests {
    use super::canonical_last_team;

    #[test]
    fn traded_player_is_assigned_to_final_listed_team() {
        assert_eq!(canonical_last_team("EDM/PIT/COL").as_deref(), Some("COL"));
        assert_eq!(canonical_last_team("nyr").as_deref(), Some("NYR"));
        assert_eq!(canonical_last_team("NOT-A-TEAM"), None);
    }
}
