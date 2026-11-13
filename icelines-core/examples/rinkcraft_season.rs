//! Offline profile adapter and controlled all-32 sandbox; existing season engine owns games.
use icelines_core::view_model::{
    rinkcraft_game as game, rinkcraft_season as adapter, team_game_forecast as forecast,
    team_season_forecast as season,
};
use std::{collections::BTreeSet, path::Path};
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn write<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice(){
        [mode,input,output] if mode=="run"=>{let request:adapter::Request=read(input)?;write(Path::new(output),&adapter::run(&request)?)?;},
        [mode,rules,mapping,calendar,output] if mode=="demo"=>demo(read(rules)?,read(mapping)?,read(calendar)?,Path::new(output))?,
        _=>return Err("Usage: rinkcraft_season run REQUEST_JSON OUTPUT_JSON | demo CHARACTER_RULES PROFILE_RULES ARCHIVED_SEASON_JSON OUTPUT_DIR".into())
    }
    Ok(())
}
fn demo(
    rules: game::GameRules,
    mapping: adapter::ProfileRules,
    calendar: season::TeamSeasonForecastView,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    // Reuse only calendar identity/dates; discard archived ratings, results and scenario effects.
    let teams: BTreeSet<_> = calendar
        .games
        .iter()
        .flat_map(|g| [g.home_team.clone(), g.away_team.clone()])
        .collect();
    let games = calendar
        .games
        .iter()
        .map(|g| forecast::TeamForecastGameInput {
            game_id: g.game_id,
            date: g.date,
            home_team: g.home_team.clone(),
            away_team: g.away_team.clone(),
            home_score: None,
            away_score: None,
            final_result: false,
            last_period: None,
        })
        .collect();
    let neutral = forecast::build_team_game_forecast(
        calendar.season,
        games,
        teams
            .iter()
            .map(|t| forecast::TeamForecastStrengthInput {
                team: t.clone(),
                strength: 50.0,
            })
            .collect(),
        forecast::TeamForecastParameters::default(),
        Some(calendar.schedule_games),
        None,
    )?;
    let mut rosters = Vec::new();
    for team in &teams {
        let mut roster = Vec::new();
        for n in 0..20 {
            let (pos, template, share) = if n < 12 {
                (["LW", "C", "RW"][n % 3], "conductor", 1.0 / 12.0)
            } else if n < 18 {
                ("D", "warden-skater", 1.0 / 6.0)
            } else {
                ("G", "setkeeper", 0.5)
            };
            let character = game::build(
                &rules,
                game::BuildRequest {
                    template_id: template.into(),
                    name: format!("{team} fictional {n}"),
                    position: pos.into(),
                    scores: game::ScoreMethod::Manual { scores: [10; 6] },
                    inspiration: None,
                },
            )?;
            roster.push(adapter::Member {
                id: format!("{team}:{n}"),
                assignment: None,
                character,
                exposure_share: share,
            });
        }
        rosters.push(adapter::TeamChange {
            team: team.clone(),
            reference: roster.clone(),
            proposed: roster,
        });
    }
    let request = adapter::Request {
        schema: adapter::SCHEMA.into(),
        kind: "authored_game_scenario_not_nhl_estimate".into(),
        label: "Rinkcraft authored profile sandbox".into(),
        character_rules: rules.clone(),
        profile_rules: mapping,
        teams: rosters,
        effective_date: neutral.schedule_start,
        end_date: None,
        as_of_date: None,
        forecast: neutral,
        config: season::TeamSeasonSimulationConfig {
            trials: 256,
            seed: 20261112,
        },
    };
    let focus = request
        .teams
        .iter()
        .position(|t| t.team == "NYR")
        .ok_or("Calendar needs NYR for controlled sandbox")?;
    let mut compact = Vec::new();
    for (name, index, ability) in [
        ("neutral", None, 0),
        ("shooting", Some(0), 3),
        ("goalie-tracking", Some(18), 0),
        ("defending", Some(12), 5),
    ] {
        let mut r = request.clone();
        if let Some(index) = index {
            let old = &r.teams[focus].proposed[index].character;
            let mut scores = [10; 6];
            scores[ability] = 20;
            r.teams[focus].proposed[index].character = game::build(
                &rules,
                game::BuildRequest {
                    template_id: old.template_id.clone(),
                    name: old.name.clone(),
                    position: old.position.clone(),
                    scores: game::ScoreMethod::Manual { scores },
                    inspiration: None,
                },
            )?;
        }
        let request_path = out.join(format!("{name}-request.json"));
        write(&request_path, &r)?;
        // Persisted input is authoritative: demo and run must parse the same f64 bytes.
        let persisted: adapter::Request =
            read(request_path.to_str().ok_or("Non UTF-8 request path")?)?;
        let result = adapter::run(&persisted)?;
        let delta = result["audit"]["teams"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["team"] == "NYR")
            .unwrap()["strength_delta"]
            .clone();
        let impact = result["impacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["team"] == "NYR")
            .unwrap()
            .clone();
        compact.push(serde_json::json!({"case":name,"kind":"authored_game_scenario_not_nhl_estimate","changed_member_index":index,"ability_index":if index.is_some(){Some(ability)}else{None},"strength_delta":delta,"impact":impact,"input_sha256":result["input_sha256"]}));
        write(&out.join(format!("{name}-result.json")), &result)?;
        println!("{name}: NYR authored strength delta {delta}");
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.profile-season-demo.0.1","seed":request.config.seed,"trials":request.config.trials,"schedule_games":request.forecast.schedule_games,
        "teams":teams.len(),"characters":request.teams.iter().map(|t|t.proposed.len()).sum::<usize>(),"cases":compact,
        "limits":["Only archived calendar reused; all team strengths neutral 50 and all profiles fictional manual 10 except named single-attribute interventions",
            "Authored mapping and game sensitivity, not NHL player ratings, fit calibration or a current-season forecast",
            "Same seed/trials and identical baseline; Monte Carlo differences are conditional on the declared model"]}),
    )?;
    Ok(())
}
