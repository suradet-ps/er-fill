//! The requisition window - the half-open time range `[start, end)` whose
//! HOSxP drug usage becomes the requisition (AGENTS.md section 7).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::schedule::ScheduleError;

/// A half-open requisition window `[start, end)`.
///
/// Start is the previous day's closing time; end is the opening time of the
/// run day. Both are local (naive) datetimes - HOSxP stores naive dates/times
/// and this app never converts timezones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequisitionWindow {
    /// Inclusive start.
    #[serde(with = "naive_dt")]
    pub start: NaiveDateTime,
    /// Exclusive end.
    #[serde(with = "naive_dt")]
    pub end: NaiveDateTime,
}

impl RequisitionWindow {
    /// Creates a window, rejecting `start >= end`.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleError::InvalidWindow`] when `start >= end`.
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Result<Self, ScheduleError> {
        if start < end {
            Ok(Self { start, end })
        } else {
            Err(ScheduleError::InvalidWindow)
        }
    }

    /// True when `datetime` falls inside `[start, end)`.
    pub fn contains(&self, datetime: NaiveDateTime) -> bool {
        self.start <= datetime && datetime < self.end
    }
}

/// Serde for `NaiveDateTime`: serializes as `"YYYY-MM-DDTHH:MM:SS"` and
/// accepts the `"YYYY-MM-DDTHH:MM"` form produced by HTML `datetime-local`
/// inputs (and by the operator editing the window manually).
mod naive_dt {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        datetime: &NaiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&datetime.format("%Y-%m-%dT%H:%M:%S").to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<NaiveDateTime, D::Error> {
        let raw = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M"))
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(
        y: i32,
        mo: u32,
        d: u32,
        h: u32,
        mi: u32,
    ) -> Result<NaiveDateTime, Box<dyn std::error::Error>> {
        chrono::NaiveDate::from_ymd_opt(y, mo, d)
            .and_then(|date| date.and_hms_opt(h, mi, 0))
            .ok_or_else(|| "valid test datetime".into())
    }

    #[test]
    fn new_rejects_start_at_or_after_end() {
        let start = dt(2026, 10, 18, 16, 0).expect("valid");
        let end = dt(2026, 10, 19, 8, 30).expect("valid");
        assert!(RequisitionWindow::new(start, end).is_ok());
        assert!(RequisitionWindow::new(end, start).is_err());
        assert!(RequisitionWindow::new(start, start).is_err());
    }

    #[test]
    fn contains_is_half_open() {
        let window = RequisitionWindow::new(
            dt(2026, 10, 18, 16, 0).expect("valid"),
            dt(2026, 10, 19, 8, 30).expect("valid"),
        )
        .expect("valid window");
        assert!(window.contains(dt(2026, 10, 18, 16, 0).expect("valid")));
        assert!(window.contains(dt(2026, 10, 19, 8, 29).expect("valid")));
        assert!(!window.contains(dt(2026, 10, 19, 8, 30).expect("valid")));
        assert!(!window.contains(dt(2026, 10, 18, 15, 59).expect("valid")));
    }

    #[test]
    fn serde_accepts_minutes_precision_and_serializes_with_seconds()
    -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{ "start": "2026-10-18T16:00", "end": "2026-10-19T08:30" }"#;
        let window: RequisitionWindow = serde_json::from_str(json)?;
        assert_eq!(window.start, dt(2026, 10, 18, 16, 0).expect("valid"));
        let back = serde_json::to_string(&window)?;
        assert!(back.contains("2026-10-18T16:00:00"));
        Ok(())
    }
}
