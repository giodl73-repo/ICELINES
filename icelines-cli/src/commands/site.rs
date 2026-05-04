//! `icelines site {build,serve,deploy}` — mkdocs site lifecycle.
//!
//! Phase King Clancy King.1.0 grouped the previously-flat
//! `Commands::Build/Serve/Deploy` under this `site` namespace so the bare
//! `icelines serve` could be reclaimed for the web dashboard. The old
//! top-level aliases still dispatch here through this module's helpers
//! (with a stderr deprecation warning, removed in v0.14).
//!
//! All three leaves delegate to the existing implementations in
//! `commands::build` and `commands::serve_deploy` — King.1.0 is a pure
//! reshape, no behavior change.
use crate::cli::SiteSubcommand;

pub async fn run(sub: SiteSubcommand) -> anyhow::Result<()> {
    match sub {
        SiteSubcommand::Build { no_site } => super::build::run(no_site).await,
        SiteSubcommand::Serve { port } => super::serve_deploy::run_serve(port).await,
        SiteSubcommand::Deploy { remote } => super::serve_deploy::run_deploy(&remote).await,
    }
}

#[cfg(test)]
mod tests {
    //! L0 fences — King.1.0 deprecation warning behavior.
    //!
    //! These tests live alongside the `system_tests.rs` L2 suite that
    //! exercises the actual binary. Here we just verify the warning text
    //! constants used by `main.rs` so that the deprecation message format
    //! is locked.
    //!
    //! The runtime dispatch path (warning emitted, then mkdocs called) is
    //! covered by the L2 system tests once those are added — King.1.0
    //! relies on the cargo build + clippy + manual smoke for now.

    /// l0_deprecation_warning_text_mentions_v0_14
    /// — every deprecation message must name v0.14 as the removal target
    ///   so users have a concrete deadline. We check that the version string
    ///   appears somewhere AFTER each deprecation block's anchor phrase.
    #[test]
    fn l0_deprecation_warning_text_mentions_v0_14() {
        let main_rs = include_str!("../main.rs");
        for anchor in [
            "'icelines build' moved",
            "'icelines serve' is being reclaimed",
            "'icelines deploy' moved",
        ] {
            let start = main_rs
                .find(anchor)
                .unwrap_or_else(|| panic!("main.rs missing deprecation anchor: {anchor}"));
            // Look for v0.14 in the next ~600 chars (a generous window;
            // bounded by char count via take to avoid byte-boundary slicing).
            let after: String = main_rs[start..].chars().take(600).collect();
            assert!(
                after.contains("v0.14"),
                "deprecation block starting at {anchor:?} must name v0.14 as removal version"
            );
        }
    }

    /// l0_deprecation_warnings_point_users_at_site_subcommands
    /// — each warning must tell the user the new path so they can fix
    ///   their scripts immediately.
    #[test]
    fn l0_deprecation_warnings_point_users_at_site_subcommands() {
        let main_rs = include_str!("../main.rs");
        assert!(
            main_rs.contains("'icelines site build'"),
            "build deprecation must direct users to 'icelines site build'"
        );
        assert!(
            main_rs.contains("'icelines site serve'"),
            "serve deprecation must direct users to 'icelines site serve'"
        );
        assert!(
            main_rs.contains("'icelines site deploy'"),
            "deploy deprecation must direct users to 'icelines site deploy'"
        );
    }
}
