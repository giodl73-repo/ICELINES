//! HTML fragment generation for lineup cards.
//! Produces markdown-embedded HTML matching the existing fantasy.css contract.

use icelines_core::{
    cross_team::{CrossTeamMetrics, WebFitClass},
    view_model::{DepthPlayerSlot, MetricValue},
};

pub fn player_cell(slot: Option<&DepthPlayerSlot>, metrics: Option<&CrossTeamMetrics>) -> String {
    let Some(s) = slot else {
        return r#"<td class="player-cell empty"><span class="player-name">—</span></td>"#
            .to_owned();
    };

    let (css, label) = match metrics {
        Some(m) => {
            let cls = m.web_fit_class().css_class();
            let avg = m.avg_other_line;
            let lbl = match m.web_fit_class() {
                WebFitClass::Elite => format!("★ avg L{avg:.1}"),
                WebFitClass::Solid => format!("~ avg L{avg:.1}"),
                WebFitClass::Buried => format!("↑ avg L{avg:.1} — underused"),
                WebFitClass::Stretch => format!("↓ avg L{avg:.1} — overextended"),
            };
            (cls, lbl)
        }
        None => ("fit", "no metrics".to_owned()),
    };

    let stat_str = match (
        decimal_metric(s, "pace_82"),
        decimal_metric(s, "goals_per_82"),
        integer_metric(s, "gp"),
    ) {
        (Some(pace), Some(gpz), Some(gp)) => {
            let ppg = pace / 82.0;
            let gpg = gpz / 82.0;
            format!(
                r#"<span class="player-gp">{gp}gp &nbsp;·&nbsp; <b>{ppg:.2}</b> pts/gp &nbsp;·&nbsp; {gpg:.2} g/gp &nbsp;·&nbsp; {pace:.0} proj</span>"#
            )
        }
        _ => r#"<span class="player-gp">no pace data</span>"#.to_owned(),
    };

    let photo = s.headshot_canonical_url.as_deref().unwrap_or("");
    let photo_tag = if photo.is_empty() {
        String::new()
    } else {
        format!(
            r#"<img class="player-photo" src="{photo}" alt="{name}" onerror="this.style.display='none'">"#,
            name = s.display_name
        )
    };

    format!(
        r#"<td class="player-cell {css}">{photo}<span class="player-name">{name}</span>{stat}<span class="player-fit-label">{label}</span></td>"#,
        css = css,
        photo = photo_tag,
        name = s.display_name,
        stat = stat_str,
        label = label,
    )
}

fn decimal_metric(slot: &DepthPlayerSlot, key: &str) -> Option<f64> {
    slot.metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            MetricValue::Integer(value) => Some(value as f64),
            MetricValue::Text(_) | MetricValue::Missing => None,
        })
}

fn integer_metric(slot: &DepthPlayerSlot, key: &str) -> Option<i64> {
    slot.metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => Some(value),
            MetricValue::Decimal(value) => Some(value.round() as i64),
            MetricValue::Text(_) | MetricValue::Missing => None,
        })
}

pub fn team_logo_url(team: &str) -> String {
    let nhl_abbrev = match team {
        "LA" => "LAK",
        "NJ" => "NJD",
        "TB" => "TBL",
        "SJ" => "SJS",
        other => other,
    };
    format!("https://assets.nhle.com/logos/nhl/svg/{nhl_abbrev}_light.svg")
}

