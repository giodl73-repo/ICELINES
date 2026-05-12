use comfy_table::Color as TableColor;
use icelines_core::{FitClass, WebFitClass};
use ratatui::{
    style::{Color as TuiColor, Modifier, Style},
    widgets::{Block, Borders},
};

pub fn table_fit_color(fit: FitClass) -> TableColor {
    match fit {
        FitClass::Elite => TableColor::Green,
        FitClass::Solid => TableColor::Yellow,
        FitClass::Buried => TableColor::Blue,
        FitClass::Stretch => TableColor::Red,
    }
}

pub fn tui_web_fit_color(fit: WebFitClass) -> TuiColor {
    match fit {
        WebFitClass::Elite => TuiColor::Green,
        WebFitClass::Solid => TuiColor::Yellow,
        WebFitClass::Buried => TuiColor::Cyan,
        WebFitClass::Stretch => TuiColor::Red,
    }
}

pub fn web_fit_ascii_label(fit: WebFitClass) -> &'static str {
    match fit {
        WebFitClass::Elite => "ELITE",
        WebFitClass::Solid => "SOLID",
        WebFitClass::Buried => "UNDERUSED",
        WebFitClass::Stretch => "OVEREXTENDED",
    }
}

pub fn web_fit_report_description(fit: WebFitClass) -> &'static str {
    match fit {
        WebFitClass::Elite => "elite - plays above their line on most teams",
        WebFitClass::Solid => "solid - fits their role well",
        WebFitClass::Buried => "underused - worth more elsewhere",
        WebFitClass::Stretch => "overextended - stretched in current role",
    }
}

pub fn tui_panel_block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(tui_panel_border_style())
        .title(title.into())
}

pub fn tui_panel_border_style() -> Style {
    Style::default().fg(TuiColor::DarkGray)
}

pub fn tui_title_style() -> Style {
    Style::default()
        .fg(TuiColor::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn tui_header_style() -> Style {
    Style::default()
        .fg(TuiColor::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn tui_meta_style() -> Style {
    Style::default().fg(TuiColor::DarkGray)
}

pub fn tui_warning_style() -> Style {
    Style::default().fg(TuiColor::Yellow)
}

pub fn tui_error_style() -> Style {
    Style::default().fg(TuiColor::Red)
}

pub fn tui_selected_style() -> Style {
    Style::default()
        .fg(TuiColor::Black)
        .bg(TuiColor::Cyan)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_cli_visual_web_fit_labels_are_ascii() {
        for fit in [
            WebFitClass::Elite,
            WebFitClass::Solid,
            WebFitClass::Buried,
            WebFitClass::Stretch,
        ] {
            assert!(web_fit_ascii_label(fit).is_ascii());
            assert!(web_fit_report_description(fit).is_ascii());
        }
        assert_eq!(web_fit_ascii_label(WebFitClass::Buried), "UNDERUSED");
        assert_eq!(web_fit_ascii_label(WebFitClass::Stretch), "OVEREXTENDED");
    }

    #[test]
    fn l0_cli_visual_web_fit_colors_follow_prince_order() {
        assert_eq!(tui_web_fit_color(WebFitClass::Elite), TuiColor::Green);
        assert_eq!(tui_web_fit_color(WebFitClass::Solid), TuiColor::Yellow);
        assert_eq!(tui_web_fit_color(WebFitClass::Buried), TuiColor::Cyan);
        assert_eq!(tui_web_fit_color(WebFitClass::Stretch), TuiColor::Red);
    }

    #[test]
    fn l0_cli_visual_tui_state_styles_are_distinct() {
        assert_ne!(tui_warning_style(), tui_error_style());
        assert_ne!(tui_selected_style(), tui_meta_style());
        assert_eq!(tui_panel_border_style().fg, Some(TuiColor::DarkGray));
    }
}
