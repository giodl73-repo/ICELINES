//! `icelines signals` — descriptive Signals surface (Phase Hurricane, WP-010 pulse-03).
//!
//! Read-only renderer over the core `PlayerSignalsView`. All Signal math lives in
//! `icelines_core::signal_metrics`; this module only resolves a player and renders
//! the typed ViewModel (Contract 3 — renderers choose layout, never recompute).
//!
//! Signals are descriptive derived metrics built from existing stat inputs. They
//! are NOT predictions, betting edges, injury signals, deployment recommendations,
//! or autonomous coaching decisions. Missing/partial evidence renders as
//! `unavailable`, never as a 0.0 player value (see `design/specs/icelines-signals.md`).

use anyhow::Context;
use icelines_core::model::TeamAbbr;
use icelines_core::season_stats::SeasonType;
use icelines_core::signal_metrics::{
    SignalEvidenceTier, SignalInput, SignalMetricUnit, SignalPolarity,
};
use icelines_core::stats_repository::PlayerView;
use icelines_core::view_model::signals::{PlayerSignalsView, SignalsSourceAuthority};

/// Render a player's Signals as a text table or a frozen `signals.v1` JSON envelope.
pub async fn run_signals(
    player: String,
    season: Option<String>,
    season_type: SeasonType,
    json: bool,
) -> anyhow::Result<()> {
    let view = build_view(&player, season.as_deref(), season_type)?;
    if json {
        println!("{}", signals_json_envelope(&view));
    } else {
        print_signals_text(&view);
    }
    Ok(())
}

/// Render a team-scoped Signals roster matrix. This is intentionally not a
/// leaderboard: rows are sorted by player name and no Signal controls rank.
pub async fn run_signals_roster(
    team: String,
    season: Option<String>,
    season_type: SeasonType,
    json: bool,
) -> anyhow::Result<()> {
    let view = build_roster_view(&team, season.as_deref(), season_type)?;
    if json {
        println!("{}", signals_roster_json_envelope(&view));
    } else {
        print_signals_roster_text(&view);
    }
    Ok(())
}

/// Resolve a player in the active `(season, season_type)` window, falling back to
/// a bundled-season name lookup + lazy career fan-out for historical players —
/// the same resolution path as `query player` (see `commands::query::run_player`).
pub(crate) fn build_view(
    name: &str,
    season: Option<&str>,
    season_type: SeasonType,
) -> anyhow::Result<PlayerSignalsView> {
    let (mut outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season, Some(season_type))?;

    let mut historical_pid: Option<icelines_core::identity::PlayerId> = None;
    {
        let active: Vec<PlayerView<'_>> = outcome
            .repo
            .skaters(season_key, season_type)
            .chain(outcome.repo.goalies(season_key, season_type))
            .collect();
        if find_signal_view(&active, name).is_none() {
            drop(active);
            if let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(name) {
                let pid = icelines_core::identity::PlayerId(pid);
                let _ = icelines_fetch::stats_loader::load_player_career_into_repo(
                    &mut outcome.repo,
                    pid,
                );
                historical_pid = Some(pid);
            }
        }
    }

    let repo = &outcome.repo;
    let mut all: Vec<PlayerView<'_>> = repo
        .skaters(season_key, season_type)
        .chain(repo.goalies(season_key, season_type))
        .collect();
    if let Some(pid) = historical_pid {
        if let Some(career) = repo.career_all(pid) {
            let career: Vec<_> = career.collect();
            if let Some(last) = career.last() {
                if let Some(v) = repo.view(pid, last.season, last.season_type) {
                    all.push(v);
                }
            }
        }
    }

    let v = find_signal_view(&all, name)
        .with_context(|| format!("player '{name}' not found — try a partial name"))?;
    let ctx = PlayerSignalsView::context_for_player(v);
    Ok(PlayerSignalsView::from_player(ctx, v))
}

fn find_signal_view<'a, 'r>(views: &'a [PlayerView<'r>], name: &str) -> Option<&'a PlayerView<'r>> {
    let needle = name.trim().to_lowercase();
    views
        .iter()
        .find(|v| v.identity.full_name.to_lowercase().contains(&needle))
}

