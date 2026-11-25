//! Named challenges and observed responses. No capability grades or inferred pressure traits.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Faceoffs {
    #[serde(default)]
    pub season: u32,
    pub player_id: u32,
    pub taken: u32,
    pub won: u32,
    pub late_taken: u32,
    pub late_won: u32,
    pub coverage_games: u32,
    pub captured_at: String,
    pub source_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
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
    let mut entries = vec![];
    let mut gaps = vec![];
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
                entries.push(Entry {id:id.into(),name:name.into(),challenge:challenge.into(),response:"Save the on-goal attempt".into(),opportunities:b.on_goal,successes:saved,result:outcome(saved,b.on_goal,"saved"),
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
                    entries.push(Entry {id:id.into(),name:name.into(),challenge:if id=="draws" {"A recorded faceoff contest"}else{"A draw late in a close regulation game"}.into(),response:"Win the credited draw".into(),opportunities:taken,successes:won,result:outcome(won,taken,"won"),definition:definition.into(),source:"NHL public play-by-play".into(),captured_at:f.captured_at.clone(),coverage:format!("{} finalized games in cached event sample",f.coverage_games),note:"Credited wins do not establish retained possession, clean wins or downstream value. Event and season-summary capture windows can differ. These counts are not clutch ratings.".into()});
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
    Record {schema:"rinkcraft.under-pressure.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,faceoffs,entries,gaps,note:"Observed challenge/response samples, not new feats, isolated skills or 1–20 grades. Small samples remain visible; zero opportunities are not zero ability. Each sample retains its own capture window.".into()}
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
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
    if let Some(c) = &p.shot_context {
        c.validate()?;
    }
    if *r != build(p, r.faceoffs.clone()) {
        return Err("Under Pressure record does not match its measurements/window".into());
    }
    Ok(())
}
pub fn attach(profiles: &mut [Profile], season: u32) {
    for p in profiles.iter_mut().filter(|p| p.season == season) {
        let faceoffs = p.under_pressure.as_ref().and_then(|r| r.faceoffs.clone());
        p.under_pressure = Some(build(p, faceoffs));
    }
}
