//! The home day laid out as 24 hour rows, and how an hour reads in a saved
//! city — gpui-free so the arithmetic can be tested on its own.

use jiff::tz::{Offset, TimeZone};
use jiff::{Span, Zoned};

use crate::clock;

pub const ROWS: usize = 24;

/// Rows add whole hours, which jiff applies in absolute time, so the day is
/// always exactly this long.
const DAY: f64 = 24.0 * 3600.0;

/// The home day `now` falls in, as 24 hour-long rows from its midnight.
pub struct Day {
    start: Zoned,
}

impl Day {
    pub fn of(now: &Zoned) -> Day {
        let start = now.start_of_day().expect("a real zone has a start of day");
        Day { start }
    }

    /// `start + i` hours.
    pub fn row(&self, i: usize) -> Zoned {
        self.start
            .checked_add(Span::new().hours(i as i64))
            .expect("24 hours past midnight never overflows a zone")
    }

    /// The row whose hour holds `at`, comparing instants so a DST day still
    /// answers honestly.
    pub fn index(&self, at: &Zoned) -> Option<usize> {
        let elapsed = at
            .timestamp()
            .duration_since(self.start.timestamp())
            .as_secs();
        (0..DAY as i64)
            .contains(&elapsed)
            .then_some((elapsed / 3600) as usize)
    }

    /// `(at − start) / 24 h`, `0.0..1.0`; `None` outside `[start, start + 24h)`.
    pub fn fraction(&self, at: &Zoned) -> Option<f64> {
        let elapsed = at
            .timestamp()
            .duration_since(self.start.timestamp())
            .as_secs_f64();
        (0.0..DAY).contains(&elapsed).then_some(elapsed / DAY)
    }
}

/// An hour line's label, as a calendar's time gutter writes it: "9 AM" or
/// "09:00", with the minutes kept in 12-hour time only where a zone is off
/// the hour ("5:30 AM").
pub fn hour_label(hour: i8, minute: i8, format: clock::Format) -> String {
    match format {
        clock::Format::TwentyFour => format!("{hour:02}:{minute:02}"),
        clock::Format::Twelve => {
            let half = if hour < 12 { "AM" } else { "PM" };
            let hour = match hour % 12 {
                0 => 12,
                hour => hour,
            };
            if minute == 0 {
                format!("{hour} {half}")
            } else {
                format!("{hour}:{minute:02} {half}")
            }
        }
    }
}

/// "UTC", "+1", "−5", "+5:30" (U+2212 minus).
pub fn offset_label(offset: Offset) -> String {
    let seconds = offset.seconds();
    if seconds == 0 {
        return "UTC".to_string();
    }
    let sign = if seconds < 0 { '\u{2212}' } else { '+' };
    let seconds = seconds.abs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if minutes == 0 {
        format!("{sign}{hours}")
    } else {
        format!("{sign}{hours}:{minutes:02}")
    }
}

