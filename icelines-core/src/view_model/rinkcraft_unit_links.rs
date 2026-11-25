//! Observed co-deployment, without inferred line numbers, stations or chemistry.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Member {
    pub player_id: u32,
    pub name: String,
    pub position: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub team_id: u64,
    pub team: String,
    pub members: Vec<Member>,
    pub seconds: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shared {
    pub team_id: u64,
    pub team: String,
    pub member: Member,
    pub seconds: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub game_id: u64,
    pub date: String,
    pub group: String,
    pub states: BTreeMap<String, u32>,
    pub unresolved_seconds: u32,
    pub units: BTreeMap<String, Vec<Unit>>,
    pub partners: Vec<Shared>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub team_id: u64,
    pub team: String,
    pub members: Vec<Member>,
    pub seconds: u32,
    pub games: u32,
    pub partners_display: String,
    pub time_display: String,
    pub share_display: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub key: String,
    pub label: String,
    pub seconds: u32,
    pub time_display: String,
    pub total_groups: usize,
    pub tied_leaders: usize,
    pub groups: Vec<Group>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partner {
    pub team_id: u64,
    pub team: String,
    pub member: Member,
    pub seconds: u32,
    pub time_display: String,
    pub share_display: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub game_ids: Vec<u64>,
    pub first_date: String,
    pub last_date: String,
    pub seconds: u32,
    pub tied_leaders: usize,
    pub leading: Option<Group>,
    pub different_from_full: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub game_ids: Vec<u64>,
    pub first_date: String,
    pub last_date: String,
    pub coverage: String,
    pub contexts: Vec<Context>,
    pub other_seconds: u32,
    pub unresolved_seconds: u32,
    pub remainder_display: String,
    pub partners: Vec<Partner>,
    pub recent: Recent,
    pub source_sha256: String,
    pub parent_event_sha256: String,
    pub note: String,
}
fn time(s: u32) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}
fn share(s: u32, total: u32) -> String {
    if total == 0 {
        "—".into()
    } else {
        format!("{:.1}%", 100. * f64::from(s) / f64::from(total))
    }
}
fn names(m: &[Member], focus: u32) -> String {
    m.iter()
        .filter(|m| m.player_id != focus)
        .map(|m| m.name.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}
fn labels(group: &str) -> [&str; 3] {
    if group == "Goalies" {
        ["5v5 defense in front", "5v4 in front", "4v5 in front"]
    } else if group == "Defense" {
        ["5v5 pair", "PP · 5v4", "PK · 4v5"]
    } else {
        ["5v5 trio", "PP · 5v4", "PK · 4v5"]
    }
}
fn context(games: &[Game], key: &str, label: &str, focus: u32) -> Context {
    let seconds = games
        .iter()
        .map(|g| g.states.get(key).copied().unwrap_or(0))
        .sum();
    let mut groups = BTreeMap::<(u64, Vec<u32>), (Unit, u32, BTreeSet<u64>)>::new();
    for g in games {
        for u in g.units.get(key).into_iter().flatten() {
            let ids = u.members.iter().map(|m| m.player_id).collect::<Vec<_>>();
            let a = groups
                .entry((u.team_id, ids))
                .or_insert_with(|| (u.clone(), 0, BTreeSet::new()));
            a.1 += u.seconds;
            a.2.insert(g.game_id);
        }
    }
    let mut values = groups
        .into_values()
        .map(|(u, s, ids)| Group {
            team_id: u.team_id,
            team: u.team,
            partners_display: names(&u.members, focus),
            members: u.members,
            seconds: s,
            games: ids.len() as u32,
            time_display: time(s),
            share_display: share(s, seconds),
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| {
        b.seconds
            .cmp(&a.seconds)
            .then(a.team_id.cmp(&b.team_id))
            .then(a.members.cmp(&b.members))
    });
    let total_groups = values.len();
    let tied_leaders = values
        .first()
        .map(|a| values.iter().filter(|b| b.seconds == a.seconds).count())
        .unwrap_or(0);
    values.truncate(2);
    Context {
        key: key.into(),
        label: label.into(),
        seconds,
        time_display: time(seconds),
        total_groups,
        tied_leaders,
        groups: values,
    }
}
pub fn build(
    p: &Profile,
    mut games: Vec<Game>,
    source_sha256: String,
    parent_event_sha256: String,
) -> Result<Record, String> {
    games.retain(|g| g.group == p.group);
    games.sort_by(|a, b| a.date.cmp(&b.date).then(a.game_id.cmp(&b.game_id)));
    if games.is_empty() {
        return Err("No matched unit appearances for this position group".into());
    }
    let mut seen = BTreeSet::new();
    for g in &games {
        if !seen.insert(g.game_id)
            || g.states.values().map(|s| u64::from(*s)).sum::<u64>()
                + u64::from(g.unresolved_seconds)
                > 3600
        {
            return Err("Duplicate or impossible unit appearance".into());
        }
    }
    let mut game_ids = games.iter().map(|g| g.game_id).collect::<Vec<_>>();
    game_ids.sort_unstable();
    let contexts = ["5v5", "5v4", "4v5"]
        .into_iter()
        .zip(labels(&p.group))
        .map(|(key, label)| context(&games, key, label, p.player_id))
        .collect::<Vec<_>>();
    let mut partners = BTreeMap::<(u64, u32), (Shared, u32)>::new();
    for g in &games {
        for s in &g.partners {
            let a = partners
                .entry((s.team_id, s.member.player_id))
                .or_insert_with(|| (s.clone(), 0));
            a.1 += s.seconds;
        }
    }
    let mut partners = partners
        .into_values()
        .map(|(s, seconds)| Partner {
            team_id: s.team_id,
            team: s.team,
            member: s.member,
            seconds,
            time_display: time(seconds),
            share_display: share(seconds, contexts[0].seconds),
        })
        .collect::<Vec<_>>();
    partners.sort_by(|a, b| {
        b.seconds
            .cmp(&a.seconds)
            .then(a.team_id.cmp(&b.team_id))
            .then(a.member.player_id.cmp(&b.member.player_id))
    });
    partners.truncate(3);
    let last = &games[games.len().saturating_sub(5)..];
    let r = context(last, "5v5", labels(&p.group)[0], p.player_id);
    let leading = (r.tied_leaders == 1).then(|| r.groups[0].clone());
    let different_from_full = if contexts[0].tied_leaders == 1 {
        leading.as_ref().map(|g| {
            g.team_id != contexts[0].groups[0].team_id
                || g.members.iter().map(|m| m.player_id).collect::<Vec<_>>()
                    != contexts[0].groups[0]
                        .members
                        .iter()
                        .map(|m| m.player_id)
                        .collect::<Vec<_>>()
        })
    } else {
        None
    };
    let recent = Recent {
        game_ids: last.iter().map(|g| g.game_id).collect(),
        first_date: last.first().unwrap().date.clone(),
        last_date: last.last().unwrap().date.clone(),
        seconds: r.seconds,
        tied_leaders: r.tied_leaders,
        leading,
        different_from_full,
    };
    let other_seconds = games
        .iter()
        .map(|g| g.states.get("other").copied().unwrap_or(0))
        .sum();
    let unresolved_seconds = games.iter().map(|g| g.unresolved_seconds).sum();
    let first_date = games.first().unwrap().date.clone();
    let last_date = games.last().unwrap().date.clone();
    let result=Record {schema:"rinkcraft.unit-links.v1".into(),player_id:p.player_id,season:p.season,group:p.group.clone(),games:p.games,coverage:format!("{} matched appearances · {} to {} · regulation",games.len(),first_date,last_date),game_ids,first_date,last_date,contexts,other_seconds,unresolved_seconds,remainder_display:format!("Other manpower {} · unresolved {}",time(other_seconds),time(unresolved_seconds)),partners,recent,source_sha256,parent_event_sha256,
        note:"Observed shared time, not assigned L1–L4, D1–D3, PP1/PP2, tactical stations, friendship or causal chemistry. Shares use the player's verified time in the named manpower state. Partner shares overlap. The last five means last five matched appearances, not necessarily the team's last five games. Only the leading two groups per state and three partners are retained on the card.".into()};
    validate(p, &result)?;
    Ok(result)
}
fn hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn rebind(p: &Profile, old: &Record, parent: &str) -> Option<Record> {
    if old.player_id != p.player_id
        || old.season != p.season
        || old.group != p.group
        || old.parent_event_sha256 != parent
    {
        return None;
    }
    let mut r = old.clone();
    r.games = p.games;
    validate(p, &r).ok().map(|_| r)
}
fn date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}
fn group_ok(g: &Group, key: &str, p: &Profile, total: u32, max_games: usize) -> bool {
    let size = match key {
        "5v4" => 5,
        "4v5" => 4,
        _ => {
            if p.group == "Forwards" {
                3
            } else {
                2
            }
        }
    };
    g.team_id > 0
        && g.team.len() == 3
        && g.team.bytes().all(|c| c.is_ascii_uppercase())
        && g.seconds > 0
        && g.seconds <= total
        && g.games > 0
        && g.games as usize <= max_games
        && g.members.len() == size
        && g.members
            .windows(2)
            .all(|w| w[0].player_id < w[1].player_id)
        && g.members.iter().all(|m| {
            m.player_id > 0
                && !m.name.trim().is_empty()
                && ["C", "L", "R", "D", "LW", "RW"].contains(&m.position.as_str())
        })
        && (g.members.iter().any(|m| m.player_id == p.player_id) == (p.group != "Goalies"))
        && (key != "5v5"
            || if p.group == "Forwards" {
                g.members.iter().all(|m| m.position != "D")
            } else {
                g.members.iter().all(|m| m.position == "D")
            })
        && g.partners_display == names(&g.members, p.player_id)
        && g.time_display == time(g.seconds)
        && g.share_display == share(g.seconds, total)
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    let bad = || Err("Invalid or stale observed unit links".into());
    if r.schema != "rinkcraft.unit-links.v1"
        || r.player_id != p.player_id
        || r.season != p.season
        || r.group != p.group
        || r.games != p.games
        || r.game_ids.is_empty()
        || r.game_ids.len() > p.games as usize
        || r.game_ids.windows(2).any(|w| w[0] >= w[1])
        || r.game_ids
            .iter()
            .any(|id| *id / 1_000_000 != u64::from(p.season / 10000) || (*id / 10000) % 100 != 2)
        || !date(&r.first_date)
        || !date(&r.last_date)
        || r.first_date > r.last_date
        || !hash(&r.source_sha256)
        || !hash(&r.parent_event_sha256)
        || r.contexts.len() != 3
    {
        return bad();
    }
    let limit = r.game_ids.len() as u64 * 3600;
    let mut total = u64::from(r.other_seconds) + u64::from(r.unresolved_seconds);
    for (c, (key, label)) in r
        .contexts
        .iter()
        .zip(["5v5", "5v4", "4v5"].into_iter().zip(labels(&p.group)))
    {
        total += u64::from(c.seconds);
        if c.key != key
            || c.label != label
            || c.time_display != time(c.seconds)
            || c.groups.len() != c.total_groups.min(2)
            || (c.total_groups == 0) != (c.tied_leaders == 0)
            || c.tied_leaders > c.total_groups
            || c.groups
                .iter()
                .any(|g| !group_ok(g, key, p, c.seconds, r.game_ids.len()))
            || c.groups.iter().map(|g| u64::from(g.seconds)).sum::<u64>() > u64::from(c.seconds)
            || c.groups.windows(2).any(|w| {
                w[0].seconds < w[1].seconds
                    || (w[0].team_id == w[1].team_id && w[0].members == w[1].members)
            })
        {
            return bad();
        }
        if c.groups.len() == 2
            && ((c.groups[0].seconds == c.groups[1].seconds) != (c.tied_leaders > 1))
        {
            return bad();
        }
    }
    if total > limit
        || r.partners.len() > 3
        || r.partners.iter().any(|s| {
            s.member.player_id == p.player_id
                || s.seconds == 0
                || s.seconds > r.contexts[0].seconds
                || s.time_display != time(s.seconds)
                || s.share_display != share(s.seconds, r.contexts[0].seconds)
        })
        || r.recent.game_ids.len() != r.game_ids.len().min(5)
        || r.recent.game_ids.iter().any(|id| !r.game_ids.contains(id))
        || r.recent.game_ids.iter().collect::<BTreeSet<_>>().len() != r.recent.game_ids.len()
        || !date(&r.recent.first_date)
        || !date(&r.recent.last_date)
        || r.recent.first_date < r.first_date
        || r.recent.last_date > r.last_date
        || r.recent.first_date > r.recent.last_date
        || r.recent.seconds > r.contexts[0].seconds
    {
        return bad();
    }
    if let Some(g) = &r.recent.leading {
        if r.recent.tied_leaders != 1
            || !group_ok(g, "5v5", p, r.recent.seconds, r.recent.game_ids.len())
        {
            return bad();
        }
    } else if r.recent.tied_leaders == 1 {
        return bad();
    }
    let changed = if r.contexts[0].tied_leaders == 1 {
        r.recent.leading.as_ref().map(|g| {
            g.team_id != r.contexts[0].groups[0].team_id
                || g.members.iter().map(|m| m.player_id).collect::<Vec<_>>()
                    != r.contexts[0].groups[0]
                        .members
                        .iter()
                        .map(|m| m.player_id)
                        .collect::<Vec<_>>()
        })
    } else {
        None
    };
    if r.recent.different_from_full != changed
        || r.coverage
            != format!(
                "{} matched appearances · {} to {} · regulation",
                r.game_ids.len(),
                r.first_date,
                r.last_date
            )
        || r.remainder_display
            != format!(
                "Other manpower {} · unresolved {}",
                time(r.other_seconds),
                time(r.unresolved_seconds)
            )
    {
        return bad();
    }
    Ok(())
}
