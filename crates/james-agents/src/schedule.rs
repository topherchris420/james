//! Routines: a task that a schedule creates.
//!
//! Standard five-field cron (`minute hour day-of-month month day-of-week`)
//! with `*`, numbers, lists, ranges and steps, evaluated at a fixed UTC offset
//! the person sets (James has no time-zone database, so daylight-saving
//! changes need the offset updated by hand).
//!
//! A routine only creates a task for its assistant. It never approves
//! anything: a write proposed during a routine run waits for the person like
//! any other. A minute that came due while James was closed is recorded as
//! missed and is **not** run late, because a late run can be the wrong run.
//! (The idea of routines as data follows SuperBot's `cron.rs`, MIT; this is
//! an independent implementation.)

use crate::clock::{Millis, civil_from_days};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cron {
    text: String,
    fields: [Vec<bool>; 5],
    dom_any: bool,
    dow_any: bool,
}

const RANGES: [(u32, u32); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];

impl Cron {
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() != 5 {
            return Err("a schedule has five fields: minute hour day month weekday".into());
        }
        let mut fields: [Vec<bool>; 5] = Default::default();
        for (i, field) in parts.iter().enumerate() {
            let (lo, hi) = RANGES[i];
            let mut set = vec![false; hi as usize + 1];
            for part in field.split(',') {
                let bad = || format!("“{field}” is not a valid schedule field");
                let (range, step) = match part.split_once('/') {
                    Some((r, s)) => (r, s.parse::<u32>().map_err(|_| bad())?),
                    None => (part, 1),
                };
                let (start, end) = if range == "*" {
                    (lo, hi)
                } else if let Some((a, b)) = range.split_once('-') {
                    (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?)
                } else {
                    let v: u32 = range.parse().map_err(|_| bad())?;
                    (v, if part.contains('/') { hi } else { v })
                };
                if start < lo || end > hi || start > end || step == 0 {
                    return Err(bad());
                }
                for v in (start..=end).step_by(step as usize) {
                    set[if i == 4 && v == 7 { 0 } else { v as usize }] = true;
                }
            }
            fields[i] = set;
        }
        Ok(Self {
            text: parts.join(" "),
            fields,
            dom_any: parts[2] == "*",
            dow_any: parts[4] == "*",
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the minute containing `at_ms` (shifted by `offset_minutes`) matches.
    pub fn matches(&self, at_ms: Millis, offset_minutes: i32) -> bool {
        let local_min = at_ms.div_euclid(60_000) + offset_minutes as i64;
        let days = local_min.div_euclid(1440);
        let mod_min = local_min.rem_euclid(1440);
        let (_, month, day) = civil_from_days(days);
        let weekday = (days + 4).rem_euclid(7) as usize; // 1970-01-01 was a Thursday
        let [min, hour, dom, mon, dow] = &self.fields;
        let (d, w) = (dom[day as usize], dow[weekday]);
        // Classic cron: when both day fields are restricted, either may match.
        let day_ok = if self.dom_any || self.dow_any {
            d && w
        } else {
            d || w
        };
        min[(mod_min % 60) as usize]
            && hour[(mod_min / 60) as usize]
            && mon[month as usize]
            && day_ok
    }

    /// Matching minutes in `(after, until]`, at most `cap` of them, as the
    /// first millisecond of each minute.
    pub fn due_between(
        &self,
        after: Millis,
        until: Millis,
        offset_minutes: i32,
        cap: usize,
    ) -> Vec<Millis> {
        let mut out = Vec::new();
        let mut minute = after.div_euclid(60_000) + 1;
        let last = until.div_euclid(60_000);
        // Bound the scan to about a year of minutes.
        let stop = last.min(minute + 527_040);
        while minute <= stop && out.len() < cap {
            if self.matches(minute * 60_000, offset_minutes) {
                out.push(minute * 60_000);
            }
            minute += 1;
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    pub id: String,
    pub name: String,
    pub cron: Cron,
    pub utc_offset_minutes: i32,
    pub assignee: String,
    pub objective: String,
    pub enabled: bool,
    pub created_at: Millis,
    pub last_fired_minute: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::parse_rfc3339;

    #[test]
    fn matches_standard_fields() {
        let monday_0730 = parse_rfc3339("2026-11-16T07:30:00Z").unwrap();
        assert!(Cron::parse("30 7 * * *").unwrap().matches(monday_0730, 0));
        assert!(Cron::parse("*/15 * * * *").unwrap().matches(monday_0730, 0));
        assert!(Cron::parse("30 7 * * 1-5").unwrap().matches(monday_0730, 0));
        assert!(!Cron::parse("30 7 * * 0,6").unwrap().matches(monday_0730, 0));
        assert!(Cron::parse("30 7 1 * 1").unwrap().matches(monday_0730, 0));
        assert!(!Cron::parse("0 8 * * *").unwrap().matches(monday_0730, 0));
        // 07:30 UTC is 08:30 at UTC+1.
        assert!(Cron::parse("30 8 * * *").unwrap().matches(monday_0730, 60));
        assert!(Cron::parse("61 * * * *").is_err());
        assert!(Cron::parse("* * *").is_err());
    }

    #[test]
    fn lists_due_minutes_in_a_window() {
        let cron = Cron::parse("0 * * * *").unwrap();
        let start = parse_rfc3339("2026-11-16T07:30:00Z").unwrap();
        let end = parse_rfc3339("2026-11-16T10:00:00Z").unwrap();
        let due = cron.due_between(start, end, 0, 10);
        assert_eq!(due.len(), 3); // 08:00, 09:00, 10:00
    }
}
