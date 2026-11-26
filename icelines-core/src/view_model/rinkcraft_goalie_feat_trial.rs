//! Two explicitly opt-in goalie candidates. The active registry is unchanged.
use super::rinkcraft_achievements::{self as feats, AchievementProfile, Input, Rule};
use serde::Serialize;

pub fn rules() -> Vec<Rule> {
    let mut rules = feats::default_rules();
    rules.extend([
        Rule{id:"goalie-save-percentage".into(),name:"Save percentage".into(),branch:"goalie".into(),metrics:vec!["save-fraction".into()],placements:vec!["task_skills".into()],note:Some("Trial candidate: NHL saves / shots against, ranked against same-season goalies. Recorded saving outcome; shot difficulty and team effects remain. Same 10-GP and 20-peer rules as the active feats.".into())},
        Rule{id:"goalie-shot-workload".into(),name:"Shot workload".into(),branch:"goalie".into(),metrics:vec!["shots-faced".into()],placements:vec!["role_coverage".into()],note:Some("Trial candidate: NHL shots against / appearances. Workload, not save quality, starts, effort or opportunity-adjusted skill. Same 10-GP and 20-peer rules as the active feats.".into())},
    ]);
    rules
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub season: u32,
    pub players: usize,
    pub eligible: usize,
    pub baseline_recognized: usize,
    pub trial_recognized: usize,
    pub newly_recognized: usize,
    pub baseline_feats_unchanged: bool,
    pub accounting_differences: Vec<serde_json::Value>,
    pub candidate_counts: Vec<CandidateCount>,
    pub profiles: Vec<AchievementProfile>,
}
#[derive(Debug, Serialize)]
pub struct CandidateCount {
    pub id: String,
    pub bronze: usize,
    pub silver: usize,
    pub gold: usize,
    pub earned: usize,
}
pub fn evaluate(input: &Input) -> Result<Report, String> {
    if input.players.iter().any(|p| p.group != "Goalies") {
        return Err("Goalie trial requires an explicit goalie-only cohort".into());
    }
    let baseline = feats::evaluate(input, &feats::default_rules())?;
    let mut trial = feats::evaluate(input, &rules())?;
    let mut accounting_differences = vec![];
    for p in &input.players {
        if let Some(((saves, shots), ga)) = p
            .counts
            .get("saves")
            .zip(p.counts.get("shots_against"))
            .zip(p.counts.get("goals_against"))
        {
            if saves + ga != *shots {
                accounting_differences.push(serde_json::json!({"player_id":p.player_id,"shots_against":shots,"saves":saves,"goals_against":ga,"goals_against_minus_unsaved_shots":ga-(shots-saves)}));
                if let Some(a) = trial
                    .profiles
                    .iter_mut()
                    .find(|a| a.player_id == p.player_id)
                {
                    a.notes.push(format!("Additional NHL accounting: {ga} reported goals against versus {} shots minus saves. Values retained; the saving formula uses reported saves / shots against. Cause of the difference is not established by this trial.",shots-saves));
                }
            }
        }
    }
    let mut old_recognized = 0;
    let mut new_recognized = 0;
    let mut newly = 0;
    for (old, new) in baseline.profiles.iter().zip(&trial.profiles) {
        for f in &old.feats {
            if new.feats.iter().find(|n| n.id == f.id) != Some(f) {
                return Err("Trial changed an existing feat".into());
            }
        }
        let a = old.earned > 0;
        let b = new.earned > 0;
        old_recognized += usize::from(a);
        new_recognized += usize::from(b);
        newly += usize::from(!a && b);
    }
    let counts = ["goalie-save-percentage", "goalie-shot-workload"]
        .into_iter()
        .map(|id| {
            let tiers = trial
                .profiles
                .iter()
                .flat_map(|p| &p.feats)
                .filter(|f| f.id == id && f.state == "earned")
                .filter_map(|f| f.tier.as_deref())
                .collect::<Vec<_>>();
            CandidateCount {
                id: id.into(),
                bronze: tiers.iter().filter(|t| **t == "Bronze").count(),
                silver: tiers.iter().filter(|t| **t == "Silver").count(),
                gold: tiers.iter().filter(|t| **t == "Gold").count(),
                earned: tiers.len(),
            }
        })
        .collect();
    Ok(Report {
        schema: "rinkcraft.goalie-feat-trial.v1".into(),
        season: input.season,
        players: input.players.len(),
        eligible: input.players.iter().filter(|p| p.games >= 10).count(),
        baseline_recognized: old_recognized,
        trial_recognized: new_recognized,
        newly_recognized: newly,
        baseline_feats_unchanged: true,
        accounting_differences,
        candidate_counts: counts,
        profiles: trial.profiles,
    })
}
