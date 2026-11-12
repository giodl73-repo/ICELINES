use icelines_core::view_model::rinkcraft_readiness_followup as followup;
fn main()->Result<(),Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    let path=match args.as_slice(){[p]=>p,[_,p]=>p,[_,p,_]=>p,_=>return Err("Usage: rinkcraft_readiness_followup INPUT.json | --verify REPLAY.json | --step REPLAY.json ACTION".into())};
    if std::fs::metadata(path)?.len()>followup::MAX_REPLAY_BYTES as u64{return Err("Follow-up input exceeds bound".into())}
    let raw=std::fs::read_to_string(path)?;if raw.len()>followup::MAX_REPLAY_BYTES{return Err("Follow-up input exceeds bound".into())}
    let file=match args.as_slice(){
        [_]=>followup::export(serde_json::from_str(&raw)?)?,
        [flag,_] if flag=="--verify"=>followup::import(&raw)?,
        [flag,_,action] if flag=="--step"=>followup::step(followup::import(&raw)?,serde_json::from_value(serde_json::Value::String(action.to_str().ok_or("Action must be UTF-8")?.into()))?)?,
        _=>return Err("Unsupported follow-up arguments".into()),
    };
    println!("{}",serde_json::to_string(&file)?);Ok(())
}
