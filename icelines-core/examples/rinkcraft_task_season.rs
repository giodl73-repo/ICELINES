use icelines_core::view_model::{
    rinkcraft_game as game, rinkcraft_season as old, rinkcraft_task_season as task,
};
use std::path::Path;
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn write<T: serde::Serialize>(path: &Path, v: &T) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, serde_json::to_string_pretty(v)?)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    match a.as_slice(){
        [mode,input,out] if mode=="run"=>{let r:task::Request=read(input)?;write(Path::new(out),&task::run(&r)?)?;},
        [mode,input,out] if mode=="inspect"=>{let r:task::Request=read(input)?;write(Path::new(out),&task::inspect(&r)?)?;},
        [mode,base,plan,out] if mode=="study"=>study(read(base)?,read(plan)?,Path::new(out))?,
        [mode,base,rules,out] if mode=="demo"=>demo(read(base)?,read(rules)?,Path::new(out))?,
        _=>return Err("Usage: rinkcraft_task_season run REQUEST OUTPUT | inspect REQUEST OUTPUT | study BASE_REQUEST PLAN OUTPUT_DIR | demo AGGREGATE_NEUTRAL_REQUEST TASK_RULES OUTPUT_DIR".into())
    }
    Ok(())
}
#[derive(serde::Deserialize)]
struct StudyPlan {
    schema: String,
    cases: Vec<task::StudyCase>,
}
fn study(
    base: task::Request,
    plan: StudyPlan,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.schema != "rinkcraft.task-balance-plan.0.1"
        || plan.cases.is_empty()
        || plan.cases.len() > 1000
    {
        return Err("Invalid task balance plan".into());
    }
    std::fs::create_dir_all(out)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for c in plan.cases {
        if !seen.insert(c.id.clone()) {
            return Err("Duplicate study ID".into());
        }
        let r = task::apply_study_case(&base, &c)?;
        let canonical: task::Request = serde_json::from_slice(&serde_json::to_vec(&r)?)?;
        let result = task::inspect(&canonical)?;
        let focus = result["teams"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["team"] == c.team)
            .ok_or("Missing study team")?
            .clone();
        let pair = result["proposed_audit"]["pair_audits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["home"] == c.team && p["away"] == "CAR")
            .ok_or("Missing study benchmark pair")?
            .clone();
        let reference_pair = result["reference_audit"]["pair_audits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["home"] == c.team && p["away"] == "CAR")
            .ok_or("Missing reference benchmark pair")?
            .clone();
        rows.push(serde_json::json!({"id":c.id,"label":c.label,"focus":focus,"benchmark_pair":pair,"reference_benchmark_pair":reference_pair,"input_sha256":result["input_sha256"],"rules_sha256":result["rules_sha256"]}));
        write(&out.join(format!("{}-inspection.json", c.id)), &result)?;
        println!(
            "{}: exact regular points delta {}",
            c.id, focus["expected_points_delta"]
        );
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.task-balance-summary.0.1","kind":"authored_game_scenario_not_nhl_estimate","seed":null,"trials":null,"cases":rows}),
    )?;
    Ok(())
}
fn demo(
    base: old::Request,
    rules: task::Rules,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let mut teams = Vec::new();
    for t in &base.teams {
        if t.reference.len() != 20 {
            return Err("Demo needs 20 fictional members per team".into());
        }
        let members = t
            .reference
            .iter()
            .map(|m| task::Member {
                id: m.id.clone(),
                character: m.character.clone(),
            })
            .collect();
        let mut units = Vec::new();
        for line in 0..4 {
            for pair in 0..3 {
                for goalie in 0..2 {
                    let id = |index: usize| t.reference[index].id.clone();
                    let f = [id(line * 3), id(line * 3 + 1), id(line * 3 + 2)];
                    let d = [id(12 + pair * 2), id(13 + pair * 2)];
                    units.push(task::Unit {
                        id: format!("line{}-pair{}-goalie{}", line + 1, pair + 1, goalie + 1),
                        weight: [0.32, 0.28, 0.24, 0.16][line]
                            * [0.45, 0.33, 0.22][pair]
                            * [0.75, 0.25][goalie],
                        creator: f[0].clone(),
                        receiver: f[1].clone(),
                        forechecker: f[2].clone(),
                        interceptor: d[0].clone(),
                        gap_defender: d[1].clone(),
                        forwards: f,
                        defense: d,
                        goalie: id(18 + goalie),
                    });
                }
            }
        }
        teams.push(task::Team {
            team: t.team.clone(),
            members,
            units,
        });
    }
    let r = task::Request {
        schema: task::SCHEMA.into(),
        kind: "authored_game_scenario_not_nhl_estimate".into(),
        character_rules: base.character_rules,
        rules,
        calendar: base.forecast,
        reference: teams.clone(),
        proposed: teams,
        config: icelines_core::view_model::team_season_forecast::TeamSeasonSimulationConfig {
            trials: 128,
            seed: 20261112,
        },
    };
    let focus = r
        .proposed
        .iter()
        .position(|t| t.team == "NYR")
        .ok_or("Demo needs NYR")?;
    let mut rows = Vec::new();
    for name in [
        "neutral",
        "aligned-specialists",
        "swapped-specialists",
        "defense-denial",
        "goalie-tracking",
        "goalie-outlet",
        "forecheck",
    ] {
        let mut q = r.clone();
        for index in 0..20 {
            let mut scores = [10; 6];
            let change = match name {
                "aligned-specialists" | "swapped-specialists" if index < 12 && index % 3 == 0 => {
                    scores[2] = 16;
                    scores[3] = 8;
                    true
                }
                "aligned-specialists" | "swapped-specialists" if index < 12 && index % 3 == 1 => {
                    scores[2] = 8;
                    scores[3] = 16;
                    true
                }
                "defense-denial" if (12..18).contains(&index) => {
                    scores[4] = 16;
                    scores[5] = 16;
                    true
                }
                "goalie-tracking" if index >= 18 => {
                    scores[0] = 16;
                    true
                }
                "goalie-outlet" if index >= 18 => {
                    scores[5] = 16;
                    true
                }
                "forecheck" if index < 12 && index % 3 == 2 => {
                    scores[5] = 16;
                    true
                }
                _ => false,
            };
            if change {
                let m = &mut q.proposed[focus].members[index];
                let c = &m.character;
                m.character = game::build(
                    &q.character_rules,
                    game::BuildRequest {
                        template_id: c.template_id.clone(),
                        name: c.name.clone(),
                        position: c.position.clone(),
                        scores: game::ScoreMethod::Manual { scores },
                        inspiration: None,
                    },
                )?;
            }
        }
        if name == "swapped-specialists" {
            for u in &mut q.proposed[focus].units {
                std::mem::swap(&mut u.creator, &mut u.receiver);
            }
        }
        let request_path = out.join(format!("{name}-request.json"));
        write(&request_path, &q)?;
        let persisted: task::Request = read(request_path.to_str().ok_or("Invalid request path")?)?;
        let result = task::run(&persisted)?;
        let pair = result["proposed_audit"]["pair_audits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["home"] == "NYR" && p["away"] == "CAR")
            .unwrap()
            .clone();
        let impact = result["impacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["team"] == "NYR")
            .unwrap()
            .clone();
        rows.push(serde_json::json!({"case":name,"pair_nyr_home_car_away":pair,"impact":impact,"input_sha256":result["input_sha256"]}));
        write(&out.join(format!("{name}-result.json")), &result)?;
        println!(
            "{name}: mean NYR standings points delta {}",
            impact["average_points_delta"]
        );
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.task-season-demo.0.1","kind":"authored_game_scenario_not_nhl_estimate","seed":r.config.seed,"trials":r.config.trials,"teams":r.proposed.len(),"characters":r.proposed.iter().map(|t|t.members.len()).sum::<usize>(),"units_per_team":24,"games":r.calendar.schedule_games,"cases":rows}),
    )?;
    Ok(())
}