#[derive(Debug, serde::Serialize)]
struct SignalsRosterView {
    schema_note: &'static str,
    team: String,
    season: u32,
    season_type: String,
    rows: Vec<PlayerSignalsView>,
    disclosures: Vec<String>,
    non_claims: Vec<String>,
    source_authority: SignalsSourceAuthority,
}

fn build_roster_view(
    team: &str,
    season: Option<&str>,
    season_type: SeasonType,
) -> anyhow::Result<SignalsRosterView> {
    let team_abbr = TeamAbbr::parse(team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;
    let (outcome, season_key, season_type) =
        crate::commands::players::load_repo_for_season(season, Some(season_type))?;

    let mut rows: Vec<PlayerSignalsView> = outcome
        .repo
        .skaters(season_key, season_type)
        .filter(|player| player.team_display() == team_abbr.0)
        .map(|player| {
            PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player)
        })
        .collect();
    rows.sort_by(|a, b| {
        a.player_name
            .cmp(&b.player_name)
            .then_with(|| a.player_id.cmp(&b.player_id))
    });

    if rows.is_empty() {
        anyhow::bail!(
            "no skaters found for {} in {} {}",
            team_abbr.0,
            season_key.0,
            season_type.label()
        );
    }

    Ok(SignalsRosterView {
        schema_note: "Team-scoped Signals discovery matrix; not a leaderboard.",
        team: team_abbr.0,
        season: season_key.0,
        season_type: season_type.label().to_string(),
        rows,
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
    })
}

// ── text rendering ────────────────────────────────────────────────────────────

fn print_signals_text(view: &PlayerSignalsView) {
    println!(
        "SIGNALS — {} ({} · {} · {} GP · {} {})",
        view.player_name,
        view.team,
        view.position,
        view.games_played,
        view.context.window.season.0,
        view.context.window.season_type.label(),
    );
    println!("{}", "═".repeat(72));

    for row in &view.rows {
        let arrow = polarity_arrow(row.polarity);
        let unit = unit_label(row.unit);
        let tier = tier_label(row.evidence_tier);
        match row.value {
            Some(value) => {
                println!(
                    "  {arrow} {:<30} {:>8} {unit}   [evidence: {tier}]",
                    row.label,
                    format_value(Some(value)),
                );
            }
            None => {
                println!(
                    "  {arrow} {:<30} {:>8} {unit}   [evidence: {tier}; missing: {}]",
                    row.label,
                    format_value(None),
                    missing_inputs_label(&row.missing_inputs),
                );
            }
        }
    }

    println!();
    println!("Methodology:");
    for row in &view.rows {
        println!("  • {:<8} {}", row.short_label, row.methodology);
    }
    println!();
    println!("Limitations:");
    for row in &view.rows {
        println!("  • {:<8} {}", row.short_label, row.limitations);
    }

    println!();
    for disclosure in &view.disclosures {
        println!("Note: {disclosure}");
    }
    for non_claim in &view.non_claims {
        println!("Disclaimer: {non_claim}");
    }
    println!();
    println!("Legend: ↑ higher is better · ↓ lower is better · = neutral");
}

fn print_signals_roster_text(view: &SignalsRosterView) {
    println!(
        "SIGNALS ROSTER — {} ({} {})",
        view.team, view.season, view.season_type
    );
    println!("{}", "=".repeat(96));
    println!("{}", view.schema_note);
    println!("Authority: {}", view.source_authority.label);
    for disclosure in &view.disclosures {
        println!("Note: {disclosure}");
    }
    for non_claim in &view.non_claims {
        println!("Disclaimer: {non_claim}");
    }
    println!();
    println!(
        "{:<26} {:<4} {:>3} {:>16} {:>16} {:>16} Evidence",
        "Player", "Pos", "GP", "Phys/60", "PMD/60", "PIM/60"
    );
    println!("{}", "-".repeat(112));
    for row in &view.rows {
        let phys = signal_cell(row, "physical-engagement-rate");
        let pmd = signal_cell(row, "puck-management-differential");
        let pim = signal_cell(row, "penalty-drag-rate");
        println!(
            "{:<26} {:<4} {:>3} {:>16} {:>16} {:>16} {}",
            truncate(&row.player_name, 26),
            row.position,
            row.games_played,
            phys.value,
            pmd.value,
            pim.value,
            row_evidence_summary(row)
        );
    }
    println!();
    println!("Legend: unavailable means missing/below-threshold evidence, never zero value truth.");
}

