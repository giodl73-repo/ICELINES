use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Action {
    Quit,
    Help,
    #[allow(dead_code)]
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
    GoToTab(usize), // '1'–'7' — jump directly to a tab
    Space,          // Space — toggle focus in split-pane screens
    Char(char),
    Backspace,
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
        Char('?') => Some(Action::Help),
        Char('/') => Some(Action::Search),
        Char('r') => Some(Action::Refresh),
        Char('i') => Some(Action::Install),
        Char('g') => Some(Action::AddToGroup),
        Char('f') => Some(Action::AddToFavorites),
        // Number keys 1–7 jump directly to a tab
        Char('1') => Some(Action::GoToTab(0)),
        Char('2') => Some(Action::GoToTab(1)),
        Char('3') => Some(Action::GoToTab(2)),
        Char('4') => Some(Action::GoToTab(3)),
        Char('5') => Some(Action::GoToTab(4)),
        Char('6') => Some(Action::GoToTab(5)),
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
