//! Business-hours schedule: the weekday/weekend time ranges and the daily
//! requisition cutoff (AGENTS.md section 7). The window algorithm itself lives in
//! `er-fill-core`; this module owns the data and its `"HH:MM"` wire format.

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

/// Errors from schedule construction and window computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleError {
    /// A [`TimeRange`] was built with `start >= end`.
    #[error("business hours range is invalid: start must be before end")]
    InvalidRange,
    /// The requisition day is the earliest representable date - no previous
    /// day exists, so no closing time can be computed.
    #[error("requisition date is too early to compute a previous closing time")]
    DateTooEarly,
    /// The computed (or overridden) window has `start >= end`.
    #[error("requisition window is invalid: start must be before end")]
    InvalidWindow,
}

/// A start/end time-of-day pair (end exclusive).
///
/// Prefer [`TimeRange::new`], which enforces `start < end`. The fields are
/// public because the schedule is plain data shared over the IPC boundary;
/// deserialization trusts the app's own settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start of business hours, inclusive.
    #[serde(with = "hm_time")]
    pub start: NaiveTime,
    /// End of business hours, exclusive (the "closing time").
    #[serde(with = "hm_time")]
    pub end: NaiveTime,
}

impl TimeRange {
    /// Creates a range, rejecting `start >= end`.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleError::InvalidRange`] when `start >= end`.
    pub fn new(start: NaiveTime, end: NaiveTime) -> Result<Self, ScheduleError> {
        if start < end {
            Ok(Self { start, end })
        } else {
            Err(ScheduleError::InvalidRange)
        }
    }
}

/// Business hours for weekdays and weekends plus the daily requisition
/// cutoff (AGENTS.md section 7).
///
/// Defaults, per the pharmacy department's workflow:
/// - weekdays (Mon-Fri) `08:30-16:30`
/// - weekends (Sat-Sun) `08:00-16:00`
/// - cutoff `08:00` - when the morning requisition is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessSchedule {
    /// Hours for Monday-Friday.
    pub weekday: TimeRange,
    /// Hours for Saturday-Sunday.
    pub weekend: TimeRange,
    /// Time of day the daily requisition is produced (window end).
    #[serde(with = "hm_time")]
    pub cutoff: NaiveTime,
}

impl Default for BusinessSchedule {
    fn default() -> Self {
        Self {
            weekday: TimeRange {
                start: hm(8, 30),
                end: hm(16, 30),
            },
            weekend: TimeRange {
                start: hm(8, 0),
                end: hm(16, 0),
            },
            cutoff: hm(8, 0),
        }
    }
}

impl BusinessSchedule {
    /// True when `date` falls on Saturday or Sunday.
    pub fn uses_weekend_hours(&self, date: NaiveDate) -> bool {
        // Fully-qualified `Datelike::weekday`: chrono 0.4.45 added a private
        // inherent `weekday` that would otherwise shadow the trait method.
        matches!(Datelike::weekday(&date), Weekday::Sat | Weekday::Sun)
    }

    /// The time range in effect on `date`.
    pub fn range_for(&self, date: NaiveDate) -> &TimeRange {
        if self.uses_weekend_hours(date) {
            &self.weekend
        } else {
            &self.weekday
        }
    }

    /// Opening time on `date`.
    pub fn opening_time(&self, date: NaiveDate) -> NaiveTime {
        self.range_for(date).start
    }

    /// Closing time on `date` - the start of the next day's out-of-hours
    /// window.
    pub fn closing_time(&self, date: NaiveDate) -> NaiveTime {
        self.range_for(date).end
    }
}

/// Builds a [`NaiveTime`] from literal hours/minutes.
///
/// The `from_hms_opt` fallback is unreachable in practice: this helper is
/// only ever called with compile-time-valid literals; it exists to keep the
/// no-panic rule (AGENTS-RUST.md section 5.5) instead of using `expect`.
fn hm(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN)
}

/// Serde for `NaiveTime` in the operator-friendly `"HH:MM"` form. Also
/// accepts `"HH:MM:SS"` so files written by other tools still load.
mod hm_time {
    use chrono::NaiveTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(time: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&time.format("%H:%M").to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveTime, D::Error> {
        let raw = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&raw, "%H:%M")
            .or_else(|_| NaiveTime::parse_from_str(&raw, "%H:%M:%S"))
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> Result<NaiveDate, Box<dyn std::error::Error>> {
        NaiveDate::from_ymd_opt(y, m, d).ok_or_else(|| "valid test date".into())
    }

    #[test]
    fn default_matches_the_pharmacy_workflow() {
        let schedule = BusinessSchedule::default();
        assert_eq!(schedule.weekday.start, hm(8, 30));
        assert_eq!(schedule.weekday.end, hm(16, 30));
        assert_eq!(schedule.weekend.start, hm(8, 0));
        assert_eq!(schedule.weekend.end, hm(16, 0));
        assert_eq!(schedule.cutoff, hm(8, 0));
    }

    #[test]
    fn weekend_detection_covers_saturday_and_sunday() {
        let schedule = BusinessSchedule::default();
        let saturday = date(2026, 10, 17).expect("valid date");
        let sunday = date(2026, 10, 18).expect("valid date");
        let monday = date(2026, 10, 19).expect("valid date");
        assert!(schedule.uses_weekend_hours(saturday));
        assert!(schedule.uses_weekend_hours(sunday));
        assert!(!schedule.uses_weekend_hours(monday));
    }

    #[test]
    fn closing_times_follow_weekday_weekend_rules() {
        let schedule = BusinessSchedule::default();
        let sunday = date(2026, 10, 18).expect("valid date");
        let monday = date(2026, 10, 19).expect("valid date");
        assert_eq!(schedule.closing_time(sunday), hm(16, 0));
        assert_eq!(schedule.closing_time(monday), hm(16, 30));
    }

    #[test]
    fn time_range_rejects_reversed_bounds() {
        assert!(TimeRange::new(hm(16, 30), hm(8, 30)).is_err());
        assert!(TimeRange::new(hm(8, 30), hm(8, 30)).is_err());
        assert!(TimeRange::new(hm(8, 30), hm(16, 30)).is_ok());
    }

    #[test]
    fn serde_round_trip_uses_hh_mm_strings() -> Result<(), Box<dyn std::error::Error>> {
        let schedule = BusinessSchedule::default();
        let json = serde_json::to_string(&schedule)?;
        assert!(json.contains("\"08:30\""));
        let back: BusinessSchedule = serde_json::from_str(&json)?;
        assert_eq!(back, schedule);
        Ok(())
    }

    #[test]
    fn serde_accepts_hh_mm_ss_too() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "weekday": { "start": "08:30:00", "end": "16:30:00" },
            "weekend": { "start": "08:00", "end": "16:00" },
            "cutoff": "08:00"
        }"#;
        let schedule: BusinessSchedule = serde_json::from_str(json)?;
        assert_eq!(schedule, BusinessSchedule::default());
        Ok(())
    }
}
