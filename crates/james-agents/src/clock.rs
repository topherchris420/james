//! Wall-clock helpers without a time-zone database: UTC RFC 3339 text and
//! civil dates from Unix time. Routines take an explicit UTC offset, because
//! the standard library cannot tell James the local zone or its daylight
//! saving rules.

use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch, UTC.
pub type Millis = i64;

pub fn now_ms() -> Millis {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Days since 1970-01-01 to (year, month, day). Howard Hinnant's algorithm.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// (year, month, day) to days since 1970-01-01.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub fn rfc3339(ms: Millis) -> String {
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000);
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    let sod = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        sod / 3600,
        (sod / 60) % 60,
        sod % 60
    )
}

/// Parses the subset [`rfc3339`] writes (milliseconds optional).
pub fn parse_rfc3339(text: &str) -> Option<Millis> {
    let b = text.as_bytes();
    if b.len() < 20 || *b.last()? != b'Z' || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| text.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let ms = match &text[19..text.len() - 1] {
        "" => 0,
        frac if frac.starts_with('.') && frac.len() <= 4 => {
            let digits = &frac[1..];
            let v: i64 = digits.parse().ok()?;
            v * 10_i64.pow(3 - digits.len() as u32)
        }
        _ => return None,
    };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let days = days_from_civil(y, mo as u32, d as u32);
    Some(((days * 86_400 + h * 3600 + mi * 60 + s) * 1000) + ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for ms in [
            0,
            1_763_294_400_000,
            951_782_400_000,
            -86_400_000,
            4_102_444_799_999,
        ] {
            assert_eq!(parse_rfc3339(&rfc3339(ms)), Some(ms), "{}", rfc3339(ms));
        }
        assert_eq!(rfc3339(1_763_294_400_000), "2025-11-16T12:00:00.000Z");
        assert_eq!(
            parse_rfc3339("2026-11-16T12:00:00Z"),
            Some(1_794_830_400_000)
        );
        assert_eq!(parse_rfc3339("2026-13-16T12:00:00Z"), None);
    }
}
