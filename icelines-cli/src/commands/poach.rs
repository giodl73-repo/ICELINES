use std::path::PathBuf;

use anyhow::{bail, Context};
use serde::Serialize;

use icelines_core::{
    model::{Position, Season, TeamAbbr},
    name::normalize_name,
    view_model::{
        default_watch_rules_view, evaluate_watch_alerts, poach_report_from_board,
        weekly_poach_report_from_board_with_watched, AvailabilityState, DeploymentSignal,
        PoachAvailabilityFilter, PoachBoardView, PoachQuery, PoachReportView, SourceKind,
        SourceState, ViewContext, ViewWindow, WatchAlertRow, WatchRule, WatchRuleMutationIntent,
        WatchRuleTrigger, WatchRulesView,
    },
};

use crate::cli::QuerySeasonType;
use crate::commands::players::{load_repo_for_season, validate_bundled_season};
use crate::config::Config;
use crate::db::{GroupDb, MemberKind, WatchRuleEventRow, WatchRuleRow};
use crate::fantasy_db::FantasyDb;

pub struct PoachArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub availability: Option<String>,
    pub top: u16,
    pub json: bool,
}

pub struct PoachReportArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub availability: Option<String>,
    pub top: u16,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub struct WeeklyReportArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub league: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub availability: Option<String>,
    pub top: u16,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub struct WatchRulesArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
}

pub struct WatchSetEnabledArgs {
    pub id: String,
    pub enabled: bool,
    pub json: bool,
}

pub struct WatchFireArgs {
    pub id: String,
    pub player: Option<String>,
    pub message: String,
    pub json: bool,
}

pub struct WatchHistoryArgs {
    pub limit: u16,
    pub json: bool,
}

pub struct WatchAlertsArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub top: u16,
    pub save: bool,
    pub json: bool,
}

pub struct WatchListArgs {
    pub json: bool,
}

pub struct WatchNoteArgs {
    pub player: String,
    pub reason: String,
    pub json: bool,
}

pub struct WatchPlayerArgs {
    pub player: String,
    pub when: String,
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
    pub save: bool,
}

pub struct WatchDeploymentArgs {
    pub team: Option<String>,
    pub line_change: bool,
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
    pub save: bool,
}

pub async fn run(args: PoachArgs) -> anyhow::Result<()> {
    let json = args.json;
    let view = build_board(args)?;
    emit_board(&view, json)
}

pub async fn run_report_poach(args: PoachReportArgs) -> anyhow::Result<()> {
    let board = build_board(PoachArgs {
        season: args.season,
        season_type: args.season_type,
        scheme: args.scheme,
        categories: args.categories,
        teams: args.teams,
        positions: args.positions,
        availability: args.availability,
        top: args.top,
        json: false,
    })?;
    let report = poach_report_from_board(board);
    let body = if args.json {
        serde_json::to_string_pretty(&report).context("serializing poach report")?
    } else {
        render_report_markdown(&report)
    };
    write_or_print(args.out.as_ref(), &body)
}

pub async fn run_report_weekly(args: WeeklyReportArgs) -> anyhow::Result<()> {
    let board = build_board(PoachArgs {
        season: args.season,
        season_type: args.season_type,
        scheme: args.scheme,
        categories: args.categories,
        teams: args.teams,
        positions: args.positions,
        availability: args.availability,
        top: args.top,
        json: false,
    })?;
    let db = GroupDb::open()?;
    let watched = watched_player_keys(&db)?;
    let report =
        weekly_poach_report_from_board_with_watched(board, &args.league, args.top, &watched);
    let body = if args.json {
        serde_json::to_string_pretty(&report).context("serializing weekly poach report")?
    } else {
        render_report_markdown(&report)
    };
    write_or_print(args.out.as_ref(), &body)
}

pub async fn run_watch_rules(args: WatchRulesArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    let mut view = default_watch_rules_view(context);
    let db = GroupDb::open()?;
    view.rules.extend(persisted_watch_rules(&db)?);
    emit_watch_view(&view, args.json)
}

pub async fn run_watch_set_enabled(args: WatchSetEnabledArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    let intent = WatchRuleMutationIntent::resolve(&args.id, args.enabled)
        .map_err(|message| anyhow::anyhow!(message))?;
    let changed = db.set_watch_rule_enabled(&intent.rule_id, intent.enabled)?;
    if !changed {
        bail!("unknown persisted watch rule '{}'", intent.rule_id);
    }
    let rule = db
        .list_watch_rules()?
        .into_iter()
        .find(|rule| rule.id == intent.rule_id)
        .context("watch rule disappeared after update")?;
    let latest_fired = latest_watch_rule_fire_times(&db)?;
    let rule = watch_rule_from_row(rule, &latest_fired)?;
    let mutation = intent.result_view(default_watch_context(), changed);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&mutation).context("serializing watch rule mutation")?
        );
    } else {
        println!(
            "{} watch rule '{}'",
            if intent.enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            rule.id
        );
    }
    Ok(())
}

