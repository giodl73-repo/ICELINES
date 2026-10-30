use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const FANTASY_MARKET_SNAPSHOT_SCHEMA: &str = "fantasy_market_snapshot.v1";
pub const FANTASY_MARKET_PULSE_SCHEMA: &str = "fantasy_market_pulse.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FantasyMarketPlayerSnapshot {
    pub provider_player_id: String,
    pub name: String,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub positions: Vec<String>,
    #[serde(default)]
    pub xrank: Option<f64>,
    #[serde(default)]
    pub adp: Option<f64>,
    #[serde(default)]
    pub rostered_pct: Option<f64>,
    #[serde(default)]
    pub drafted_pct: Option<f64>,
    #[serde(default)]
    pub adds: Option<u32>,
    #[serde(default)]
    pub drops: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FantasyMarketSnapshot {
    pub schema: String,
    pub provider: String,
    pub captured_at: DateTime<Utc>,
    #[serde(default)]
    pub source_url: Option<String>,
    pub players: Vec<FantasyMarketPlayerSnapshot>,
}

impl FantasyMarketSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != FANTASY_MARKET_SNAPSHOT_SCHEMA {
            return Err(format!(
                "unsupported market snapshot schema '{}'; expected {FANTASY_MARKET_SNAPSHOT_SCHEMA}",
                self.schema
            ));
        }
        if self.provider.trim().is_empty() {
            return Err("market snapshot provider is required".to_owned());
        }
        let mut player_ids = BTreeSet::new();
        for player in &self.players {
            let player_id = player.provider_player_id.trim();
            if player_id.is_empty() {
                return Err("every market player requires a provider_player_id".to_owned());
            }
            if !player_ids.insert(player_id.to_owned()) {
                return Err(format!(
                    "duplicate provider_player_id '{}' in market snapshot",
                    player.provider_player_id
                ));
            }
            if player.name.trim().is_empty() {
                return Err(format!("market player '{player_id}' requires a name"));
            }
            validate_positive_metric("xrank", player.xrank, player_id)?;
            validate_positive_metric("adp", player.adp, player_id)?;
            validate_percentage("rostered_pct", player.rostered_pct, player_id)?;
            validate_percentage("drafted_pct", player.drafted_pct, player_id)?;
        }
        Ok(())
    }
}

fn validate_positive_metric(
    metric: &str,
    value: Option<f64>,
    player_id: &str,
) -> Result<(), String> {
    if let Some(value) = value {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!(
                "{metric} for market player '{player_id}' must be finite and positive"
            ));
        }
    }
    Ok(())
}

