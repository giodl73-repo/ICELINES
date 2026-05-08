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
