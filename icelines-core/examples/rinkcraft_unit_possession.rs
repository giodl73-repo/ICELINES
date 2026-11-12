//! Read-only explicit-unit authoring, replay, continuation and legacy freezing.
use icelines_core::view_model::{
    rinkcraft_game as game, rinkcraft_possession::Action, rinkcraft_units as units,
};
use std::path::Path;
fn read(path: &Path, max: usize) -> Result<String, Box<dyn std::error::Error>> {
    if std::fs::metadata(path)?.len() > max as u64 {
        return Err("Input file exceeds bound".into());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.len() > max {
        return Err("Input file exceeds bound".into());
    }
    Ok(raw)
}
fn blueprint(path: &Path) -> Result<units::Input, Box<dyn std::error::Error>> {
    let b: units::Blueprint = serde_json::from_str(&read(path, units::MAX_INPUT_BYTES)?)?;
    b.validate_shape()?;
    let base = path.parent().unwrap_or(Path::new("."));
    let cr: game::GameRules =
        serde_json::from_str(&read(&base.join(&b.character_rules_path), 65536)?)?;
    let rules: units::Rules = serde_json::from_str(&read(&base.join(&b.unit_rules_path), 8192)?)?;
    Ok(units::build_input(cr, rules, b)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let file=match args.as_slice() {
        [path]=>units::export(blueprint(Path::new(path))?)?,
        [flag,path] if flag=="--verify"=>units::import(&read(Path::new(path),units::MAX_REPLAY_BYTES)?)?,
        [flag,path] if flag=="--freeze-legacy"=>units::export(units::freeze_legacy(&read(Path::new(path),units::MAX_REPLAY_BYTES)?)?)?,
        [flag,path] if flag=="--input"=>{
            let input=serde_json::from_str(&read(Path::new(path),units::MAX_INPUT_BYTES)?)?;units::export(input)?
        },
        [flag,path] if flag=="--auto"=>{
            let mut file=units::import(&read(Path::new(path),units::MAX_REPLAY_BYTES)?)?;units::complete(&mut file.input)?;units::export(file.input)?
        },
        [flag,path,action] if flag=="--step"=>{
            let mut file=units::import(&read(Path::new(path),units::MAX_REPLAY_BYTES)?)?;
            let action:Action=serde_json::from_value(serde_json::Value::String(action.to_str().ok_or("Action must be UTF-8")?.into()))?;
            file.input.request.actions.push(action);units::export(file.input)?
        },
        _=>return Err("Usage: rinkcraft_unit_possession BLUEPRINT.json | --verify REPLAY.json | --freeze-legacy LEGACY.json | --input INPUT.json | --auto REPLAY.json | --step REPLAY.json ACTION".into()),
    };
    println!("{}", serde_json::to_string(&file)?);
    Ok(())
}
