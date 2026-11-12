use std::path::Path;
fn read(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let report=match args.as_slice(){
        [mode,root,prior,prior_toi] if mode=="budget"=>icelines_core::roster_budget::build(Path::new(root),Path::new(prior),Path::new(prior_toi),84.0,280.0,20.0)?,
        [mode,box_path,shifts,team] if mode=="units"=>icelines_core::observed_units::reconstruct(&read(Path::new(box_path))?,&read(Path::new(shifts))?,team.parse()?)?,
        [mode,manifest,player] if mode=="history"=>icelines_core::player_line_history::build(serde_json::from_value(read(Path::new(manifest))?)?,player.parse()?)?,
        [mode,manifest,player] if mode=="deployment-history"=>icelines_core::deployment_history::build(serde_json::from_value(read(Path::new(manifest))?)?,player.parse()?,None)?,
        [mode,manifest,player,faces] if mode=="deployment-history"=>icelines_core::deployment_history::build(serde_json::from_value(read(Path::new(manifest))?)?,player.parse()?,Some(Path::new(faces)))?,
        [mode,box_path,home,away] if mode=="html-shifts"=>icelines_core::shift_html::parse_reports(&read(Path::new(box_path))?,&std::fs::read_to_string(home)?,&std::fs::read_to_string(away)?)?,
        [mode,manifest,faces,teams] if mode=="center-lines"=>icelines_core::center_lines::build(serde_json::from_value(read(Path::new(manifest))?)?,Path::new(faces),Path::new(teams),None)?,
        [mode,manifest,faces,teams,toi] if mode=="center-lines"=>icelines_core::center_lines::build(serde_json::from_value(read(Path::new(manifest))?)?,Path::new(faces),Path::new(teams),Some(Path::new(toi)))?,
        _=>return Err("Usage: rinkcraft_roster_budget budget CAPTURE PRIOR_SUMMARY PRIOR_TOI | units BOX SHIFTS TEAM_ID | history APPEARANCE_MANIFEST PLAYER_ID | deployment-history APPEARANCE_MANIFEST PLAYER_ID [GAME_FACEOFFS] | html-shifts BOX HOME_HTML AWAY_HTML | center-lines GAME_MANIFEST FACEOFFS TEAMS [EV_TOI]".into())};
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
