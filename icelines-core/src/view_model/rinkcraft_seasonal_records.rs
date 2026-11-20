//! Stateless research records. Caller-supplied cohorts are never certified as NHL-wide.
use super::rinkcraft_edge::EdgeEvidence;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub minimum_games: u32,
    pub minimum_minutes: f64,
    pub minimum_conversion_shots: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            minimum_games: 10,
            minimum_minutes: 100.,
            minimum_conversion_shots: 25,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub player_id: u32,
    pub season: u32,
    pub name: String,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub goals: Option<u32>,
    pub shots: Option<u32>,
    pub official_source_sha256: String,
    pub edge: Option<EdgeEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub reference_label: String,
    pub season_type: String,
    pub strength_scope: String,
    pub settings: Settings,
    pub records: Vec<Record>,
}
#[derive(Debug, Clone, Copy)]
pub enum Family {
    Skating,
    Shooting,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub family: String,
    pub state: String,
    pub candidate: Option<String>,
}
/// The same exclusive decision rule is used by exports and the shared engine.
pub fn classify_pair(
    family: Family,
    eligible: bool,
    peak: Option<(f64, u64)>,
    activity: Option<(f64, u64)>,
) -> Result<Decision, String> {
    for (p, n) in [peak, activity].into_iter().flatten() {
        if !p.is_finite() || !(0. ..=100.).contains(&p) || n == 0 {
            return Err("invalid metric comparison".into());
        }
    }
    let (id, one, two, both) = match family {
        Family::Skating => (
            "recorded-skating-speed",
            "top-gear",
            "burst-engine",
            "two-gears",
        ),
        Family::Shooting => (
            "recorded-shot-power-volume",
            "shot-power",
            "shot-volume",
            "power-volume",
        ),
    };
    let mut d = Decision {
        family: id.into(),
        state: "insufficient_exposure".into(),
        candidate: None,
    };
    if !eligible {
        return Ok(d);
    }
    d.state = "unassessed".into();
    let (Some((p, pn)), Some((a, an))) = (peak, activity) else {
        return Ok(d);
    };
    if pn < 20 || an < 20 {
        return Ok(d);
    }
    d.state = "assessed".into();
    d.candidate = match (p >= 90., a >= 90.) {
        (true, true) => Some(both.into()),
        (true, false) => Some(one.into()),
        (false, true) => Some(two.into()),
        _ => None,
    };
    Ok(d)
}
pub fn conversion(goals: Option<u32>, shots: Option<u32>) -> Option<f64> {
    match (goals, shots) {
        (Some(g), Some(s)) if s > 0 && g <= s => Some(f64::from(g) * 100. / f64::from(s)),
        _ => None,
    }
}
pub fn midrank(sorted: &[f64], value: f64) -> Result<f64, String> {
    if !value.is_finite()
        || sorted.is_empty()
        || sorted.iter().any(|v| !v.is_finite())
        || sorted.windows(2).any(|w| w[0] > w[1])
    {
        return Err("finite sorted nonempty reference required".into());
    }
    Ok(midrank_validated(sorted, value))
}
fn midrank_validated(sorted: &[f64], value: f64) -> f64 {
    let a = sorted.partition_point(|v| *v < value);
    let b = sorted.partition_point(|v| *v <= value);
    100. * (a as f64 + 0.5 * (b - a) as f64) / sorted.len() as f64
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Measurement {
    pub key: String,
    pub value: Option<f64>,
    pub percentile: Option<f64>,
    pub peers: usize,
    pub reference_sha256: Option<String>,
    pub source_sha256: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub player_id: u32,
    pub season: u32,
    pub name: String,
    pub group: String,
    pub eligible: bool,
    pub measurements: Vec<Measurement>,
    pub families: Vec<Decision>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub status: String,
    pub reference_label: String,
    pub reference_scope: String,
    pub input_sha256: String,
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub notes: Vec<String>,
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn edge_value(
    edge: &EdgeEvidence,
    key: &str,
    unit: &str,
    count: bool,
) -> Result<Option<f64>, String> {
    let rows: Vec<_> = edge.measures.iter().filter(|m| m.key == key).collect();
    if rows.len() > 1 {
        return Err("duplicate tracking metric".into());
    }
    let Some(m) = rows.first() else {
        return Ok(None);
    };
    if m.unit != unit
        || !m.value.is_finite()
        || m.value < 0.
        || count && (m.value.fract() != 0. || m.value > f64::from(u32::MAX))
        || !count && m.value == 0.
    {
        return Err("invalid tracking value or unit".into());
    }
    Ok(Some(m.value))
}
const KEYS: [&str; 6] = [
    "skating_peak_mph",
    "bursts",
    "bursts_per_60",
    "shot_peak_mph",
    "shots_per_60",
    "conversion_pct",
];
pub fn evaluate(input: &Input) -> Result<Catalog, String> {
    if input.schema != "icelines.seasonal-record-input.v1"
        || input.season_type != "regular"
        || input.strength_scope != "all strengths"
        || input.reference_label.trim().is_empty()
        || input.reference_label.len() > 200
        || input.records.is_empty()
        || input.records.len() > 10000
    {
        return Err("invalid seasonal-record schema, scope or cohort size".into());
    }
    let s = &input.settings;
    if s.minimum_games == 0
        || !s.minimum_minutes.is_finite()
        || s.minimum_minutes <= 0.
        || s.minimum_conversion_shots == 0
    {
        return Err("positive exposure settings required".into());
    }
    let mut records = input.records.clone();
    records.sort_by_key(|r| (r.season, r.player_id));
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    let mut sources = Vec::new();
    for r in &records {
        if r.player_id == 0
            || r.season < 20212022
            || r.season / 10000 + 1 != r.season % 10000
            || !seen.insert((r.season, r.player_id))
            || !matches!(r.group.as_str(), "Forwards" | "Defense")
            || r.games == 0
            || !r.minutes.is_finite()
            || r.minutes <= 0.
            || !hash(&r.official_source_sha256)
        {
            return Err("invalid or duplicate skater observation".into());
        }
        let (mut peak, mut bursts, mut shot) = (None, None, None);
        let mut tracking = Vec::new();
        if let Some(e) = &r.edge {
            if e.schema != "icelines.rinkcraft-edge.v1"
                || e.player_id != r.player_id
                || e.season != r.season
                || e.group != r.group
                || e.games != r.games
                || e.season_type != "regular"
                || e.strength_scope != "all strengths"
                || !hash(&e.source.sha256)
                || e.source.bytes == 0
                || e.source.bytes > 2 * 1024 * 1024
                || e.source.url
                    != format!(
                        "https://api-web.nhle.com/v1/edge/skater-detail/{}/{}/2",
                        r.player_id, r.season
                    )
                || chrono::DateTime::parse_from_rfc3339(&e.captured_at).is_err()
            {
                return Err("tracking/official identity, scope or source mismatch".into());
            }
            peak = edge_value(e, "max-skating-speed", "Mph", false)?;
            bursts = edge_value(e, "speed-bursts-over-20", "Count", true)?;
            shot = edge_value(e, "max-shot-speed", "Mph", false)?;
            tracking.push(e.source.sha256.clone());
        }
        let burst_rate = bursts.map(|n| n * 60. / r.minutes);
        let shot_rate = r.shots.map(|n| f64::from(n) * 60. / r.minutes);
        let conversion = if r.shots.is_some_and(|n| n >= s.minimum_conversion_shots) {
            conversion(r.goals, r.shots)
        } else {
            None
        };
        let row = [peak, bursts, burst_rate, shot, shot_rate, conversion];
        if row.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite derived observation".into());
        }
        let official = vec![r.official_source_sha256.clone()];
        let mut mixed = tracking.clone();
        mixed.extend(official.clone());
        mixed.sort();
        mixed.dedup();
        sources.push([
            tracking.clone(),
            tracking.clone(),
            mixed,
            tracking,
            official.clone(),
            official,
        ]);
        values.push(row);
    }
    let eligible = |r: &Record| r.games >= s.minimum_games && r.minutes >= s.minimum_minutes;
    type PoolKey = (u32, String, usize);
    let mut pools: BTreeMap<PoolKey, Vec<(u32, f64, Vec<String>)>> = BTreeMap::new();
    for (i, r) in records.iter().enumerate().filter(|(_, r)| eligible(r)) {
        for (k, value) in values[i].iter().enumerate() {
            if let Some(v) = value {
                pools
                    .entry((r.season, r.group.clone(), k))
                    .or_default()
                    .push((r.player_id, *v, sources[i][k].clone()));
            }
        }
    }
    let mut references = BTreeMap::new();
    for (key, pool) in pools {
        let conversion_gate = if key.2 == 5 {
            Some(s.minimum_conversion_shots)
        } else {
            None
        };
        let reference_hash = digest(
            &serde_json::to_vec(&(
                &key,
                s.minimum_games,
                s.minimum_minutes,
                conversion_gate,
                &pool,
            ))
            .map_err(|e| e.to_string())?,
        );
        let mut sorted: Vec<_> = pool.into_iter().map(|(_, v, _)| v).collect();
        sorted.sort_by(f64::total_cmp);
        references.insert(key, (sorted, reference_hash));
    }
    let mut profiles = Vec::new();
    for (i, r) in records.iter().enumerate() {
        let mut measurements = Vec::new();
        let mut ranks = [None; 6];
        for (k, key) in KEYS.iter().enumerate() {
            let reference = references.get(&(r.season, r.group.clone(), k));
            let peers = reference.map_or(0, |(p, _)| p.len());
            let percentile = if eligible(r) {
                values[i][k]
                    .zip(reference)
                    .map(|(v, (p, _))| midrank_validated(p, v))
            } else {
                None
            };
            ranks[k] = percentile.map(|p| (p, peers as u64));
            measurements.push(Measurement {
                key: (*key).into(),
                value: values[i][k],
                percentile,
                peers,
                reference_sha256: reference.map(|(_, h)| h.clone()),
                source_sha256: if values[i][k].is_some() {
                    sources[i][k].clone()
                } else {
                    vec![]
                },
            });
        }
        profiles.push(Profile {
            player_id: r.player_id,
            season: r.season,
            name: r.name.clone(),
            group: r.group.clone(),
            eligible: eligible(r),
            measurements,
            families: vec![
                classify_pair(Family::Skating, eligible(r), ranks[0], ranks[2])?,
                classify_pair(Family::Shooting, eligible(r), ranks[3], ranks[4])?,
            ],
        });
    }
    let canonical = Input {
        schema: input.schema.clone(),
        reference_label: input.reference_label.clone(),
        season_type: input.season_type.clone(),
        strength_scope: input.strength_scope.clone(),
        settings: s.clone(),
        records,
    };
    Ok(Catalog{schema:"icelines.seasonal-record-catalog.v1".into(),status:"research_candidates_not_awarded".into(),reference_label:input.reference_label.clone(),reference_scope:"caller-supplied cohort; separate season/position/metric pools; not certified NHL-wide".into(),input_sha256:digest(&serde_json::to_vec(&canonical).map_err(|e|e.to_string())?),settings:s.clone(),profiles,notes:vec!["One candidate per family and player/season. P90 and exposure settings are authored hypotheses; minimum 20 measured peers per required family metric.".into(),"No capability grades, production awards, class inference or simulation bonuses. Raw conversion is diagnostic only; its sample gate does not affect either family.".into(),"Official TOI is not equivalent sprint/shooting opportunity time. Maxima, workload and raw conversion remain deployment/coverage dependent.".into(),"Source hashes identify supplied observations; this function does not acquire, authenticate or certify completeness of the source cohort. No network or resident-package mutation.".into()]})
}
pub fn evaluate_json(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("seasonal-record input exceeds 32 MiB".into());
    }
    let input: Input = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    serde_json::to_string(&evaluate(&input)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracking_binding_units_and_reference_strata_are_enforced() {
        let mut input = sample();
        let e:EdgeEvidence=serde_json::from_value(serde_json::json!({
            "schema":"icelines.rinkcraft-edge.v1","player_id":1,"season":20252026,"group":"Forwards","games":20,
            "season_type":"regular","strength_scope":"all strengths","captured_at":"2026-11-20T00:00:00Z",
            "source":{"url":"https://api-web.nhle.com/v1/edge/skater-detail/1/20252026/2","sha256":"b".repeat(64),"bytes":100},
            "measures":[{"key":"max-skating-speed","label":"Speed","value":24.,"unit":"Mph","provider_percentile":null,"provider_average":null,"source_field":"test","attribute_id":null,"component_id":null,"interpretation":"test"}],"locations":[],"notes":[]
        })).unwrap();
        input.records[0].edge = Some(e.clone());
        let catalog = evaluate(&input).unwrap();
        assert_eq!(catalog.profiles[0].measurements[0].value, Some(24.));
        assert_eq!(catalog.profiles[0].measurements[0].peers, 1);
        assert_eq!(catalog.profiles[0].families[0].state, "unassessed");
        input.records[0].edge.as_mut().unwrap().games = 21;
        assert!(evaluate(&input).is_err());
        input.records[0].edge = Some(e);
        input.records[0].edge.as_mut().unwrap().measures[0].unit = "Miles".into();
        assert!(evaluate(&input).is_err());
        input.records[0].edge = None;
        let mut earlier = input.records[0].clone();
        earlier.season = 20242025;
        earlier.shots = Some(1000);
        input.records.push(earlier);
        let mut defense = input.records[0].clone();
        defense.player_id = 100;
        defense.group = "Defense".into();
        defense.shots = Some(1000);
        input.records.push(defense);
        let result = evaluate(&input).unwrap();
        let forward = result.profiles.iter().find(|r| r.player_id == 20).unwrap();
        assert_eq!(forward.measurements[4].peers, 20);
        assert_eq!(forward.measurements[4].percentile, Some(97.5));
    }
    fn sample() -> Input {
        Input {
            schema: "icelines.seasonal-record-input.v1".into(),
            reference_label: "Test supplied cohort".into(),
            season_type: "regular".into(),
            strength_scope: "all strengths".into(),
            settings: Settings::default(),
            records: (1..=20)
                .map(|i| Record {
                    player_id: i,
                    season: 20252026,
                    name: format!("Player {i}"),
                    group: "Forwards".into(),
                    games: 20,
                    minutes: 200.,
                    goals: Some(0),
                    shots: Some(i),
                    official_source_sha256: "a".repeat(64),
                    edge: None,
                })
                .collect(),
        }
    }
    #[test]
    fn unknown_tracking_and_zero_conversion_never_become_negative_feats() {
        let mut input = sample();
        input.settings.minimum_conversion_shots = 10;
        let c = evaluate(&input).unwrap();
        let p = c.profiles.last().unwrap();
        assert_eq!(p.measurements[4].value, Some(6.));
        assert_eq!(p.measurements[5].value, Some(0.));
        assert_eq!(p.measurements[5].percentile, Some(50.));
        assert!(p
            .families
            .iter()
            .all(|d| d.state == "unassessed" && d.candidate.is_none()));
        assert!(c.reference_scope.contains("caller-supplied"));
    }
    #[test]
    fn exact_boundaries_and_exclusive_family() {
        for (f, id) in [
            (Family::Skating, "two-gears"),
            (Family::Shooting, "power-volume"),
        ] {
            assert_eq!(
                classify_pair(f, true, Some((90., 20)), Some((90., 20)))
                    .unwrap()
                    .candidate
                    .as_deref(),
                Some(id)
            );
            assert_eq!(
                classify_pair(f, true, Some((99., 19)), Some((99., 20)))
                    .unwrap()
                    .state,
                "unassessed"
            );
            assert_eq!(
                classify_pair(f, false, None, None).unwrap().state,
                "insufficient_exposure"
            );
            assert!(classify_pair(f, true, Some((101., 20)), None).is_err());
        }
    }
    #[test]
    fn cohort_order_and_metric_sample_gate_do_not_change_families() {
        let input = sample();
        let a = evaluate(&input).unwrap();
        let mut reversed = input.clone();
        reversed.records.reverse();
        let b = evaluate(&reversed).unwrap();
        assert_eq!(a.input_sha256, b.input_sha256);
        assert_eq!(a.profiles, b.profiles);
        reversed.settings.minimum_conversion_shots = 50;
        let c = evaluate(&reversed).unwrap();
        for (a, b) in a.profiles.iter().zip(c.profiles) {
            assert_eq!(a.families, b.families);
            assert_eq!(a.measurements[4], b.measurements[4]);
        }
    }
    #[test]
    fn duplicate_mixed_scope_and_bad_exposure_are_rejected() {
        let mut i = sample();
        i.records.push(i.records[0].clone());
        assert!(evaluate(&i).is_err());
        let mut i = sample();
        i.season_type = "playoff".into();
        assert!(evaluate(&i).is_err());
        let mut i = sample();
        i.records[0].minutes = f64::NAN;
        assert!(evaluate(&i).is_err());
        let mut i = sample();
        i.records[0].group = "Goalies".into();
        assert!(evaluate(&i).is_err());
        assert!(midrank(&[], 1.).is_err());
        assert!(midrank(&[2., 1.], 1.).is_err());
        assert_eq!(midrank(&[1., 2., 2., 4.], 2.).unwrap(), 50.);
    }
}
