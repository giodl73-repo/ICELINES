//! Thin JSON adapter for the opportunity research prototype.
use icelines_core::player_fit::{build_player_fit, PlayerFitInput};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .ok_or("usage: player_fit INPUT.json OUTPUT.json")?;
    let output = args
        .next()
        .ok_or("usage: player_fit INPUT.json OUTPUT.json")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let input: PlayerFitInput = serde_json::from_slice(&std::fs::read(input)?)?;
    let view = build_player_fit(input).map_err(std::io::Error::other)?;
    std::fs::write(output, serde_json::to_vec_pretty(&view)?)?;
    Ok(())
}
