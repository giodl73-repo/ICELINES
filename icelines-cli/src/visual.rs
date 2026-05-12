use comfy_table::Color as TableColor;
use icelines_core::{FitClass, WebFitClass};
use ratatui::style::Color as TuiColor;

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
}
