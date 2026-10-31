//! The dates of `pgfcalendar`, as Julian day numbers.

/// The Julian day number of a date of the Gregorian calendar.
pub fn julian(year: i64, month: i64, day: i64) -> i64 {
    let a = (14 - month) / 12;
    let y = year + 4800 - a;
    let m = month + 12 * a - 3;
    day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

/// The date (year, month, day) of a Julian day number.
pub fn date(j: i64) -> (i64, i64, i64) {
    let a = j + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - 146097 * b / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    (year, month, day)
}

/// The day of the week of a Julian day number, 0 for Monday.
pub fn weekday(j: i64) -> i64 {
    j.rem_euclid(7)
}

/// `2000-01-20`, `2000-02-last` (the month's last day), with an offset in days (`2000-01-01+-25`).
/// Without a year (`01-20`, in tests), the year is `year`.
pub fn parse_date(s: &str, year: Option<i64>) -> Option<i64> {
    let s = s.trim();
    let (d, offset) = match s.split_once('+') {
        Some((d, o)) => (d, o.trim().parse::<i64>().ok()?),
        None => (s, 0),
    };
    let parts: Vec<&str> = d.trim().split('-').map(str::trim).collect();
    let (y, m, day) = match parts.as_slice() {
        [y, m, d] => (y.parse::<i64>().ok()?, m.parse::<i64>().ok()?, *d),
        [m, d] => (year?, m.parse::<i64>().ok()?, *d),
        _ => return None,
    };
    let j = if day == "last" { julian(y + (m == 12) as i64, m % 12 + 1, 1) - 1 } else { julian(y, m, day.parse().ok()?) };
    Some(j + offset)
}

/// The Easter Sunday of a year (the Gregorian computus), as a Julian day number.
pub fn easter(y: i64) -> i64 {
    let g = y % 19 + 1;
    let c = y / 100 + 1;
    let x = 3 * c / 4 - 12;
    let z = (8 * c + 5) / 25 - 5;
    let d = 5 * y / 4 - x - 10;
    let mut e = (11 * g + 20 + z - x).rem_euclid(30);
    if (e == 25 && g > 11) || e == 24 {
        e += 1;
    }
    let mut n = 44 - e;
    if n < 21 {
        n += 30;
    }
    n += 7 - (d + n).rem_euclid(7);
    if n > 31 {
        julian(y, 4, n - 31)
    } else {
        julian(y, 3, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn julian_days_and_back() {
        assert_eq!(julian(2000, 1, 1), 2451545);
        assert_eq!(date(2451545), (2000, 1, 1));
        // 2000-01-01 was a Saturday
        assert_eq!(weekday(2451545), 5);
        assert_eq!(parse_date("2000-02-last", None), Some(julian(2000, 2, 29)));
        assert_eq!(parse_date("2000-12-last", None), Some(julian(2000, 12, 31)));
        assert_eq!(parse_date("2000-01-01+-1", None), Some(julian(1999, 12, 31)));
        assert_eq!(parse_date("01-20", Some(2007)), Some(julian(2007, 1, 20)));
        assert_eq!(date(easter(2024)), (2024, 3, 31));
    }
}