fn validate_percentage(metric: &str, value: Option<f64>, player_id: &str) -> Result<(), String> {
    if let Some(value) = value {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            return Err(format!(
                "{metric} for market player '{player_id}' must be finite and between 0 and 100"
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMarketDirection {
    Rising,
    Mixed,
    NewObservation,
    Steady,
    Falling,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMarketPulseRow {
    pub provider_player_id: String,
    pub name: String,
    pub team: Option<String>,
    pub positions: Vec<String>,
    pub direction: FantasyMarketDirection,
    pub xrank: Option<f64>,
    /// Positive means the player's XRank improved (the rank number fell).
    pub xrank_improvement: Option<f64>,
    pub adp: Option<f64>,
    /// Positive means the player's ADP improved (the pick number fell).
    pub adp_improvement: Option<f64>,
    pub rostered_pct: Option<f64>,
    pub rostered_pct_change: Option<f64>,
    pub drafted_pct: Option<f64>,
    pub drafted_pct_change: Option<f64>,
    pub adds: Option<u32>,
    pub adds_change: Option<i64>,
    pub drops: Option<u32>,
    pub drops_change: Option<i64>,
    pub net_adds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyMarketPulseSummary {
    pub players: usize,
    pub rising: usize,
    pub mixed: usize,
    pub new_observations: usize,
    pub steady: usize,
    pub falling: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMarketPulseView {
    pub schema: String,
    pub provider: String,
    pub captured_at: DateTime<Utc>,
    pub previous_captured_at: Option<DateTime<Utc>>,
    pub source_url: Option<String>,
    pub summary: FantasyMarketPulseSummary,
    pub players: Vec<FantasyMarketPulseRow>,
}

pub fn build_fantasy_market_pulse(
    current: FantasyMarketSnapshot,
    previous: Option<&FantasyMarketSnapshot>,
) -> Result<FantasyMarketPulseView, String> {
    current.validate()?;
    if let Some(previous) = previous {
        previous.validate()?;
        if !current
            .provider
            .trim()
            .eq_ignore_ascii_case(previous.provider.trim())
        {
            return Err(format!(
                "market snapshot provider mismatch: current '{}' vs previous '{}'",
                current.provider, previous.provider
            ));
        }
        if previous.captured_at > current.captured_at {
            return Err(format!(
                "previous market snapshot {} is later than current snapshot {}",
                previous.captured_at, current.captured_at
            ));
        }
    }

    let previous_by_id = previous
        .map(|snapshot| {
            snapshot
                .players
                .iter()
                .map(|player| (player.provider_player_id.trim(), player))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut players = current
        .players
        .iter()
        .map(|player| {
            let prior = previous_by_id
                .get(player.provider_player_id.trim())
                .copied();
            build_row(player, prior)
        })
        .collect::<Vec<_>>();
    players.sort_by(compare_rows);

    let summary = FantasyMarketPulseSummary {
        players: players.len(),
        rising: count_direction(&players, FantasyMarketDirection::Rising),
        mixed: count_direction(&players, FantasyMarketDirection::Mixed),
        new_observations: count_direction(&players, FantasyMarketDirection::NewObservation),
        steady: count_direction(&players, FantasyMarketDirection::Steady),
        falling: count_direction(&players, FantasyMarketDirection::Falling),
    };

    Ok(FantasyMarketPulseView {
        schema: FANTASY_MARKET_PULSE_SCHEMA.to_owned(),
        provider: current.provider.trim().to_owned(),
        captured_at: current.captured_at,
        previous_captured_at: previous.map(|snapshot| snapshot.captured_at),
        source_url: current.source_url.clone(),
        summary,
        players,
    })
}

fn build_row(
    current: &FantasyMarketPlayerSnapshot,
    previous: Option<&FantasyMarketPlayerSnapshot>,
) -> FantasyMarketPulseRow {
    let xrank_improvement = inverse_delta(current.xrank, previous.and_then(|row| row.xrank));
    let adp_improvement = inverse_delta(current.adp, previous.and_then(|row| row.adp));
    let rostered_pct_change = delta(
        current.rostered_pct,
        previous.and_then(|row| row.rostered_pct),
    );
    let drafted_pct_change = delta(
        current.drafted_pct,
        previous.and_then(|row| row.drafted_pct),
    );
    let adds_change = integer_delta(current.adds, previous.and_then(|row| row.adds));
    let drops_change = integer_delta(current.drops, previous.and_then(|row| row.drops));
    let net_adds = current
        .adds
        .zip(current.drops)
        .map(|(adds, drops)| i64::from(adds) - i64::from(drops));
    let direction = classify_direction(
        previous.is_some(),
        [
            xrank_improvement,
            adp_improvement,
            rostered_pct_change,
            drafted_pct_change,
            net_adds.map(|value| value as f64),
        ],
    );

    FantasyMarketPulseRow {
        provider_player_id: current.provider_player_id.clone(),
        name: current.name.clone(),
        team: current.team.clone(),
        positions: current.positions.clone(),
        direction,
        xrank: current.xrank,
        xrank_improvement,
        adp: current.adp,
        adp_improvement,
        rostered_pct: current.rostered_pct,
        rostered_pct_change,
        drafted_pct: current.drafted_pct,
        drafted_pct_change,
        adds: current.adds,
        adds_change,
        drops: current.drops,
        drops_change,
        net_adds,
    }
}

fn delta(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    current
        .zip(previous)
        .map(|(current, previous)| current - previous)
}

fn inverse_delta(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    current
        .zip(previous)
        .map(|(current, previous)| previous - current)
}

fn integer_delta(current: Option<u32>, previous: Option<u32>) -> Option<i64> {
    current
        .zip(previous)
        .map(|(current, previous)| i64::from(current) - i64::from(previous))
}

fn classify_direction(has_previous: bool, signals: [Option<f64>; 5]) -> FantasyMarketDirection {
    if !has_previous {
        return FantasyMarketDirection::NewObservation;
    }
    let positive = signals.iter().flatten().any(|value| *value > 0.0);
    let negative = signals.iter().flatten().any(|value| *value < 0.0);
    match (positive, negative) {
        (true, true) => FantasyMarketDirection::Mixed,
        (true, false) => FantasyMarketDirection::Rising,
        (false, true) => FantasyMarketDirection::Falling,
        (false, false) => FantasyMarketDirection::Steady,
    }
}

fn count_direction(players: &[FantasyMarketPulseRow], direction: FantasyMarketDirection) -> usize {
    players
        .iter()
        .filter(|player| player.direction == direction)
        .count()
}

fn compare_rows(left: &FantasyMarketPulseRow, right: &FantasyMarketPulseRow) -> Ordering {
    direction_order(left.direction)
        .cmp(&direction_order(right.direction))
        .then_with(|| compare_optional_desc(left.rostered_pct_change, right.rostered_pct_change))
        .then_with(|| right.net_adds.cmp(&left.net_adds))
        .then_with(|| compare_optional_desc(left.adp_improvement, right.adp_improvement))
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        .then_with(|| left.provider_player_id.cmp(&right.provider_player_id))
}

fn direction_order(direction: FantasyMarketDirection) -> u8 {
    match direction {
        FantasyMarketDirection::Rising => 0,
        FantasyMarketDirection::Mixed => 1,
        FantasyMarketDirection::NewObservation => 2,
        FantasyMarketDirection::Steady => 3,
        FantasyMarketDirection::Falling => 4,
    }
}

fn compare_optional_desc(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured_at(day: u32) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("2026-09-{day:02}T14:00:00Z"))
            .expect("test timestamp should parse")
            .with_timezone(&Utc)
    }

    fn player(id: &str, name: &str) -> FantasyMarketPlayerSnapshot {
        FantasyMarketPlayerSnapshot {
            provider_player_id: id.to_owned(),
            name: name.to_owned(),
            team: Some("SEA".to_owned()),
            positions: vec!["RW".to_owned()],
            xrank: Some(100.0),
            adp: Some(120.0),
            rostered_pct: Some(10.0),
            drafted_pct: Some(5.0),
            adds: Some(100),
            drops: Some(100),
        }
    }

    fn snapshot(day: u32, players: Vec<FantasyMarketPlayerSnapshot>) -> FantasyMarketSnapshot {
        FantasyMarketSnapshot {
            schema: FANTASY_MARKET_SNAPSHOT_SCHEMA.to_owned(),
            provider: "yahoo".to_owned(),
            captured_at: captured_at(day),
            source_url: None,
            players,
        }
    }

    #[test]
    fn classifies_rising_falling_mixed_steady_and_new_rows() {
        let prior = snapshot(
            7,
            vec![
                player("rise", "Rise"),
                player("fall", "Fall"),
                player("mixed", "Mixed"),
                player("steady", "Steady"),
            ],
        );
        let mut rising = player("rise", "Rise");
        rising.rostered_pct = Some(12.0); // +2 points and +30 net adds.
        rising.adds = Some(140);
        rising.drops = Some(110);
        let mut falling = player("fall", "Fall");
        falling.rostered_pct = Some(8.0); // -2 points and -30 net adds.
        falling.adds = Some(90);
        falling.drops = Some(120);
        let mut mixed = player("mixed", "Mixed");
        mixed.adp = Some(110.0); // +10 ADP improvement but -1 rostered point.
        mixed.rostered_pct = Some(9.0);
        let current = snapshot(
            8,
            vec![
                rising,
                falling,
                mixed,
                player("steady", "Steady"),
                player("new", "New"),
            ],
        );

        let view = build_fantasy_market_pulse(current, Some(&prior))
            .expect("valid snapshots should compare");
        let by_id = view
            .players
            .iter()
            .map(|row| (row.provider_player_id.as_str(), row.direction))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(by_id["rise"], FantasyMarketDirection::Rising);
        assert_eq!(by_id["fall"], FantasyMarketDirection::Falling);
        assert_eq!(by_id["mixed"], FantasyMarketDirection::Mixed);
        assert_eq!(by_id["steady"], FantasyMarketDirection::Steady);
        assert_eq!(by_id["new"], FantasyMarketDirection::NewObservation);
    }

    #[test]
    fn lower_rank_and_adp_are_positive_improvements() {
        let prior = snapshot(7, vec![player("one", "One")]);
        let mut current_player = player("one", "One");
        current_player.xrank = Some(90.0);
        current_player.adp = Some(100.0);
        let view = build_fantasy_market_pulse(snapshot(8, vec![current_player]), Some(&prior))
            .expect("valid snapshots should compare");
        assert_eq!(view.players[0].xrank_improvement, Some(10.0));
        assert_eq!(view.players[0].adp_improvement, Some(20.0));
    }

    #[test]
    fn duplicate_names_compare_by_provider_id() {
        let mut first_prior = player("1", "Sebastian Aho");
        first_prior.rostered_pct = Some(10.0);
        let mut second_prior = player("2", "Sebastian Aho");
        second_prior.rostered_pct = Some(20.0);
        let prior = snapshot(7, vec![first_prior, second_prior]);
        let mut first_current = player("1", "Sebastian Aho");
        first_current.rostered_pct = Some(11.0);
        let mut second_current = player("2", "Sebastian Aho");
        second_current.rostered_pct = Some(18.0);
        let view = build_fantasy_market_pulse(
            snapshot(8, vec![first_current, second_current]),
            Some(&prior),
        )
        .expect("duplicate names with unique IDs should compare");
        let changes = view
            .players
            .iter()
            .map(|row| (row.provider_player_id.as_str(), row.rostered_pct_change))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(changes["1"], Some(1.0));
        assert_eq!(changes["2"], Some(-2.0));
    }

    #[test]
    fn missing_metrics_remain_missing_instead_of_becoming_zero() {
        let prior = snapshot(7, vec![player("one", "One")]);
        let mut current = player("one", "One");
        current.adp = None;
        current.rostered_pct = None;
        let view = build_fantasy_market_pulse(snapshot(8, vec![current]), Some(&prior))
            .expect("missing optional metrics should remain valid");
        assert_eq!(view.players[0].adp_improvement, None);
        assert_eq!(view.players[0].rostered_pct_change, None);
    }

    #[test]
    fn validates_identity_ranges_provider_and_chronology() {
        let mut duplicate = snapshot(8, vec![player("same", "One"), player("same", "Two")]);
        assert!(duplicate.validate().is_err());
        duplicate.players[1].provider_player_id = "other".to_owned();
        duplicate.players[1].rostered_pct = Some(100.1);
        assert!(duplicate.validate().is_err());

        let mut other_provider = snapshot(7, vec![player("one", "One")]);
        other_provider.provider = "espn".to_owned();
        assert!(build_fantasy_market_pulse(
            snapshot(8, vec![player("one", "One")]),
            Some(&other_provider)
        )
        .is_err());
        assert!(build_fantasy_market_pulse(
            snapshot(7, vec![player("one", "One")]),
            Some(&snapshot(8, vec![player("one", "One")]))
        )
        .is_err());
    }

    #[test]
    fn accepts_percentage_boundaries() {
        let mut boundary = player("one", "One");
        boundary.rostered_pct = Some(0.0);
        boundary.drafted_pct = Some(100.0);
        assert!(snapshot(8, vec![boundary]).validate().is_ok());
    }
}
