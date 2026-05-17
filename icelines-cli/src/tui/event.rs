use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Help,
    Back,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Search,
    Tab,
    /// Shift-Tab — cycle tabs in reverse.
    TabPrev,
    Refresh,
    Install,        // 'i' on Fetch+Install screen
    AddToGroup,     // 'g' — open group picker on any player-list screen
    AddToFavorites, // 'f' — instant add to Favorites group (no picker)
    GoToTab(usize), // number keys jump directly to a tab
    Space,          // Space — toggle focus in split-pane screens
    Char(char),
    Backspace,
    /// Phase Adams.3 — Ctrl+H toggles the Favorites side pane in
    /// MDI mode. No-op in SDI.
    ToggleFavoritesPane,
    /// Phase Adams.3 — Ctrl+L toggles the Schedule side pane in
    /// MDI mode. No-op in SDI.
    ToggleSchedulePane,
}

/// Poll for the next terminal event, returning None on timeout.
pub async fn next_event(timeout: Duration) -> Result<Option<Action>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let ev = event::read()?;
    Ok(map_event(ev))
}

fn map_event(ev: Event) -> Option<Action> {
    match ev {
        Event::Key(k) => map_key(k),
        _ => None,
    }
}

fn map_key(k: crossterm::event::KeyEvent) -> Option<Action> {
    // Ignore Release and Repeat — only handle Press to prevent double-fire
    if k.kind != KeyEventKind::Press {
        return None;
    }
    use KeyCode::*;
    match k.code {
        Char('q') if k.modifiers == KeyModifiers::NONE => Some(Action::Quit),
        Char('c') if k.modifiers == KeyModifiers::CONTROL => Some(Action::Quit),
        // Phase Adams.3 — MDI side-pane toggles. Many terminals
        // legacy-map Ctrl+H to Backspace (CSI ^H) — crossterm
        // surfaces them as Char('h') + CONTROL, so we filter on
        // the modifier match. The Backspace key (with no CONTROL)
        // still goes to KeyCode::Backspace below.
        Char('h') if k.modifiers == KeyModifiers::CONTROL => Some(Action::ToggleFavoritesPane),
        Char('l') if k.modifiers == KeyModifiers::CONTROL => Some(Action::ToggleSchedulePane),
        Char('?') => Some(Action::Help),
        Char('/') => Some(Action::Search),
        Char('r') => Some(Action::Refresh),
        Char('i') => Some(Action::Install),
        Char('g') => Some(Action::AddToGroup),
        Char('f') => Some(Action::AddToFavorites),
        // Number keys jump directly to a tab. 0 maps to the tenth tab.
        Char('1') => Some(Action::GoToTab(0)),
        Char('2') => Some(Action::GoToTab(1)),
        Char('3') => Some(Action::GoToTab(2)),
        Char('4') => Some(Action::GoToTab(3)),
        Char('5') => Some(Action::GoToTab(4)),
        Char('6') => Some(Action::GoToTab(5)),
        Char('7') => Some(Action::GoToTab(6)),
        Char('8') => Some(Action::GoToTab(7)),
        Char('9') => Some(Action::GoToTab(8)),
        Char('0') => Some(Action::GoToTab(9)),
        Char(' ') => Some(Action::Space),
        Char(c) => Some(Action::Char(c)),
        Esc => Some(Action::Escape),
        Backspace => Some(Action::Backspace),
        Up => Some(Action::Up),
        Down => Some(Action::Down),
        Left => Some(Action::Left),
        Right => Some(Action::Right),
        Enter => Some(Action::Enter),
        // Tab handling: many terminals deliver Shift-Tab as a distinct
        // `BackTab` keycode; others deliver `Tab` with `SHIFT`. Cover both.
        Tab if k.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::TabPrev),
        BackTab => Some(Action::TabPrev),
        Tab => Some(Action::Tab),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Press)
    }

    #[test]
    fn l0_mdi_ctrl_h_and_ctrl_l_map_to_side_pane_toggles() {
        assert!(matches!(
            map_key(key(KeyCode::Char('h'), KeyModifiers::CONTROL)),
            Some(Action::ToggleFavoritesPane)
        ));
        assert!(matches!(
            map_key(key(KeyCode::Char('l'), KeyModifiers::CONTROL)),
            Some(Action::ToggleSchedulePane)
        ));
    }

    #[test]
    fn l0_shift_tab_maps_to_reverse_tab_action() {
        assert!(matches!(
            map_key(key(KeyCode::Tab, KeyModifiers::SHIFT)),
            Some(Action::TabPrev)
        ));
        assert!(matches!(
            map_key(key(KeyCode::BackTab, KeyModifiers::NONE)),
            Some(Action::TabPrev)
        ));
    }

    #[test]
    fn l0_key_release_events_are_ignored() {
        let key = crossterm::event::KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );

        assert!(map_key(key).is_none());
    }
}
