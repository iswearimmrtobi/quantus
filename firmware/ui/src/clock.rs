//! The time of day: the clock (the system's, moved by [`set`]), the time zones with their summer
//! time, and the calendar to turn one into the other. The handheld keeps UTC; the local time is
//! worked out here for the zone set in SETUP, so the board needs no `TZ` and the simulator shows
//! the same as the board.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::settings::Zone;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;

/// Before this (2025-01-01 UTC) the clock has not been set: the board starts at 1970.
const SET_SINCE: i64 = 1_735_689_600_000;

/// Added to the system clock, by [`set`].
static OFFSET: AtomicI64 = AtomicI64::new(0);

/// Now, in milliseconds since 1970 (UTC), or `None` while the clock is not set.
pub fn now() -> Option<i64> {
    let system = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as i64;
    let now = system + OFFSET.load(Ordering::Relaxed);
    (now >= SET_SINCE).then_some(now)
}

/// Moves the clock so it says `utc_ms` now. A board that can set the system clock does that
/// instead (and network time then keeps it right).
pub fn set(utc_ms: i64) {
    let system = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as i64);
    OFFSET.store(utc_ms - system, Ordering::Relaxed);
}

/// When summer time applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    None,
    /// The EU's: from the last Sunday of March to the last Sunday of October, 01:00 UTC.
    Eu,
    /// The US's: from the second Sunday of March to the first Sunday of November, 02:00 local.
    Us,
    /// Australia's south-east: from the first Sunday of October to the first Sunday of April,
    /// 02:00 standard time.
    Au,
}

impl Zone {
    /// Minutes from UTC without summer time.
    pub fn standard_minutes(self) -> i32 {
        match self {
            Zone::Utc | Zone::London => 0,
            Zone::Berlin => 60,
            Zone::Helsinki => 120,
            Zone::Moscow => 180,
            Zone::Dubai => 240,
            Zone::India => 330,
            Zone::China => 480,
            Zone::Tokyo => 540,
            Zone::Sydney => 600,
            Zone::NewYork => -300,
            Zone::Chicago => -360,
            Zone::Denver => -420,
            Zone::LosAngeles => -480,
        }
    }

    fn rule(self) -> Rule {
        match self {
            Zone::London | Zone::Berlin | Zone::Helsinki => Rule::Eu,
            Zone::NewYork | Zone::Chicago | Zone::Denver | Zone::LosAngeles => Rule::Us,
            Zone::Sydney => Rule::Au,
            _ => Rule::None,
        }
    }

    /// Minutes from UTC at `utc_ms`, with summer time if it applies then.
    pub fn offset_minutes(self, utc_ms: i64) -> i32 {
        let standard = self.standard_minutes();
        let year = civil(utc_ms.div_euclid(DAY_MS)).0;
        // The changes as UTC instants, in milliseconds.
        let at = |month, nth: Nth, hour: i64, offset: i32| {
            sunday(year, month, nth) * DAY_MS + hour * 3_600_000 - i64::from(offset) * MINUTE_MS
        };
        let summer = match self.rule() {
            Rule::None => false,
            Rule::Eu => (at(3, Nth::Last, 1, 0)..at(10, Nth::Last, 1, 0)).contains(&utc_ms),
            Rule::Us => (at(3, Nth::Second, 2, standard)..at(11, Nth::First, 2, standard + 60)).contains(&utc_ms),
            Rule::Au => !(at(4, Nth::First, 2, standard)..at(10, Nth::First, 2, standard)).contains(&utc_ms),
        };
        standard + if summer { 60 } else { 0 }
    }
}

#[derive(Clone, Copy)]
enum Nth {
    First,
    Second,
    Last,
}

/// The day (since 1970) of the `nth` Sunday of `month` in `year`.
fn sunday(year: i32, month: u8, nth: Nth) -> i64 {
    let first = days(year, month, 1);
    let first_sunday = first + (7 - weekday(first)) % 7;
    match nth {
        Nth::First => first_sunday,
        Nth::Second => first_sunday + 7,
        Nth::Last => {
            let last = days(year, month, days_in_month(year, month));
            last - weekday(last) % 7
        }
    }
}

/// 0 for Sunday to 6 for Saturday; 1970-01-01 was a Thursday.
fn weekday(days: i64) -> i64 {
    (days + 4).rem_euclid(7)
}

/// A local date and time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
    /// 0 for Monday to 6 for Sunday.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    /// Minutes from UTC, with summer time if it applies.
    pub offset_minutes: i32,
}

/// `utc_ms` in `zone`.
pub fn local(utc_ms: i64, zone: Zone) -> Local {
    let offset_minutes = zone.offset_minutes(utc_ms);
    let wall = utc_ms + i64::from(offset_minutes) * MINUTE_MS;
    let day = wall.div_euclid(DAY_MS);
    let minutes = wall.rem_euclid(DAY_MS) / MINUTE_MS;
    let (year, month, date) = civil(day);
    Local {
        year,
        month,
        day: date,
        weekday: ((weekday(day) + 6) % 7) as u8,
        hour: (minutes / 60) as u8,
        minute: (minutes % 60) as u8,
        offset_minutes,
    }
}

/// The instant (UTC) the clocks in `zone` show the date and time given. In the hour summer time
/// skips, an hour later; in the hour it repeats, the first time.
pub fn to_utc(year: i32, month: u8, day: u8, hour: u8, minute: u8, zone: Zone) -> i64 {
    let wall = days(year, month, day) * DAY_MS + (i64::from(hour) * 60 + i64::from(minute)) * MINUTE_MS;
    let standard = zone.standard_minutes();
    let at = |offset: i32| wall - i64::from(offset) * MINUTE_MS;
    // Summer time first: in the repeated hour it is the earlier instant.
    [standard + 60, standard]
        .into_iter()
        .find(|&offset| zone.offset_minutes(at(offset)) == offset)
        .map_or(at(standard), at)
}

pub fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// 28 to 31; 0 for a month that is not 1 to 12.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days from 1970-01-01 to the date (proleptic Gregorian; Howard Hinnant's algorithm).
fn days(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let month = i64::from(month);
    let of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_era_days - 719_468
}

/// The date of day `days` since 1970-01-01.
fn civil(days: i64) -> (i32, u8, u8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = (of_year - (153 * shifted + 2) / 5 + 1) as u8;
    let month = if shifted < 10 { shifted + 3 } else { shifted - 9 } as u8;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year as i32, month, day)
}