pub fn bar_fill(pts: f32, max_pts: f32, width: u32) -> u32 {
    if max_pts <= 0.0 {
        return 0;
    }
    ((pts / max_pts * width as f32) as u32).min(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{
        cross_team::{CrossTeamMetrics, WebFitClass},
        identity::PlayerId,
        model::{Position, TeamAbbr},
        view_model::{
            DeploymentEvidence, DepthPlayerSlot, DepthSlotKind, MetricCell, MetricUnit,
            MetricValue, SemanticToken, StatKey, ValuePrecision,
        },
    };

    fn slot(name: &str, pace: Option<f64>, gp: Option<i64>) -> DepthPlayerSlot {
        DepthPlayerSlot {
            player_id: PlayerId(8478402),
            display_name: name.to_owned(),
            team: TeamAbbr("EDM".into()),
            headshot_canonical_url: None,
            slot: DepthSlotKind::Extra,
            position: Position::Center,
            evidence: DeploymentEvidence::Estimated,
            metrics: vec![
                decimal_metric_cell("pace_82", pace),
                decimal_metric_cell("goals_per_82", pace.map(|p| p * 0.4)),
                MetricCell {
                    key: StatKey::from("gp"),
                    label: "GP".to_owned(),
                    value: gp.map(MetricValue::Integer).unwrap_or(MetricValue::Missing),
                    unit: MetricUnit::Games,
                    precision: ValuePrecision::Integer,
                    token: None,
                },
            ],
            tokens: vec![SemanticToken::SupportingEvidence],
        }
    }

    fn decimal_metric_cell(key: &str, value: Option<f64>) -> MetricCell {
        MetricCell {
            key: StatKey::from(key),
            label: key.to_owned(),
            value: value
                .map(MetricValue::Decimal)
                .unwrap_or(MetricValue::Missing),
            unit: MetricUnit::Per82,
            precision: ValuePrecision::OneDecimal,
            token: None,
        }
    }

    /// Build a CrossTeamMetrics whose `web_fit_class()` resolves to the
    /// requested class. Encodes the thresholds in cross_team.rs:
    /// Buried: own - avg > 0.75
    /// Elite:  avg <= own + 0.5
    /// Solid:  avg <= own + 1.25
    /// Stretch: else
    fn metrics_for(class: WebFitClass) -> CrossTeamMetrics {
        let (own, avg) = match class {
            WebFitClass::Buried => (4, 2.0), // delta +2.0
            WebFitClass::Elite => (1, 1.0),
            WebFitClass::Solid => (1, 2.0),
            WebFitClass::Stretch => (1, 3.0),
        };
        let m = CrossTeamMetrics {
            player_nhl_id: Some(8478402),
            own_line: own,
            avg_other_line: avg,
            delta: own as f32 - avg,
        };
        debug_assert_eq!(
            m.web_fit_class(),
            class,
            "fixture must resolve to the requested class"
        );
        m
    }

    // ── team_logo_url ────────────────────────────────────────────────────────

    /// Yahoo / ESPN abbrev variants must map to their NHL canonical form
    /// before being substituted into the assets URL. The four legacy
    /// abbrevs LA / NJ / TB / SJ are the full rewrite set.
    #[test]
    fn l0_team_logo_url_rewrites_legacy_abbrevs() {
        assert!(team_logo_url("LA").contains("/LAK_light.svg"));
        assert!(team_logo_url("NJ").contains("/NJD_light.svg"));
        assert!(team_logo_url("TB").contains("/TBL_light.svg"));
        assert!(team_logo_url("SJ").contains("/SJS_light.svg"));
    }

    /// Canonical 3-letter abbrevs pass through unchanged.
    #[test]
    fn l0_team_logo_url_passes_canonical_abbrevs_through() {
        assert!(team_logo_url("EDM").ends_with("/EDM_light.svg"));
        assert!(team_logo_url("WPG").ends_with("/WPG_light.svg"));
    }

    // ── bar_fill ─────────────────────────────────────────────────────────────

    /// Zero (or negative) max yields zero fill — must not divide-by-zero
    /// or produce NaN that would corrupt the inline-style integer.
    #[test]
    fn l0_bar_fill_zero_max_returns_zero() {
        assert_eq!(bar_fill(50.0, 0.0, 120), 0);
        assert_eq!(bar_fill(50.0, -1.0, 120), 0);
    }

    /// Half the max → half the width (within integer truncation).
    #[test]
    fn l0_bar_fill_half_proportional() {
        assert_eq!(bar_fill(50.0, 100.0, 120), 60);
    }

    /// pts ≥ max must clamp to width — no overflow into the next column.
    #[test]
    fn l0_bar_fill_clamps_to_width_when_pts_exceed_max() {
        assert_eq!(bar_fill(200.0, 100.0, 120), 120);
        assert_eq!(bar_fill(100.0, 100.0, 120), 120);
    }

    // ── player_cell ──────────────────────────────────────────────────────────

    /// None slot renders the empty placeholder cell — the empty class
    /// hook is what the CSS targets to dim the row.
    #[test]
    fn l0_player_cell_none_renders_empty_placeholder() {
        let html = player_cell(None, None);
        assert!(html.contains(r#"class="player-cell empty""#));
        assert!(html.contains(">—<"));
    }

    /// Filled slot with all pace fields renders gp / pts/gp / projection.
    /// Asserts both the value formatting (2-decimal ppg, integer pace)
    /// and the wrapping spans the CSS uses.
    #[test]
    fn l0_player_cell_filled_emits_pace_block() {
        let s = slot("Connor McDavid", Some(140.0), Some(82));
        let html = player_cell(Some(&s), None);
        assert!(html.contains("Connor McDavid"));
        assert!(html.contains("82gp"));
        // 140 / 82 ≈ 1.71 ppg
        assert!(html.contains("1.71"));
        assert!(html.contains("140"));
        assert!(html.contains(r#"class="player-name""#));
    }

    /// Filled slot whose pace fields are all None falls back to the
    /// "no pace data" hint instead of formatting NaN/zero.
    #[test]
    fn l0_player_cell_filled_no_pace_uses_fallback_string() {
        let s = slot("Sample Player", None, None);
        let html = player_cell(Some(&s), None);
        assert!(html.contains("no pace data"));
        assert!(!html.contains("pts/gp"));
    }

    /// Metrics block sets the css class + label per WebFitClass — locks
    /// the icon mapping (★ / ~ / ↑ / ↓) the CSS keys off.
    #[test]
    fn l0_player_cell_metrics_emits_class_and_icon() {
        let s = slot("Connor McDavid", Some(140.0), Some(82));

        let elite = player_cell(Some(&s), Some(&metrics_for(WebFitClass::Elite)));
        assert!(elite.contains("fit-elite"), "Elite must use fit-elite");
        assert!(elite.contains('★'), "Elite must use ★");

        let solid = player_cell(Some(&s), Some(&metrics_for(WebFitClass::Solid)));
        assert!(solid.contains("fit-solid"), "Solid must use fit-solid");

        let buried = player_cell(Some(&s), Some(&metrics_for(WebFitClass::Buried)));
        assert!(buried.contains("fit-buried"), "Buried must use fit-buried");
        assert!(buried.contains('↑'), "Buried must use ↑");
        assert!(buried.contains("underused"));

        let stretch = player_cell(Some(&s), Some(&metrics_for(WebFitClass::Stretch)));
        assert!(
            stretch.contains("fit-stretch"),
            "Stretch must use fit-stretch"
        );
        assert!(stretch.contains('↓'), "Stretch must use ↓");
        assert!(stretch.contains("overextended"));
    }
}
