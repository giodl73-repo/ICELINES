//! A diverse front-card selection of earned statistical task feats, not new qualifications.
use super::rinkcraft_achievements::{AchievementProfile, Feat};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub feat_id: String,
    pub name: String,
    pub family: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repertoire {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub season_type: String,
    pub group: String,
    pub games: u32,
    pub skills: Vec<Skill>,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    pub feat_id: String,
    pub name: String,
    pub tier: String,
}
fn highlight(id: &str) -> Option<(&'static str, &'static str)> {
    task(id).or_else(|| {
        Some(match id {
            "scoring-hitter" => ("Score & hit", "contact"),
            "triple-threat" => ("Hit, block & score", "contact"),
            "contact-disruptor" => ("Hit & disrupt", "disruption"),
            "triple-offense" => ("Shoot, score & assist", "creation"),
            "busy-starter" => ("Starting workload", "workload"),
            "shutout-starter" => ("Starts & shutouts", "shutouts"),
            "volume-producer" => ("Shots & points", "creation"),
            "physical-draws" => ("Hit & draw", "drawing"),
            "shot-generator" => ("Shot volume", "scoring"),
            "production" => ("Point production", "creation"),
            "disruption-triple" => ("Hit, block & disrupt", "disruption"),
            "pp-striker" => ("PP goals", "power_play"),
            "pp-producer" => ("PP points", "power_play"),
            "pp-dual-threat" => ("PP goals & assists", "power_play"),
            _ => return None,
        })
    })
}
fn weakest(f: &Feat) -> f64 {
    f.conditions
        .iter()
        .filter_map(|c| c.percentile)
        .fold(100., f64::min)
}
fn highlights(p: &AchievementProfile) -> Vec<Highlight> {
    let mut out = vec![];
    let mut families = BTreeSet::new();
    let mut metrics = BTreeSet::new();
    let mut feats = p
        .feats
        .iter()
        .filter(|f| {
            f.state == "earned"
                && matches!(f.tier.as_deref(), Some("Gold" | "Silver" | "Bronze"))
                && !f.placements.iter().any(|s| s == "risk_flags")
                && f.placements.iter().any(|s| {
                    [
                        "task_skills",
                        "playing_tendencies",
                        "distinctive_features",
                        "role_coverage",
                    ]
                    .contains(&s.as_str())
                })
                && !f.conditions.is_empty()
                && highlight(&f.id).is_some()
        })
        .collect::<Vec<_>>();
    feats.sort_by(|a, b| {
        strength(b)
            .0
            .cmp(&strength(a).0)
            .then_with(|| weakest(b).total_cmp(&weakest(a)))
            .then(b.required.cmp(&a.required))
            .then(a.id.cmp(&b.id))
    });
    for f in feats {
        let (name, family) = highlight(&f.id).unwrap();
        if families.contains(family)
            || !f
                .conditions
                .iter()
                .all(|c| !metrics.contains(c.key.as_str()))
        {
            continue;
        }
        out.push(Highlight {
            feat_id: f.id.clone(),
            name: name.into(),
            tier: f.tier.clone().unwrap(),
        });
        families.insert(family);
        metrics.extend(f.conditions.iter().map(|c| c.key.clone()));
        if out.len() == 2 {
            break;
        }
    }
    out
}
fn task(id: &str) -> Option<(&'static str, &'static str)> {
    Some(match id {
        "shooting-scorer" => ("Shoot & score", "scoring"),
        "goal-output" => ("Goal production", "scoring"),
        "dual-creator" => ("Shoot & assist", "creation"),
        "assist-output" => ("Assist production", "creation"),
        "shot-shield" => ("Hit & block", "contact"),
        "hit-parade" => ("Body contact", "contact"),
        "scoring-shield" => ("Score & block", "blocking"),
        "shot-blocker" => ("Block shots", "blocking"),
        "disrupting-scorer" => ("Disrupt & score", "disruption"),
        "puck-thief" => ("Takeaways", "disruption"),
        "penalty-magnet" => ("Draw penalties", "drawing"),
        "pp-helper" => ("PP assists", "power_play"),
        "high-save-workload" => ("Busy saves", "saving"),
        _ => return None,
    })
}
fn strength(f: &Feat) -> (u8, usize) {
    (
        match f.tier.as_deref() {
            Some("Gold") => 3,
            Some("Silver") => 2,
            Some("Bronze") => 1,
            _ => 0,
        },
        f.required,
    )
}
pub fn select(p: &AchievementProfile) -> Repertoire {
    let mut skills = Vec::new();
    let mut families = BTreeSet::new();
    let mut metrics = BTreeSet::new();
    loop {
        let mut candidates: Vec<_> = p
            .feats
            .iter()
            .filter_map(|f| {
                let (name, family) = task(&f.id)?;
                if f.state != "earned"
                    || !f.placements.iter().any(|v| v == "task_skills")
                    || families.contains(family)
                {
                    return None;
                }
                let novelty = f
                    .conditions
                    .iter()
                    .filter(|c| !metrics.contains(c.key.as_str()))
                    .count();
                (novelty > 0).then_some((f, name, family, novelty))
            })
            .collect();
        candidates.sort_by(|a, b| {
            strength(b.0)
                .0
                .cmp(&strength(a.0).0)
                .then_with(|| b.3.cmp(&a.3))
                .then_with(|| b.0.required.cmp(&a.0.required))
                .then_with(|| {
                    let weakest = |f: &Feat| {
                        f.conditions
                            .iter()
                            .filter_map(|c| c.percentile)
                            .fold(100., f64::min)
                    };
                    weakest(b.0).total_cmp(&weakest(a.0))
                })
                .then_with(|| a.0.id.cmp(&b.0.id))
        });
        let Some((f, name, family, _)) = candidates.first() else {
            break;
        };
        skills.push(Skill {
            feat_id: f.id.clone(),
            name: (*name).into(),
            family: (*family).into(),
        });
        families.insert((*family).to_string());
        metrics.extend(f.conditions.iter().map(|c| c.key.clone()));
        if skills.len() == 5 {
            break;
        }
    }
    Repertoire {highlights:highlights(p),schema:"rinkcraft.task-repertoire.v2".into(),player_id:p.player_id,season:p.season,season_type:p.season_type.clone(),group:p.group.clone(),games:p.games,skills,
        note:"Task selection v2: earned tier first, then additional metric keys, condition count and weakest percentile; one per family, up to five. Identity highlights prefer earned tier and weakest percentile, with different families and non-overlapping metric keys, up to two. Short-handed outcomes and cautions remain in their profile categories. These are recorded stat patterns, separate from documentary class or coaching assignment. Full feats remain in the library.".into()}
}
