// Phase Masterton.1 — declarative TUI chrome contract.
//
// Each TUI screen module exports a `chrome()` accessor returning
// `ScreenChrome { title, keybinds }`. The shell (screens/mod.rs)
// renders both consistently: the title goes right-aligned on the
// header row alongside the tab strip; the keybinds render as
// chips on the footer row, with the transient flash slot
// overlaying the chips when present.
//
// Pre-Masterton: each screen pushed its own keybind hints
// imperatively via `app.status = "Queries · p:projections  ←/→:edit
//  Tab:focus results"`. That mixed permanent ("here's what you can
// do on this screen") with transient ("Saved query 'centerleaders'")
// in a single string. Post-Masterton: keybinds are declarative
// (rendered as chips); transient feedback continues through the
// status field as a flash overlay.

#![allow(clippy::module_name_repetitions)]

/// Phase Masterton.1 — declarative chrome contract for a TUI
/// screen. Each per-screen module exports a `chrome()` accessor
/// returning this struct; the shell renders header + footer from
/// it consistently across screens.
#[derive(Debug, Clone)]
pub struct ScreenChrome {
    /// Screen title shown in the breadcrumb area of the header.
    /// Examples: "Stats / Queries", "Stats / Queries / Filter",
    /// "Schedule — week of 2026-04-29".
    pub title: String,
    /// Keybind hints rendered as chips in the footer row. Order
    /// is priority order — at narrow widths the chrome renderer
    /// drops trailing chips with a `…` indicator (per spec
    /// glass-2). Most-important keybinds first.
    pub keybinds: Vec<KeyHint>,
}

/// One keybind hint — a key (or key combo) and a short label
/// describing what it does. Both are `&'static str` because every
/// hint is a compile-time constant; the chrome renderer doesn't
/// allocate per-frame for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl KeyHint {
    pub const fn new(key: &'static str, action: &'static str) -> Self {
        Self { key, action }
    }
}

/// Default keybinds always available. The chrome renderer appends
/// these to whatever each screen declares so screens don't repeat
/// global keys (Tab/y/Shift+P/F/?/q work from every screen).
///
/// These are NOT included in a screen's `chrome.keybinds` list —
/// the renderer adds them automatically so screen authors only
/// declare screen-specific hints.
///
/// Order matters: the first chips render leftmost; under
/// overflow (per spec glass-2) trailing chips drop with a `…`
/// indicator. Most-important global keys first.
pub const GLOBAL_KEYBINDS: &[KeyHint] = &[
    KeyHint::new("Tab", "next tab"),
    KeyHint::new("y", "season"),
    KeyHint::new("Shift+P", "toggle type"),
    KeyHint::new("F", "admin"),
    KeyHint::new("?", "help"),
    KeyHint::new("q", "quit"),
];

impl Default for ScreenChrome {
    /// Empty chrome — no title, no keybinds. Used for screens that
    /// haven't declared a chrome accessor yet (the chrome renderer
    /// falls back to global keybinds only). Should not appear in
    /// production once Masterton.1 covers all main screens.
    fn default() -> Self {
        Self {
            title: String::new(),
            keybinds: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `KeyHint::new` is `const` — required so the per-screen
    /// chrome accessors can declare `&'static [KeyHint]` arrays
    /// without runtime allocation.
    #[test]
    fn l0_chrome_keyhint_new_is_const() {
        const HINT: KeyHint = KeyHint::new("f", "filter");
        assert_eq!(HINT.key, "f");
        assert_eq!(HINT.action, "filter");
    }

    /// `ScreenChrome` clones cleanly — the renderer takes an
    /// owned chrome from the accessor and may pass it to
    /// sub-helpers that want their own copy.
    #[test]
    fn l0_chrome_clones() {
        let c = ScreenChrome {
            title: "Test".to_owned(),
            keybinds: vec![KeyHint::new("a", "alpha")],
        };
        let cloned = c.clone();
        assert_eq!(cloned.title, "Test");
        assert_eq!(cloned.keybinds.len(), 1);
        assert_eq!(cloned.keybinds[0].key, "a");
    }

    /// `Default` produces an empty chrome — used by the renderer
    /// fallback when a screen hasn't declared a chrome accessor.
    #[test]
    fn l0_chrome_default_is_empty() {
        let c = ScreenChrome::default();
        assert_eq!(c.title, "");
        assert!(c.keybinds.is_empty());
    }

    /// `GLOBAL_KEYBINDS` leads with Tab and ends with q — pins
    /// the head/tail of the list so a refactor can't silently
    /// reorder the user-facing chip layout. The middle of the
    /// list is checked separately by the
    /// `*_carry_navigation_keys` test below.
    #[test]
    fn l0_chrome_global_keybinds_head_is_tab_tail_is_quit() {
        assert!(!GLOBAL_KEYBINDS.is_empty());
        assert_eq!(GLOBAL_KEYBINDS[0].key, "Tab");
        assert_eq!(
            GLOBAL_KEYBINDS[GLOBAL_KEYBINDS.len() - 1].key,
            "q"
        );
    }

    /// `KeyHint` derives `PartialEq` so tests can compare hints
    /// directly without explicit field-by-field assertions.
    #[test]
    fn l0_chrome_keyhint_partial_eq() {
        let a = KeyHint::new("f", "filter");
        let b = KeyHint::new("f", "filter");
        let c = KeyHint::new("g", "favorites");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// `KeyHint` is `Copy` — the chrome renderer iterates over
    /// hint slices and the copy ergonomics make rendering simpler.
    #[test]
    fn l0_chrome_keyhint_is_copy() {
        let h = KeyHint::new("f", "filter");
        let _h2 = h; // copy
        let _h3 = h; // can use original after move
        assert_eq!(h.key, "f");
    }

    /// `GLOBAL_KEYBINDS` includes the cross-screen keys today
    /// hardcoded into render_nav's hint string (Tab / y / Shift+P
    /// / F / ? / q). Pin so a future refactor doesn't lose them.
    #[test]
    fn l0_chrome_global_keybinds_carry_navigation_keys() {
        let keys: Vec<&str> = GLOBAL_KEYBINDS.iter().map(|h| h.key).collect();
        for needed in ["Tab", "y", "Shift+P", "F", "?", "q"] {
            assert!(
                keys.contains(&needed),
                "GLOBAL_KEYBINDS must include {needed:?}; got: {keys:?}"
            );
        }
    }
}
