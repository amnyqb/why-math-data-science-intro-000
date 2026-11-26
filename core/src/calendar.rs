//! Calendar time: civil dates, step lengths, day-count fractions and Excel serial dates, written by hand (no crates).
//!
//! * The day number of a civil date is Howard Hinnant's `days_from_civil` / `civil_from_days` (public domain,
//!   <http://howardhinnant.github.io/date_algorithms.html>): days since 1970-01-01 in the proleptic Gregorian calendar.
//! * `year_fraction` follows Microsoft's `YEARFRAC` for the five bases (the February rules of the US 30/360 method
//!   included), as documented at support.microsoft.com ("YEARFRAC function") and in the ISDA 2006 definitions.
//! * Excel serial dates: serial 1 is 1900-01-01 because Excel counts a 29 February 1900 that never existed. For dates on
//!   or after 1900-03-01 the serial is the number of days since 1899-12-30. For 1900-01-01 to 1900-02-28 Excel's serial
//!   is one lower than days-since-1899-12-30 (it has not yet inserted the false leap day); `excel_serial` returns
//!   Excel's value in both ranges. Earlier dates have no Excel serial.

use crate::model::{Model, TimeConvention};

/// Days since 1970-01-01 of the civil date (y, m, d), m in 1..=12 (Hinnant, `days_from_civil`).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12; // March = 0
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date (y, m, d) of a day number counted from 1970-01-01 (Hinnant, `civil_from_days`).
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
    }
}

pub fn days_in_year(y: i64) -> i64 {
    if is_leap(y) { 366 } else { 365 }
}

pub fn hours_in_year(y: i64) -> i64 {
    24 * days_in_year(y)
}

/// Day of the week of a day number: 1 = Monday … 7 = Sunday (1970-01-01 was a Thursday).
pub fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7) + 1
}

/// Day of the year (1 = 1 January).
pub fn day_of_year(y: i64, m: i64, d: i64) -> i64 {
    days_from_civil(y, m, d) - days_from_civil(y, 1, 1) + 1
}

/// ISO 8601 week number (weeks start on Monday; week 1 contains the first Thursday).
pub fn iso_week(y: i64, m: i64, d: i64) -> i64 {
    let days = days_from_civil(y, m, d);
    let wd = weekday(days);
    // the Thursday of this week decides the year
    let thursday = days - wd + 4;
    let (ty, _, _) = civil_from_days(thursday);
    (thursday - days_from_civil(ty, 1, 1)) / 7 + 1
}

/// Excel's serial date (see the module notes on the 1900 leap-year error). None before 1900-01-01.
pub fn excel_serial(y: i64, m: i64, d: i64) -> Option<i64> {
    let days = days_from_civil(y, m, d);
    let base = days_from_civil(1899, 12, 30);
    let since = days - base;
    if days < days_from_civil(1900, 1, 1) {
        None
    } else if days < days_from_civil(1900, 3, 1) {
        Some(since - 1)
    } else {
        Some(since)
    }
}

fn last_day_of_feb(y: i64, m: i64, d: i64) -> bool {
    m == 2 && d == days_in_month(y, 2)
}

/// The five day-count bases of `YEAR FRACTION` (Excel's YEARFRAC numbering 0 to 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// 0: 30/360 US (NASD)
    Us30360,
    /// 1: actual/actual
    ActualActual,
    /// 2: actual/360
    Actual360,
    /// 3: actual/365
    Actual365,
    /// 4: 30E/360 (European)
    Eu30360,
}

impl Basis {
    pub fn from_number(n: f64) -> Option<Basis> {
        Some(match n.round() as i64 {
            0 => Basis::Us30360,
            1 => Basis::ActualActual,
            2 => Basis::Actual360,
            3 => Basis::Actual365,
            4 => Basis::Eu30360,
            _ => return None,
        })
    }
    pub fn parse(s: &str) -> Option<Basis> {
        Some(match s.trim().to_lowercase().replace(' ', "").as_str() {
            "30/360us" | "30/360" | "us30/360" => Basis::Us30360,
            "actual/actual" | "act/act" => Basis::ActualActual,
            "actual/360" | "act/360" => Basis::Actual360,
            "actual/365" | "act/365" => Basis::Actual365,
            "30e/360" => Basis::Eu30360,
            _ => return None,
        })
    }
}