struct SignalCell {
    value: String,
}

fn signal_cell(view: &PlayerSignalsView, key: &str) -> SignalCell {
    let row = view
        .rows
        .iter()
        .find(|row| row.cli_key == key)
        .expect("current signal key");
    SignalCell {
        value: match row.value {
            Some(value) => format!(
                "{} {}",
                format_value(Some(value)),
                tier_label(row.evidence_tier)
            ),
            None => format!("unavailable {}", tier_label(row.evidence_tier)),
        },
    }
}

fn row_evidence_summary(view: &PlayerSignalsView) -> String {
    let mut parts: Vec<String> = Vec::new();
    for row in &view.rows {
        if row.evidence_tier != SignalEvidenceTier::Full || !row.missing_inputs.is_empty() {
            parts.push(format!(
                "{}: {} missing {}",
                row.short_label,
                tier_label(row.evidence_tier),
                missing_inputs_label(&row.missing_inputs)
            ));
        }
    }
    if parts.is_empty() {
        "all full".to_string()
    } else {
        parts.join("; ")
    }
}

fn truncate(value: &str, width: usize) -> String {
    let count = value.chars().count();
    if count <= width {
        value.to_string()
    } else {
        let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Format a Signal value to two decimals, or `unavailable` for missing evidence.
/// A missing value is NEVER rendered as `0.00` (spec §Evidence contract).
fn format_value(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v:.2}"),
        None => "unavailable".to_string(),
    }
}

fn polarity_arrow(polarity: SignalPolarity) -> char {
    match polarity {
        SignalPolarity::HigherIsBetter => '↑',
        SignalPolarity::LowerIsBetter => '↓',
        SignalPolarity::Neutral => '=',
    }
}

fn unit_label(unit: SignalMetricUnit) -> &'static str {
    match unit {
        SignalMetricUnit::Per60 => "per 60",
    }
}

fn tier_label(tier: SignalEvidenceTier) -> &'static str {
    match tier {
        SignalEvidenceTier::Full => "full",
        SignalEvidenceTier::Partial => "partial",
        SignalEvidenceTier::Missing => "missing",
    }
}

fn input_label(input: SignalInput) -> &'static str {
    match input {
        SignalInput::SampleSize => "sample size",
        SignalInput::Realtime => "realtime",
        SignalInput::IceTime => "ice time",
    }
}

fn missing_inputs_label(inputs: &[SignalInput]) -> String {
    if inputs.is_empty() {
        return "none".to_string();
    }
    inputs
        .iter()
        .map(|i| input_label(*i))
        .collect::<Vec<_>>()
        .join(", ")
}

// ── JSON envelope (frozen signals.v1) ─────────────────────────────────────────

fn signals_json_envelope(view: &PlayerSignalsView) -> String {
    let data = serde_json::to_value(view).unwrap_or(serde_json::Value::Null);
    let envelope = serde_json::json!({
        "schema": "signals.v1",
        "schema_version": 1,
        "route": "signals",
        "data": data,
        "meta": {
            "season": view.context.window.season.0.to_string(),
            "season_type": view.context.window.season_type.label(),
            "player_id": view.player_id,
            "player_name": view.player_name,
            "signal_count": view.rows.len(),
            "source_authority": &view.source_authority,
        },
    });
    serde_json::to_string_pretty(&envelope).unwrap_or_default()
}