pub async fn run_watch_fire(args: WatchFireArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    if !db.list_watch_rules()?.iter().any(|rule| rule.id == args.id) {
        bail!("unknown persisted watch rule '{}'", args.id);
    }
    let entity_ref = args
        .player
        .as_deref()
        .map(normalize_name)
        .map(|key| format!("player:{key}"));
    let event = db.record_watch_rule_event(&args.id, entity_ref.as_deref(), &args.message)?;
    emit_watch_rule_event(&event, args.json)
}

pub async fn run_watch_history(args: WatchHistoryArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    let events = db.list_watch_rule_events(args.limit as usize)?;
    emit_watch_rule_events(&events, args.json)
}

pub async fn run_watch_alerts(args: WatchAlertsArgs) -> anyhow::Result<()> {
    let board = build_board(PoachArgs {
        season: args.season,
        season_type: args.season_type,
        scheme: "yahoo-standard".to_string(),
        categories: Vec::new(),
        teams: Vec::new(),
        positions: Vec::new(),
        availability: None,
        top: args.top,
        json: false,
    })?;
    let db = GroupDb::open()?;
    let watched = watched_player_keys(&db)?;
    let view = evaluate_watch_alerts(&board, &watched);
    let saved = if args.save {
        persist_watch_alerts(&db, &view.alerts)?
    } else {
        Vec::new()
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&WatchAlertsCliView {
                view: &view,
                saved_events: saved.iter().map(WatchRuleEventView::from).collect(),
            })
            .context("serializing watch alerts")?
        );
    } else {
        emit_watch_alerts(&view.alerts);
        if args.save {
            println!("Saved {} new alert event(s).", saved.len());
        }
    }
    Ok(())
}

pub async fn run_watch_list(args: WatchListArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    let rows = watchlist_rows(&db)?;
    emit_watchlist_rows(&rows, args.json)
}

pub async fn run_watch_note(args: WatchNoteArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    ensure_watchlist(&db)?;
    let key = normalize_name(&args.player);
    db.add_member_kind("Watchlist", &key, MemberKind::Player)?;
    db.upsert_watch_note(MemberKind::Player, &key, &args.reason, "manual")?;
    let row = watchlist_row_for(&db, MemberKind::Player, key)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&row).context("serializing watch note")?
        );
    } else {
        println!(
            "Watching '{}': {}",
            args.player,
            row.reason.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub async fn run_watch_player(args: WatchPlayerArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    let view = WatchRulesView {
        context,
        rules: vec![player_watch_rule(&args.player, &args.when)],
        warnings: Vec::new(),
    };
    if args.save {
        let db = GroupDb::open()?;
        persist_watch_rules(&db, &view.rules)?;
    }
    emit_watch_view(&view, args.json)
}

pub async fn run_watch_deployment(args: WatchDeploymentArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    let team = args.team.map(|team| TeamAbbr(team.trim().to_uppercase()));
    let label = if args.line_change {
        "Deployment line-change watch"
    } else {
        "Deployment promotion watch"
    };
    let view = WatchRulesView {
        context,
        rules: vec![WatchRule {
            id: "deployment-preview".to_string(),
            label: match &team {
                Some(team) => format!("{label} for {}", team.as_str()),
                None => label.to_string(),
            },
            enabled: true,
            trigger: WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            last_fired: None,
            unsupported_sources: vec![SourceKind::Shifts],
        }],
        warnings: Vec::new(),
    };
    if args.save {
        let db = GroupDb::open()?;
        persist_watch_rules(&db, &view.rules)?;
    }
    emit_watch_view(&view, args.json)
}

fn build_board(args: PoachArgs) -> anyhow::Result<PoachBoardView> {
    let (outcome, season, season_type) =
        load_repo_for_season(args.season.as_deref(), Some(args.season_type.to_core()))?;

    let mut query = PoachQuery::new(season, season_type, args.scheme);
    query.categories = normalize_categories(args.categories);
    query.teams = args
        .teams
        .into_iter()
        .map(|team| TeamAbbr(team.trim().to_uppercase()))
        .collect();
    query.positions = parse_positions(args.positions)?;
    query.availability_filter = parse_availability_filter(args.availability.as_deref())?;
    query.limit = Some(args.top);
    query.sort = Some("poach_score".to_string());
    if let Ok(Some(rosters)) = active_fantasy_rostered_player_keys() {
        query =
            query.with_imported_league_availability(rosters.all_rostered, rosters.user_rostered);
    }

    Ok(PoachBoardView::from_repository(&outcome.repo, query))
}

struct ActiveFantasyRosters {
    all_rostered: Vec<String>,
    user_rostered: Vec<String>,
}

fn active_fantasy_rostered_player_keys() -> anyhow::Result<Option<ActiveFantasyRosters>> {
    let db = FantasyDb::open()?;
    let Some(league) = db.get_active_league()? else {
        return Ok(None);
    };
    let user_team_id = db.get_user_team(&league.id)?.map(|team| team.id);
    let mut all_rostered = Vec::new();
    let mut user_rostered = Vec::new();
    for team in db.list_teams(&league.id)? {
        let roster = db.list_roster(&team.id)?;
        if Some(team.id.as_str()) == user_team_id.as_deref() {
            user_rostered.extend(roster.iter().cloned());
        }
        all_rostered.extend(roster);
    }
    Ok(Some(ActiveFantasyRosters {
        all_rostered,
        user_rostered,
    }))
}

fn emit_board(view: &PoachBoardView, json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing poach board")?
        );
    } else {
        print_table(view);
    }
    Ok(())
}

