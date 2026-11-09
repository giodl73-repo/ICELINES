//! Descriptive playing methods. Missing behavior is never a low skill grade.
use super::rinkcraft_numbers::{NumericalCatalog, NumericalProfile, Source};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementSnapshot {
    pub player_id: u32,
    pub season: u32,
    pub games: u32,
    pub max_mph: Option<f64>,
    pub bursts_over_20: Option<f64>,
    pub distance_miles: Option<f64>,
    pub source: Source,
    pub accessed: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleEvidence {
    pub protocol: String,
    pub season: u32,
    pub measured: Vec<StyleMeasure>,
    pub gaps: Vec<StyleGap>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleMeasure {
    pub key: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub numerator: f64,
    pub denominator: f64,
    pub scope: String,
    pub meaning: String,
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleGap {
    pub axis: String,
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleComparison {
    pub schema: String,
    pub rows: Vec<StyleComparisonRow>,
    pub shared_gaps: Vec<StyleGap>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleComparisonRow {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub left: Option<StyleMeasure>,
    pub right: Option<StyleMeasure>,
    pub difference: Option<f64>,
    pub status: String,
    pub reason: String,
}
/// Compare known descriptive definitions; never fill a missing method with zero.
pub fn compare(
    left: &super::rinkcraft_numbers::EvidenceCard,
    right: &super::rinkcraft_numbers::EvidenceCard,
) -> Option<StyleComparison> {
    let a = left.style_evidence.as_ref();
    let b = right.style_evidence.as_ref();
    if a.is_none() && b.is_none() {
        return None;
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for m in a
        .into_iter()
        .flat_map(|s| &s.measured)
        .chain(b.into_iter().flat_map(|s| &s.measured))
    {
        if !keys.insert(&m.key) {
            continue;
        }
        let x = a.and_then(|s| s.measured.iter().find(|v| v.key == m.key));
        let y = b.and_then(|s| s.measured.iter().find(|v| v.key == m.key));
        let (status,reason) = match (x,y) {
            (Some(x),Some(y)) if contexts_match(left,right,x,y) => ("matched","Same descriptive definition and compatible source scope; difference is left minus right."),
            (Some(_),Some(_)) => ("incompatible","Definition, unit, situation or source frame differs; no difference is computed."),
            (Some(_),None) => ("left_only","Only the left player has this measurement; the right value is unknown."),
            (None,Some(_)) => ("right_only","Only the right player has this measurement; the left value is unknown."),
            _ => unreachable!(),
        };
        rows.push(StyleComparisonRow {
            key: m.key.clone(),
            label: m.label.clone(),
            unit: m.unit.clone(),
            left: x.cloned(),
            right: y.cloned(),
            difference: if status == "matched" {
                x.zip(y).map(|(x, y)| x.value - y.value)
            } else {
                None
            },
            status: status.into(),
            reason: reason.into(),
        });
    }
    let shared_gaps = a
        .into_iter()
        .flat_map(|s| &s.gaps)
        .filter(|g| b.is_some_and(|s| s.gaps.iter().any(|v| v.axis == g.axis)))
        .cloned()
        .collect();
    Some(StyleComparison {
        schema: "icelines.rinkcraft-style-comparison.v1".into(),
        rows,
        shared_gaps,
    })
}
fn contexts_match(
    left: &super::rinkcraft_numbers::EvidenceCard,
    right: &super::rinkcraft_numbers::EvidenceCard,
    x: &StyleMeasure,
    y: &StyleMeasure,
) -> bool {
    if left.season != right.season || left.group != right.group || x.unit != y.unit {
        return false;
    }
    let expected = match x.key.as_str() {
        "high-danger-shot-share" => Some(("Pct", "5on5 · provider-defined unblocked attempts")),
        "contact-frequency" => Some(("Per60", "5on5 · recorded hits / ice time")),
        _ => None,
    };
    if let Some((unit, scope)) = expected {
        let fingerprints = |m: &StyleMeasure| {
            m.sources
                .iter()
                .map(|s| (s.url.clone(), s.sha256.clone()))
                .collect::<std::collections::BTreeSet<_>>()
        };
        return x.unit == unit
            && x.scope == scope
            && y.scope == scope
            && fingerprints(x) == fingerprints(y);
    }
    let unit = match x.key.as_str() {
        "max-skating-speed" => "Mph",
        "speed-bursts-per-game" => "PerGame",
        "distance-per-game" => "MilesPerGame",
        _ => return false,
    };
    let valid = |card: &super::rinkcraft_numbers::EvidenceCard, m: &StyleMeasure| {
        m.scope.starts_with("All strengths · ")
            && m.sources.len() == 1
            && m.sources[0].url
                == format!(
                    "https://api-web.nhle.com/v1/edge/skater-detail/{}/{}/2",
                    card.player_id, card.season
                )
    };
    x.unit == unit && valid(left, x) && valid(right, y)
}
#[allow(clippy::too_many_arguments)] // A table-like definition plus its source components.
fn measure(
    key: &str,
    label: &str,
    n: f64,
    d: f64,
    unit: &str,
    scope: &str,
    meaning: &str,
    sources: &[Source],
) -> Option<StyleMeasure> {
    (n.is_finite() && n >= 0. && d.is_finite() && d > 0.).then(|| StyleMeasure {
        key: key.into(),
        label: label.into(),
        value: n / d,
        unit: unit.into(),
        numerator: n,
        denominator: d,
        scope: scope.into(),
        meaning: meaning.into(),
        sources: sources.to_vec(),
    })
}
fn profile_style(
    p: &NumericalProfile,
    season: u32,
    sources: &[Source],
    movement: Option<&MovementSnapshot>,
) -> StyleEvidence {
    let mut measured = Vec::new();
    let mut gaps = Vec::new();
    let mut gap = |axis: &str, reason: &str| {
        gaps.push(StyleGap {
            axis: axis.into(),
            reason: reason.into(),
        })
    };
    if p.group == "Goalies" {
        gap(
            "Depth & movement",
            "Saving outcomes do not measure depth choice, lateral technique or recovery mechanics.",
        );
        gap(
            "Rebound method",
            "No inspected rebound-direction or decision denominator.",
        );
        gap(
            "Puck exchanges",
            "No retrieval, leave/pass choice or completion records.",
        );
    } else {
        let high = p
            .metrics
            .iter()
            .find(|m| m.key == "high-danger-per-60")
            .and_then(|m| m.numerator);
        let attempts = p
            .metrics
            .iter()
            .find(|m| m.key == "shot-quality")
            .and_then(|m| m.denominator);
        if let Some(m) = high.zip(attempts).filter(|(n,d)| n <= d).and_then(|(n,d)| measure(
            "high-danger-shot-share", "High-danger attempt share", n, d, "Pct", "5on5 · provider-defined unblocked attempts",
            "Fraction of this player's unblocked attempts in MoneyPuck's high-danger category. Shot selection, not net-front occupation, screening or finishing skill.", sources)) { measured.push(m); }
        if let Some(m) = p.metrics.iter().find(|m| m.key == "contact-per-60") {
            if let Some(n) = m.numerator.zip(m.denominator).and_then(|(n,d)| measure(
                "contact-frequency", "Recorded contact frequency", n * 3600., d, "Per60", "5on5 · recorded hits / ice time",
                "Contact involvement; does not measure pressure, puck wins, strength or contact success.", sources)) { measured.push(n); }
        }
        if let Some(m) = movement {
            let src = std::slice::from_ref(&m.source);
            let scope = format!(
                "All strengths · {} GP · NHL EDGE · captured {}",
                m.games, m.accessed
            );
            for v in [
                m.max_mph.and_then(|n| measure("max-skating-speed", "Maximum skating speed", n, 1., "Mph", &scope, "Observed season maximum; puck possession and route direction are not supplied. More exposure gives more chances to record a maximum.", src)),
                m.bursts_over_20.and_then(|n| measure("speed-bursts-per-game", "20+ mph bursts / game", n, m.games as f64, "PerGame", &scope, "Recorded bursts divided by games played; not bursts per skating minute, puck-carrying pace or acceleration.", src)),
                m.distance_miles.and_then(|n| measure("distance-per-game", "Skating miles / game", n, m.games as f64, "MilesPerGame", &scope, "Distance divided by games played; includes deployment and movement without the puck, not transport skill.", src)),
            ].into_iter().flatten() { measured.push(v); }
        } else {
            gap(
                "Skating pace",
                "No retained, source-verified tracking snapshot for this player and season.",
            );
        }
        gap(
            "Carry / pass / dump",
            "Needs entry and exit attempts by method, including failures and resolved control.",
        );
        gap(
            "North–south / east–west",
            "Needs direction relative to attack, by phase; both can occur in one play.",
        );
        gap("Puck hunting & recovery", "Needs pursuit opportunities, pressure actions and separate recoveries; hits and takeaways are insufficient.");
        gap("Passing & deception", "Needs passing attempts, destinations, pressure and reception context; assists are outcomes.");
        if measured.is_empty() {
            gap(
                "Recorded behavior",
                "No valid shot/contact denominator in this snapshot.",
            );
        }
    }
    StyleEvidence { protocol: "rinkcraft.playing-methods.0.1.0-experimental".into(), season, measured, gaps,
        limitations: vec!["Descriptive methods and involvement; no ability grade, role probability or automatic class change.".into(),
        "Scopes remain separate: five-on-five shot/contact measures and all-strength tracking. No mixed-source percentile or overall style score.".into()] }
}
pub fn enrich(catalog: &mut NumericalCatalog, movement: &[MovementSnapshot]) -> Result<(), String> {
    catalog.validate()?;
    let mut ids = std::collections::BTreeSet::new();
    for m in movement {
        let expected = format!(
            "https://api-web.nhle.com/v1/edge/skater-detail/{}/{}/2",
            m.player_id, m.season
        );
        if m.season != catalog.season
            || !ids.insert(m.player_id)
            || m.source.url != expected
            || m.source.sha256.len() != 64
            || !m.source.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || m.accessed.is_empty()
            || m.games == 0
            || [m.max_mph, m.bursts_over_20, m.distance_miles]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || v < 0.)
            || !catalog
                .profiles
                .iter()
                .any(|p| p.player_id == m.player_id && p.group != "Goalies")
        {
            return Err("invalid or incompatible movement snapshot".into());
        }
    }
    for p in &mut catalog.profiles {
        p.style_evidence = Some(profile_style(
            p,
            catalog.season,
            &catalog.sources,
            movement.iter().find(|m| m.player_id == p.player_id),
        ));
    }
    Ok(())
}
pub fn validate(s: &StyleEvidence, season: u32) -> Result<(), String> {
    let mut keys = std::collections::BTreeSet::new();
    if s.season != season || s.protocol != "rinkcraft.playing-methods.0.1.0-experimental" {
        return Err("incompatible style evidence".into());
    }
    for m in &s.measured {
        if !keys.insert(&m.key)
            || !matches!(
                m.unit.as_str(),
                "Pct" | "Per60" | "Mph" | "PerGame" | "MilesPerGame"
            )
            || ![m.value, m.numerator, m.denominator]
                .iter()
                .all(|v| v.is_finite())
            || m.value < 0.
            || m.numerator < 0.
            || m.denominator <= 0.
            || (m.value - m.numerator / m.denominator).abs() > 1e-9
            || (m.unit == "Pct" && m.value > 1.)
            || m.sources.is_empty()
            || m.sources.iter().any(|s| {
                !s.url.starts_with("https://")
                    || s.sha256.len() != 64
                    || !s.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            })
        {
            return Err("invalid style measurement".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparisons_preserve_unknowns_and_reject_context_changes() {
        use super::super::rinkcraft_numbers::{self, InputRow};
        let source = Source {
            url: "https://example.org/skaters.csv".into(),
            sha256: "a".repeat(64),
            bytes: 100,
        };
        let rows = (1..=2)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 20,
                seconds: 30000.,
                values: [
                    ("high_danger_shots".into(), id as f64 * 10.),
                    ("unblocked_shots".into(), 100.),
                    ("hits".into(), id as f64 * 5.),
                ]
                .into_iter()
                .collect(),
            })
            .collect::<Vec<_>>();
        let mut c = rinkcraft_numbers::build(&rows, 20242025, &[], vec![source]).unwrap();
        enrich(&mut c, &[]).unwrap();
        let a = c.card(1, None).unwrap().unwrap();
        let mut b = c.card(2, None).unwrap().unwrap();
        let result = compare(&a, &b).unwrap();
        assert_eq!(result.rows[0].status, "matched");
        assert_eq!(result.rows[0].difference, Some(-0.1));
        assert!(!result.shared_gaps.is_empty());
        b.style_evidence.as_mut().unwrap().measured[0].sources[0].sha256 = "b".repeat(64);
        let result = compare(&a, &b).unwrap();
        assert_eq!(result.rows[0].status, "incompatible");
        assert_eq!(result.rows[0].difference, None);
        b.style_evidence.as_mut().unwrap().measured.remove(0);
        let result = compare(&a, &b).unwrap();
        assert_eq!(result.rows[0].status, "left_only");
        assert!(result.rows[0].right.is_none());
        assert!(result.rows[0].difference.is_none());
        b.style_evidence = None;
        assert!(compare(&a, &b)
            .unwrap()
            .rows
            .iter()
            .all(|r| r.status == "left_only"));
        let mut none = a.clone();
        none.style_evidence = None;
        assert!(compare(&none, &b).is_none());
    }
    #[test]
    fn shot_selection_is_a_share_contact_is_a_rate_and_invalid_evidence_rejects() {
        use super::super::rinkcraft_numbers::{self, InputRow};
        let source = Source {
            url: "https://example.org/source".into(),
            sha256: "b".repeat(64),
            bytes: 100,
        };
        let row = InputRow {
            player_id: 2,
            name: "Synthetic".into(),
            group: "Defense".into(),
            games: 20,
            seconds: 30000.,
            values: [
                ("high_danger_shots".into(), 20.),
                ("unblocked_shots".into(), 80.),
                ("hits".into(), 10.),
            ]
            .into_iter()
            .collect(),
        };
        let mut c = rinkcraft_numbers::build(&[row], 20242025, &[], vec![source]).unwrap();
        enrich(&mut c, &[]).unwrap();
        let s = c.profiles[0].style_evidence.as_ref().unwrap();
        assert_eq!(s.measured[0].value, 0.25);
        assert_eq!(s.measured[1].value, 1.2);
        assert!(s.gaps.iter().any(|g| g.axis == "Skating pace"));
        assert_eq!(
            c.card(2, None).unwrap().unwrap().style_evidence.as_ref(),
            Some(s)
        );
        c.validate().unwrap();
        c.profiles[0].style_evidence.as_mut().unwrap().measured[0].denominator = 0.;
        assert!(c.validate().is_err());
    }
    #[test]
    fn methods_keep_opportunities_scopes_and_gaps_separate() {
        let c = NumericalProfile {
            player_id: 1,
            name: "Example".into(),
            group: "Forwards".into(),
            situation: "5on5".into(),
            games: 10,
            minutes: 300.,
            minimum_minutes: 300.,
            eligible_peers: 20,
            metrics: vec![],
            style_evidence: None,
        };
        let movement = MovementSnapshot {
            player_id: 1,
            season: 20242025,
            games: 10,
            max_mph: Some(23.),
            bursts_over_20: Some(80.),
            distance_miles: Some(30.),
            source: Source {
                url: "https://api-web.nhle.com/v1/edge/skater-detail/1/20242025/2".into(),
                sha256: "a".repeat(64),
                bytes: 100,
            },
            accessed: "2026-11-08".into(),
        };
        let s = profile_style(&c, 20242025, &[], Some(&movement));
        assert_eq!(s.measured[1].value, 8.);
        assert_eq!(s.measured[2].value, 3.);
        assert!(s
            .measured
            .iter()
            .all(|m| m.scope.starts_with("All strengths")));
        assert!(s.gaps.iter().any(|g| g.axis == "North–south / east–west"));
        let mut goalie = c;
        goalie.group = "Goalies".into();
        assert!(profile_style(&goalie, 20242025, &[], None)
            .measured
            .is_empty());
        assert!(measure("x", "x", 0., 0., "Pct", "x", "x", &[]).is_none());
    }
}
