//! Seeded neutral-site round robin and final, using the versioned abstract route.
use super::*;
pub mod report;
pub mod study;
pub const TOURNAMENT_SCHEMA: &str = "rinkcraft.tournament-request.0.1";
pub const TACTICAL_SCHEMA: &str = "rinkcraft.tournament-request.0.2";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrant {
    pub roster: Team,
    pub starter_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub kind: String,
    pub character_rules: game::GameRules,
    pub rules: Rules,
    pub teams: Vec<Entrant>,
    pub seed: u64,
    pub batch_trials: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tactics: Option<matchups::tactics::Rules>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftMember {
    pub id: String,
    pub build: game::BuildRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftTeam {
    pub team: String,
    pub starter_id: String,
    pub members: Vec<DraftMember>,
    pub units: Vec<Unit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub schema: String,
    pub kind: String,
    pub character_rules: game::GameRules,
    pub rules: Rules,
    pub teams: Vec<DraftTeam>,
    pub seed: u64,
    pub batch_trials: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tactics: Option<matchups::tactics::Rules>,
}
pub fn build(draft: Draft) -> Result<Request, String> {
    if draft.schema
        != if draft.tactics.is_some() {
            "rinkcraft.tournament-draft.0.2"
        } else {
            "rinkcraft.tournament-draft.0.1"
        }
        || draft.kind != KIND
        || !(2..=16).contains(&draft.teams.len())
    {
        return Err("Invalid tournament draft".into());
    }
    let mut teams = Vec::new();
    for t in draft.teams {
        if t.members.len() > 40 || t.units.len() > 48 {
            return Err("Oversize draft roster/units".into());
        }
        let mut members = Vec::new();
        for m in t.members {
            members.push(Member {
                id: m.id,
                character: game::build(&draft.character_rules, m.build)?,
            });
        }
        teams.push(Entrant {
            starter_id: t.starter_id,
            roster: Team {
                team: t.team,
                members,
                units: t.units,
            },
        });
    }
    Ok(Request {
        schema: if draft.tactics.is_some() {
            TACTICAL_SCHEMA
        } else {
            TOURNAMENT_SCHEMA
        }
        .into(),
        kind: draft.kind,
        character_rules: draft.character_rules,
        rules: draft.rules,
        teams,
        seed: draft.seed,
        batch_trials: draft.batch_trials,
        tactics: draft.tactics,
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GameResult {
    pub id: String,
    pub phase: String,
    pub home: String,
    pub away: String,
    pub home_starter: String,
    pub away_starter: String,
    pub regulation_home_goals: u32,
    pub regulation_away_goals: u32,
    pub home_goals: u32,
    pub away_goals: u32,
    pub resolution: String,
    pub winner: String,
    pub matchup_index: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Standing {
    pub team: String,
    pub games: u32,
    pub wins: u32,
    pub regulation_wins: u32,
    pub regulation_losses: u32,
    pub overtime_losses: u32,
    pub points: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub tie_lot: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub seed: u64,
    pub games: Vec<GameResult>,
    pub standings: Vec<Standing>,
    pub champion: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TitleFrequency {
    pub team: String,
    pub titles: u32,
    pub frequency: f64,
    pub monte_carlo_standard_error: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultSet {
    pub schema: String,
    pub kind: String,
    pub input_sha256: String,
    pub model_id: String,
    pub request: Request,
    pub active_teams: Vec<Entrant>,
    pub matchups: Vec<matchups::Matchup>,
    pub illustrative: Run,
    pub title_frequencies: Vec<TitleFrequency>,
    pub tournament_rules: Vec<String>,
    pub limits: Vec<String>,
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }
}
fn stream(seed: u64, key: &str, channel: &str) -> Rng {
    let bytes = Sha256::digest(serde_json::to_vec(&(seed, key, channel)).unwrap());
    let mut n = [0; 8];
    n.copy_from_slice(&bytes[..8]);
    Rng(u64::from_le_bytes(n))
}
fn sample_poisson(mean: f64, rng: &mut Rng) -> u32 {
    let stop = (-mean).exp();
    let mut product = 1.0;
    let mut count = 0;
    loop {
        product *= rng.uniform();
        if product <= stop {
            return count;
        }
        count += 1;
    }
}
fn play(
    seed: u64,
    id: &str,
    phase: &str,
    index: usize,
    m: &matchups::Matchup,
    starters: &BTreeMap<String, String>,
) -> GameResult {
    let home = sample_poisson(m.home_model_goals_mean, &mut stream(seed, id, "home_goals"));
    let away = sample_poisson(m.away_model_goals_mean, &mut stream(seed, id, "away_goals"));
    let home_wins = if home == away {
        stream(seed, id, "decider").uniform() < m.probabilities.home_overtime_win
    } else {
        home > away
    };
    GameResult {
        id: id.into(),
        phase: phase.into(),
        home: m.home.clone(),
        away: m.away.clone(),
        home_starter: starters[&m.home].clone(),
        away_starter: starters[&m.away].clone(),
        regulation_home_goals: home,
        regulation_away_goals: away,
        home_goals: home + u32::from(home == away && home_wins),
        away_goals: away + u32::from(home == away && !home_wins),
        resolution: if home == away {
            "abstract_deciding_goal"
        } else {
            "regulation"
        }
        .into(),
        winner: if home_wins {
            m.home.clone()
        } else {
            m.away.clone()
        },
        matchup_index: index,
    }
}
fn rank(rows: &mut [Standing]) {
    rows.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.regulation_wins.cmp(&a.regulation_wins))
            .then(b.wins.cmp(&a.wins))
            .then(
                (i64::from(b.goals_for) - i64::from(b.goals_against))
                    .cmp(&(i64::from(a.goals_for) - i64::from(a.goals_against))),
            )
            .then(b.goals_for.cmp(&a.goals_for))
            .then(b.tie_lot.cmp(&a.tie_lot))
            .then(a.team.cmp(&b.team))
    });
}
fn one(
    seed: u64,
    teams: &[Entrant],
    pairs: &[matchups::Matchup],
    lookup: &BTreeMap<(String, String), usize>,
) -> Run {
    let names: Vec<_> = teams.iter().map(|t| t.roster.team.clone()).collect();
    let starters: BTreeMap<_, _> = teams
        .iter()
        .map(|t| (t.roster.team.clone(), t.starter_id.clone()))
        .collect();
    let mut rows: BTreeMap<_, _> = names
        .iter()
        .map(|name| {
            (
                name.clone(),
                Standing {
                    team: name.clone(),
                    games: 0,
                    wins: 0,
                    regulation_wins: 0,
                    regulation_losses: 0,
                    overtime_losses: 0,
                    points: 0,
                    goals_for: 0,
                    goals_against: 0,
                    tie_lot: stream(seed, name, "standing_lot").next(),
                },
            )
        })
        .collect();
    let mut games = Vec::new();
    for i in 0..names.len() {
        for j in i + 1..names.len() {
            let index = lookup[&(names[i].clone(), names[j].clone())];
            let g = play(
                seed,
                &format!("rr-{i}-{j}"),
                "round_robin",
                index,
                &pairs[index],
                &starters,
            );
            for name in [&g.home, &g.away] {
                let s = rows.get_mut(name).unwrap();
                let home = *name == g.home;
                let win = *name == g.winner;
                let overtime = g.resolution != "regulation";
                s.games += 1;
                s.goals_for += if home { g.home_goals } else { g.away_goals };
                s.goals_against += if home { g.away_goals } else { g.home_goals };
                if win {
                    s.wins += 1;
                    s.points += 2;
                    if !overtime {
                        s.regulation_wins += 1;
                    }
                } else if overtime {
                    s.overtime_losses += 1;
                    s.points += 1;
                } else {
                    s.regulation_losses += 1;
                }
            }
            games.push(g);
        }
    }
    let mut standings: Vec<_> = rows.into_values().collect();
    rank(&mut standings);
    let index = lookup[&(standings[0].team.clone(), standings[1].team.clone())];
    let final_game = play(seed, "final", "final", index, &pairs[index], &starters);
    let champion = final_game.winner.clone();
    games.push(final_game);
    Run {
        seed,
        games,
        standings,
        champion,
    }
}
pub fn run(input: &Request) -> Result<ResultSet, String> {
    let n = input.teams.len();
    let games = n.saturating_mul(n.saturating_sub(1)) / 2 + 1;
    if input.schema
        != if input.tactics.is_some() {
            TACTICAL_SCHEMA
        } else {
            TOURNAMENT_SCHEMA
        }
        || input.kind != KIND
        || !(2..=16).contains(&n)
        || input.batch_trials == 0
        || input.batch_trials > 100000
        || games * input.batch_trials as usize > 2000000
        || input.rules.home_goals_multiplier != 1.0
    {
        return Err("Tournament needs 2-16 teams, neutral sites and 1-100000 trials within two million games".into());
    }
    // Validate the full roster and all declared units before conditioning on the starter.
    matchups::project(
        &input.character_rules,
        &input.rules,
        &input
            .teams
            .iter()
            .map(|e| e.roster.clone())
            .collect::<Vec<_>>(),
    )?;
    let mut active = input.teams.clone();
    active.sort_by(|a, b| a.roster.team.cmp(&b.roster.team));
    for team in &mut active {
        if !team
            .roster
            .members
            .iter()
            .any(|m| m.id == team.starter_id && m.character.branch == game::Branch::Goalie)
        {
            return Err("Starter must be a rostered goalie".into());
        }
        team.roster.units.retain(|u| u.goalie == team.starter_id);
        let total: f64 = team.roster.units.iter().map(|u| u.weight).sum();
        if total <= 0.0 {
            return Err("Starter has no declared deployment units".into());
        }
        for u in &mut team.roster.units {
            u.weight /= total;
        }
    }
    let rosters = active.iter().map(|e| e.roster.clone()).collect::<Vec<_>>();
    let pairs = if let Some(tactics) = &input.tactics {
        let (expanded, pairs) =
            matchups::tactics::project(&input.character_rules, &input.rules, &rosters, tactics)?;
        for (entry, roster) in active.iter_mut().zip(expanded) {
            entry.roster = roster;
        }
        pairs
    } else {
        matchups::project(&input.character_rules, &input.rules, &rosters)?
    };
    let lookup: BTreeMap<_, _> = pairs
        .iter()
        .enumerate()
        .map(|(i, p)| ((p.home.clone(), p.away.clone()), i))
        .collect();
    let illustrative = one(input.seed, &active, &pairs, &lookup);
    let mut titles: BTreeMap<String, u32> =
        active.iter().map(|e| (e.roster.team.clone(), 0)).collect();
    for trial in 0..input.batch_trials {
        let result = one(
            input.seed.wrapping_add(trial as u64),
            &active,
            &pairs,
            &lookup,
        );
        *titles.get_mut(&result.champion).unwrap() += 1;
    }
    let frequencies = titles
        .into_iter()
        .map(|(team, titles)| {
            let frequency = titles as f64 / input.batch_trials as f64;
            TitleFrequency {
                team,
                titles,
                frequency,
                monte_carlo_standard_error: (frequency * (1.0 - frequency)
                    / input.batch_trials as f64)
                    .sqrt(),
            }
        })
        .collect();
    let mut result=ResultSet{schema:"rinkcraft.tournament-result.0.1".into(),kind:KIND.into(),input_sha256:hash(input)?,model_id:"rinkcraft.task-matchup.0.1".into(),request:input.clone(),
        active_teams:active,matchups:pairs,illustrative,title_frequencies:frequencies,
        tournament_rules:vec!["Neutral-site single round robin, top two advance to one final; final excluded from round-robin standings".into(),
            "Win 2 points; tied-regulation loss 1; regulation loss 0".into(),"Standings: points, regulation wins, total wins, goal difference, goals for, seeded lot, stable team ID".into(),
            "Independent Poisson regulation goals; tied regulation uses home mean / sum means to award one abstract deciding goal".into(),
            "SplitMix64 streams derived by SHA-256(seed, game ID, channel); separate home/away/decider streams; batch seed = base + trial modulo 2^64".into()],
        limits:vec!["Authored abstract game results, not continuous 60-minute possessions, shifts or empirical NHL forecasts".into(),
            "Uses frozen passing/clear-view route; newer arrival/support/clock scenes are not substituted".into(),
            "Skater Skating and goalie Positioning/Movement/Rebounds/Recovery are unused; no class bonuses".into(),
            "Task exposure is expected checks per modeled attack window, not actual goals, assists, shots, saves, minutes or game-event counts".into(),
            "One declared starter per team for all games; original joint units are conditioned and renormalized on that goalie; backup remains rostered".into(),
            "No PP/PK, penalties, fatigue, injuries, shift changes, rebounds, later-arriver recovery or commitment/cover costs".into(),
            "Title frequencies describe Monte Carlo sampling under fixed authored rules, not empirical skill uncertainty or proven balance".into()]};
    if input.tactics.is_some() {
        result.schema = "rinkcraft.tournament-result.0.2".into();
        result.model_id = "rinkcraft.task-matchup.0.2".into();
        result.limits[1]="Uses declared route/context mixtures and optional forward task cycling; not an optimized policy or free choice from an identical puck state".into();
        result.limits[2]="Route-specific capabilities only: Carry uses Skating; goalie Positioning/Movement/Rebounds/Recovery remain unused; no class bonuses".into();
        result.limits.push("One common legacy neutral-window anchor is retained across variants; shorter routes can increase scoring and trigger existing rate caps; no per-route normalization".into());
        result.limits.push("Forward cycling is an authored opportunity policy, not a universal coaching assignment or an ability grade; specialists may benefit from declared roles".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> Request {
        let q = super::super::tests::fixture();
        let mut rules = q.rules;
        rules.home_goals_multiplier = 1.0;
        Request {
            schema: TOURNAMENT_SCHEMA.into(),
            kind: KIND.into(),
            character_rules: q.character_rules,
            rules,
            teams: q
                .reference
                .into_iter()
                .take(4)
                .map(|roster| Entrant {
                    starter_id: roster.units[0].goalie.clone(),
                    roster,
                })
                .collect(),
            seed: 20261114,
            batch_trials: 12,
            tactics: None,
        }
    }
    #[test]
    fn tournament_is_seeded_replayable_and_batch_reconciles() {
        let q = fixture();
        let a = run(&q).unwrap();
        let b = run(&q).unwrap();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(a.illustrative.games.len(), 7);
        assert_eq!(
            a.title_frequencies.iter().map(|x| x.titles).sum::<u32>(),
            12
        );
        assert!((a.title_frequencies.iter().map(|x| x.frequency).sum::<f64>() - 1.0).abs() < 1e-12);
        let mut other = q.clone();
        other.seed += 1;
        assert_ne!(
            a.illustrative.games,
            run(&other).unwrap().illustrative.games
        );
    }
    #[test]
    fn game_scores_and_standings_reconcile_without_counting_final() {
        let r = run(&fixture()).unwrap();
        let rr: Vec<_> = r
            .illustrative
            .games
            .iter()
            .filter(|g| g.phase == "round_robin")
            .collect();
        assert_eq!(rr.len(), 6);
        assert_eq!(
            r.illustrative.champion,
            r.illustrative.games.last().unwrap().winner
        );
        let mut pairs = BTreeSet::new();
        let mut total_points = 0;
        for g in &r.illustrative.games {
            assert_ne!(g.home_goals, g.away_goals);
            assert_eq!(
                g.winner,
                if g.home_goals > g.away_goals {
                    &g.home
                } else {
                    &g.away
                }
                .clone()
            );
            if g.resolution == "regulation" {
                assert_ne!(g.regulation_home_goals, g.regulation_away_goals);
                assert_eq!(g.home_goals, g.regulation_home_goals);
            } else {
                assert_eq!(g.regulation_home_goals, g.regulation_away_goals);
                assert_eq!(
                    g.home_goals + g.away_goals,
                    g.regulation_home_goals + g.regulation_away_goals + 1
                );
            }
            if g.phase == "round_robin" {
                assert!(pairs.insert((&g.home, &g.away)));
                total_points += if g.resolution == "regulation" { 2 } else { 3 };
            }
        }
        for s in &r.illustrative.standings {
            assert_eq!(s.games, 3);
            assert_eq!(s.games, s.wins + s.regulation_losses + s.overtime_losses);
            assert_eq!(s.points, 2 * s.wins + s.overtime_losses);
            let gf: u32 = rr
                .iter()
                .map(|g| {
                    if g.home == s.team {
                        g.home_goals
                    } else if g.away == s.team {
                        g.away_goals
                    } else {
                        0
                    }
                })
                .sum();
            assert_eq!(gf, s.goals_for);
        }
        assert_eq!(
            r.illustrative
                .standings
                .iter()
                .map(|x| x.points)
                .sum::<u32>(),
            total_points
        );
        assert_eq!(
            r.illustrative
                .standings
                .iter()
                .map(|x| x.goals_for)
                .sum::<u32>(),
            r.illustrative
                .standings
                .iter()
                .map(|x| x.goals_against)
                .sum::<u32>()
        );
    }
    #[test]
    fn odd_team_tournaments_and_input_order_are_supported() {
        let mut q = fixture();
        q.teams.truncate(3);
        let a = run(&q).unwrap();
        assert_eq!(a.illustrative.games.len(), 4);
        q.teams.reverse();
        let b = run(&q).unwrap();
        assert_eq!(a.illustrative, b.illustrative);
        assert!(a.illustrative.standings.iter().all(|s| s.games == 2));
    }
    #[test]
    fn tied_regulation_adds_only_one_deciding_goal() {
        let r = run(&fixture()).unwrap();
        let mut m = r.matchups[0].clone();
        m.home_model_goals_mean = 0.0;
        m.away_model_goals_mean = 0.0;
        let starters = r
            .active_teams
            .iter()
            .map(|t| (t.roster.team.clone(), t.starter_id.clone()))
            .collect();
        for chance in [0.0, 1.0] {
            m.probabilities.home_overtime_win = chance;
            let g = play(7, "tie", "final", 0, &m, &starters);
            assert_eq!(g.resolution, "abstract_deciding_goal");
            assert_eq!(g.regulation_home_goals + g.regulation_away_goals, 0);
            assert_eq!(g.home_goals + g.away_goals, 1);
            assert_eq!(g.home_goals, if chance == 1.0 { 1 } else { 0 });
        }
    }
    #[test]
    fn starter_is_fixed_and_bad_bench_exports_are_not_hidden() {
        let mut q = fixture();
        let team = &mut q.teams[0];
        let mut backup = team
            .roster
            .members
            .iter()
            .find(|m| m.id == team.starter_id)
            .unwrap()
            .clone();
        backup.id.push_str("-backup");
        let mut u = team.roster.units[0].clone();
        u.id.push_str("-backup");
        u.goalie = backup.id.clone();
        u.weight = 0.25;
        team.roster.units[0].weight = 0.75;
        team.roster.members.push(backup);
        team.roster.units.push(u);
        let r = run(&q).unwrap();
        let active = r
            .active_teams
            .iter()
            .find(|e| e.roster.team == q.teams[0].roster.team)
            .unwrap();
        assert_eq!(active.roster.units.len(), 1);
        assert_eq!(active.roster.units[0].weight, 1.0);
        assert_eq!(q.teams[0].roster.units.len(), 2);
        assert!(r
            .illustrative
            .games
            .iter()
            .filter(|g| g.home == active.roster.team)
            .all(|g| g.home_starter == active.starter_id));
        q.teams[0]
            .roster
            .members
            .last_mut()
            .unwrap()
            .character
            .capabilities[0]
            .score = 20;
        assert!(run(&q).is_err());
    }
    #[test]
    fn poisson_sampler_matches_its_game_probability_model() {
        let mut h = stream(13, "sampling", "home");
        let mut a = stream(13, "sampling", "away");
        let mut sum = 0.0;
        let mut wins = 0;
        let mut ties = 0;
        for _ in 0..40000 {
            let x = sample_poisson(3.0, &mut h);
            let y = sample_poisson(2.0, &mut a);
            sum += x as f64;
            if x > y {
                wins += 1;
            }
            if x == y {
                ties += 1;
            }
        }
        let p = outcome(3.0, 2.0);
        assert!((sum / 40000.0 - 3.0).abs() < 0.04);
        assert!((wins as f64 / 40000.0 - p.home_regulation_win).abs() < 0.01);
        assert!((ties as f64 / 40000.0 - p.overtime).abs() < 0.01);
    }
    #[test]
    fn standings_tie_break_is_explicit_and_deterministic() {
        let mut rows = vec![
            Standing {
                team: "A".into(),
                games: 3,
                wins: 2,
                regulation_wins: 1,
                regulation_losses: 1,
                overtime_losses: 0,
                points: 4,
                goals_for: 8,
                goals_against: 7,
                tie_lot: 10,
            },
            Standing {
                team: "B".into(),
                games: 3,
                wins: 2,
                regulation_wins: 1,
                regulation_losses: 1,
                overtime_losses: 0,
                points: 4,
                goals_for: 8,
                goals_against: 7,
                tie_lot: 20,
            },
        ];
        rank(&mut rows);
        assert_eq!(rows[0].team, "B");
        rows[1].regulation_wins = 2;
        rank(&mut rows);
        assert_eq!(rows[0].team, "A");
        rows[1].regulation_wins = 2;
        rows[1].goals_against = 5;
        rank(&mut rows);
        assert_eq!(rows[0].team, "B");
    }
    #[test]
    fn invalid_size_usage_and_venue_reject() {
        let q = fixture();
        let mut bad = q.clone();
        bad.batch_trials = 0;
        assert!(run(&bad).is_err());
        bad = q.clone();
        bad.rules.home_goals_multiplier = 1.1;
        assert!(run(&bad).is_err());
        bad = q.clone();
        bad.teams.truncate(1);
        assert!(run(&bad).is_err());
        bad = q.clone();
        bad.teams[0].starter_id = bad.teams[0].roster.units[0].creator.clone();
        assert!(run(&bad).is_err());
        bad = q.clone();
        bad.teams[0].starter_id = "absent".into();
        assert!(run(&bad).is_err());
        bad = q;
        bad.teams[1].roster.team = bad.teams[0].roster.team.clone();
        assert!(run(&bad).is_err());
    }
}