pub(crate) fn render_report_markdown(report: &PoachReportView) -> String {
    let mut out = String::new();
    out.push_str("# Fantasy Poacher\n\n");
    out.push_str(&format!(
        "- Season: {}\n- Type: {:?}\n- Scheme: {}\n- Window: {:?}\n\n",
        report.context.view_context.window.season,
        report.context.view_context.window.season_type,
        report.scoring_scheme,
        report.window
    ));
    if !report.scoring_categories.is_empty() {
        out.push_str(&format!(
            "- Categories: {}\n\n",
            report.scoring_categories.join(", ")
        ));
    }

    if !report.omissions.is_empty() {
        out.push_str("## Source Omissions\n\n");
        for omission in &report.omissions {
            out.push_str(&format!("- {omission}\n"));
        }
        out.push('\n');
    }

    for section in &report.sections {
        out.push_str(&format!("## {}\n\n", section.title));
        if section.rows.is_empty() {
            out.push_str("No candidates matched this report.\n\n");
            continue;
        }
        out.push_str("| Rank | Player | Team | Pos | Score | Confidence | Why |\n");
        out.push_str("|---:|---|---|---|---:|---|---|\n");
        for (idx, row) in section.rows.iter().enumerate() {
            let why = row
                .explanations
                .first()
                .map(|explanation| explanation.message.as_str())
                .unwrap_or("No explanation");
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.1} | {:?} | {} |\n",
                idx + 1,
                markdown_cell(&row.display_name),
                row.team.as_str(),
                row.position.abbreviation(),
                row.score.final_score,
                row.confidence,
                markdown_cell(why)
            ));
        }
        out.push('\n');
    }
    out
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn write_or_print(out: Option<&PathBuf>, body: &str) -> anyhow::Result<()> {
    match out {
        Some(path) if path.as_os_str() != "-" => {
            std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
        }
        _ => {
            print!("{body}");
        }
    }
    Ok(())
}

fn watch_context(
    season: Option<&str>,
    season_type: QuerySeasonType,
) -> anyhow::Result<ViewContext> {
    let cfg = Config::load()?;
    let season_str = match season {
        Some(season) => {
            validate_bundled_season(season)?;
            season.to_string()
        }
        None => cfg.season_str(),
    };
    let season_u32 = season_str
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{season_str}' is not a YYYYZZZZ id"))?;
    let mut context = ViewContext::new(ViewWindow::new(Season(season_u32), season_type.to_core()));
    context.completeness = icelines_core::Completeness::Partial;
    context.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::missing(SourceKind::Shifts),
        SourceState::missing(SourceKind::Schedule),
        SourceState::missing(SourceKind::FantasyImport),
    ];
    Ok(context)
}

