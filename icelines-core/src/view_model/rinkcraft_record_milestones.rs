//! Literal career count markers, separate from comparative feats and capabilities.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Serialize, Deserialize)]
pub struct TrackRule {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub branch: String,
    pub unit: String,
    pub divisor: u64,
    pub priority: u32,
    pub steps: Vec<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Registry {
    pub schema: String,
    pub status: String,
    pub tracks: Vec<TrackRule>,
}
#[derive(Serialize, Deserialize)]
pub struct Input {
    pub player_id: u32,
    pub first_season: u32,
    pub last_season: u32,
    pub season_type: String,
    pub as_of: String,
    pub group: String,
    pub verified_counts: BTreeMap<String, u64>,
    pub source_input_sha256: String,
}
#[derive(Serialize, Deserialize)]
pub struct Step {
    pub threshold: u64,
    pub required_raw: u64,
    pub earned: Option<bool>,
}
#[derive(Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub unit: String,
    pub priority: u32,
    pub raw_value: Option<u64>,
    pub divisor: u64,
    pub value: Option<f64>,
    pub state: String,
    pub highest: Option<u64>,
    pub next: Option<u64>,
    pub next_raw: Option<u64>,
    pub remaining_raw: Option<u64>,
    pub steps: Vec<Step>,
}
#[derive(Serialize, Deserialize)]
pub struct RecordMilestones {
    pub schema: String,
    pub player_id: u32,
    pub scope_kind: String,
    pub first_season: u32,
    pub last_season: u32,
    pub season_type: String,
    pub as_of: String,
    pub group: String,
    pub rules_sha256: String,
    pub source_input_sha256: String,
    pub tracks: Vec<Track>,
    pub notice: String,
}
pub fn registry() -> Registry {
    serde_json::from_str(include_str!("rinkcraft_record_milestones.json"))
        .expect("embedded record registry")
}
pub fn evaluate(input: &Input, rules: &Registry) -> Result<RecordMilestones, String> {
    if input.player_id == 0
        || input.season_type != "regular"
        || input.first_season > input.last_season
        || !matches!(input.group.as_str(), "Forwards" | "Defense" | "Goalies")
        || chrono::DateTime::parse_from_rfc3339(&input.as_of).is_err()
    {
        return Err("invalid career record scope".into());
    }
    for s in [input.first_season, input.last_season] {
        if !(1900..=2200).contains(&(s / 10000)) || s / 10000 + 1 != s % 10000 {
            return Err("invalid record season".into());
        }
    }
    if rules.schema != "rinkcraft.record-milestones.v1"
        || rules.status != "authored_record_markers_not_ability_tiers"
    {
        return Err("unsupported record registry".into());
    }
    let allowed = [
        "games",
        "goals",
        "assists",
        "points",
        "pp_points",
        "sh_points",
        "hits",
        "blocks",
        "starts",
        "wins",
        "shutouts",
        "time_seconds",
    ];
    let mut ids = BTreeSet::new();
    for r in &rules.tracks {
        if !ids.insert(&r.id)
            || r.id.is_empty()
            || r.name.is_empty()
            || !allowed.contains(&r.metric.as_str())
            || r.branch
                != match r.metric.as_str() {
                    "games" | "time_seconds" => "all",
                    "starts" | "wins" | "shutouts" => "goalie",
                    _ => "skater",
                }
            || r.steps.is_empty()
            || r.steps.contains(&0)
            || r.steps.windows(2).any(|w| w[0] >= w[1])
            || r.divisor != if r.metric == "time_seconds" { 60 } else { 1 }
        {
            return Err("invalid milestone track".into());
        }
        for s in &r.steps {
            if s.checked_mul(r.divisor).is_none() {
                return Err("milestone overflow".into());
            }
        }
    }
    let mut tracks = Vec::new();
    for r in &rules.tracks {
        if r.branch != "all" && (r.branch == "goalie") != (input.group == "Goalies") {
            continue;
        }
        let raw = input.verified_counts.get(&r.metric).copied();
        let highest =
            raw.and_then(|v| r.steps.iter().rev().find(|s| v >= **s * r.divisor).copied());
        let next = r
            .steps
            .iter()
            .find(|s| raw.is_none_or(|v| v < **s * r.divisor))
            .copied();
        let next_raw = next.map(|s| s * r.divisor);
        tracks.push(Track {
            id: r.id.clone(),
            name: r.name.clone(),
            metric: r.metric.clone(),
            unit: r.unit.clone(),
            priority: r.priority,
            raw_value: raw,
            divisor: r.divisor,
            value: raw.map(|v| v as f64 / r.divisor as f64),
            state: if raw.is_none() {
                "unavailable"
            } else if highest.is_some() {
                "recorded"
            } else {
                "not_reached"
            }
            .into(),
            highest,
            next,
            next_raw,
            remaining_raw: raw.zip(next_raw).map(|(v, n)| n - v),
            steps: r
                .steps
                .iter()
                .map(|s| Step {
                    threshold: *s,
                    required_raw: *s * r.divisor,
                    earned: raw.map(|v| v >= *s * r.divisor),
                })
                .collect(),
        });
    }
    tracks.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
    Ok(RecordMilestones{schema:"rinkcraft.career-records.v1".into(),player_id:input.player_id,scope_kind:"career".into(),first_season:input.first_season,last_season:input.last_season,season_type:input.season_type.clone(),as_of:input.as_of.clone(),group:input.group.clone(),rules_sha256:format!("{:x}",Sha256::digest(serde_json::to_vec(rules).map_err(|e|e.to_string())?)),source_input_sha256:input.source_input_sha256.clone(),tracks,notice:"Authored literal NHL regular-season record markers, not official award definitions, ability tiers, comparative percentiles, career classes or simulation bonuses. Only the highest reached marker per track is displayed; lower steps are the same record history, not independent feats.".into()})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            player_id: 1,
            first_season: 20132014,
            last_season: 20262027,
            season_type: "regular".into(),
            as_of: "2026-11-19T14:00:00Z".into(),
            group: "Forwards".into(),
            verified_counts: [("points".into(), 1275), ("games".into(), 817)].into(),
            source_input_sha256: "test".into(),
        }
    }
    #[test]
    fn highest_only_and_exact_remaining() {
        let p = evaluate(&input(), &registry()).unwrap();
        let t = p.tracks.iter().find(|t| t.id == "points").unwrap();
        assert_eq!(t.highest, Some(1000));
        assert_eq!(t.next, Some(1500));
        assert_eq!(t.remaining_raw, Some(225));
    }
    #[test]
    fn unknown_is_not_zero() {
        let mut i = input();
        let p = evaluate(&i, &registry()).unwrap();
        assert_eq!(
            p.tracks.iter().find(|t| t.id == "goals").unwrap().state,
            "unavailable"
        );
        i.verified_counts.insert("goals".into(), 0);
        assert_eq!(
            evaluate(&i, &registry())
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "goals")
                .unwrap()
                .state,
            "not_reached"
        );
    }
    #[test]
    fn goalie_has_its_own_tracks() {
        let mut i = input();
        i.group = "Goalies".into();
        let p = evaluate(&i, &registry()).unwrap();
        assert!(!p.tracks.iter().any(|t| t.id == "points"));
        assert!(p.tracks.iter().any(|t| t.id == "wins"));
    }
    #[test]
    fn reject_bad_order_divisor_and_scope() {
        let mut r = registry();
        r.tracks[0].steps = vec![500, 100];
        assert!(evaluate(&input(), &r).is_err());
        r = registry();
        r.tracks[0].divisor = 60;
        assert!(evaluate(&input(), &r).is_err());
        let mut i = input();
        i.season_type = "playoff".into();
        assert!(evaluate(&i, &registry()).is_err());
    }
    #[test]
    fn minutes_compare_exact_seconds() {
        let mut i = input();
        i.verified_counts.insert("time_seconds".into(), 599999);
        let p = evaluate(&i, &registry()).unwrap();
        let t = p.tracks.iter().find(|t| t.id == "minutes").unwrap();
        assert_eq!(t.highest, None);
        assert_eq!(t.remaining_raw, Some(1));
    }
}
