//! Read-only CLI projection of the UI-neutral card document.
//!
//! This adapter imports document grammar only. It contains no roster scoring,
//! season simulation, scenario, or player-impact builder.

use std::fmt::Write as _;

use icelines_core::{CardDocumentView, CardSectionView, MetricValue};

pub fn render_card(card: &CardDocumentView) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "{}", card.title.to_ascii_uppercase());
    if let Some(subtitle) = &card.subtitle {
        let _ = writeln!(text, "{subtitle}");
    }
    for page in &card.pages {
        let _ = writeln!(text);
        let _ = writeln!(
            text,
            "PAGE {} — {}",
            page.order,
            page.display_label.as_deref().unwrap_or(&page.literal_label)
        );
        for section in &page.sections {
            match section {
                CardSectionView::IdentityHeader(_) => {}
                CardSectionView::Lineup(lineup) => {
                    for group in &lineup.groups {
                        let players = group
                            .slots
                            .iter()
                            .map(|slot| {
                                let name = slot.subject_label.as_deref().unwrap_or("Open");
                                let score = slot
                                    .metrics
                                    .first()
                                    .map(|metric| metric.display_text.as_str())
                                    .unwrap_or("—");
                                format!("{} {name} ({score})", slot.label)
                            })
                            .collect::<Vec<_>>()
                            .join(" · ");
                        let _ = writeln!(text, "{:<10} {players}", group.label);
                    }
                }
                CardSectionView::MetricStrip(section) => {
                    if let Some(title) = &section.title {
                        let _ = writeln!(text, "{title}");
                    }
                    for metric in &section.metrics {
                        let _ = writeln!(
                            text,
                            "  {:<30} {}",
                            metric.metric.label, metric.display_text
                        );
                    }
                }
                CardSectionView::ProbabilityRange(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for range in &section.ranges {
                        let _ = writeln!(text, "  {:<30} {}", range.label, range.display_text);
                    }
                }
                CardSectionView::ScenarioBridge(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for metric in &section.metrics {
                        let delta =
                            metric
                                .comparison
                                .as_ref()
                                .map_or(String::new(), |comparison| match &comparison.delta {
                                    MetricValue::Decimal(value) => format!(" ({value:+.2})"),
                                    MetricValue::Integer(value) => format!(" ({value:+})"),
                                    _ => String::new(),
                                });
                        let _ = writeln!(
                            text,
                            "  {:<30} {}{delta}",
                            metric.metric.label, metric.display_text
                        );
                    }
                }
                CardSectionView::PlayerList(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for row in &section.rows {
                        let _ = writeln!(
                            text,
                            "  {}{}",
                            row.name,
                            row.role
                                .as_ref()
                                .map_or(String::new(), |role| format!(" — {role}"))
                        );
                        for metric in &row.metrics {
                            let _ = writeln!(
                                text,
                                "    {:<28} {}",
                                metric.metric.label, metric.display_text
                            );
                        }
                    }
                }
                CardSectionView::StateNotice(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for warning in &section.warnings {
                        let _ = writeln!(text, "  WARNING: {}", warning.message);
                    }
                }
                CardSectionView::Methodology(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for limitation in &section.limitations {
                        let _ = writeln!(text, "  - {limitation}");
                    }
                }
                CardSectionView::Provenance(_) => {}
                CardSectionView::Decision(section) => {
                    let _ = writeln!(text, "{}: {}", section.title, section.recommendation);
                    for reason in &section.rationale {
                        let _ = writeln!(text, "  {reason}");
                    }
                    for alternative in &section.alternatives {
                        let _ = writeln!(text, "  - {}", alternative.label);
                        if let Some(detail) = &alternative.detail {
                            let _ = writeln!(text, "    {detail}");
                        }
                    }
                }
                CardSectionView::Timeline(section) => {
                    let _ = writeln!(text, "{}", section.title);
                    for item in &section.items {
                        let _ = writeln!(text, "  {} — {}", item.effective_at, item.label);
                    }
                }
            }
        }
    }
    wrap_card_text(&text, 80)
}

pub fn render_team_card(card: &CardDocumentView) -> String {
    render_card(card)
}

fn wrap_card_text(input: &str, width: usize) -> String {
    let mut output = String::new();
    for line in input.lines() {
        let indent = line
            .chars()
            .take_while(|character| *character == ' ')
            .count();
        let continuation = " ".repeat(indent.min(width.saturating_sub(1)));
        let mut remaining = line.to_string();
        loop {
            if remaining.chars().count() <= width {
                let _ = writeln!(output, "{remaining}");
                break;
            }
            let byte_limit = remaining
                .char_indices()
                .nth(width)
                .map_or(remaining.len(), |(index, _)| index);
            let candidate = &remaining[..byte_limit];
            let split = candidate
                .rfind(char::is_whitespace)
                .filter(|index| *index > indent)
                .unwrap_or(byte_limit);
            let _ = writeln!(output, "{}", remaining[..split].trim_end());
            let rest = remaining[split..].trim_start().to_string();
            if rest.is_empty() {
                break;
            }
            remaining = format!("{continuation}{rest}");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::wrap_card_text;

    #[test]
    fn text_wraps_to_eighty_columns_and_preserves_indent() {
        let input = format!("  {}", ["Kartye"; 20].join(" · "));
        let wrapped = wrap_card_text(&input, 80);
        assert!(wrapped.lines().all(|line| line.chars().count() <= 80));
        assert!(wrapped.lines().skip(1).all(|line| line.starts_with("  ")));
    }

    #[test]
    fn renderer_has_no_scoring_or_simulation_builder_imports() {
        let source = include_str!("card_renderer.rs");
        let adapter = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "build_team_ceiling",
            "build_team_lineup_projection",
            "build_team_prognosis_card",
            "simulate_team_season_forecast",
            "team_ceiling_player_lens_score",
        ] {
            assert!(!adapter.contains(forbidden), "renderer imports {forbidden}");
        }
    }
}