fn default_watch_context() -> ViewContext {
    watch_context(None, QuerySeasonType::Regular).unwrap_or_else(|_| {
        ViewContext::new(ViewWindow::new(
            Season(icelines_core::CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        ))
    })
}

fn player_watch_rule(player: &str, trigger: &str) -> WatchRule {
    let normalized_trigger = trigger.trim().to_ascii_lowercase();
    let (rule_trigger, unsupported_sources) = match normalized_trigger.as_str() {
        "available" | "availability" => (
            WatchRuleTrigger::AvailabilityChanged {
                player_id: None,
                state: AvailabilityState::Unknown,
            },
            vec![SourceKind::FantasyImport],
        ),
        "pp1" | "pp2" | "top-six" | "promotion" | "line-change" => (
            WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            vec![SourceKind::Shifts],
        ),
        _ => (
            WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            vec![SourceKind::Shifts],
        ),
    };

    WatchRule {
        id: format!("player-{}", slug(player)),
        label: format!("Watch {player} when {normalized_trigger}"),
        enabled: true,
        trigger: rule_trigger,
        last_fired: None,
        unsupported_sources,
    }
}

fn emit_watch_view(view: &WatchRulesView, json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(view).context("serializing watch rules")?
        );
        return Ok(());
    }

    println!("{:<28} {:<7} Unsupported", "Rule", "Enabled");
    for rule in &view.rules {
        let unsupported = if rule.unsupported_sources.is_empty() {
            "-".to_string()
        } else {
            rule.unsupported_sources
                .iter()
                .map(|source| format!("{source:?}").to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "{:<28} {:<7} {}",
            truncate(&rule.label, 28),
            if rule.enabled { "yes" } else { "no" },
            unsupported
        );
    }
    Ok(())
}

fn emit_watch_alerts(alerts: &[icelines_core::view_model::WatchAlertRow]) {
    if alerts.is_empty() {
        println!("No fantasy watch alerts.");
        return;
    }
    println!(
        "{:<11} {:<24} {:<24} Reason",
        "Severity", "Player", "Trigger"
    );
    for alert in alerts {
        println!(
            "{:<11} {:<24} {:<24} {}",
            format!("{:?}", alert.severity).to_ascii_lowercase(),
            truncate(&alert.display_name, 24),
            format!("{:?}", alert.trigger).to_ascii_lowercase(),
            truncate(&alert.reason, 80)
        );
    }
}

#[derive(Debug, Serialize)]
struct WatchAlertsCliView<'a> {
    #[serde(flatten)]
    view: &'a icelines_core::view_model::WatchAlertsView,
    saved_events: Vec<WatchRuleEventView<'a>>,
}

fn persist_watch_alerts(
    db: &GroupDb,
    alerts: &[WatchAlertRow],
) -> anyhow::Result<Vec<WatchRuleEventRow>> {
    let existing = db.list_watch_rule_events(10_000)?;
    let mut saved = Vec::new();
    for alert in alerts {
        let rule_id = watch_alert_rule_id(alert);
        ensure_watch_alert_rule(db, alert, &rule_id)?;
        let entity_ref = format!("player:{}", normalize_name(&alert.display_name));
        let is_duplicate = existing.iter().any(|event| {
            event.rule_id == rule_id
                && event.entity_ref.as_deref() == Some(entity_ref.as_str())
                && event.message == alert.reason
        }) || saved.iter().any(|event: &WatchRuleEventRow| {
            event.rule_id == rule_id
                && event.entity_ref.as_deref() == Some(entity_ref.as_str())
                && event.message == alert.reason
        });
        if is_duplicate {
            continue;
        }
        saved.push(db.record_watch_rule_event(&rule_id, Some(&entity_ref), &alert.reason)?);
    }
    Ok(saved)
}

fn ensure_watch_alert_rule(
    db: &GroupDb,
    alert: &WatchAlertRow,
    rule_id: &str,
) -> anyhow::Result<()> {
    let trigger = match alert.trigger {
        icelines_core::view_model::WatchAlertTrigger::WatchedAvailable => {
            WatchRuleTrigger::AvailabilityChanged {
                player_id: Some(alert.player_id),
                state: AvailabilityState::ImportedAvailable,
            }
        }
        icelines_core::view_model::WatchAlertTrigger::WatchedDeploymentSignal => {
            WatchRuleTrigger::PlayerPromoted {
                player_id: Some(alert.player_id),
                evidence: DeploymentSignal::Unknown,
            }
        }
        icelines_core::view_model::WatchAlertTrigger::UserRosterDropRisk => {
            WatchRuleTrigger::CategoryThreshold {
                category: "drop_risk".to_string(),
                threshold: 1.0,
            }
        }
    };
    db.upsert_watch_rule(
        rule_id,
        &format!("Fantasy alert: {:?}", alert.trigger),
        true,
        &serde_json::to_string(&trigger).context("serializing alert rule trigger")?,
        &serde_json::to_string(&alert.unsupported_sources)
            .context("serializing alert unsupported sources")?,
    )
}

