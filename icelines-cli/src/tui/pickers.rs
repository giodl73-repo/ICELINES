// Phase Norris.6 — `<Picker>State` repeats the module name in the
// type identifier. Same canonical pattern as Norris.1-4.
#![allow(clippy::module_name_repetitions)]

//! Phase Norris.6 — cross-screen overlay state.
//!
//! Holds the working state of overlays that aren't tied to a single
//! screen:
//!
//! - **DatePickerState** — shared between the Tonight (Scores) and
//!   Schedule tabs. The `d` key on either screen opens the same
//!   overlay; `picker_target` says which screen to apply the picked
//!   date to. Lives on `App` as `app.date_picker` (replaces the
//!   pre-Norris-6 `scores_picker_*` + `picker_target` fields).
//!
//! - **GroupPickerState** — shared between the player card and
//!   team roster. The `g` key opens the picker for a specific
//!   player; the picker lists the user's groups so they can add
//!   the player. Lives on `App` as `app.group_picker`.
//!
//! Both are cross-screen by design (which is why they don't live
//! in any single screen module).

use crate::tui::app::PickerTarget;

// ── Date picker (Foster.1.4) ────────────────────────────────────────────────

/// Phase Norris.6 — date-picker overlay state.
///
/// Replaces the pre-Norris.6 `scores_picker_*` and `picker_target`
/// fields on `App`. Held as `app.date_picker`.
#[derive(Debug)]
pub struct DatePickerState {
    /// True while the `d`-key date picker is visible.
    pub open: bool,
    /// Text being typed in the picker's input box.
    pub input: String,
    /// Validation error to display below the input.
    pub err: Option<String>,
    /// Which screen the picker is bound to (Scores or Schedule).
    pub target: PickerTarget,
}

impl Default for DatePickerState {
    fn default() -> Self {
        Self {
            open: false,
            input: String::new(),
            err: None,
            target: PickerTarget::default(),
        }
    }
}

// ── Group picker (player → group membership) ────────────────────────────────

/// Phase Norris.6 — group-picker overlay state.
///
/// Replaces the pre-Norris.6 `group_picker_*` fields on `App`.
/// Held as `app.group_picker`.
#[derive(Debug, Default)]
pub struct GroupPickerState {
    /// True while the picker overlay is visible.
    pub open: bool,
    /// Group names available to add the selected player to.
    pub list: Vec<String>,
    /// `(normalized_name, full_name)` of the player about to be
    /// added. None when the picker isn't bound to a target.
    pub player: Option<(String, String)>,
}

#[cfg(test)]
mod norris_state_tests {
    use super::*;

    // ── Phase Norris.6 — DatePickerState contract ──────────────────────────

    /// Default open is false — the overlay starts hidden.
    #[test]
    fn l0_norris_date_picker_default_closed() {
        let s = DatePickerState::default();
        assert!(!s.open);
    }

    /// Default input is empty.
    #[test]
    fn l0_norris_date_picker_default_input_empty() {
        let s = DatePickerState::default();
        assert_eq!(s.input, "");
    }

    /// Default has no validation error.
    #[test]
    fn l0_norris_date_picker_default_no_err() {
        let s = DatePickerState::default();
        assert!(s.err.is_none());
    }

    /// Default target is `PickerTarget::default()` (Scores —
    /// matches the legacy pre-Norris.6 init).
    #[test]
    fn l0_norris_date_picker_default_target_matches_picker_target_default() {
        let s = DatePickerState::default();
        assert_eq!(s.target, PickerTarget::default());
    }

    /// `App::new` wires `app.date_picker` through default.
    #[test]
    fn l0_norris_date_picker_app_new_uses_default() {
        let app = crate::tui::app::App::new(false);
        assert!(!app.date_picker.open);
        assert_eq!(app.date_picker.input, "");
        assert!(app.date_picker.err.is_none());
        assert_eq!(app.date_picker.target, PickerTarget::default());
    }

    /// Debug derive renders without panic (forge-1 sanity).
    #[test]
    fn l0_norris_date_picker_default_debug_renders() {
        let s = DatePickerState::default();
        let dbg = format!("{:?}", s);
        assert!(
            dbg.contains("DatePickerState"),
            "Debug output must include the struct name; got: {dbg}"
        );
    }

    // ── Phase Norris.6 — GroupPickerState contract ─────────────────────────

    /// Default open is false.
    #[test]
    fn l0_norris_group_picker_default_closed() {
        let s = GroupPickerState::default();
        assert!(!s.open);
    }

    /// Default list is empty.
    #[test]
    fn l0_norris_group_picker_default_list_empty() {
        let s = GroupPickerState::default();
        assert!(s.list.is_empty());
    }

    /// Default player binding is None — overlay isn't bound to a
    /// target yet.
    #[test]
    fn l0_norris_group_picker_default_player_none() {
        let s = GroupPickerState::default();
        assert!(s.player.is_none());
    }

    /// `App::new` wires `app.group_picker` through default.
    #[test]
    fn l0_norris_group_picker_app_new_uses_default() {
        let app = crate::tui::app::App::new(false);
        assert!(!app.group_picker.open);
        assert!(app.group_picker.list.is_empty());
        assert!(app.group_picker.player.is_none());
    }

    /// Debug derive renders without panic.
    #[test]
    fn l0_norris_group_picker_default_debug_renders() {
        let s = GroupPickerState::default();
        let dbg = format!("{:?}", s);
        assert!(
            dbg.contains("GroupPickerState"),
            "Debug output must include the struct name; got: {dbg}"
        );
    }
}