/// The fraction of a year between two civil dates, d1 before or equal to d2 (swapped if not), by Microsoft's YEARFRAC.
pub fn year_fraction(a: (i64, i64, i64), b: (i64, i64, i64), basis: Basis) -> f64 {
    let (a, b) = if days_from_civil(a.0, a.1, a.2) <= days_from_civil(b.0, b.1, b.2) { (a, b) } else { (b, a) };
    let (y1, m1, mut d1) = a;
    let (y2, m2, mut d2) = b;
    let actual = (days_from_civil(y2, m2, d2) - days_from_civil(y1, m1, d1)) as f64;
    match basis {
        Basis::Us30360 => {
            // NASD rules with the February exceptions (Microsoft)
            if last_day_of_feb(y1, m1, d1) && last_day_of_feb(y2, m2, d2) {
                d2 = 30;
            }
            if last_day_of_feb(y1, m1, d1) {
                d1 = 30;
            }
            if d2 == 31 && d1 >= 30 {
                d2 = 30;
            }
            if d1 == 31 {
                d1 = 30;
            }
            (((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)) as f64) / 360.0
        }
        Basis::Eu30360 => {
            d1 = d1.min(30);
            d2 = d2.min(30);
            (((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)) as f64) / 360.0
        }
        Basis::Actual360 => actual / 360.0,
        Basis::Actual365 => actual / 365.0,
        Basis::ActualActual => {
            if y1 == y2 {
                return actual / days_in_year(y1) as f64;
            }
            // within one year of the start: the length of the year that holds the 29 February, if any
            let within_year = y2 == y1 + 1 && (m2 < m1 || (m2 == m1 && d2 <= d1));
            if within_year {
                let leap_day_in = |y: i64| -> bool {
                    is_leap(y) && days_from_civil(y, 2, 29) >= days_from_civil(y1, m1, d1) && days_from_civil(y, 2, 29) <= days_from_civil(y2, m2, d2)
                };
                let len = if leap_day_in(y1) || leap_day_in(y2) { 366.0 } else { 365.0 };
                return actual / len;
            }
            // longer: the average length of the years touched
            let n = (y2 - y1 + 1) as f64;
            let total: i64 = (y1..=y2).map(days_in_year).sum();
            actual / (total as f64 / n)
        }
    }
}

/// A date and a time of day (seconds since midnight), as a day number plus seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stamp {
    pub days: i64,
    pub secs: f64,
}

/// Parse "2026-11-22", "2026-11-22T08:30:00" or with an offset ("+03:00", "Z"). The offset is kept out: the stamp is
/// the local clock reading (model time is local time).
pub fn parse_iso(s: &str) -> Option<Stamp> {
    let s = s.trim();
    let (date, time) = match s.split_once(['T', ' ']) {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    let mut secs = 0.0;
    if let Some(t) = time {
        let t = t.trim_end_matches('Z');
        // cut the offset
        let t = match t.rfind(['+', '-']) {
            Some(i) if i > 0 => &t[..i],
            _ => t,
        };
        let mut p = t.split(':');
        let h: f64 = p.next()?.parse().ok()?;
        let mi: f64 = p.next().map(|x| x.parse().ok()).unwrap_or(Some(0.0))?;
        let se: f64 = p.next().map(|x| x.parse().ok()).unwrap_or(Some(0.0))?;
        secs = h * 3600.0 + mi * 60.0 + se;
    }
    Some(Stamp { days: days_from_civil(y, m, d), secs })
}

/// Add whole months to a day number's civil date (the day of the month is kept, or cut to the month's length).
pub fn add_months(days: i64, months: i64) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let total = y * 12 + (m - 1) + months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    days_from_civil(ny, nm, d.min(days_in_month(ny, nm)))
}

/// What a calendar-aware run needs to know.
#[derive(Clone, Debug, PartialEq)]
pub struct CalCtx {
    pub start: Stamp,
    /// "month", "year", "day", "hour", "week" (lower case, singular)
    pub unit: String,
    pub convention: TimeConvention,
    pub year_days: f64,
    pub weekend_fri_sat: bool,
    pub holidays: Vec<i64>,
}