fn watch_alert_rule_id(alert: &WatchAlertRow) -> String {
    match alert.trigger {
        icelines_core::view_model::WatchAlertTrigger::WatchedAvailable => "alert-watched-available",
        icelines_core::view_model::WatchAlertTrigger::WatchedDeploymentSignal => {
            "alert-watched-deployment"
        }
        icelines_core::view_model::WatchAlertTrigger::UserRosterDropRisk => {
            "alert-user-roster-drop-risk"
        }
    }
    .to_string()
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct WatchlistRow {
    kind: String,
    key: String,
    reason: Option<String>,
    source: Option<String>,
    updated_at: Option<String>,
}

fn ensure_watchlist(db: &GroupDb) -> anyhow::Result<()> {
    let exists = db
        .list_groups()?
        .iter()
        .any(|group| group.name == "Watchlist");
    if !exists {
        db.create_group("Watchlist", "Fantasy poacher watchlist")?;
    }
    Ok(())
}

fn persist_watch_rules(db: &GroupDb, rules: &[WatchRule]) -> anyhow::Result<()> {
    for rule in rules {
        let trigger_json =
            serde_json::to_string(&rule.trigger).context("serializing watch rule trigger")?;
        let unsupported_sources_json = serde_json::to_string(&rule.unsupported_sources)
            .context("serializing watch rule unsupported sources")?;
        db.upsert_watch_rule(
            &rule.id,
            &rule.label,
            rule.enabled,
            &trigger_json,
            &unsupported_sources_json,
        )?;
    }
    Ok(())
}

fn persisted_watch_rules(db: &GroupDb) -> anyhow::Result<Vec<WatchRule>> {
    let latest_fired = latest_watch_rule_fire_times(db)?;
    db.list_watch_rules()?
        .into_iter()
        .map(|row| watch_rule_from_row(row, &latest_fired))
        .collect()
}

fn latest_watch_rule_fire_times(
    db: &GroupDb,
) -> anyhow::Result<std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>> {
    let mut latest = std::collections::HashMap::new();
    for event in db.list_watch_rule_events(10_000)? {
        if latest.contains_key(&event.rule_id) {
            continue;
        }
        if let Ok(fired_at) = chrono::DateTime::parse_from_rfc3339(&event.fired_at) {
            latest.insert(event.rule_id, fired_at.with_timezone(&chrono::Utc));
        }
    }
    Ok(latest)
}

fn watch_rule_from_row(
    row: WatchRuleRow,
    latest_fired: &std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>,
) -> anyhow::Result<WatchRule> {
    let trigger = serde_json::from_str(&row.trigger_json)
        .with_context(|| format!("parsing persisted watch rule trigger '{}'", row.id))?;
    let unsupported_sources =
        serde_json::from_str(&row.unsupported_sources_json).with_context(|| {
            format!(
                "parsing persisted watch rule unsupported sources '{}'",
                row.id
            )
        })?;
    Ok(WatchRule {
        last_fired: latest_fired.get(&row.id).copied(),
        id: row.id,
        label: row.label,
        enabled: row.enabled,
        trigger,
        unsupported_sources,
    })
}

fn emit_watch_rule_event(event: &WatchRuleEventRow, json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&WatchRuleEventView::from(event))
                .context("serializing watch rule event")?
        );
    } else {
        let entity = event.entity_ref.as_deref().unwrap_or("-");
        println!(
            "{}  {}  {}  {}",
            event.fired_at, event.rule_id, entity, event.message
        );
    }
    Ok(())
}

fn emit_watch_rule_events(events: &[WatchRuleEventRow], json: bool) -> anyhow::Result<()> {
    if json {
        let views: Vec<_> = events.iter().map(WatchRuleEventView::from).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&views).context("serializing watch rule events")?
        );
    } else if events.is_empty() {
        println!("No watch rule alerts recorded.");
    } else {
        for event in events {
            emit_watch_rule_event(event, false)?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct WatchRuleEventView<'a> {
    id: i64,
    rule_id: &'a str,
    entity_ref: Option<&'a str>,
    message: &'a str,
    fired_at: &'a str,
}

impl<'a> From<&'a WatchRuleEventRow> for WatchRuleEventView<'a> {
    fn from(row: &'a WatchRuleEventRow) -> Self {
        Self {
            id: row.id,
            rule_id: &row.rule_id,
            entity_ref: row.entity_ref.as_deref(),
            message: &row.message,
            fired_at: &row.fired_at,
        }
    }
}

