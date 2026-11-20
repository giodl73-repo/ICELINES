//! Bounded readiness assessment or exact report verification. No mutation of unit scenes.
use icelines_core::view_model::rinkcraft_goalie_readiness as readiness;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let (verify, path) = match args.as_slice() {
        [path] => (false, path),
        [flag, path] if flag == "--verify" => (true, path),
        _ => {
            return Err(
                "Usage: rinkcraft_goalie_readiness INPUT.json | --verify REPORT.json".into(),
            )
        }
    };
    if std::fs::metadata(path)?.len() > readiness::MAX_REPORT_BYTES as u64 {
        return Err("Readiness input exceeds bound".into());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.len() > readiness::MAX_REPORT_BYTES {
        return Err("Readiness input exceeds bound".into());
    }
    let report = if verify {
        readiness::import(&raw)?
    } else {
        readiness::export(serde_json::from_str(&raw)?)?
    };
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