/// "America/New_York · EDT · daylight time"; "EDT · daylight time" without an
/// IANA name.
pub fn zone_detail(zone: &TimeZone, at: &Zoned) -> String {
    let info = zone.to_offset_info(at.timestamp());
    let kind = if info.dst().is_dst() {
        "daylight"
    } else {
        "standard"
    };
    let tail = format!("{} · {kind} time", info.abbreviation());
    match zone.iana_name() {
        Some(name) => format!("{name} · {tail}"),
        None => tail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An instant, as read in `zone`.
    fn at(instant: &str, zone: &str) -> Zoned {
        instant
            .parse::<jiff::Timestamp>()
            .expect("a valid timestamp")
            .to_zoned(TimeZone::get(zone).expect("a zone this machine knows"))
    }

    fn zone(name: &str) -> TimeZone {
        TimeZone::get(name).expect("a zone this machine knows")
    }

    #[test]
    fn rows_of_a_plain_day_are_an_hour_apart() {
        let now = at("2026-08-15T14:30:00Z", "UTC");
        let day = Day::of(&now);
        assert_eq!(day.row(0).datetime().to_string(), "2026-08-15T00:00:00");
        assert_eq!(day.row(1).datetime().to_string(), "2026-08-15T01:00:00");
        assert_eq!(day.row(23).datetime().to_string(), "2026-08-15T23:00:00");
    }

    #[test]
    fn index_covers_the_start_last_minute_and_just_outside() {
        let now = at("2026-08-15T00:00:00Z", "UTC");
        let day = Day::of(&now);
        assert_eq!(day.index(&day.row(0)), Some(0));
        let last_minute = at("2026-08-15T23:59:00Z", "UTC");
        assert_eq!(day.index(&last_minute), Some(23));
        let just_before = at("2026-08-14T23:59:59Z", "UTC");
        assert_eq!(day.index(&just_before), None);
        let just_after = at("2026-08-16T00:00:00Z", "UTC");
        assert_eq!(day.index(&just_after), None);
    }

    #[test]
    fn index_switches_rows_exactly_on_the_hour() {
        let now = at("2026-08-15T00:00:00Z", "UTC");
        let day = Day::of(&now);
        assert_eq!(day.index(&day.row(1)), Some(1));
        let just_before = day
            .row(1)
            .checked_sub(Span::new().seconds(1))
            .expect("a second before row 1");
        assert_eq!(day.index(&just_before), Some(0));
    }

    #[test]
    fn fraction_at_midday_is_one_half() {
        let now = at("2026-08-15T00:00:00Z", "UTC");
        let day = Day::of(&now);
        let midday = at("2026-08-15T12:00:00Z", "UTC");
        assert_eq!(day.fraction(&midday), Some(0.5));
    }

    #[test]
    fn a_dst_change_day_still_has_24_rows_an_hour_apart_by_instant() {
        // New York springs forward on 2026-03-08.
        let now = at("2026-03-08T12:00:00Z", "America/New_York");
        let day = Day::of(&now);
        for i in 0..23 {
            let span = day.row(i + 1).timestamp().as_second() - day.row(i).timestamp().as_second();
            assert_eq!(span, 3600, "row {i} to {}", i + 1);
        }
        assert_eq!(day.index(&day.row(23)), Some(23));
    }

    #[test]
    fn hour_label_both_formats() {
        for (hour, minute, expected) in [(0, 0, "00:00"), (9, 30, "09:30"), (23, 0, "23:00")] {
            assert_eq!(
                hour_label(hour, minute, clock::Format::TwentyFour),
                expected
            );
        }
        for (hour, minute, expected) in [
            (0, 0, "12 AM"),
            (9, 0, "9 AM"),
            (5, 30, "5:30 AM"),
            (12, 0, "12 PM"),
            (13, 45, "1:45 PM"),
            (23, 0, "11 PM"),
        ] {
            assert_eq!(hour_label(hour, minute, clock::Format::Twelve), expected);
        }
    }

    #[test]
    fn offset_label_formats() {
        assert_eq!(offset_label(Offset::from_seconds(0).unwrap()), "UTC");
        assert_eq!(offset_label(Offset::from_seconds(3600).unwrap()), "+1");
        assert_eq!(
            offset_label(Offset::from_seconds(-5 * 3600).unwrap()),
            "\u{2212}5"
        );
        assert_eq!(
            offset_label(Offset::from_seconds(5 * 3600 + 1800).unwrap()),
            "+5:30"
        );
        assert_eq!(
            offset_label(Offset::from_seconds(-(3 * 3600 + 1800)).unwrap()),
            "\u{2212}3:30"
        );
    }

    #[test]
    fn zone_detail_reads_the_abbreviation_and_daylight_status() {
        let july = at("2026-07-15T12:00:00Z", "America/New_York");
        assert_eq!(
            zone_detail(&zone("America/New_York"), &july),
            "America/New_York · EDT · daylight time"
        );
        let january = at("2026-01-15T12:00:00Z", "America/New_York");
        assert_eq!(
            zone_detail(&zone("America/New_York"), &january),
            "America/New_York · EST · standard time"
        );
    }
}