fn watchlist_rows(db: &GroupDb) -> anyhow::Result<Vec<WatchlistRow>> {
    match db.list_members_with_kind("Watchlist") {
        Ok(members) => members
            .into_iter()
            .map(|(key, kind)| watchlist_row_for(db, kind, key))
            .collect(),
        Err(err) if err.to_string().contains("group 'Watchlist' not found") => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn watched_player_keys(db: &GroupDb) -> anyhow::Result<Vec<String>> {
    Ok(watchlist_rows(db)?
        .into_iter()
        .filter(|row| row.kind == "player")
        .map(|row| row.key)
        .collect())
}

fn watchlist_row_for(db: &GroupDb, kind: MemberKind, key: String) -> anyhow::Result<WatchlistRow> {
    let note = db.watch_note(kind, &key)?;
    Ok(WatchlistRow {
        kind: kind.as_str().to_string(),
        key,
        reason: note.as_ref().map(|note| note.reason.clone()),
        source: note.as_ref().map(|note| note.source.clone()),
        updated_at: note.map(|note| note.updated_at),
    })
}

fn emit_watchlist_rows(rows: &[WatchlistRow], json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(rows).context("serializing watchlist")?
        );
        return Ok(());
    }

    if rows.is_empty() {
        println!("Watchlist is empty. Add from TUI Poach with `w` or run `icelines watch note <player> <reason>`.");
        return Ok(());
    }

    println!("{:<8} {:<28} Reason", "Kind", "Key");
    for row in rows {
        println!(
            "{:<8} {:<28} {}",
            row.kind,
            truncate(&row.key, 28),
            truncate(row.reason.as_deref().unwrap_or("-"), 80)
        );
    }
    Ok(())
}

fn slug(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn normalize_categories(categories: Vec<String>) -> Vec<String> {
    categories
        .into_iter()
        .map(|category| category.trim().to_ascii_lowercase())
        .filter(|category| !category.is_empty())
        .collect()
}

fn parse_positions(values: Vec<String>) -> anyhow::Result<Vec<Position>> {
    values
        .into_iter()
        .map(|value| parse_position(&value))
        .collect()
}

fn parse_position(value: &str) -> anyhow::Result<Position> {
    match value.trim().to_ascii_uppercase().as_str() {
        "C" => Ok(Position::Center),
        "LW" | "L" => Ok(Position::LeftWing),
        "RW" | "R" => Ok(Position::RightWing),
        "D" => Ok(Position::Defense),
        "G" => Ok(Position::Goalie),
        other => bail!("unknown position '{other}' - valid: C, LW, RW, D, G"),
    }
}

fn parse_availability_filter(value: Option<&str>) -> anyhow::Result<PoachAvailabilityFilter> {
    let Some(value) = value else {
        return Ok(PoachAvailabilityFilter::Any);
    };
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "" | "any" | "all" => Ok(PoachAvailabilityFilter::Any),
        "available" | "free" | "free_agent" | "free_agents" => {
            Ok(PoachAvailabilityFilter::Available)
        }
        "not_on_user_roster" | "not_user_roster" | "not_mine" => {
            Ok(PoachAvailabilityFilter::NotOnUserRoster)
        }
        "watched" | "watchlist" => Ok(PoachAvailabilityFilter::Watched),
        "imported_available" | "imported_free" | "league_available" => {
            Ok(PoachAvailabilityFilter::ImportedAvailable)
        }
        "unknown" => Ok(PoachAvailabilityFilter::Unknown),
        other => bail!(
            "unknown availability filter '{other}' - valid: any, available, imported_available, not_on_user_roster, watched, unknown"
        ),
    }
}

fn availability_label(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Unknown => "unknown",
        AvailabilityState::Available => "available",
        AvailabilityState::RosteredByUser => "my-roster",
        AvailabilityState::ImportedAvailable => "free",
        AvailabilityState::ImportedRostered => "rostered",
        AvailabilityState::Watched => "watched",
    }
}

