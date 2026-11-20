//! Experimental statistical feats: conjunctions of observed stats, not abilities.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub player_id: u32,
    pub name: String,
    pub group: String,
    pub games: u32,
    pub counts: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub season: u32,
    pub players: Vec<PlayerInput>,
    pub source_urls: Vec<String>,
    pub source_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub branch: String,
    pub metrics: Vec<String>,
    pub placements: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub raw_thresholds: BTreeMap<String, RawThreshold>,
    pub key: String,
    pub label: String,
    pub value: Option<f64>,
    pub unit: String,
    pub numerator: Option<f64>,
    pub denominator: Option<f64>,
    pub percentile: Option<f64>,
    pub peers: usize,
    pub reference_sha256: String,
    pub qualifies: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawThreshold {
    pub value: f64,
    pub inclusive: bool,
}
/// Exact raw boundaries for the existing midrank tier rules, including ties.
pub fn raw_thresholds(values: &[f64]) -> BTreeMap<String, RawThreshold> {
    let mut sorted = values.to_vec();
    if sorted.len() < 20 || sorted.iter().any(|v| !v.is_finite()) {
        return BTreeMap::new();
    }
    sorted.sort_by(f64::total_cmp);
    let mut result = BTreeMap::new();
    for target in [60., 75., 90.] {
        let mut below = 0;
        while below < sorted.len() {
            let value = sorted[below];
            let tied = sorted[below..].iter().take_while(|v| **v == value).count();
            let at = 100. * (below as f64 + 0.5 * tied as f64) / sorted.len() as f64;
            let above = 100. * (below + tied) as f64 / sorted.len() as f64;
            if at >= target || above >= target {
                result.insert(
                    format!("{target:.0}"),
                    RawThreshold {
                        value,
                        inclusive: at >= target,
                    },
                );
                break;
            }
            below += tied;
        }
    }
    result
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feat {
    pub id: String,
    pub name: String,
    pub state: String,
    pub placements: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub tier: Option<String>,
    pub matched: usize,
    pub required: usize,
    pub conditions: Vec<Condition>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchievementProfile {
    pub player_id: u32,
    pub season: u32,
    pub season_type: String,
    pub group: String,
    pub games: u32,
    pub minimum_games: u32,
    pub rules_sha256: String,
    pub source_sha256: String,
    pub source_urls: Vec<String>,
    pub earned: usize,
    pub feats: Vec<Feat>,
    pub notes: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Catalog {
    pub protocol: String,
    pub profiles: Vec<AchievementProfile>,
}
pub fn default_rules() -> Vec<Rule> {
    serde_json::from_str(include_str!("rinkcraft_achievement_rules.json")).expect("embedded rules")
}
pub fn from_league<R: crate::SeasonStatsRead>(
    views: Vec<crate::stats_repository::ReadPlayerView<'_, R>>,
    season: u32,
    player: u32,
) -> Result<Option<AchievementProfile>, String> {
    let mut players = Vec::new();
    for view in views {
        let r = &view.stats;
        if r.season().0 != season || r.season_type() != crate::season_stats::SeasonType::Regular {
            return Err("mixed canonical stat-feat window".into());
        }
        let mut counts = BTreeMap::new();
        let group;
        if let Some(g) = r.goalie() {
            group = "Goalies";
            for (k, v) in [
                ("starts", g.games_started),
                ("shutouts", g.shutouts),
                ("shots_against", g.shots_against),
                ("saves", g.saves),
            ] {
                counts.insert(k.into(), v as f64);
            }
        } else {
            group = if r.position() == crate::model::Position::Defense {
                "Defense"
            } else {
                "Forwards"
            };
            let t = r.totals();
            for (k, v) in [
                ("points", t.points),
                ("goals", t.goals),
                ("assists", t.assists),
                ("shots", t.shots),
                ("pim", t.pim),
                ("pp_goals", t.pp_goals),
                ("pp_points", t.pp_points),
                ("sh_goals", t.sh_goals),
                ("sh_points", t.sh_points),
            ] {
                counts.insert(k.into(), v as f64);
            }
            let pp_assists = t
                .pp_points
                .checked_sub(t.pp_goals)
                .ok_or("incoherent canonical PP points")?;
            counts.insert("pp_assists".into(), pp_assists as f64);
            if let Some(rt) = r.realtime() {
                for (k, v) in [
                    ("hits", rt.hits),
                    ("blocks", rt.blocked_shots),
                    ("takeaways", rt.takeaways),
                    ("giveaways", rt.giveaways),
                ] {
                    counts.insert(k.into(), v as f64);
                }
            }
        }
        players.push(PlayerInput {
            player_id: r.player_id().0,
            name: view.identity.full_name.clone(),
            group: group.into(),
            games: r.totals().gp,
            counts,
        });
    }
    players.sort_by_key(|p| p.player_id);
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&players).map_err(|e| e.to_string())?)
    );
    Ok(evaluate(
        &Input {
            season,
            players,
            source_urls: vec![],
            source_sha256: hash,
        },
        &default_rules(),
    )?
    .profiles
    .into_iter()
    .find(|p| p.player_id == player))
}
fn metric(key: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    Some(match key {
        "hits" => ("Hits / GP", "hits", "games", "PerGame"),
        "blocks" => ("Blocks / GP", "blocks", "games", "PerGame"),
        "points" => ("Points / GP", "points", "games", "PerGame"),
        "goals" => ("Goals / GP", "goals", "games", "PerGame"),
        "assists" => ("Assists / GP", "assists", "games", "PerGame"),
        "shots" => ("Shots / GP", "shots", "games", "PerGame"),
        "takeaways" => ("Takeaways / GP", "takeaways", "games", "PerGame"),
        "giveaways" => ("Giveaways / GP", "giveaways", "games", "PerGame"),
        "pim" => ("PIM / GP", "pim", "games", "PerGame"),
        "drawn" => ("Penalties drawn / GP", "drawn", "games", "PerGame"),
        "taken" => ("Penalties taken / GP", "taken", "games", "PerGame"),
        "pp-goals" => ("Season PP goals", "pp_goals", "one", "Count"),
        "pp-assists" => ("Season PP assists", "pp_assists", "one", "Count"),
        "pp-points" => ("Season PP points", "pp_points", "one", "Count"),
        "sh-goals" => ("Season SH goals", "sh_goals", "one", "Count"),
        "sh-points" => ("Season SH points", "sh_points", "one", "Count"),
        "starts" => ("Season starts", "starts", "one", "Count"),
        "shutouts" => ("Season shutouts", "shutouts", "one", "Count"),
        "shots-faced" => ("Shots faced / GP", "shots_against", "games", "PerGame"),
        "save-fraction" => ("Save fraction", "saves", "shots_against", "Pct"),
        _ => return None,
    })
}
fn min_gp(_group: &str) -> u32 {
    10
}
const SAMPLE_NOTE: &str = "Stat feats use a common 10 GP minimum for skaters and goalies, including reference cohorts. At least 20 measured peers are required per stat. GP stays visible as sample context.";

