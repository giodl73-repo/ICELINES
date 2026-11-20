//! Opportunity-level measurement intake for the complete six-skater battery.
//! Descriptive task measurements only: no capability scores or context adjustment.
use super::rinkcraft_capability_index::rules;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    TrainEarly,
    TrainLate,
    HeldOut,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    TechnicalFailure,
    ChoiceFailure,
    OpponentWins,
    TeammateFailure,
    JointOrAmbiguous,
    Untested,
    Censored,
    OutOfScope,
}
impl Outcome {
    fn resolved(self) -> bool {
        matches!(
            self,
            Self::Success
                | Self::TechnicalFailure
                | Self::ChoiceFailure
                | Self::OpponentWins
                | Self::TeammateFailure
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub id: u64,
    pub date: NaiveDate,
    pub split: Split,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub id: u32,
    pub position: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    /// Underlying observation/provider lineage, not URL count.
    pub lineage: String,
    pub sha256: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub unit: String,
    pub minimum: f64,
    pub maximum: f64,
    /// What is being measured, including the registered endpoint/target.
    pub definition: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub component_id: String,
    pub opportunity_rule: String,
    pub success_rule: String,
    pub attribution_rule: String,
    pub channels: BTreeMap<String, Channel>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Protocol {
    pub id: String,
    pub registry_sha256: String,
    pub selection_rule: String,
    pub selection_status: String,
    pub window: String,
    /// Each cell is one named task/context stratum, never automatically pooled.
    pub context_cells: BTreeMap<String, BTreeMap<String, String>>,
    pub games: Vec<Game>,
    pub players: Vec<Player>,
    pub sources: Vec<Source>,
    /// All 23 components must have an explicit collection contract.
    pub tasks: Vec<Task>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub player_id: u32,
    pub component_id: String,
    pub game_id: u64,
    pub episode_id: String,
    pub opportunity_id: String,
    pub context_cell: String,
    pub source_id: String,
    pub source_locator: String,
    pub assessor: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub five_on_five_both_goalies: bool,
    pub outcome: Outcome,
    /// Required for resolved Reading observations; asserted by the coding process,
    /// not independently established by this validator.
    pub choice_coded_before_outcome: bool,
    /// Values absent/censored remain absent; zeros are actual measurements.
    pub measurements: BTreeMap<String, f64>,
    pub missing_measurements: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub protocol: Protocol,
    pub observations: Vec<Observation>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Counts {
    pub outcomes: BTreeMap<Outcome, u64>,
    pub eligible: u64,
    pub resolved: u64,
    /// All non-success resolved outcomes, not synonymous with technical errors.
    pub non_success: u64,
    pub out_of_scope: u64,
}
impl Counts {
    fn add(&mut self, outcome: Outcome) {
        *self.outcomes.entry(outcome).or_default() += 1;
        if outcome == Outcome::OutOfScope {
            self.out_of_scope += 1;
        } else {
            self.eligible += 1;
        }
        if outcome.resolved() {
            self.resolved += 1;
            if outcome != Outcome::Success {
                self.non_success += 1;
            }
        }
    }
    fn success_rate(&self) -> Option<f64> {
        (self.resolved > 0).then(|| {
            *self.outcomes.get(&Outcome::Success).unwrap_or(&0) as f64 / self.resolved as f64
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Distribution {
    pub unit: String,
    pub measured: usize,
    pub missing: usize,
    pub minimum: Option<f64>,
    pub median: Option<f64>,
    pub p90: Option<f64>,
    pub maximum: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct Cell {
    pub player_id: u32,
    pub attribute_id: String,
    pub component_id: String,
    pub context_cell: String,
    pub split: Split,
    pub counts: Counts,
    pub success_per_resolved: Option<f64>,
    pub games_with_observations: usize,
    pub games_with_resolved_observations: usize,
    pub source_lineages: Vec<String>,
    pub per_game: BTreeMap<u64, Counts>,
    /// Delete-one-game jackknife SE of the unadjusted ratio. Not a calibrated
    /// confidence interval, and absent with fewer than three resolved games.
    pub success_rate_game_jackknife_se: Option<f64>,
    pub channels: BTreeMap<String, Distribution>,
}
#[derive(Debug, Serialize)]
pub struct Coverage {
    pub player_id: u32,
    pub attribute_id: String,
    pub component_id: String,
    pub rows: usize,
    pub resolved: u64,
    pub splits_with_resolved: Vec<Split>,
    pub measurement_status: String,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub status: String,
    pub input_sha256: String,
    pub registry_sha256: String,
    pub protocol: Protocol,
    pub observations: usize,
    pub linked_episodes: usize,
    pub coverage: Vec<Coverage>,
    pub cells: Vec<Cell>,
    pub limits: Vec<String>,
}
fn text(value: &str) -> bool {
    !value.trim().is_empty()
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn quantile(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let x = (sorted.len() - 1) as f64 * q;
    let lo = x.floor() as usize;
    let hi = x.ceil() as usize;
    Some(sorted[lo] * (1. - x.fract()) + sorted[hi] * x.fract())
}
fn jackknife(games: &BTreeMap<u64, Counts>, total: &Counts) -> Option<f64> {
    let used = games
        .values()
        .filter(|g| g.resolved > 0)
        .collect::<Vec<_>>();
    if used.len() < 3 {
        return None;
    }
    let successes = *total.outcomes.get(&Outcome::Success).unwrap_or(&0);
    let leave = used
        .iter()
        .map(|g| {
            (successes - g.outcomes.get(&Outcome::Success).unwrap_or(&0)) as f64
                / (total.resolved - g.resolved) as f64
        })
        .collect::<Vec<_>>();
    let mean = leave.iter().sum::<f64>() / leave.len() as f64;
    Some(
        ((leave.len() - 1) as f64 / leave.len() as f64
            * leave.iter().map(|x| (x - mean).powi(2)).sum::<f64>())
        .sqrt(),
    )
}

/// Parses and validates a complete-battery intake. The file CLI separately
/// authenticates source bytes; this pure API cannot authenticate caller records.
pub fn measure(raw: &str) -> Result<Report, String> {
    let input: Input = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if input.schema != "rinkcraft.task-measurement-input.v1" {
        return Err("Unknown input schema".into());
    }
    let p = &input.protocol;
    let registry = rules();
    if p.registry_sha256 != registry.source_registry_sha256
        || ![&p.id, &p.selection_rule, &p.window]
            .iter()
            .all(|v| text(v))
        || !["prospective", "retrospective_exploratory", "synthetic_test"]
            .contains(&p.selection_status.as_str())
    {
        return Err("Missing protocol, unknown selection status or registry mismatch".into());
    }
    let ownership = registry
        .attributes
        .iter()
        .filter(|a| a.branch == "skater")
        .flat_map(|a| {
            a.components
                .iter()
                .map(move |c| (c.id.clone(), a.id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    if ownership.len() != 23 {
        return Err("Unexpected skater battery".into());
    }
    let mut tasks = BTreeMap::new();
    for t in &p.tasks {
        if !ownership.contains_key(&t.component_id)
            || tasks.insert(t.component_id.as_str(), t).is_some()
            || ![&t.opportunity_rule, &t.success_rule, &t.attribution_rule]
                .iter()
                .all(|v| text(v))
            || t.channels.is_empty()
        {
            return Err("Invalid, duplicate or incomplete task contract".into());
        }
        for (key, c) in &t.channels {
            if !text(key)
                || !text(&c.unit)
                || !text(&c.definition)
                || !c.minimum.is_finite()
                || !c.maximum.is_finite()
                || c.minimum > c.maximum
            {
                return Err("Invalid measurement channel".into());
            }
        }
    }
    if tasks.len() != ownership.len() {
        return Err("All 23 task contracts are required".into());
    }
    let mut players = BTreeMap::new();
    for player in &p.players {
        if player.id == 0
            || !["C", "L", "R", "D"].contains(&player.position.as_str())
            || players.insert(player.id, player).is_some()
        {
            return Err("Invalid/duplicate skater; goalies require a separate battery".into());
        }
    }
    if players.is_empty() || p.games.is_empty() || p.context_cells.is_empty() {
        return Err("Empty sampling frame".into());
    }
    for (id, context) in &p.context_cells {
        if !text(id)
            || !["pressure", "zone", "task_variant", "support", "duty"]
                .iter()
                .all(|k| context.get(*k).is_some_and(|v| text(v)))
        {
            return Err("Context must name pressure, zone, task variant, support and duty (unknown is explicit)".into());
        }
    }
    let mut games = BTreeMap::new();
    let mut ranges: BTreeMap<Split, (NaiveDate, NaiveDate)> = BTreeMap::new();
    for g in &p.games {
        if g.id == 0 || games.insert(g.id, g).is_some() {
            return Err("Invalid/duplicate game".into());
        }
        ranges
            .entry(g.split)
            .and_modify(|r| {
                r.0 = r.0.min(g.date);
                r.1 = r.1.max(g.date);
            })
            .or_insert((g.date, g.date));
    }
    if ranges.len() != 3
        || ranges[&Split::TrainEarly].1 >= ranges[&Split::TrainLate].0
        || ranges[&Split::TrainLate].1 >= ranges[&Split::HeldOut].0
    {
        return Err("Three strictly chronological nonoverlapping game blocks required".into());
    }
    let mut sources = BTreeMap::new();
    for s in &p.sources {
        if ![&s.id, &s.lineage, &s.url].iter().all(|v| text(v))
            || !digest(&s.sha256)
            || sources.insert(s.id.as_str(), s).is_some()
        {
            return Err("Invalid/duplicate source".into());
        }
    }
    let mut seen = BTreeSet::new();
    let mut episodes = BTreeSet::new();
    let mut groups: BTreeMap<(u32, String, String, Split), Vec<&Observation>> = BTreeMap::new();
    for row in &input.observations {
        let task = tasks
            .get(row.component_id.as_str())
            .ok_or("Unknown task component")?;
        let game = games
            .get(&row.game_id)
            .ok_or("Observation outside registered game frame")?;
        if !players.contains_key(&row.player_id)
            || !p.context_cells.contains_key(&row.context_cell)
            || !sources.contains_key(row.source_id.as_str())
            || ![
                &row.episode_id,
                &row.opportunity_id,
                &row.source_locator,
                &row.assessor,
            ]
            .iter()
            .all(|v| text(v))
            || !row.start_seconds.is_finite()
            || !row.end_seconds.is_finite()
            || row.start_seconds < 0.
            || row.end_seconds < row.start_seconds
        {
            return Err(
                "Invalid actor, context, source, locator, assessor or observation interval".into(),
            );
        }
        if !seen.insert((
            row.player_id,
            row.game_id,
            &row.component_id,
            &row.opportunity_id,
        )) {
            return Err("Duplicate actor-component opportunity, even across source copies".into());
        }
        if !row.five_on_five_both_goalies && row.outcome != Outcome::OutOfScope {
            return Err("Non-primary manpower must be out_of_scope".into());
        }
        if row.component_id.starts_with("REA-")
            && row.outcome.resolved()
            && !row.choice_coded_before_outcome
        {
            return Err("Resolved Reading requires pre-outcome choice coding".into());
        }
        if !row.outcome.resolved()
            && (!row.measurements.is_empty() || !row.missing_measurements.is_empty())
        {
            return Err(
                "Unresolved/excluded opportunities cannot supply performance measurements".into(),
            );
        }
        if row.outcome.resolved() {
            for (key, c) in &task.channels {
                match (row.measurements.get(key), row.missing_measurements.get(key)) {
                    (Some(v), None) if v.is_finite() && *v >= c.minimum && *v <= c.maximum => {}
                    (None, Some(reason)) if text(reason) => {}
                    _ => {
                        return Err(
                            "Every channel needs one valid value or an explicit missingness reason"
                                .into(),
                        )
                    }
                }
            }
            if row
                .measurements
                .keys()
                .chain(row.missing_measurements.keys())
                .any(|k| !task.channels.contains_key(k))
            {
                return Err("Undeclared measurement channel".into());
            }
        }
        episodes.insert((row.game_id, &row.episode_id));
        groups
            .entry((
                row.player_id,
                row.component_id.clone(),
                row.context_cell.clone(),
                game.split,
            ))
            .or_default()
            .push(row);
    }
    let mut cells = Vec::new();
    for ((player_id, component, context, split), rows) in groups {
        let mut counts = Counts::default();
        let mut per_game = BTreeMap::<u64, Counts>::new();
        let mut lineages = BTreeSet::new();
        for row in &rows {
            counts.add(row.outcome);
            per_game.entry(row.game_id).or_default().add(row.outcome);
            lineages.insert(sources[row.source_id.as_str()].lineage.clone());
        }
        let channels = tasks[component.as_str()]
            .channels
            .iter()
            .map(|(key, spec)| {
                let mut values = rows
                    .iter()
                    .filter_map(|r| r.measurements.get(key).copied())
                    .collect::<Vec<_>>();
                values.sort_by(f64::total_cmp);
                let missing = rows
                    .iter()
                    .filter(|r| r.missing_measurements.contains_key(key))
                    .count();
                (
                    key.clone(),
                    Distribution {
                        unit: spec.unit.clone(),
                        measured: values.len(),
                        missing,
                        minimum: values.first().copied(),
                        median: quantile(&values, 0.5),
                        p90: quantile(&values, 0.9),
                        maximum: values.last().copied(),
                    },
                )
            })
            .collect();
        cells.push(Cell {
            player_id,
            attribute_id: ownership[&component].clone(),
            component_id: component,
            context_cell: context,
            split,
            success_per_resolved: counts.success_rate(),
            games_with_observations: per_game.len(),
            games_with_resolved_observations: per_game.values().filter(|g| g.resolved > 0).count(),
            success_rate_game_jackknife_se: jackknife(&per_game, &counts),
            counts,
            per_game,
            source_lineages: lineages.into_iter().collect(),
            channels,
        });
    }
    let mut coverage = Vec::new();
    for player in players.keys() {
        for (component, attribute) in &ownership {
            let relevant = cells
                .iter()
                .filter(|c| c.player_id == *player && c.component_id == *component)
                .collect::<Vec<_>>();
            let resolved = relevant.iter().map(|c| c.counts.resolved).sum();
            let splits = relevant
                .iter()
                .filter(|c| c.counts.resolved > 0)
                .map(|c| c.split)
                .collect::<BTreeSet<_>>();
            coverage.push(Coverage {
                player_id: *player,
                attribute_id: attribute.clone(),
                component_id: component.clone(),
                rows: relevant
                    .iter()
                    .map(|c| c.counts.eligible + c.counts.out_of_scope)
                    .sum::<u64>() as usize,
                resolved,
                splits_with_resolved: splits.into_iter().collect(),
                measurement_status: if resolved == 0 {
                    "no_resolved_opportunities"
                } else {
                    "unadjusted_observations_not_validated_capability"
                }
                .into(),
            });
        }
    }
    Ok(Report { schema: "rinkcraft.task-measurement-report.v1".into(),
        status: "descriptive_measurements_not_capability_scores".into(), input_sha256: format!("{:x}", Sha256::digest(raw.as_bytes())),
        registry_sha256: registry.source_registry_sha256, protocol: input.protocol.clone(), observations: input.observations.len(),
        linked_episodes: episodes.len(), coverage, cells,
        limits: vec![
            "Protocol, task attribution, visibility and pre-outcome coding are caller assertions, not independently verified by intake.".into(),
            "Success/resolved includes all resolved outcome causes; non-success is not an individual technical-error rate.".into(),
            "Continuous summaries are unadjusted empirical type-7 quantiles; joint task constraints and context models remain necessary.".into(),
            "Game jackknife SE assumes games are independent; it is not a calibrated interval, and zero SE does not establish certainty.".into(),
            "No context-cell pooling, reliability certification, calibration, 1-20 score, goalie technique grade or simulator update.".into(),
        ] })
}
