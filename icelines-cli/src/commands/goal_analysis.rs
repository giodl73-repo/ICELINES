use anyhow::Result;
use icelines_fetch::goal_analysis::{self, GoalTarget};
use std::path::PathBuf;
pub async fn run(
    target: Option<String>,
    game: Option<u64>,
    event: Option<u32>,
    output: PathBuf,
    offline: Option<PathBuf>,
    data_root: Option<PathBuf>,
) -> Result<()> {
    let goal = match (target, game, event) {
        (Some(t), None, None) => GoalTarget::parse(&t)?,
        (None, Some(g), Some(e)) => GoalTarget::new(g, e)?,
        _ => anyhow::bail!("Provide a goal URL or both --game-id and --event-id"),
    };
    let root = data_root.unwrap_or_else(goal_analysis::default_data_root);
    let result = goal_analysis::prepare(
        goal,
        &root,
        crate::config::live_feeds_enabled(),
        offline.as_deref(),
    )
    .await?;
    let html = icelines_web::goal_analysis::render_goal_analysis_report(&result)?;
    std::fs::create_dir_all(&output)?;
    icelines_fetch::atomic_write::write_json_atomic(&output.join("analysis.json"), &result)?;
    icelines_fetch::atomic_write::write_bytes_atomic(
        &output.join("analysis.html"),
        html.as_bytes(),
    )?;
    println!(
        "Goal {} · {} frames",
        goal.key(),
        result["summary"]["frame_count"]
    );
    println!("Report: {}", output.join("analysis.html").display());
    println!("Evidence: {}", output.join("analysis.json").display());
    Ok(())
}