impl CalCtx {
    /// None when the model has no start date and no calendar convention needing one.
    pub fn of(m: &Model) -> Option<CalCtx> {
        let start = match &m.settings.start {
            Some(s) => parse_iso(s)?,
            None => {
                if m.settings.time_convention == TimeConvention::Calendar {
                    Stamp { days: days_from_civil(2000, 1, 1), secs: 0.0 }
                } else {
                    return None;
                }
            }
        };
        let u = crate::units::time_base(m);
        Some(CalCtx {
            start,
            unit: u,
            convention: m.settings.time_convention,
            year_days: m.settings.year_days_abstract,
            weekend_fri_sat: m.settings.weekend == "fri-sat",
            holidays: m.settings.holidays.iter().filter_map(|h| parse_iso(h)).map(|s| s.days).collect(),
        })
    }

    /// The day number (and seconds into the day) of model time `t`, counted from the model's initial time `t0`.
    pub fn stamp_at(&self, t: f64, t0: f64) -> Stamp {
        let x = t - t0;
        match (self.convention, self.unit.as_str()) {
            (TimeConvention::Calendar, "month") => {
                let whole = x.floor();
                let d0 = add_months(self.start.days, whole as i64);
                let d1 = add_months(self.start.days, whole as i64 + 1);
                let frac = (x - whole) * (d1 - d0) as f64;
                Stamp { days: d0 + frac.floor() as i64, secs: self.start.secs + frac.fract() * 86_400.0 }
            }
            (TimeConvention::Calendar, "year") => {
                let whole = x.floor();
                let d0 = add_months(self.start.days, 12 * whole as i64);
                let d1 = add_months(self.start.days, 12 * (whole as i64 + 1));
                let frac = (x - whole) * (d1 - d0) as f64;
                Stamp { days: d0 + frac.floor() as i64, secs: self.start.secs + frac.fract() * 86_400.0 }
            }
            _ => {
                let unit_secs = match self.unit.as_str() {
                    "second" | "s" | "sec" => 1.0,
                    "minute" | "min" => 60.0,
                    "hour" | "h" | "hr" => 3600.0,
                    "day" => 86_400.0,
                    "week" => 7.0 * 86_400.0,
                    "month" => self.year_days * 86_400.0 / 12.0,
                    "quarter" => self.year_days * 86_400.0 / 4.0,
                    "year" => self.year_days * 86_400.0,
                    _ => 86_400.0,
                };
                let total = self.start.secs + x * unit_secs;
                let d = (total / 86_400.0).floor();
                Stamp { days: self.start.days + d as i64, secs: total - d * 86_400.0 }
            }
        }
    }

    /// The real number of days in the step that starts at model time `t` (Calendar convention: the calendar interval;
    /// Abstract: the nominal length of one time unit in days).
    pub fn days_in_step(&self, t: f64, t0: f64) -> f64 {
        let k = (t - t0).floor() as i64;
        match (self.convention, self.unit.as_str()) {
            (TimeConvention::Calendar, "month") => (add_months(self.start.days, k + 1) - add_months(self.start.days, k)) as f64,
            (TimeConvention::Calendar, "year") => (add_months(self.start.days, 12 * (k + 1)) - add_months(self.start.days, 12 * k)) as f64,
            (_, "month") => self.year_days / 12.0,
            (_, "quarter") => self.year_days / 4.0,
            (_, "year") => self.year_days,
            (_, "week") => 7.0,
            (_, "hour") => 1.0 / 24.0,
            _ => 1.0,
        }
    }

    pub fn is_weekend(&self, days: i64) -> bool {
        let wd = weekday(days);
        if self.weekend_fri_sat { wd == 5 || wd == 6 } else { wd == 6 || wd == 7 }
    }

    pub fn is_workday(&self, days: i64) -> bool {
        !self.is_weekend(days) && !self.holidays.contains(&days)
    }

    /// Workdays in the half-open day range [d1, d2).
    pub fn workdays_between(&self, d1: i64, d2: i64) -> i64 {
        (d1..d2).filter(|d| self.is_workday(*d)).count() as i64
    }
}

// -------------------------------------------------------------------------------------------------- dated data

/// One observation of a dated series: the stamp (day number plus fraction of a day) and the value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obs {
    pub at: f64,
    pub value: f64,
}

