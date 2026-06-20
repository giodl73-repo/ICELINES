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
use icelines_core::season_stats::SeasonType;
use icelines_core::signal_metrics::{
    SignalEvidenceTier, SignalInput, SignalMetricUnit, SignalPolarity,
};
use icelines_core::stats_repository::PlayerView;
use icelines_core::view_model::signals::PlayerSignalsView;

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
}
