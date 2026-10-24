//! The overnight requisition window algorithm (AGENTS.md section 7).
//!
//! The run day is always the current date; the operator only chooses one of
//! four fixed ranges. Every range starts on the previous day and ends on the
//! run day, so a run on 24 Oct with 20:30-08:00 covers 23 Oct 20:30 through
//! 24 Oct 08:00.

use chrono::{Days, NaiveDate, NaiveDateTime};
use er_refill_models::{RequisitionWindow, WindowError, WindowPreset};

/// Computes the window for `preset` on the run date `today`.
///
/// - Start: `today - 1 day` at the preset's start time.
/// - End: `today` at the preset's end time.
///
/// # Errors
///
/// - [`WindowError::DateTooEarly`] when `today` is the earliest
///   representable date (no previous day exists).
/// - [`WindowError::InvalidWindow`] when the resulting window would be
///   empty or reversed.
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use er_refill_core::window_for;
/// use er_refill_models::WindowPreset;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let today = NaiveDate::from_ymd_opt(2026, 10, 24).ok_or("valid date")?;
/// let window = window_for(WindowPreset::TwentyThirtyToEight, today)?;
/// assert_eq!(window.start.to_string(), "2026-10-23 20:30:00");
/// assert_eq!(window.end.to_string(), "2026-10-24 08:00:00");
/// # Ok(())
/// # }
/// ```
pub fn window_for(
    preset: WindowPreset,
    today: NaiveDate,
) -> Result<RequisitionWindow, WindowError> {
    let previous_day = today
        .checked_sub_days(Days::new(1))
        .ok_or(WindowError::DateTooEarly)?;
    let start: NaiveDateTime = previous_day.and_time(preset.start());
    let end: NaiveDateTime = today.and_time(preset.end());
    RequisitionWindow::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> Result<NaiveDate, Box<dyn std::error::Error>> {
        NaiveDate::from_ymd_opt(y, m, d).ok_or_else(|| "valid test date".into())
    }

    fn datetime(
        y: i32,
        m: u32,
        d: u32,
        h: u32,
        mi: u32,
    ) -> Result<NaiveDateTime, Box<dyn std::error::Error>> {
        date(y, m, d)?
            .and_hms_opt(h, mi, 0)
            .ok_or_else(|| "valid test time".into())
    }

    #[test]
    fn every_preset_matches_the_requisition_table() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (
                WindowPreset::TwentyThirtyToEight,
                datetime(2026, 10, 23, 20, 30)?,
                datetime(2026, 10, 24, 8, 0)?,
            ),
            (
                WindowPreset::TwentyThirtyToEightThirty,
                datetime(2026, 10, 23, 20, 30)?,
                datetime(2026, 10, 24, 8, 30)?,
            ),
            (
                WindowPreset::SixteenToEight,
                datetime(2026, 10, 23, 16, 0)?,
                datetime(2026, 10, 24, 8, 0)?,
            ),
            (
                WindowPreset::SixteenToEightThirty,
                datetime(2026, 10, 23, 16, 0)?,
                datetime(2026, 10, 24, 8, 30)?,
            ),
        ];
        let today = date(2026, 10, 24)?;
        for (preset, expected_start, expected_end) in cases {
            let window = window_for(preset, today)?;
            assert_eq!(window.start, expected_start, "start for {preset:?}");
            assert_eq!(window.end, expected_end, "end for {preset:?}");
        }
        Ok(())
    }

    #[test]
    fn window_uses_the_previous_day_for_every_weekday() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (date(2026, 10, 19)?, date(2026, 10, 18)?),
            (date(2026, 10, 20)?, date(2026, 10, 19)?),
            (date(2026, 10, 24)?, date(2026, 10, 23)?),
            (date(2026, 10, 25)?, date(2026, 10, 24)?),
        ];
        for (today, expected_start_date) in cases {
            let window = window_for(WindowPreset::SixteenToEightThirty, today)?;
            assert_eq!(window.start.date(), expected_start_date);
            assert_eq!(window.end.date(), today);
        }
        Ok(())
    }

    #[test]
    fn window_for_min_date_returns_date_too_early() {
        assert_eq!(
            window_for(WindowPreset::SixteenToEight, NaiveDate::MIN),
            Err(WindowError::DateTooEarly)
        );
    }
}
