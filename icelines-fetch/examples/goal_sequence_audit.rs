//! Offline adapter for the shared goal-sequence audit.
use anyhow::Result;
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        a.len() == 5,
        "usage: goal_sequence_audit BUNDLE LANDING BOXSCORE PLAY_BY_PLAY OUTPUT"
    );
    let b = icelines_fetch::goal_visualizer::load_bundle(std::path::Path::new(&a[0]))?;
    let read = |path: &str| -> Result<serde_json::Value> {
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    };
    let result = icelines_fetch::goal_sequence_audit::audit_bundle(
        &b,
        &read(&a[1])?,
        &read(&a[2])?,
        &read(&a[3])?,
    )?;
    std::fs::write(&a[4], serde_json::to_vec_pretty(&result)?)?;
    println!(
        "Audited {} timed goal sequences; preserved native units and unknown assignments",
        b.events.len()
    );
    Ok(())
}
