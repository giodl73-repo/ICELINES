//! First capability adapter: a declared reachable skater tries to secure a loose
//! puck against one opposing skater's pressure. Access and puck control differ.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub player: String,
    pub character: game::GameCharacter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    pub schema: String,
    pub character_rules: game::GameRules,
    pub score_rule: contest::ScoreRule,
    pub members: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftBinding {
    pub player: String,
    pub build: game::BuildRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub schema: String,
    pub episode: Request,
    pub character_rules: game::GameRules,
    pub score_rule: contest::ScoreRule,
    pub members: Vec<DraftBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub task: String,
    pub collector: String,
    pub pressure: String,
    pub capability_ids: [String; 2],
    pub scores: [u8; 2],
    pub score_rule: contest::ScoreRule,
    pub modifiers: [i16; 2],
    pub exact_counts: [u16; 3],
    pub pair_index: u16,
    pub dice: [u8; 2],
    pub totals: [i16; 2],
    pub outcome: contest::Outcome,
}

pub fn build(draft: Draft) -> Result<Request, String> {
    if !matches!(
        (draft.schema.as_str(), draft.episode.schema.as_str()),
        ("rinkcraft.event-draft.0.2", "rinkcraft.event-episode.0.2")
            | ("rinkcraft.event-draft.0.3", "rinkcraft.event-episode.0.3")
            | ("rinkcraft.event-draft.0.4", "rinkcraft.event-episode.0.4")
            | ("rinkcraft.event-draft.0.5", "rinkcraft.event-episode.0.5")
            | ("rinkcraft.event-draft.0.6", "rinkcraft.event-episode.0.6")
            | ("rinkcraft.event-draft.0.7", "rinkcraft.event-episode.0.7")
    ) || draft.episode.profiles.is_some()
    {
        return Err(
            "draft requires a matching supported episode without populated profiles".into(),
        );
    }
    let members = draft
        .members
        .into_iter()
        .map(|m| {
            Ok(Binding {
                player: m.player,
                character: game::build(&draft.character_rules, m.build)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut q = draft.episode;
    q.profiles = Some(Profiles {
        schema: "rinkcraft.event-profiles.0.1".into(),
        character_rules: draft.character_rules,
        score_rule: draft.score_rule,
        members,
    });
    validate(&q)?;
    Ok(q)
}

pub(super) fn validate(q: &Request) -> Result<(), String> {
    match (q.schema.as_str(), &q.profiles) {
        ("rinkcraft.event-episode.0.1", None) => {
            if q.steps.iter().any(|s| {
                matches!(
                    s.action,
                    Action::CollectUnderPressure { .. }
                        | Action::PassUnderPressure { .. }
                        | Action::PassWithContext { .. }
                        | Action::ResolveShot { .. }
                        | Action::ReboundFollowup { .. }
                )
            }) {
                return Err("capability tasks require an explicit 0.2 episode".into());
            }
        }
        (
            "rinkcraft.event-episode.0.2"
            | "rinkcraft.event-episode.0.3"
            | "rinkcraft.event-episode.0.4"
            | "rinkcraft.event-episode.0.5"
            | "rinkcraft.event-episode.0.6"
            | "rinkcraft.event-episode.0.7",
            Some(p),
        ) => {
            if p.schema != "rinkcraft.event-profiles.0.1"
                || p.members.is_empty()
                || p.members.len() > 12
            {
                return Err("invalid profile schema or binding count".into());
            }
            p.character_rules.validate()?;
            let rules = serde_json::to_string(&p.character_rules).map_err(|e| e.to_string())?;
            let mut ids = BTreeSet::new();
            for m in &p.members {
                let player = q
                    .players
                    .iter()
                    .find(|player| player.id == m.player)
                    .ok_or("profile bound to unknown player")?;
                if !ids.insert(&m.player)
                    || player.goalie != (m.character.branch == game::Branch::Goalie)
                {
                    return Err("duplicate or incompatible player/profile binding".into());
                }
                game::import_from_json(
                    &rules,
                    &serde_json::to_string(&m.character).map_err(|e| e.to_string())?,
                )?;
            }
            for step in &q.steps {
                if matches!(step.action, Action::PassUnderPressure { .. })
                    && q.schema == "rinkcraft.event-episode.0.2"
                {
                    return Err("passing tasks require explicit 0.3 episode".into());
                }
                if let Action::CollectUnderPressure {
                    collector,
                    pressure,
                } = &step.action
                {
                    if collector == pressure {
                        return Err("collector cannot pressure itself".into());
                    }
                    for id in [collector, pressure] {
                        if profile(p, id)?.branch != game::Branch::Skater {
                            return Err("collection task does not map goalie capabilities".into());
                        }
                    }
                }
            }
        }
        _ => return Err("unsupported episode schema/profile combination".into()),
    }
    if let Some(p) = &q.profiles {
        passing::validate(q, p)?;
        pass_context::validate(q, p)?;
        shot::validate(q, p)?;
        followup::validate(q, p)?;
    }
    Ok(())
}

pub(super) fn profile<'a>(p: &'a Profiles, id: &str) -> Result<&'a game::GameCharacter, String> {
    p.members
        .iter()
        .find(|m| m.player == id)
        .map(|m| &m.character)
        .ok_or_else(|| format!("missing verified profile for {id}"))
}

pub(super) fn resolve(
    p: &Profiles,
    collector: &str,
    pressure: &str,
    pair_index: u16,
) -> Result<Audit, String> {
    if pair_index >= 400 {
        return Err("contest pair index must be 0..399".into());
    }
    let a = profile(p, collector)?.capabilities[1].score;
    let b = profile(p, pressure)?.capabilities[5].score;
    let c = contest::Contest {
        actor_score: a,
        opposition_score: b,
        score_rule: p.score_rule,
        actor_adjustment: 0,
        opposition_adjustment: 0,
    };
    let exact = c.exact_counts()?;
    let dice = [(pair_index / 20 + 1) as u8, (pair_index % 20 + 1) as u8];
    let r = c.resolve(dice[0], dice[1])?;
    Ok(Audit {
        task: "skater_pressure_collection".into(),
        collector: collector.into(),
        pressure: pressure.into(),
        capability_ids: ["handling".into(), "defending".into()],
        scores: [a, b],
        score_rule: p.score_rule,
        modifiers: c.terms()?,
        exact_counts: [exact.actor_wins, exact.stalemates, exact.opposition_wins],
        pair_index,
        dice,
        totals: [r.actor_total, r.opposition_total],
        outcome: r.outcome,
    })
}
