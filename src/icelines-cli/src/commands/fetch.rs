use crate::cli::FetchSubcommand;

pub async fn run(args: FetchSubcommand) -> anyhow::Result<()> {
    let sub = match args {
        FetchSubcommand::Rosters   => "fetch rosters",
        FetchSubcommand::Stats     => "fetch stats",
        FetchSubcommand::All       => "fetch all",
        FetchSubcommand::Positions => "fetch positions",
    };
    println!("icelines {sub}: not yet implemented in Phase 1 scaffolding");
    Ok(())
}