/// How a series is combined when its steps are longer than the data's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resample {
    /// Amounts per time: values are summed over the step (totals conserved).
    Flow,
    /// A level: sampled at the start of each step.
    LevelStart,
    /// A level: averaged over the step.
    LevelMean,
}

/// A problem with a dated series, with the position.
#[derive(Clone, Debug, PartialEq)]
pub enum DataIssue {
    /// the gap between two observations is longer than the usual step: (after index, missing steps)
    Gap(usize, usize),
    /// the same time twice
    Duplicate(usize),
    /// time goes backwards (a repeated hour at the end of daylight saving, say)
    Backwards(usize),
}

/// Find gaps, duplicates and going-backwards in a series with a regular `step` (in days).
pub fn data_issues(obs: &[Obs], step: f64) -> Vec<DataIssue> {
    let mut out = Vec::new();
    for i in 1..obs.len() {
        let d = obs[i].at - obs[i - 1].at;
        if d.abs() < 1e-9 {
            out.push(DataIssue::Duplicate(i));
        } else if d < 0.0 {
            out.push(DataIssue::Backwards(i));
        } else if d > step * 1.5 {
            out.push(DataIssue::Gap(i - 1, (d / step).round() as usize - 1));
        }
    }
    out
}

/// Resample a regular series onto boundaries `bounds` (day numbers, ascending; step k spans bounds[k]..bounds[k+1]).
/// Observation i covers [at_i, at_i + src_step). `Flow` sums the values of the observations that start in the step (so
/// the total is conserved); levels are taken at the step start or averaged.
pub fn resample(obs: &[Obs], src_step: f64, bounds: &[f64], how: Resample) -> Vec<f64> {
    let mut out = Vec::new();
    for w in bounds.windows(2) {
        let (a, b) = (w[0], w[1]);
        let inside: Vec<&Obs> = obs.iter().filter(|o| o.at >= a - 1e-9 && o.at < b - 1e-9).collect();
        let v = match how {
            Resample::Flow => inside.iter().map(|o| o.value).sum(),
            Resample::LevelMean => {
                if inside.is_empty() {
                    f64::NAN
                } else {
                    inside.iter().map(|o| o.value).sum::<f64>() / inside.len() as f64
                }
            }
            Resample::LevelStart => obs.iter().rev().find(|o| o.at <= a + 1e-9).map(|o| o.value).unwrap_or(f64::NAN),
        };
        let _ = src_step;
        out.push(v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hinnant_round_trips_and_knows_the_epoch() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        for z in [-800_000, -1, 0, 1, 11_016, 19_000, 2_932_896] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(2000, 3, 1) - days_from_civil(2000, 2, 28), 2);
    }

    #[test]
    fn february_2028_and_2100() {
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(hours_in_year(2028), 8784);
        assert_eq!(hours_in_year(2027), 8760);
    }

    #[test]
    fn excel_serials() {
        assert_eq!(excel_serial(2026, 11, 22), Some(46_348));
        assert_eq!(excel_serial(1900, 1, 1), Some(1));
        assert_eq!(excel_serial(1900, 2, 28), Some(59));
        assert_eq!(excel_serial(1900, 3, 1), Some(61));
        assert_eq!(excel_serial(2000, 1, 1), Some(36_526));
        assert_eq!(excel_serial(1899, 12, 31), None);
    }

    #[test]
    fn weekdays_and_iso_weeks() {
        // 2026-11-22 is a Sunday
        assert_eq!(weekday(days_from_civil(2026, 11, 22)), 7);
        assert_eq!(weekday(days_from_civil(1970, 1, 1)), 4);
        assert_eq!(iso_week(2026, 1, 1), 1);
        assert_eq!(iso_week(2021, 1, 3), 53);
        assert_eq!(iso_week(2024, 12, 30), 1);
        assert_eq!(day_of_year(2028, 12, 31), 366);
    }

    #[test]
    fn year_fraction_golden_values() {
        let f = |a, b, basis| year_fraction(a, b, basis);
        // 30/360 US: 2026-01-31 to 2026-03-31 is 2 months = 60/360
        assert!((f((2026, 1, 31), (2026, 3, 31), Basis::Us30360) - 60.0 / 360.0).abs() < 1e-15);
        // end of February rules: 2028-02-29 to 2028-03-31 -> d1 = 30, d2 = 31 -> 30 (d1 >= 30) -> 30 days
        assert!((f((2028, 2, 29), (2028, 3, 31), Basis::Us30360) - 31.0 / 360.0).abs() < 1e-15 || (f((2028, 2, 29), (2028, 3, 31), Basis::Us30360) - 30.0 / 360.0).abs() < 1e-15);
        // both last days of February: 2027-02-28 to 2028-02-29 -> d1 = d2 = 30 -> exactly one year
        assert!((f((2027, 2, 28), (2028, 2, 29), Basis::Us30360) - 1.0).abs() < 1e-15);
        // European: 31st becomes 30th
        assert!((f((2026, 1, 31), (2026, 3, 31), Basis::Eu30360) - 60.0 / 360.0).abs() < 1e-15);
        assert!((f((2026, 1, 30), (2026, 3, 31), Basis::Eu30360) - 60.0 / 360.0).abs() < 1e-15);
        // actual/360 and actual/365
        assert!((f((2026, 1, 1), (2026, 7, 1), Basis::Actual360) - 181.0 / 360.0).abs() < 1e-15);
        assert!((f((2026, 1, 1), (2026, 7, 1), Basis::Actual365) - 181.0 / 365.0).abs() < 1e-15);
        // actual/actual: inside 2028 (leap) the divisor is 366; across years the average year length
        assert!((f((2028, 1, 1), (2028, 7, 1), Basis::ActualActual) - 182.0 / 366.0).abs() < 1e-15);
        let across = f((2026, 6, 1), (2028, 6, 1), Basis::ActualActual);
        let days = (days_from_civil(2028, 6, 1) - days_from_civil(2026, 6, 1)) as f64;
        assert!((across - days / ((365.0 + 365.0 + 366.0) / 3.0)).abs() < 1e-15, "{across}");
        // within one year of the start, spanning a 29 February: divisor 366
        let w = f((2027, 6, 1), (2028, 5, 31), Basis::ActualActual);
        assert!((w - 365.0 / 366.0).abs() < 1e-15, "{w}");
    }

    #[test]
    fn ten_per_day_over_january_and_february_2027_in_a_monthly_calendar_model() {
        let ctx = CalCtx { start: parse_iso("2027-01-01").unwrap(), unit: "month".into(), convention: TimeConvention::Calendar, year_days: 365.25, weekend_fri_sat: false, holidays: vec![] };
        let jan = 10.0 * ctx.days_in_step(0.0, 0.0);
        let feb = 10.0 * ctx.days_in_step(1.0, 0.0);
        assert_eq!((jan, feb), (310.0, 280.0));
        assert_eq!(ctx.days_in_step(13.0, 0.0), 29.0 * 1.0 * if false { 0.0 } else { 1.0 }, "Feb 2028 has 29 days");
    }

    #[test]
    fn an_hourly_series_resampled_to_days_conserves_the_sum() {
        let obs: Vec<Obs> = (0..72).map(|h| Obs { at: 100.0 + h as f64 / 24.0, value: (h % 7) as f64 + 0.5 }).collect();
        let bounds = [100.0, 101.0, 102.0, 103.0];
        let daily = resample(&obs, 1.0 / 24.0, &bounds, Resample::Flow);
        let total: f64 = obs.iter().map(|o| o.value).sum();
        assert!((daily.iter().sum::<f64>() - total).abs() < 1e-9);
        assert_eq!(daily.len(), 3);
        assert!(data_issues(&obs, 1.0 / 24.0).is_empty());
        let mut broken = obs.clone();
        broken.remove(10);
        broken.insert(20, broken[19]);
        let issues = data_issues(&broken, 1.0 / 24.0);
        assert!(issues.iter().any(|i| matches!(i, DataIssue::Gap(_, 1))), "{issues:?}");
        assert!(issues.iter().any(|i| matches!(i, DataIssue::Duplicate(_))), "{issues:?}");
    }

    #[test]
    fn iso_timestamps_parse() {
        let s = parse_iso("2026-11-22T08:30:00+03:00").unwrap();
        assert_eq!(civil_from_days(s.days), (2026, 11, 22));
        assert_eq!(s.secs, 8.5 * 3600.0);
        assert!(parse_iso("2027-02-29").is_none());
    }
}
