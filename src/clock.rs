//! Small time helpers.

use std::time::{Duration, Instant};

const SECS_PER_DAY: i64 = 86_400;

/// The instant `d` ago, or now if the clock hasn't run that long yet (`Instant` can't go
/// before the machine's boot on some platforms).
pub fn ago(d: Duration) -> Instant {
    let now = Instant::now();
    now.checked_sub(d).unwrap_or(now)
}

/// The local wall-clock time of `unix` (seconds) as `14:22`.
pub fn local_hm(unix: u64) -> String {
    let (h, m, _) = time_of_day(unix, local_offset(unix));
    format!("{h:02}:{m:02}")
}

/// The local wall-clock time of `unix` (seconds) as `14:22:05`.
pub fn local_hms(unix: u64) -> String {
    let (h, m, s) = time_of_day(unix, local_offset(unix));
    format!("{h:02}:{m:02}:{s:02}")
}

/// Hours, minutes and seconds of the day at `unix`, `offset` seconds east of UTC.
fn time_of_day(unix: u64, offset: i64) -> (i64, i64, i64) {
    let secs = (unix as i64 + offset).rem_euclid(SECS_PER_DAY);
    (secs / 3600, secs / 60 % 60, secs % 60)
}

/// Seconds east of UTC the local clock was at `unix`.
#[cfg(unix)]
fn local_offset(unix: u64) -> i64 {
    let t = unix as libc::time_t;
    // SAFETY: localtime_r only writes the `tm` it is given; both live for the call.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i64
}

/// Seconds east of UTC the local clock is now.
// shortcut: today's offset, so a time on the far side of a daylight-saving change is an hour off.
#[cfg(windows)]
fn local_offset(_unix: u64) -> i64 {
    use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
    // From Win32_System_SystemServices, a feature this one constant isn't worth.
    const TIME_ZONE_ID_DAYLIGHT: u32 = 2;
    const TIME_ZONE_ID_INVALID: u32 = u32::MAX;
    // SAFETY: GetTimeZoneInformation only fills the struct it is given.
    let mut tz: TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
    let id = unsafe { GetTimeZoneInformation(&mut tz) };
    if id == TIME_ZONE_ID_INVALID {
        return 0;
    }
    // Bias is minutes to add to local time to get UTC.
    let bias = tz.Bias + if id == TIME_ZONE_ID_DAYLIGHT { tz.DaylightBias } else { tz.StandardBias };
    -i64::from(bias) * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_underflows() {
        let _ = super::ago(std::time::Duration::from_secs(u64::MAX / 4));
        assert!(super::ago(std::time::Duration::from_millis(1)) <= std::time::Instant::now());
    }

    /// 2026-10-22 14:22:05 UTC.
    const AT: u64 = 1_792_678_925;

    #[test]
    fn the_time_of_day_follows_the_offset_and_wraps_at_midnight() {
        assert_eq!(time_of_day(AT, 0), (14, 22, 5));
        assert_eq!(time_of_day(AT, 2 * 3600), (16, 22, 5));
        assert_eq!(time_of_day(AT, -5 * 3600 - 1800), (8, 52, 5));
        assert_eq!(time_of_day(AT, 10 * 3600), (0, 22, 5), "past midnight east");
        assert_eq!(time_of_day(AT - 15 * 3600, -3600), (22, 22, 5), "before midnight west");
        assert_eq!(time_of_day(0, -3600), (23, 0, 0), "before the epoch's day");
    }

    #[test]
    fn local_times_are_padded_clock_faces() {
        let (hm, hms) = (local_hm(AT), local_hms(AT));
        assert!(hms.starts_with(&hm), "{hm} / {hms}");
        let digits = |s: &str| s.split(':').all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()));
        assert!(hm.len() == 5 && hms.len() == 8 && digits(&hm) && digits(&hms), "{hm} / {hms}");
    }
}
