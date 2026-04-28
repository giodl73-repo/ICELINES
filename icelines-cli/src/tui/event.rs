use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Action {
    Quit,
    Help,
    #[allow(dead_code)] Back,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Search,
    Tab,
    Refresh,
    Install,   // 'i' on Fetch+Install screen — install selected season
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
        Char(c)   => Some(Action::Char(c)),
        Esc       => Some(Action::Escape),
        Backspace => Some(Action::Backspace),
        Up    => Some(Action::Up),
        Down  => Some(Action::Down),
        Left  => Some(Action::Left),
        Right => Some(Action::Right),
        Enter => Some(Action::Enter),
        Tab       => Some(Action::Tab),
        _         => None,
    }
}
