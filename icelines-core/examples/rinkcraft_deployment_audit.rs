//! Offline audit of an explicitly captured league window; no network access.
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "Usage: rinkcraft_deployment_audit CAPTURE_DIR PRIOR_SUMMARY.json PRIOR_TOI.json"
                .into(),
        );
    }
    let report = icelines_core::deployment_audit::audit_capture(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        84.0,
        280.0,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
