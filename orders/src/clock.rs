//! Dates and times on the receipt, in UK time: GMT in winter, and BST (an
//! hour ahead) from 01:00 GMT on the last Sunday in March to 01:00 GMT on the
//! last Sunday in October. Worked out here, so the service needs no
//! time-zone files.

use std::time::{SystemTime, UNIX_EPOCH};

/// A moment on a UK clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UkTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Seconds since the start of 1970, GMT.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

impl UkTime {
    /// The UK clock at `seconds` since the start of 1970, GMT.
    pub fn at(seconds: i64) -> UkTime {
        let year = civil(seconds.div_euclid(86_400)).0;
        let summer_from = last_sunday(year, 3) * 86_400 + 3_600;
        let summer_to = last_sunday(year, 10) * 86_400 + 3_600;
        let local = if (summer_from..summer_to).contains(&seconds) {
            seconds + 3_600
        } else {
            seconds
        };
        let (year, month, day) = civil(local.div_euclid(86_400));
        let in_day = local.rem_euclid(86_400);
        UkTime {
            year,
            month,
            day,
            hour: (in_day / 3_600) as u32,
            minute: (in_day % 3_600 / 60) as u32,
        }
    }

    /// "23 Nov 2026, 12:41 UK time"
    pub fn written(&self) -> String {
        format!(
            "{} {} {}, {:02}:{:02} UK time",
            self.day,
            MONTHS[self.month as usize - 1],
            self.year,
            self.hour,
            self.minute
        )
    }
}

/// The date of a day counted from 1 January 1970 (Howard Hinnant's method).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The day number (from 1 January 1970) of a date.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The day number of the last Sunday of a month with 31 days (March or
/// October). 1 January 1970 was a Thursday.
fn last_sunday(year: i64, month: u32) -> i64 {
    let last = days_from_civil(year, month, 31);
    let weekday = (last + 4).rem_euclid(7); // 0 is Sunday
    last - weekday
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seconds since 1970 for a GMT date and time.
    fn gmt(year: i64, month: u32, day: u32, hour: i64, minute: i64) -> i64 {
        days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60
    }

    #[test]
    fn dates_round_trip() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2026, 11, 23), 20_780);
        for days in [-1, 0, 59, 365, 10_957, 11_016, 20_780, 50_000] {
            let (y, m, d) = civil(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil(days_from_civil(2028, 2, 29)), (2028, 2, 29));
    }

    #[test]
    fn summer_time_starts_and_ends_at_one_in_the_morning_gmt() {
        // 2026: from 29 March to 25 October.
        assert_eq!(last_sunday(2026, 3), days_from_civil(2026, 3, 29));
        assert_eq!(last_sunday(2026, 10), days_from_civil(2026, 10, 25));
        let before = UkTime::at(gmt(2026, 3, 29, 0, 59));
        assert_eq!((before.hour, before.minute), (0, 59));
        let after = UkTime::at(gmt(2026, 3, 29, 1, 0));
        assert_eq!((after.hour, after.minute), (2, 0));
        let summer = UkTime::at(gmt(2026, 10, 25, 0, 59));
        assert_eq!((summer.day, summer.hour), (25, 1));
        let winter = UkTime::at(gmt(2026, 10, 25, 1, 0));
        assert_eq!((winter.day, winter.hour), (25, 1));
        // A summer evening goes past midnight on the UK clock.
        let late = UkTime::at(gmt(2027, 7, 31, 23, 30));
        assert_eq!((late.month, late.day, late.hour), (8, 1, 0));
    }

    #[test]
    fn the_receipt_writes_it_out() {
        assert_eq!(
            UkTime::at(gmt(2026, 11, 23, 12, 41)).written(),
            "23 Nov 2026, 12:41 UK time"
        );
        assert_eq!(
            UkTime::at(gmt(2027, 6, 1, 8, 5)).written(),
            "1 Jun 2027, 09:05 UK time"
        );
    }
}
