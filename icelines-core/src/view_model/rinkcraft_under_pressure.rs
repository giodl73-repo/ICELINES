//! Named challenges and observed responses. No capability grades or inferred pressure traits.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Draws {
    pub taken: u32,
    pub won: u32,
}
/// Credited actions, without a denominator of failed or available actions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionCounts {
    pub blocks: u32,
    pub takeaways: u32,
    pub late_lead_blocks: u32,
    pub late_lead_takeaways: u32,
}
impl ActionCounts {
    pub fn add(&mut self, other: &Self) {
        self.blocks += other.blocks;
        self.takeaways += other.takeaways;
        self.late_lead_blocks += other.late_lead_blocks;
        self.late_lead_takeaways += other.late_lead_takeaways;
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSample {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<Exposure>,
    pub season: u32,
    pub player_id: u32,
    pub counts: ActionCounts,
    pub roster_games: u32,
    pub coverage_games: u32,
    pub captured_at: String,
    pub source_sha256: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Exposure {
    pub game_ids: Vec<u64>,
    pub late_lead_seconds: u32,
    pub unresolved_team_seconds: u32,
    pub late_lead_blocks: u32,
    pub late_lead_takeaways: u32,
    pub source_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionObservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<String>,
    pub name: String,
    pub result: String,
    pub baseline: String,
    pub definition: String,
    pub coverage: String,
    pub captured_at: String,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Breakdown {
    pub key: String,
    pub name: String,
    pub opportunities: u32,
    pub successes: u32,
    pub result: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Faceoffs {
    #[serde(default)]
    pub season: u32,
    pub player_id: u32,
    pub taken: u32,
    pub won: u32,
    pub late_taken: u32,
    pub late_won: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub late_states: Option<BTreeMap<String, Draws>>,
    pub coverage_games: u32,
    pub captured_at: String,
    pub source_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakdown: Vec<Breakdown>,
    pub id: String,
    pub name: String,
    pub challenge: String,
    pub response: String,
    pub opportunities: u32,
    pub successes: u32,
    pub result: String,
    pub definition: String,
    pub source: String,
    pub captured_at: String,
    pub coverage: String,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_sample: Option<ActionSample>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionObservation>,
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub faceoffs: Option<Faceoffs>,
    pub entries: Vec<Entry>,
    pub gaps: Vec<String>,
    pub note: String,
}
fn outcome(success: u32, opportunities: u32, action: &str) -> String {
    if opportunities == 0 {
        "No observed opportunities".into()
    } else {
        format!(
            "{success}/{opportunities} {action} · {:.1}%",
            100. * success as f64 / opportunities as f64
        )
    }
}
pub fn build(p: &Profile, faceoffs: Option<Faceoffs>) -> Record {
    build_with_actions(p, faceoffs, None)
}
pub fn rebuild(p: &Profile) -> Record {
    build_with_actions(
        p,
        p.under_pressure.as_ref().and_then(|r| r.faceoffs.clone()),
        p.under_pressure
            .as_ref()
            .and_then(|r| r.action_sample.clone()),
    )
}
pub fn build_with_actions(
    p: &Profile,
    faceoffs: Option<Faceoffs>,
    mut action_sample: Option<ActionSample>,
) -> Record {
    // A routine replay can retain a paired shift study only while its parent
    // event sample remains exactly the same. Changed event captures drop it.
    if let Some(s) = action_sample.as_mut().filter(|s| s.exposure.is_none()) {
        if let Some(old) = p
            .under_pressure
            .as_ref()
            .and_then(|r| r.action_sample.as_ref())
        {
            let mut bare = old.clone();
            bare.exposure = None;
            if bare == *s {
                s.exposure = old.exposure.clone();
            }
        }
    }
    let mut entries = vec![];
    let mut gaps = vec![];
    let actions = action_sample.as_ref().map(|s| vec![ActionObservation {
        exposure: s.exposure.as_ref().map(|e| format!("Matched shift subset: {} roster games; {}:{:02} on ice protecting a late one-goal lead. In those games: {} blocks · {} takeaways. {}:{:02} of team-context clock unresolved. Second-resolution intervals; no opportunity success rate.",e.game_ids.len(),e.late_lead_seconds/60,e.late_lead_seconds%60,e.late_lead_blocks,e.late_lead_takeaways,e.unresolved_team_seconds/60,e.unresolved_team_seconds%60)),
        name: "Protecting lead".into(),
        result: format!("{} blocks · {} takeaways", s.counts.late_lead_blocks, s.counts.late_lead_takeaways),
        baseline: format!("All situations: {} blocks · {} takeaways", s.counts.blocks, s.counts.takeaways),
        definition: "Credited, named-player opponent-shot blocks and takeaways in the third regulation period at 15:00 or later, with the player's team leading by exactly one before the event. Teammate-blocked shots and unassigned other-block events are excluded.".into(),
        coverage: format!("On roster in {} of {} cached finalized games; not time on ice", s.roster_games, s.coverage_games),
        captured_at: s.captured_at.clone(),
        note: "Action counts only. The feed does not enumerate failed or available blocks/takeaways, possession retention, or responsible defensive assignments. Zero counts do not establish no deployment or no defensive contribution; no success rate or clutch rating is inferred.".into(),
    }]).unwrap_or_default();
    if p.group == "Goalies" {
        if let Some(c) = p.shot_context.as_ref().filter(|c| {
            c.source_url
                == format!(
                    "https://peter-tanner.com/moneypuck/downloads/shots_{}.zip",
                    p.season / 10000
                )
        }) {
            for (id, name, challenge) in [
                (
                    "fast",
                    "Rapid follow-ups",
                    "Another on-goal attempt within 0–1 second",
                ),
                (
                    "cross",
                    "Cross-slot rebounds",
                    "A provider-tagged cross-slot rebound reaches the net",
                ),
                (
                    "rebound",
                    "Rebound saves",
                    "A provider-tagged rebound reaches the net",
                ),
            ] {
                let b = c.data.buckets.get(id).cloned().unwrap_or_default();
                let saved = b.on_goal.saturating_sub(b.goals);
                entries.push(Entry {breakdown:vec![],id:id.into(),name:name.into(),challenge:challenge.into(),response:"Save the on-goal attempt".into(),opportunities:b.on_goal,successes:saved,result:outcome(saved,b.on_goal,"saved"),
                    definition:if id=="fast" {"Provider-tagged rebounds with 0–1 second since the prior event; denominator is on-goal attempts only."}else if id=="cross" {"Provider royal-road rebound flag; denominator is on-goal attempts only."}else{"Provider rebound flag; denominator is on-goal attempts only."}.into(),
                    source:"MoneyPuck.com shot archive".into(),captured_at:c.captured_at.clone(),coverage:format!("{} unblocked attempts in archive sample",c.data.attempts),
                    note:"Buckets overlap. Misses are excluded from save opportunities. Outcomes do not measure travel, reset speed or isolated technique; a preceding miss need not be a goalie-created rebound.".into()});
            }
        } else {
            gaps.push("Follow-up saving: shot-sequence sample unavailable.".into());
        }
        gaps.push("Traffic tracking: no measured screen or sightline opportunities.".into());
    } else {
        if let Some(f) = &faceoffs {
            if f.taken > 0 {
                for (id,name,taken,won,definition) in [("draws","Contested draws",f.taken,f.won,"NHL credited faceoff wins divided by wins plus losses; all recorded manpower situations."),("late-draws","Late-close draws",f.late_taken,f.late_won,"Third-period faceoffs at 15:00 or later with a pre-draw score gap of at most one; regulation only. This is an authored context definition.")] {
                    let breakdown=if id=="late-draws" && f.late_taken>0 { f.late_states.as_ref().map(|states| [("tied","Tied"),("leading","Leading by one"),("trailing","Trailing by one")].into_iter().map(|(key,name)| {let c=states.get(key).cloned().unwrap_or_default();Breakdown {key:key.into(),name:name.into(),opportunities:c.taken,successes:c.won,result:outcome(c.won,c.taken,"won")}}).collect()).unwrap_or_default() } else {vec![]};
                    entries.push(Entry {breakdown,id:id.into(),name:name.into(),challenge:if id=="draws" {"A recorded faceoff contest"}else{"A draw late in a close regulation game"}.into(),response:"Win the credited draw".into(),opportunities:taken,successes:won,result:outcome(won,taken,"won"),definition:definition.into(),source:"NHL public play-by-play".into(),captured_at:f.captured_at.clone(),coverage:format!("{} finalized games in cached event sample",f.coverage_games),note:"Credited wins do not establish retained possession, clean wins or downstream value. Event and season-summary capture windows can differ. These counts are not clutch ratings.".into()});
                }
            } else {
                gaps.push(
                    "Faceoffs: no observed draw opportunities in the supplied event sample.".into(),
                );
            }
        } else {
            gaps.push("Faceoffs: contextual event sample not supplied.".into());
        }
        gaps.extend(
            [
                "Press resistance: no pressure-tagged possession opportunities.",
                "Contested retrievals: no tracked contest opportunities.",
                "Entry denial: no complete defended-entry opportunity sample.",
            ]
            .map(str::to_owned),
        );
    }
    Record {action_sample,actions,schema:"rinkcraft.under-pressure.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,faceoffs,entries,gaps,note:"Observed challenge/response samples, not new feats, isolated skills or 1–20 grades. Small samples remain visible; zero opportunities are not zero ability. Each sample retains its own capture window.".into()}
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if let Some(s) = &r.action_sample {
        if let Some(e) = &s.exposure {
            let limit = e.game_ids.len() as u64 * 300;
            if e.game_ids.is_empty()
                || e.game_ids.len() > s.roster_games as usize
                || e.game_ids.windows(2).any(|w| w[0] >= w[1])
                || e.game_ids.iter().any(|id| {
                    *id / 1_000_000 != (p.season / 10000) as u64 || (*id / 10000) % 100 != 2
                })
                || u64::from(e.late_lead_seconds) + u64::from(e.unresolved_team_seconds) > limit
                || e.late_lead_blocks > s.counts.late_lead_blocks
                || e.late_lead_takeaways > s.counts.late_lead_takeaways
                || e.source_sha256.len() != 64
                || !e.source_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err("Invalid matched shift exposure".into());
            }
        }
        if p.group == "Goalies"
            || s.player_id != p.player_id
            || s.season != p.season
            || s.roster_games == 0
            || s.roster_games > s.coverage_games
            || s.counts.late_lead_blocks > s.counts.blocks
            || s.counts.late_lead_takeaways > s.counts.takeaways
            || s.source_sha256.len() != 64
            || !s.source_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || chrono::DateTime::parse_from_rfc3339(&s.captured_at).is_err()
        {
            return Err("Invalid contextual action sample".into());
        }
    }
    if let Some(f) = &r.faceoffs {
        if p.group == "Goalies"
            || f.player_id != p.player_id
            || f.season != p.season
            || f.won > f.taken
            || f.late_won > f.late_taken
            || f.late_taken > f.taken
            || f.late_won > f.won
            || f.late_taken - f.late_won > f.taken - f.won
            || f.coverage_games == 0
            || f.source_sha256.len() != 64
            || !f.source_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || chrono::DateTime::parse_from_rfc3339(&f.captured_at).is_err()
        {
            return Err("Invalid faceoff challenge sample".into());
        }
    }
    if let Some(states) = r.faceoffs.as_ref().and_then(|f| f.late_states.as_ref()) {
        let f = r.faceoffs.as_ref().unwrap();
        if states
            .iter()
            .any(|(k, c)| !["tied", "leading", "trailing"].contains(&k.as_str()) || c.won > c.taken)
            || states.values().map(|c| u64::from(c.taken)).sum::<u64>() != u64::from(f.late_taken)
            || states.values().map(|c| u64::from(c.won)).sum::<u64>() != u64::from(f.late_won)
        {
            return Err("Late draw partitions do not match aggregate".into());
        }
    }
    if let Some(c) = &p.shot_context {
        c.validate()?;
    }
    if *r != build_with_actions(p, r.faceoffs.clone(), r.action_sample.clone()) {
        return Err("Under Pressure record does not match its measurements/window".into());
    }
    Ok(())
}
pub fn attach(profiles: &mut [Profile], season: u32) {
    for p in profiles.iter_mut().filter(|p| p.season == season) {
        p.under_pressure = Some(rebuild(p));
    }
}