fn signals_roster_json_envelope(view: &SignalsRosterView) -> String {
    let data = serde_json::to_value(view).unwrap_or(serde_json::Value::Null);
    let envelope = serde_json::json!({
        "schema": "signals-roster.v1",
        "schema_version": 1,
        "route": "signals-roster",
        "data": data,
        "meta": {
            "season": view.season.to_string(),
            "season_type": view.season_type,
            "team": view.team,
            "player_count": view.rows.len(),
            "non_promotion": "team-scoped discovery matrix; not a leaderboard, StatId, filter, or cache metric family",
            "source_authority": &view.source_authority,
        },
    });
    serde_json::to_string_pretty(&envelope).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_signals_format_value_never_zero_fills_missing() {
        // Missing evidence must read as "unavailable", not a 0.0 player value.
        assert_eq!(format_value(None), "unavailable");
        assert_ne!(format_value(None), "0.00");
        assert_eq!(format_value(Some(9.0)), "9.00");
        assert_eq!(format_value(Some(0.6)), "0.60");
    }

    #[test]
    fn l0_signals_polarity_arrows_are_distinct() {
        assert_eq!(polarity_arrow(SignalPolarity::HigherIsBetter), '↑');
        assert_eq!(polarity_arrow(SignalPolarity::LowerIsBetter), '↓');
        assert_eq!(polarity_arrow(SignalPolarity::Neutral), '=');
    }

    #[test]
    fn l0_signals_tier_labels_map() {
        assert_eq!(tier_label(SignalEvidenceTier::Full), "full");
        assert_eq!(tier_label(SignalEvidenceTier::Partial), "partial");
        assert_eq!(tier_label(SignalEvidenceTier::Missing), "missing");
    }

    #[test]
    fn l0_signals_missing_inputs_label_joins_or_none() {
        assert_eq!(missing_inputs_label(&[]), "none");
        assert_eq!(
            missing_inputs_label(&[SignalInput::Realtime, SignalInput::IceTime]),
            "realtime, ice time"
        );
    }

    #[test]
    fn l0_signals_roster_row_evidence_names_missing_inputs() {
        let repo = icelines_core::fixtures::test_repo_with(
            icelines_core::fixtures::identity(8478402).build(),
            icelines_core::fixtures::stats(8478402, 20252026, "EDM").build(),
        );
        let player = repo
            .view(
                icelines_core::identity::PlayerId(8478402),
                icelines_core::model::Season(20252026),
                SeasonType::Regular,
            )
            .expect("player view");
        let view =
            PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player);

        let summary = row_evidence_summary(&view);
        assert!(summary.contains("Phys/60: partial missing realtime"));
        assert!(summary.contains("PMD/60: partial missing realtime"));
        assert!(signal_cell(&view, "physical-engagement-rate")
            .value
            .contains("unavailable partial"));
    }

    #[test]
    fn l0_signals_roster_json_meta_carries_source_authority() {
        let row = roster_signal_row();
        let view = SignalsRosterView {
            schema_note: "Team-scoped Signals discovery matrix; not a leaderboard.",
            team: "EDM".to_string(),
            season: 20252026,
            season_type: "regular".to_string(),
            rows: vec![row],
            disclosures: Vec::new(),
            non_claims: Vec::new(),
            source_authority: SignalsSourceAuthority::default(),
        };

        let json: serde_json::Value =
            serde_json::from_str(&signals_roster_json_envelope(&view)).expect("valid json");
        assert_eq!(
            json["meta"]["source_authority"]["coverage_state"],
            serde_json::json!("descriptive_derived")
        );
        assert_eq!(
            json["meta"]["source_authority"]["blocked_claims"],
            serde_json::json!([
                "prediction",
                "betting_edge",
                "injury_signal",
                "deployment_recommendation",
                "player_quality_grade",
                "autonomous_coaching_decision",
                "stat_catalog_promotion",
                "leaderboard_ranking"
            ])
        );
        assert_eq!(
            json["data"]["source_authority"]["covered_metrics"],
            serde_json::json!([
                "physical_engagement_rate",
                "puck_management_differential",
                "penalty_drag_rate"
            ])
        );
    }

    #[test]
    fn l0_signals_roster_text_authority_line_uses_shared_label() {
        let authority = SignalsSourceAuthority::default();
        assert!(authority.label.contains("Signals authority"));
        assert!(authority
            .blocked_claims
            .contains(&"leaderboard_ranking".to_string()));
    }

    fn roster_signal_row() -> PlayerSignalsView {
        let repo = icelines_core::fixtures::test_repo_with(
            icelines_core::fixtures::identity(8478402).build(),
            icelines_core::fixtures::stats(8478402, 20252026, "EDM").build(),
        );
        let player = repo
            .view(
                icelines_core::identity::PlayerId(8478402),
                icelines_core::model::Season(20252026),
                SeasonType::Regular,
            )
            .expect("player view");
        PlayerSignalsView::from_player(PlayerSignalsView::context_for_player(&player), &player)
    }
}
