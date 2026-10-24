//! The requisition window - the half-open time range `[start, end)` whose
//! HOSxP drug usage becomes the requisition (AGENTS.md section 7).
//!
//! The run day is always the current date and the window is always overnight:
//! it starts on the previous day at one of four fixed evening/afternoon times
//! and ends on the run day at one of two fixed morning times.

use chrono::{NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

/// Errors from window construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WindowError {
    /// The requisition day is the earliest representable date - no previous
    /// day exists, so no window start can be computed.
    #[error("requisition date is too early to compute a previous day")]
    DateTooEarly,
    /// The window has `start >= end`.
    #[error("requisition window is invalid: start must be before end")]
    InvalidWindow,
}

/// The four fixed overnight ranges the operator can choose from.
///
/// Every preset starts on the previous day and ends on the run day, so a run
/// on 24 Oct with [`WindowPreset::TwentyThirtyToEight`] covers
/// 23 Oct 20:30 through 24 Oct 08:00. Thai labels live in the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPreset {
    /// 20:30 (previous day) to 08:00 (run day).
    TwentyThirtyToEight,
    /// 20:30 (previous day) to 08:30 (run day).
    TwentyThirtyToEightThirty,
    /// 16:00 (previous day) to 08:00 (run day).
    SixteenToEight,
    /// 16:00 (previous day) to 08:30 (run day).
    SixteenToEightThirty,
}

impl WindowPreset {
    /// Every selectable preset, in display order.
    pub const ALL: [Self; 4] = [
        Self::TwentyThirtyToEight,
        Self::TwentyThirtyToEightThirty,
        Self::SixteenToEight,
        Self::SixteenToEightThirty,
    ];

    /// Start time on the previous day.
    pub fn start(self) -> NaiveTime {
        match self {
            Self::TwentyThirtyToEight | Self::TwentyThirtyToEightThirty => at(20, 30),
            Self::SixteenToEight | Self::SixteenToEightThirty => at(16, 0),
        }
    }

    /// End time on the run day.
    pub fn end(self) -> NaiveTime {
        match self {
            Self::TwentyThirtyToEight | Self::SixteenToEight => at(8, 0),
            Self::TwentyThirtyToEightThirty | Self::SixteenToEightThirty => at(8, 30),
        }
    }
}

/// A half-open requisition window `[start, end)`.
///
/// Both endpoints are local (naive) datetimes - HOSxP stores naive dates and
/// times and this app never converts timezones.
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
    /// Returns [`WindowError::InvalidWindow`] when `start >= end`.
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Result<Self, WindowError> {
        if start < end {
            Ok(Self { start, end })
        } else {
            Err(WindowError::InvalidWindow)
        }
    }

    /// True when `datetime` falls inside `[start, end)`.
    pub fn contains(&self, datetime: NaiveDateTime) -> bool {
        self.start <= datetime && datetime < self.end
    }
}

/// Builds a [`NaiveTime`] from literal hours/minutes.
///
/// The `from_hms_opt` fallback is unreachable in practice: this helper is
/// only ever called with compile-time-valid literals; it exists to keep the
/// no-panic rule (AGENTS-RUST.md section 5.5) instead of using `expect`.
fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN)
}

/// Serde for `NaiveDateTime`: serializes as `"YYYY-MM-DDTHH:MM:SS"` and
/// accepts the `"YYYY-MM-DDTHH:MM"` form produced by HTML `datetime-local`
/// inputs.
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
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> Result<NaiveDate, Box<dyn std::error::Error>> {
        NaiveDate::from_ymd_opt(y, m, d).ok_or_else(|| "valid test date".into())
    }

    fn dt(
        y: i32,
        mo: u32,
        d: u32,
        h: u32,
        mi: u32,
    ) -> Result<NaiveDateTime, Box<dyn std::error::Error>> {
        date(y, mo, d)?
            .and_hms_opt(h, mi, 0)
            .ok_or_else(|| "valid test datetime".into())
    }

    #[test]
    fn presets_expose_the_four_fixed_ranges() {
        assert_eq!(WindowPreset::TwentyThirtyToEight.start(), at(20, 30));
        assert_eq!(WindowPreset::TwentyThirtyToEight.end(), at(8, 0));
        assert_eq!(WindowPreset::TwentyThirtyToEightThirty.end(), at(8, 30));
        assert_eq!(WindowPreset::SixteenToEight.start(), at(16, 0));
        assert_eq!(WindowPreset::SixteenToEight.end(), at(8, 0));
        assert_eq!(WindowPreset::SixteenToEightThirty.end(), at(8, 30));
        assert_eq!(WindowPreset::ALL.len(), 4);
    }

    #[test]
    fn new_rejects_start_at_or_after_end() {
        let start = dt(2026, 10, 23, 16, 0).expect("valid");
        let end = dt(2026, 10, 24, 8, 30).expect("valid");
        assert!(RequisitionWindow::new(start, end).is_ok());
        assert!(RequisitionWindow::new(end, start).is_err());
        assert!(RequisitionWindow::new(start, start).is_err());
    }

    #[test]
    fn contains_is_half_open() {
        let window = RequisitionWindow::new(
            dt(2026, 10, 23, 16, 0).expect("valid"),
            dt(2026, 10, 24, 8, 30).expect("valid"),
        )
        .expect("valid window");
        assert!(window.contains(dt(2026, 10, 23, 16, 0).expect("valid")));
        assert!(window.contains(dt(2026, 10, 24, 8, 29).expect("valid")));
        assert!(!window.contains(dt(2026, 10, 24, 8, 30).expect("valid")));
        assert!(!window.contains(dt(2026, 10, 23, 15, 59).expect("valid")));
    }

    #[test]
    fn serde_accepts_minutes_precision_and_serializes_with_seconds()
    -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{ "start": "2026-10-23T16:00", "end": "2026-10-24T08:30" }"#;
        let window: RequisitionWindow = serde_json::from_str(json)?;
        assert_eq!(window.start, dt(2026, 10, 23, 16, 0)?);
        let back = serde_json::to_string(&window)?;
        assert!(back.contains("2026-10-23T16:00:00"));
        Ok(())
    }
}
