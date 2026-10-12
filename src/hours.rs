//! The home day laid out as hour rows, and how an hour reads in a saved
//! city — gpui-free so the arithmetic can be tested on its own.

use jiff::tz::{Offset, TimeZone};
use jiff::{Span, Zoned};

use crate::clock;

/// The home day `now` falls in, as hour-long rows from its midnight: 24 on
/// most days, 23 or 25 on the days the clocks change, because rows add whole
/// hours in absolute time.
pub struct Day {
    start: Zoned,
    /// How long the day really is, from this midnight to the next.
    seconds: i64,
}

impl Day {
    pub fn of(now: &Zoned) -> Day {
        let start = now.start_of_day().expect("a real zone has a start of day");
        let end = start
            .tomorrow()
            .and_then(|tomorrow| tomorrow.start_of_day())
            .expect("a real zone has a next day");
        let seconds = end.timestamp().duration_since(start.timestamp()).as_secs();
        Day { start, seconds }
    }

    /// How many rows the day takes: its hours, rounded up so a day that
    /// changes by half an hour keeps its last partial hour.
    pub fn rows(&self) -> usize {
        (self.seconds as usize).div_ceil(3600)
    }

    /// `start + i` hours.
    pub fn row(&self, i: usize) -> Zoned {
        self.start
            .checked_add(Span::new().hours(i as i64))
            .expect("a day's hours past midnight never overflow a zone")
    }

    /// The row whose hour holds `at`, comparing instants so a DST day still
    /// answers honestly.
    pub fn index(&self, at: &Zoned) -> Option<usize> {
        let elapsed = at
            .timestamp()
            .duration_since(self.start.timestamp())
            .as_secs();
        (0..self.seconds)
            .contains(&elapsed)
            .then_some((elapsed / 3600) as usize)
    }

    /// How far through the rows `at` falls, `0.0..1.0`; `None` outside the
    /// day.
    pub fn fraction(&self, at: &Zoned) -> Option<f64> {
        let elapsed = at
            .timestamp()
            .duration_since(self.start.timestamp())
            .as_secs_f64();
        (0.0..self.seconds as f64)
            .contains(&elapsed)
            .then_some(elapsed / (self.rows() * 3600) as f64)
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
    fn a_plain_day_has_24_rows() {
        let now = at("2026-08-15T14:30:00Z", "America/New_York");
        assert_eq!(Day::of(&now).rows(), 24);
    }

    #[test]
    fn a_spring_forward_day_has_23_rows_ending_on_its_own_last_hour() {
        // New York springs forward on 2026-03-08.
        let now = at("2026-03-08T12:00:00Z", "America/New_York");
        let day = Day::of(&now);
        assert_eq!(day.rows(), 23);
        assert_eq!(day.row(2).hour(), 3);
        assert_eq!(day.row(22).datetime().to_string(), "2026-03-08T23:00:00");
        let last_minute = at("2026-03-09T03:59:00Z", "America/New_York");
        assert_eq!(day.index(&last_minute), Some(22));
    }

    #[test]
    fn a_fall_back_day_has_25_rows_repeating_the_doubled_hour() {
        // New York falls back on 2026-11-01.
        let now = at("2026-11-01T12:00:00Z", "America/New_York");
        let day = Day::of(&now);
        assert_eq!(day.rows(), 25);
        assert_eq!(day.row(1).hour(), 1);
        assert_eq!(day.row(2).hour(), 1);
        assert_eq!(day.row(24).datetime().to_string(), "2026-11-01T23:00:00");
        let late = at("2026-11-02T04:30:00Z", "America/New_York");
        assert_eq!(day.index(&late), Some(24));
        assert!(day.fraction(&late).is_some());
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
