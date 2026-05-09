//! Phase Foster.5 — `Timeframe` enum.
//!
//! This module ships in F.2 because `FavoritesView` carries a
//! `Timeframe` field; F.5 expands it with namespaced filter atoms
//! and TUI keybind plumbing. Keeping the type here means the F.2
//! schema is stable and F.5 only adds, never renames.

use chrono::{Datelike, Duration, Months, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    Day,
    Week,
    Month,
    Season,
}

impl Timeframe {
    /// Resolve the timeframe to an inclusive date range anchored at
    /// `date`. Week starts Monday (ISO 8601, deterministic across
    /// locales). `Season` returns the NHL season window enclosing the
    /// date — Oct 1 of `season_year` through June 30 of
    /// `season_year + 1`. Offseason dates (July–September) snap to
    /// the **upcoming** season.
    pub fn range(self, date: NaiveDate) -> (NaiveDate, NaiveDate) {
        match self {
            Self::Day => (date, date),
            Self::Week => {
                let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
                (monday, monday + Duration::days(6))
            }
            Self::Month => {
                let first = date.with_day(1).expect("with_day(1) always valid");
                let last = first
                    .checked_add_months(Months::new(1))
                    .and_then(|d| d.pred_opt())
                    .expect("month rollover always valid");
                (first, last)
            }
            Self::Season => {
                let year = date.year();
                let (start_year, end_year) = if date.month() >= 7 {
                    // July onwards = upcoming/current season starting Oct of this year
                    (year, year + 1)
                } else {
                    // Jan-June = current season started last October
                    (year - 1, year)
                };
                let start = NaiveDate::from_ymd_opt(start_year, 10, 1).expect("Oct 1 always valid");
                let end = NaiveDate::from_ymd_opt(end_year, 6, 30).expect("Jun 30 always valid");
                (start, end)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, dy: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dy).unwrap()
    }

    #[test]
    fn l0_foster5_day_collapses_to_single_date() {
        let (s, e) = Timeframe::Day.range(d(2026, 1, 15));
        assert_eq!(s, e);
        assert_eq!(s, d(2026, 1, 15));
    }

    #[test]
    fn l0_foster5_week_anchors_to_monday() {
        // 2026-01-15 is a Thursday; Monday of that week is Jan 12.
        let (s, e) = Timeframe::Week.range(d(2026, 1, 15));
        assert_eq!(s, d(2026, 1, 12), "Mon of week of Thu 2026-01-15");
        assert_eq!(e, d(2026, 1, 18), "Sun (Mon+6)");
    }

    #[test]
    fn l0_foster5_week_on_a_monday_starts_today() {
        // 2026-01-12 is itself a Monday.
        let (s, e) = Timeframe::Week.range(d(2026, 1, 12));
        assert_eq!(s, d(2026, 1, 12));
        assert_eq!(e, d(2026, 1, 18));
    }

    #[test]
    fn l0_foster5_month_january_31_days() {
        let (s, e) = Timeframe::Month.range(d(2026, 1, 15));
        assert_eq!(s, d(2026, 1, 1));
        assert_eq!(e, d(2026, 1, 31));
    }

    #[test]
    fn l0_foster5_month_february_handles_leap() {
        // 2024 is a leap year.
        let (s, e) = Timeframe::Month.range(d(2024, 2, 14));
        assert_eq!(s, d(2024, 2, 1));
        assert_eq!(e, d(2024, 2, 29));
        // 2026 is not.
        let (s2, e2) = Timeframe::Month.range(d(2026, 2, 14));
        assert_eq!(s2, d(2026, 2, 1));
        assert_eq!(e2, d(2026, 2, 28));
    }

    #[test]
    fn l0_foster5_season_in_january_anchors_to_prior_october() {
        let (s, e) = Timeframe::Season.range(d(2026, 1, 15));
        // Jan 2026 belongs to the 2025-26 season → Oct 1 2025 to Jun 30 2026
        assert_eq!(s, d(2025, 10, 1));
        assert_eq!(e, d(2026, 6, 30));
    }

    #[test]
    fn l0_foster5_season_in_october_anchors_to_current_october() {
        let (s, e) = Timeframe::Season.range(d(2025, 11, 1));
        assert_eq!(s, d(2025, 10, 1));
        assert_eq!(e, d(2026, 6, 30));
    }

    #[test]
    fn l0_foster5_season_in_offseason_july_anchors_to_upcoming() {
        // July → upcoming season Oct 1 of same year
        let (s, e) = Timeframe::Season.range(d(2026, 7, 15));
        assert_eq!(s, d(2026, 10, 1));
        assert_eq!(e, d(2027, 6, 30));
    }

    #[test]
    fn l0_foster5_serde_round_trip_lowercase() {
        let s = serde_json::to_string(&Timeframe::Week).unwrap();
        assert_eq!(s, "\"week\"");
        let back: Timeframe = serde_json::from_str(&s).unwrap();
        assert_eq!(back, Timeframe::Week);
    }
}