fn print_table(view: &PoachBoardView) {
    if let Some(empty) = &view.empty_state {
        println!("{}", empty.title);
        if let Some(detail) = &empty.detail {
            println!("{detail}");
        }
        return;
    }

    println!(
        "{:<4} {:<26} {:<4} {:<3} {:>6} {:<10} {:<12} {:<28} Risk",
        "Rank", "Player", "Team", "Pos", "Score", "Avail", "Confidence", "Why"
    );
    for (idx, row) in view.rows.iter().enumerate() {
        let why = row
            .explanations
            .first()
            .map(|explanation| explanation.message.as_str())
            .unwrap_or("No explanation");
        let risk = row.risk_summary.as_deref().unwrap_or("-");
        println!(
            "{:<4} {:<26} {:<4} {:<3} {:>6.1} {:<10} {:<12} {:<28} {}",
            idx + 1,
            truncate(&row.display_name, 26),
            row.team.as_str(),
            row.position.abbreviation(),
            row.score.final_score,
            availability_label(row.availability),
            format!("{:?}", row.confidence).to_ascii_lowercase(),
            truncate(why, 28),
            truncate(risk, 24)
        );
    }

    if view.context.completeness != icelines_core::Completeness::Complete {
        println!(
            "\nSource state: {:?}. Missing schedule/import/shift data is disclosed, not scored as negative evidence.",
            view.context.completeness
        );
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();
    if len <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let mut out = String::new();
    for ch in value.chars().take(max_chars - 3) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_poach_normalizes_categories() {
        assert_eq!(
            normalize_categories(vec![
                " Hits".to_string(),
                "BLOCKS".to_string(),
                "".to_string()
            ]),
            vec!["hits".to_string(), "blocks".to_string()]
        );
    }

    #[test]
    fn l0_poach_parses_positions() {
        assert_eq!(
            parse_positions(vec!["C".to_string(), "lw".to_string()]).unwrap(),
            vec![Position::Center, Position::LeftWing]
        );
    }

    #[test]
    fn l0_poach_rejects_bad_position() {
        let err = parse_position("bad").unwrap_err().to_string();
        assert!(err.contains("valid: C, LW, RW, D, G"));
    }

    #[test]
    fn l0_poach_parses_availability_filter_aliases() {
        assert_eq!(
            parse_availability_filter(None).unwrap(),
            PoachAvailabilityFilter::Any
        );
        assert_eq!(
            parse_availability_filter(Some("imported-available")).unwrap(),
            PoachAvailabilityFilter::ImportedAvailable
        );
        assert_eq!(
            parse_availability_filter(Some("free-agent")).unwrap(),
            PoachAvailabilityFilter::Available
        );
    }

    #[test]
    fn l0_poach_truncates_long_text() {
        assert_eq!(truncate("abcdef", 4), "a...");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate("abcdef", 3), "...");
    }

    #[test]
    fn l0_poach_markdown_escapes_table_cells() {
        assert_eq!(markdown_cell("A|B"), "A\\|B");
    }

    #[test]
    fn l0_poach_report_markdown_includes_empty_section() {
        let context = icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        ));
        let report = PoachReportView {
            context: icelines_core::view_model::poach_report_context(context, "test-report"),
            scoring_scheme: "yahoo-standard".to_string(),
            scoring_categories: vec!["hits".to_string(), "blocks".to_string()],
            window: icelines_core::view_model::PoachWindow::Days14,
            source_state: Vec::new(),
            warnings: Vec::new(),
            omissions: vec!["schedule: unavailable".to_string()],
            sections: vec![icelines_core::view_model::PoachReportSection {
                id: "top_adds".to_string(),
                title: "Top Adds".to_string(),
                rows: Vec::new(),
            }],
        };

        let md = render_report_markdown(&report);

        assert!(md.contains("# Fantasy Poacher"));
        assert!(md.contains("Categories: hits, blocks"));
        assert!(md.contains("schedule: unavailable"));
        assert!(md.contains("No candidates matched this report."));
    }

    #[test]
    fn l0_poach_weekly_report_names_expected_sections() {
        let season = icelines_core::Season(20252026);
        let season_type = icelines_core::season_stats::SeasonType::Regular;
        let query = PoachQuery::new(season, season_type, "yahoo-standard");
        let board = PoachBoardView::new(
            icelines_core::ViewContext::new(icelines_core::ViewWindow::new(season, season_type)),
            query,
            "yahoo-standard",
        );

        let report =
            icelines_core::view_model::weekly_poach_report_from_board(board, "Main League", 20);
        let section_ids: Vec<_> = report
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect();

        assert_eq!(report.context.report_id, "weekly-main-league");
        assert_eq!(report.context.title, "Weekly Fantasy Prep");
        assert_eq!(report.context.sections.len(), report.sections.len());
        assert_eq!(
            section_ids,
            vec![
                "top_adds",
                "category_specialists",
                "deployment_risers",
                "risk_discounts",
                "watched_player_alerts"
            ]
        );
    }

    #[test]
    fn l0_watch_default_rules_disclose_unsupported_sources() {
        let context = icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        ));
        let view = default_watch_rules_view(context);

        assert_eq!(view.rules.len(), 5);
        assert!(view
            .rules
            .iter()
            .any(|rule| rule.unsupported_sources.contains(&SourceKind::Shifts)));
        assert!(view.rules.iter().any(|rule| rule
            .unsupported_sources
            .contains(&SourceKind::FantasyImport)));
    }

    #[test]
    fn l0_watch_player_rule_names_player_and_trigger() {
        let rule = player_watch_rule("Matthew Knies", "pp1");

        assert_eq!(rule.id, "player-matthew-knies");
        assert!(rule.label.contains("Matthew Knies"));
        assert!(rule.unsupported_sources.contains(&SourceKind::Shifts));
    }

    #[test]
    fn l0_watch_persisted_rules_round_trip_into_view_rules() {
        let db = GroupDb::open_in_memory().expect("open db");
        let rule = player_watch_rule("Matthew Knies", "pp1");

        persist_watch_rules(&db, std::slice::from_ref(&rule)).expect("persist rule");
        let rules = persisted_watch_rules(&db).expect("read persisted rules");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule.id);
        assert_eq!(rules[0].label, rule.label);
        assert_eq!(rules[0].trigger, rule.trigger);
        assert_eq!(rules[0].unsupported_sources, rule.unsupported_sources);
    }

    #[test]
    fn l0_watch_set_enabled_updates_persisted_rule() {
        let db = GroupDb::open_in_memory().expect("open db");
        let rule = player_watch_rule("Matthew Knies", "pp1");
        persist_watch_rules(&db, std::slice::from_ref(&rule)).expect("persist rule");

        assert!(db
            .set_watch_rule_enabled("player-matthew-knies", false)
            .expect("disable rule"));
        let rules = persisted_watch_rules(&db).expect("read persisted rules");

        assert_eq!(rules.len(), 1);
        assert!(!rules[0].enabled);
    }

    #[test]
    fn l0_watch_persisted_rules_include_last_fired_history() {
        let db = GroupDb::open_in_memory().expect("open db");
        let rule = player_watch_rule("Matthew Knies", "pp1");
        persist_watch_rules(&db, std::slice::from_ref(&rule)).expect("persist rule");
        db.record_watch_rule_event(
            "player-matthew-knies",
            Some("player:matthew knies"),
            "PP1 usage crossed threshold",
        )
        .expect("record event");

        let rules = persisted_watch_rules(&db).expect("read persisted rules");

        assert_eq!(rules.len(), 1);
        assert!(rules[0].last_fired.is_some());
    }

    #[test]
    fn l0_watch_history_event_view_serializes() {
        let db = GroupDb::open_in_memory().expect("open db");
        let rule = player_watch_rule("Matthew Knies", "pp1");
        persist_watch_rules(&db, std::slice::from_ref(&rule)).expect("persist rule");
        let event = db
            .record_watch_rule_event(
                "player-matthew-knies",
                Some("player:matthew knies"),
                "PP1 usage crossed threshold",
            )
            .expect("record event");

        let json = serde_json::to_value(WatchRuleEventView::from(&event)).expect("event json");

        assert_eq!(json["rule_id"], "player-matthew-knies");
        assert_eq!(json["entity_ref"], "player:matthew knies");
        assert_eq!(json["message"], "PP1 usage crossed threshold");
    }

    #[test]
    fn l0_watch_alert_persistence_dedupes_same_alert() {
        let db = GroupDb::open_in_memory().expect("open db");
        let alert = WatchAlertRow {
            player_id: icelines_core::identity::PlayerId(8478402),
            display_name: "Connor McDavid".to_string(),
            trigger: icelines_core::view_model::WatchAlertTrigger::WatchedAvailable,
            severity: icelines_core::view_model::WatchAlertSeverity::Opportunity,
            reason: "Connor McDavid is available and has poach score 90.0.".to_string(),
            unsupported_sources: Vec::new(),
        };

        let first = persist_watch_alerts(&db, std::slice::from_ref(&alert)).expect("first persist");
        let second = persist_watch_alerts(&db, &[alert]).expect("second persist");
        let history = db.list_watch_rule_events(20).expect("history");

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].rule_id, "alert-watched-available");
        assert_eq!(
            history[0].entity_ref.as_deref(),
            Some("player:connor mcdavid")
        );
    }

    #[test]
    fn l0_watch_slug_strips_punctuation() {
        assert_eq!(slug("A. Player Jr."), "a-player-jr");
    }

    #[test]
    fn l0_watchlist_rows_include_notes() {
        let db = GroupDb::open_in_memory().expect("open db");
        db.create_group("Watchlist", "").expect("create watchlist");
        db.add_member_kind("Watchlist", "matthew knies", MemberKind::Player)
            .expect("add player");
        db.upsert_watch_note(
            MemberKind::Player,
            "matthew knies",
            "Poach score 72.0; PP1 promotion",
            "manual",
        )
        .expect("note");

        let rows = watchlist_rows(&db).expect("watchlist rows");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "player");
        assert_eq!(rows[0].key, "matthew knies");
        assert_eq!(
            rows[0].reason.as_deref(),
            Some("Poach score 72.0; PP1 promotion")
        );
        assert_eq!(rows[0].source.as_deref(), Some("manual"));
    }

    #[test]
    fn l0_watchlist_rows_missing_group_is_empty() {
        let db = GroupDb::open_in_memory().expect("open db");

        let rows = watchlist_rows(&db).expect("watchlist rows");

        assert!(rows.is_empty());
    }
}
