//! Turning an instant into what a saved row shows: the local time somewhere
//! else, and a hint when it is not the same day there as it is here.

use std::time::Duration;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Unit, Zoned};

/// What a saved row shows for one city.
#[derive(Debug, PartialEq, Eq)]
pub struct Reading {
    /// The local time as "HH:MM", 24-hour.
    pub time: String,
    /// How the local date differs from home's, when it does.
    pub day: Option<String>,
}

/// Shown when the city's zone is missing from this machine's tzdb, so there is
/// no time to show but the city still deserves its row.
pub const UNKNOWN: &str = "–:–";

/// A small pad so a timer firing a hair early still lands inside the next
/// minute, instead of waking up again a few milliseconds later.
const PAD: Duration = Duration::from_millis(50);

/// The reading for `zone` at the instant `now`, whose own zone is home.
pub fn reading(now: &Zoned, zone: &TimeZone) -> Reading {
    let there = now.with_time_zone(zone.clone());
    Reading {
        time: format!("{:02}:{:02}", there.hour(), there.minute()),
        day: day(now.date(), there.date()),
    }
}

/// How to describe `there` relative to `here`, or `None` on the same date.
fn day(here: Date, there: Date) -> Option<String> {
    let days = there.since((Unit::Day, here)).ok()?.get_days();
    match days {
        0 => None,
        1 => Some("+1 day".to_string()),
        -1 => Some("−1 day".to_string()),
        days => Some(format!("{days:+} days")),
    }
}

/// The instant at which `zone`'s wall clock reads `text` — "HH:MM", today over
/// there — expressed in home's zone so day hints stay relative to home. `None`
/// when the text does not read as such a time.
pub fn pin(text: &str, zone: &TimeZone, now: &Zoned) -> Option<Zoned> {
    let (hour, minute) = text.split_once(':')?;
    let hour: i8 = hour.trim().parse().ok()?;
    let minute: i8 = minute.trim().parse().ok()?;
    if !(0..24).contains(&hour) || !(0..60).contains(&minute) {
        return None;
    }
    let date = now.with_time_zone(zone.clone()).date();
    // A time skipped or repeated by a DST change resolves to jiff's compatible
    // choice rather than failing.
    let there = date.at(hour, minute, 0, 0).to_zoned(zone.clone()).ok()?;
    Some(there.with_time_zone(now.time_zone().clone()))
}