fn assess_feat(feat: &mut Feat, games: u32, group: &str) {
    let complete =
        !feat.conditions.is_empty() && feat.conditions.iter().all(|c| c.percentile.is_some());
    let bottleneck = feat
        .conditions
        .iter()
        .filter_map(|c| c.percentile)
        .fold(100., f64::min);
    let caution = feat.placements.iter().any(|p| p == "risk_flags");
    feat.tier = if complete && games >= min_gp(group) {
        [(90., "Gold"), (75., "Silver"), (60., "Bronze")]
            .into_iter()
            .find(|(threshold, _)| bottleneck >= *threshold)
            .map(|(_, tier)| {
                if caution {
                    match tier {
                        "Gold" => "Marked",
                        "Silver" => "Elevated",
                        _ => "Notable",
                    }
                } else {
                    tier
                }
                .to_string()
            })
    } else {
        None
    };
    feat.matched = feat
        .conditions
        .iter()
        .filter(|c| c.qualifies == Some(true))
        .count();
    feat.state = if games < min_gp(group) {
        "ineligible"
    } else if !complete {
        "unavailable"
    } else if feat.tier.is_none() {
        "not_earned"
    } else {
        "earned"
    }
    .into();
}

/// Reassess saved observations with the common ten-game eligibility rule.
/// This also upgrades older profiles whose ranks were withheld below the GP gate.
pub fn refresh_profiles(profiles: &mut [AchievementProfile]) {
    let mut references = BTreeMap::<(u32, String, String), BTreeMap<u32, f64>>::new();
    for p in profiles.iter().filter(|p| p.games >= min_gp(&p.group)) {
        for c in p.feats.iter().flat_map(|f| &f.conditions) {
            if let Some(value) = c.value.filter(|v| v.is_finite()) {
                references
                    .entry((p.season, p.group.clone(), c.key.clone()))
                    .or_default()
                    .insert(p.player_id, value);
            }
        }
    }
    let references = references
        .into_iter()
        .map(|(key, peers)| {
            let rows: Vec<_> = peers.into_iter().collect();
            let hash = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(key.0, &key.1, &key.2, &rows)).expect("finite reference")
                )
            );
            let values = rows.into_iter().map(|(_, value)| value).collect::<Vec<_>>();
            let cuts = raw_thresholds(&values);
            (key, (values, hash, cuts))
        })
        .collect::<BTreeMap<_, _>>();
    for p in profiles {
        for f in &mut p.feats {
            for c in &mut f.conditions {
                let reference = references.get(&(p.season, p.group.clone(), c.key.clone()));
                let peers = reference.map(|(values, _, _)| values);
                c.peers = peers.map_or(0, Vec::len);
                c.percentile = c
                    .value
                    .filter(|_| p.games >= min_gp(&p.group) && c.peers >= 20)
                    .and_then(|v| {
                        peers.map(|peers| {
                            100. * (peers.iter().filter(|x| **x < v).count() as f64
                                + 0.5 * peers.iter().filter(|x| **x == v).count() as f64)
                                / peers.len() as f64
                        })
                    });
                c.qualifies = c.percentile.map(|v| v >= 60.);
                if let Some((_, hash, cuts)) = reference {
                    c.reference_sha256 = hash.clone();
                    c.raw_thresholds = cuts.clone();
                } else {
                    c.raw_thresholds.clear();
                }
            }
            assess_feat(f, p.games, &p.group);
        }
        p.earned = p.feats.iter().filter(|f| f.state == "earned").count();
        p.minimum_games = min_gp(&p.group);
        for note in &mut p.notes {
            *note = note
                .replace(
                    "reference cohort minimum 20 skater GP, 15 goalie GP",
                    "minimum 10 GP for skaters and goalies",
                )
                .replace(
                    "minimum 20 skater GP, 15 goalie GP",
                    "minimum 10 GP for skaters and goalies",
                );
        }
        if !p.notes.iter().any(|n| n == SAMPLE_NOTE) {
            p.notes.push(SAMPLE_NOTE.into());
        }
    }
}
fn components(p: &PlayerInput, key: &str) -> (Option<f64>, Option<f64>) {
    let Some((_, num, den, _)) = metric(key) else {
        return (None, None);
    };
    (
        p.counts.get(num).copied(),
        match den {
            "games" => Some(f64::from(p.games)),
            "one" => Some(1.),
            _ => p.counts.get(den).copied(),
        },
    )
}
fn value(p: &PlayerInput, key: &str) -> Option<f64> {
    let (n, d) = components(p, key);
    n.zip(d)
        .filter(|(n, d)| n.is_finite() && *n >= 0. && d.is_finite() && *d > 0.)
        .map(|(n, d)| n / d)
}
pub fn evaluate(input: &Input, rules: &[Rule]) -> Result<Catalog, String> {
    if input.season / 10000 + 1 != input.season % 10000
        || !(1900..=2200).contains(&(input.season / 10000))
    {
        return Err("invalid achievement season".into());
    }
    let mut ids = BTreeSet::new();
    for p in &input.players {
        if !ids.insert(p.player_id)
            || p.player_id == 0
            || !matches!(p.group.as_str(), "Forwards" | "Defense" | "Goalies")
            || p.counts.values().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("invalid achievement actor or observed counts".into());
        }
    }
    let mut rule_ids = BTreeSet::new();
    for r in rules {
        let keys: BTreeSet<_> = r.metrics.iter().collect();
        if !rule_ids.insert(&r.id)
            || r.id.is_empty()
            || r.name.is_empty()
            || r.placements.is_empty()
            || r.placements.iter().any(|c| {
                !matches!(
                    c.as_str(),
                    "task_skills"
                        | "playing_tendencies"
                        | "distinctive_features"
                        | "risk_flags"
                        | "role_coverage"
                )
            })
            || !matches!(r.branch.as_str(), "skater" | "goalie")
            || !(1..=3).contains(&r.metrics.len())
            || keys.len() != r.metrics.len()
            || r.metrics.iter().any(|m| {
                metric(m).is_none()
                    || (r.branch == "goalie")
                        != matches!(
                            m.as_str(),
                            "starts" | "shutouts" | "shots-faced" | "save-fraction"
                        )
            })
        {
            return Err(
                "feat rules require one to three distinct supported stats in one branch".into(),
            );
        }
    }
    let rules_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(rules).map_err(|e| e.to_string())?)
    );
    let mut references = BTreeMap::new();
    for group in ["Forwards", "Defense", "Goalies"] {
        for key in rules
            .iter()
            .flat_map(|r| &r.metrics)
            .collect::<BTreeSet<_>>()
        {
            let mut peers: Vec<_> = input
                .players
                .iter()
                .filter(|x| x.group == group && x.games >= min_gp(group))
                .filter_map(|x| value(x, key).map(|v| (x.player_id, v)))
                .collect();
            peers.sort_by_key(|x| x.0);
            let hash = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(input.season, group, key, &peers))
                        .map_err(|e| e.to_string())?
                )
            );
            let thresholds = raw_thresholds(&peers.iter().map(|(_, v)| *v).collect::<Vec<_>>());
            references.insert((group.to_owned(), key.clone()), (peers, hash, thresholds));
        }
    }
    let mut profiles = Vec::new();
    for p in &input.players {
        let eligible = p.games >= min_gp(&p.group);
        let mut feats = Vec::new();
        for rule in rules
            .iter()
            .filter(|r| (r.branch == "goalie") == (p.group == "Goalies"))
        {
            let mut conditions = Vec::new();
            for key in &rule.metrics {
                let (peers, reference_sha256, raw_thresholds) = references
                    .get(&(p.group.clone(), key.clone()))
                    .expect("registered reference");
                let v = value(p, key);
                let percentile = if eligible && peers.len() >= 20 {
                    v.map(|v| {
                        100. * (peers.iter().filter(|(_, x)| *x < v).count() as f64
                            + 0.5 * peers.iter().filter(|(_, x)| *x == v).count() as f64)
                            / peers.len() as f64
                    })
                } else {
                    None
                };
                let (numerator, denominator) = components(p, key);
                let (label, _, _, unit) = metric(key).unwrap();
                conditions.push(Condition {
                    raw_thresholds: raw_thresholds.clone(),
                    key: key.clone(),
                    label: label.into(),
                    value: v,
                    unit: unit.into(),
                    numerator,
                    denominator,
                    percentile,
                    peers: peers.len(),
                    reference_sha256: reference_sha256.clone(),
                    qualifies: percentile.map(|p| p >= 60.),
                });
            }
            feats.push(Feat {
                id: rule.id.clone(),
                name: rule.name.clone(),
                state: String::new(),
                placements: rule.placements.clone(),
                note: rule.note.clone(),
                tier: None,
                matched: conditions
                    .iter()
                    .filter(|c| c.qualifies == Some(true))
                    .count(),
                required: conditions.len(),
                conditions,
            });
            assess_feat(feats.last_mut().expect("new feat"), p.games, &p.group);
        }
        let earned = feats.iter().filter(|f| f.state == "earned").count();
        profiles.push(AchievementProfile{player_id:p.player_id,season:input.season,season_type:"regular".into(),group:p.group.clone(),games:p.games,minimum_games:min_gp(&p.group),rules_sha256:rules_sha256.clone(),source_sha256:input.source_sha256.clone(),source_urls:input.source_urls.clone(),earned,feats,notes:vec!["Experimental stat feats, not measured task mastery, class assignments or 1–20 abilities.".into(),"All conditions must pass: Bronze P60, Silver P75, Gold P90; caution flags use Notable/Elevated/Marked instead. Compare same-season F/D/G separately; minimum 10 GP for skaters and goalies and 20 measured peers per stat.".into(),"Counts per appearance describe production/activity, not per-minute efficiency or pressure-tested success. Related stats are correlated; conjunctions are not independent evidence.".into()]});
    }
    for p in &mut profiles {
        p.notes.push(SAMPLE_NOTE.into());
    }
    Ok(Catalog {
        protocol: "rinkcraft.stat-feats.0.1-experimental".into(),
        profiles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            season: 20242025,
            source_urls: vec![],
            source_sha256: "test".into(),
            players: (1..=40)
                .map(|n| PlayerInput {
                    player_id: n,
                    name: format!("Example {n}"),
                    group: "Forwards".into(),
                    games: 40,
                    counts: BTreeMap::from([
                        ("hits".into(), n as f64),
                        ("blocks".into(), n as f64),
                        ("points".into(), n as f64),
                    ]),
                })
                .collect(),
        }
    }
    #[test]
    fn canonical_scope_and_incoherent_pp_counts_reject() {
        use crate::{
            fixtures, identity::PlayerId, model::Season, season_stats::SeasonType,
            stats_repository::StatsRepository,
        };
        let mut repo = StatsRepository::with_lru_cap(2);
        repo.upsert_identity(fixtures::identity(1).build()).unwrap();
        let mut row = fixtures::stats(1, 20242025, "AAA").build();
        row.totals.pp_points = 0;
        row.totals.pp_goals = 1;
        repo.upsert_stats(row).unwrap();
        assert!(from_league(
            vec![repo
                .view(PlayerId(1), Season(20242025), SeasonType::Regular)
                .unwrap()],
            20242025,
            1
        )
        .is_err());
        assert!(from_league(
            vec![repo
                .view(PlayerId(1), Season(20242025), SeasonType::Regular)
                .unwrap()],
            20252026,
            1
        )
        .is_err());
    }
    #[test]
    fn conjunctions_tiers_and_three_stat_rules() {
        let c = evaluate(&input(), &default_rules()).unwrap();
        let p = c.profiles.last().unwrap();
        let f = p.feats.iter().find(|f| f.id == "triple-threat").unwrap();
        assert_eq!(f.tier.as_deref(), Some("Gold"));
        assert_eq!(f.matched, 3);
        assert!(f.conditions.iter().all(|c| c.percentile == Some(98.75)));
    }
    #[test]
    fn missing_is_not_zero_and_one_condition_cannot_carry_the_badge() {
        let mut i = input();
        i.players[39].counts.remove("blocks");
        let c = evaluate(&i, &default_rules()).unwrap();
        assert_eq!(
            c.profiles[39]
                .feats
                .iter()
                .find(|f| f.id == "shot-shield")
                .unwrap()
                .state,
            "unavailable"
        );
        i.players[39].counts.insert("blocks".into(), 0.);
        let c = evaluate(&i, &default_rules()).unwrap();
        let f = c.profiles[39]
            .feats
            .iter()
            .find(|f| f.id == "shot-shield")
            .unwrap();
        assert_eq!(f.state, "not_earned");
        assert_eq!(f.matched, 1);
    }
    #[test]
    fn eligibility_ties_and_rule_validation() {
        let mut i = input();
        i.players[0].games = 1;
        let c = evaluate(&i, &default_rules()).unwrap();
        assert!(c.profiles[0].feats.iter().all(|f| f.state == "ineligible"));
        for p in &mut i.players {
            p.games = 40;
            p.counts.insert("hits".into(), 1.);
        }
        let c = evaluate(&i, &default_rules()).unwrap();
        assert_eq!(c.profiles[39].feats[0].conditions[0].percentile, Some(50.));
        let mut r = default_rules();
        let repeated = r[0].metrics[0].clone();
        r[0].metrics.push(repeated);
        assert!(evaluate(&i, &r).is_err());
    }
}
