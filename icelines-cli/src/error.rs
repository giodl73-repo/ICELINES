/// Translate crate errors into user-facing messages and exit.
pub fn handle_error(e: anyhow::Error) -> ! {
    eprintln!("error: {}", e);
    // Print the full cause chain, skipping the root (already printed above).
    let chain: Vec<_> = e.chain().skip(1).collect();
    for (i, cause) in chain.iter().enumerate() {
        if i == 0 {
            eprintln!("caused by:");
        }
        eprintln!("  {}: {}", i, cause);
    }
    std::process::exit(1);
}