/// How long to wait before the displayed minute changes.
pub fn until_next_minute(now: &Zoned) -> Duration {
    // A leap second reads as :60, which is still inside the minute.
    let second = now.second().clamp(0, 59) as u64;
    let nanosecond = now.subsec_nanosecond().max(0) as u32;
    // The nanoseconds carry into a whole second when there are none to drop.
    Duration::new(59 - second, 1_000_000_000 - nanosecond) + PAD
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An instant, read from the given zone as home.
    fn at(instant: &str, home: &str) -> Zoned {
        instant
            .parse::<jiff::Timestamp>()
            .expect("a valid timestamp")
            .to_zoned(zone(home))
    }

    fn zone(name: &str) -> TimeZone {
        TimeZone::get(name).expect("a zone this machine knows")
    }

    #[test]
    fn reads_the_time_in_another_zone() {
        let now = at("2026-08-15T12:34:56Z", "UTC");
        assert_eq!(reading(&now, &zone("UTC")).time, "12:34");
        // +09:00, year round.
        assert_eq!(reading(&now, &zone("Asia/Tokyo")).time, "21:34");
        // -07:00 in August.
        assert_eq!(reading(&now, &zone("America/Los_Angeles")).time, "05:34");
    }

    #[test]
    fn pads_the_hour_and_the_minute() {
        let now = at("2026-08-15T09:05:00Z", "UTC");
        assert_eq!(reading(&now, &zone("UTC")).time, "09:05");
        assert_eq!(reading(&now, &zone("Europe/London")).time, "10:05");
    }

    #[test]
    fn the_same_date_needs_no_hint() {
        let now = at("2026-08-15T12:00:00Z", "UTC");
        assert_eq!(reading(&now, &zone("Europe/Paris")).day, None);
    }

    #[test]
    fn a_zone_far_enough_east_is_a_day_ahead() {
        // 22:00 UTC is already the 16th at +14:00.
        let now = at("2026-08-15T22:00:00Z", "UTC");
        let there = reading(&now, &zone("Pacific/Kiritimati"));
        assert_eq!(there.time, "12:00");
        assert_eq!(there.day.as_deref(), Some("+1 day"));
    }

    #[test]
    fn a_zone_far_enough_west_is_a_day_behind() {
        // Just past midnight UTC is still the 14th in California.
        let now = at("2026-08-15T00:30:00Z", "UTC");
        let there = reading(&now, &zone("America/Los_Angeles"));
        assert_eq!(there.time, "17:30");
        assert_eq!(there.day.as_deref(), Some("−1 day"));
    }

    #[test]
    fn the_hint_is_relative_to_home_not_to_utc() {
        // Home in Tokyo: it is the 16th here, so London is the one behind and
        // Kiritimati, a day ahead of UTC, is level with us.
        let now = at("2026-08-15T22:00:00Z", "Asia/Tokyo");
        assert_eq!(now.date().to_string(), "2026-08-16");
        assert_eq!(
            reading(&now, &zone("Europe/London")).day.as_deref(),
            Some("−1 day")
        );
        assert_eq!(reading(&now, &zone("Pacific/Kiritimati")).day, None);
    }

    #[test]
    fn a_fixed_offset_zone_reads_the_same_way() {
        let now = at("2026-08-15T23:15:00Z", "UTC");
        let there = reading(&now, &TimeZone::fixed(jiff::tz::offset(5)));
        assert_eq!(there.time, "04:15");
        assert_eq!(there.day.as_deref(), Some("+1 day"));
    }

    #[test]
    fn an_unknown_zone_does_not_resolve() {
        assert!(TimeZone::get("Mars/Olympus_Mons").is_err());
    }

    #[test]
    fn pins_a_time_in_another_zone() {
        // 12:00Z is 21:00 in Tokyo, so "today there" is still the 15th; 09:30
        // that morning in Tokyo was 00:30Z.
        let now = at("2026-08-15T12:00:00Z", "UTC");
        let pinned = pin("09:30", &zone("Asia/Tokyo"), &now).expect("a valid pin");
        assert_eq!(reading(&pinned, &zone("Asia/Tokyo")).time, "09:30");
        assert_eq!(reading(&pinned, &zone("UTC")).time, "00:30");
    }

    #[test]
    fn a_pin_keeps_home_as_the_day_reference() {
        // Kiritimati is already on the 16th; pinning 09:00 there lands at
        // 19:00Z on the 15th, so from home it reads as a day ahead.
        let now = at("2026-08-15T12:00:00Z", "UTC");
        let pinned = pin("09:00", &zone("Pacific/Kiritimati"), &now).expect("a valid pin");
        let there = reading(&pinned, &zone("Pacific/Kiritimati"));
        assert_eq!(there.time, "09:00");
        assert_eq!(there.day.as_deref(), Some("+1 day"));
    }

    #[test]
    fn a_time_skipped_by_dst_still_pins() {
        // New York's clocks jump from 02:00 to 03:00 on 2026-03-08; 02:30
        // resolves to the other side of the gap instead of failing.
        let now = at("2026-03-08T12:00:00Z", "UTC");
        let pinned = pin("02:30", &zone("America/New_York"), &now).expect("a valid pin");
        assert_eq!(reading(&pinned, &zone("America/New_York")).time, "03:30");
    }

    #[test]
    fn text_that_is_not_a_time_does_not_pin() {
        let now = at("2026-08-15T12:00:00Z", "UTC");
        for text in ["", "noon", "12", "25:00", "12:60", "-1:30", "1:2:3"] {
            assert_eq!(pin(text, &zone("UTC"), &now), None, "{text:?}");
        }
    }

    #[test]
    fn the_wait_runs_to_the_top_of_the_next_minute() {
        let now = at("2026-08-15T12:00:00Z", "UTC");
        assert_eq!(until_next_minute(&now), Duration::from_millis(60_050));

        let now = at("2026-08-15T12:00:59Z", "UTC");
        assert_eq!(until_next_minute(&now), Duration::from_millis(1_050));

        let now = at("2026-08-15T12:00:30.250Z", "UTC");
        assert_eq!(until_next_minute(&now), Duration::from_millis(29_800));
    }
}
