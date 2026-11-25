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
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub season_type: String,
    pub group: String,
    pub games: u32,
    pub skills: Vec<Skill>,
    pub note: String,
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
            b.3.cmp(&a.3)
                .then_with(|| strength(b.0).cmp(&strength(a.0)))
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
    Repertoire {schema:"rinkcraft.task-repertoire.v1".into(),player_id:p.player_id,season:p.season,season_type:p.season_type.clone(),group:p.group.clone(),games:p.games,skills,
        note:"Selected from earned stat feats: prefer additional measured tasks, then tier, condition count and weakest percentile; one per family, up to five. These are production/activity patterns, not pressure-tested mastery. Full earned and locked feats remain in the library.".into()}
}
