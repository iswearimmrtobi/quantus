//! The calendar, the time zones and their summer time.

use quantus_ui::clock::{self, Local, Zone};

/// 2026-03-29 is the last Sunday of March, 2026-10-25 the last of October.
fn utc(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> i64 {
    clock::to_utc(year, month, day, hour, minute, Zone::Utc)
}

#[test]
fn utc_is_the_calendar() {
    assert_eq!(utc(1970, 1, 1, 0, 0), 0);
    assert_eq!(utc(2026, 11, 2, 14, 30), 1_793_629_800_000);
    let local = clock::local(1_793_629_800_000, Zone::Utc);
    assert_eq!(local, Local { year: 2026, month: 11, day: 2, weekday: 0, hour: 14, minute: 30, offset_minutes: 0 });
}

#[test]
fn the_eu_changes_at_one_in_the_morning_utc() {
    assert_eq!(Zone::Berlin.offset_minutes(utc(2026, 3, 29, 0, 59)), 60);
    assert_eq!(Zone::Berlin.offset_minutes(utc(2026, 3, 29, 1, 0)), 120);
    assert_eq!(Zone::Berlin.offset_minutes(utc(2026, 10, 25, 0, 59)), 120);
    assert_eq!(Zone::Berlin.offset_minutes(utc(2026, 10, 25, 1, 0)), 60);
    assert_eq!(Zone::London.offset_minutes(utc(2026, 7, 1, 12, 0)), 60);
    assert_eq!(Zone::Helsinki.offset_minutes(utc(2026, 1, 1, 12, 0)), 120);
}

#[test]
fn the_us_changes_at_two_local() {
    // 2026: the second Sunday of March is the 8th, the first of November the 1st.
    assert_eq!(Zone::NewYork.offset_minutes(utc(2026, 3, 8, 6, 59)), -300);
    assert_eq!(Zone::NewYork.offset_minutes(utc(2026, 3, 8, 7, 0)), -240);
    assert_eq!(Zone::NewYork.offset_minutes(utc(2026, 11, 1, 5, 59)), -240);
    assert_eq!(Zone::NewYork.offset_minutes(utc(2026, 11, 1, 6, 0)), -300);
    assert_eq!(Zone::LosAngeles.offset_minutes(utc(2026, 7, 1, 12, 0)), -420);
}

#[test]
fn sydney_has_summer_time_around_new_year() {
    assert_eq!(Zone::Sydney.offset_minutes(utc(2026, 1, 15, 0, 0)), 660);
    assert_eq!(Zone::Sydney.offset_minutes(utc(2026, 7, 15, 0, 0)), 600);
    // 2026-10-04 02:00 standard time is 2026-10-03 16:00 UTC.
    assert_eq!(Zone::Sydney.offset_minutes(utc(2026, 10, 3, 15, 59)), 600);
    assert_eq!(Zone::Sydney.offset_minutes(utc(2026, 10, 3, 16, 0)), 660);
}

#[test]
fn zones_without_summer_time_and_half_hours() {
    let local = clock::local(utc(2026, 12, 31, 20, 0), Zone::India);
    assert_eq!((local.year, local.month, local.day, local.hour, local.minute), (2027, 1, 1, 1, 30));
    assert_eq!(Zone::Tokyo.offset_minutes(utc(2026, 7, 1, 0, 0)), 540);
}

#[test]
fn local_and_back_agree() {
    let start = utc(2025, 12, 30, 0, 0);
    for zone in Zone::ALL {
        for step in 0..24 * 400 {
            let at = start + step * 37 * 60_000;
            let local = clock::local(at, *zone);
            let back = clock::to_utc(local.year, local.month, local.day, local.hour, local.minute, *zone);
            // The repeated hour gives back its first instant.
            assert!(back == at - at % 60_000 || back + 3_600_000 == at - at % 60_000, "{zone:?} at {at}");
        }
    }
}

#[test]
fn the_skipped_hour_means_an_hour_later() {
    let at = clock::to_utc(2026, 3, 29, 2, 30, Zone::Berlin);
    assert_eq!(at, utc(2026, 3, 29, 1, 30));
    assert_eq!(clock::local(at, Zone::Berlin).hour, 3);
}

#[test]
fn the_calendar_knows_leap_years() {
    assert_eq!(clock::days_in_month(2024, 2), 29);
    assert_eq!(clock::days_in_month(2100, 2), 28);
    assert_eq!(clock::days_in_month(2000, 2), 29);
    assert_eq!(clock::days_in_month(2026, 13), 0);
    let local = clock::local(utc(2028, 2, 29, 23, 0), Zone::Berlin);
    assert_eq!((local.month, local.day, local.weekday), (3, 1, 2));
}

#[test]
fn set_moves_the_clock() {
    let wanted = utc(2030, 6, 1, 12, 0);
    clock::set(wanted);
    let now = clock::now().expect("set");
    assert!((now - wanted).abs() < 1000);
}
