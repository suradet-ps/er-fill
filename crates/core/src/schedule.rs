//! The out-of-hours requisition window algorithm (AGENTS.md section 7).
//!
//! The window for a morning run on day `D` is `[closing(D - 1), cutoff(D))`.
//! Every day has a closing time (weekday or weekend hours), so consecutive
//! windows chain correctly: Monday's window starts at Sunday 16:00, Satur-
//! day's at Friday 16:30, and so on. Usage during business hours stays with
//! the pharmacy's in-hours process - this app covers out-of-hours only.

use chrono::{Days, NaiveDate, NaiveDateTime};
use er_fill_models::{BusinessSchedule, RequisitionWindow, ScheduleError};

/// Computes the requisition window for a run on `today`.
///
/// - Start: `today - 1 day` at that day's closing time.
/// - End: `today` at the schedule's cutoff time (default 08:00).
///
/// # Errors
///
/// - [`ScheduleError::DateTooEarly`] when `today` is the earliest
///   representable date (no previous day exists).
/// - [`ScheduleError::InvalidWindow`] when the resulting window would be
///   empty or reversed (e.g. a cutoff before the previous day's closing).
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use er_fill_core::window_for;
/// use er_fill_models::BusinessSchedule;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let monday = NaiveDate::from_ymd_opt(2026, 10, 19).ok_or("valid date")?;
/// let window = window_for(&BusinessSchedule::default(), monday)?;
/// assert_eq!(window.start.to_string(), "2026-10-18 16:00:00"); // Sunday closing
/// assert_eq!(window.end.to_string(), "2026-10-19 08:00:00");   // Monday cutoff
/// # Ok(())
/// # }
/// ```
pub fn window_for(
    schedule: &BusinessSchedule,
    today: NaiveDate,
) -> Result<RequisitionWindow, ScheduleError> {
    let previous_day = today
        .checked_sub_days(Days::new(1))
        .ok_or(ScheduleError::DateTooEarly)?;
    let start: NaiveDateTime = previous_day.and_time(schedule.closing_time(previous_day));
    let end: NaiveDateTime = today.and_time(schedule.cutoff);
    RequisitionWindow::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use er_fill_models::TimeRange;

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
    fn window_for_monday_runs_from_sunday_closing_to_monday_cutoff()
    -> Result<(), Box<dyn std::error::Error>> {
        let monday = date(2026, 10, 19)?;
        let window = window_for(&BusinessSchedule::default(), monday)?;
        assert_eq!(window.start, datetime(2026, 10, 18, 16, 0)?);
        assert_eq!(window.end, datetime(2026, 10, 19, 8, 0)?);
        Ok(())
    }

    #[test]
    fn window_for_tuesday_runs_from_monday_closing() -> Result<(), Box<dyn std::error::Error>> {
        let tuesday = date(2026, 10, 20)?;
        let window = window_for(&BusinessSchedule::default(), tuesday)?;
        assert_eq!(window.start, datetime(2026, 10, 19, 16, 30)?);
        assert_eq!(window.end, datetime(2026, 10, 20, 8, 0)?);
        Ok(())
    }

    #[test]
    fn window_for_saturday_runs_from_friday_closing() -> Result<(), Box<dyn std::error::Error>> {
        let saturday = date(2026, 10, 17)?;
        let window = window_for(&BusinessSchedule::default(), saturday)?;
        assert_eq!(window.start, datetime(2026, 10, 16, 16, 30)?);
        assert_eq!(window.end, datetime(2026, 10, 17, 8, 0)?);
        Ok(())
    }

    #[test]
    fn window_for_sunday_runs_from_saturday_closing() -> Result<(), Box<dyn std::error::Error>> {
        let sunday = date(2026, 10, 18)?;
        let window = window_for(&BusinessSchedule::default(), sunday)?;
        assert_eq!(window.start, datetime(2026, 10, 17, 16, 0)?);
        assert_eq!(window.end, datetime(2026, 10, 18, 8, 0)?);
        Ok(())
    }

    #[test]
    fn window_for_custom_hours_are_respected() -> Result<(), Box<dyn std::error::Error>> {
        let schedule = BusinessSchedule {
            weekday: TimeRange::new(
                NaiveTime::from_hms_opt(9, 0, 0).ok_or("valid")?,
                NaiveTime::from_hms_opt(17, 0, 0).ok_or("valid")?,
            )?,
            weekend: TimeRange::new(
                NaiveTime::from_hms_opt(10, 0, 0).ok_or("valid")?,
                NaiveTime::from_hms_opt(15, 0, 0).ok_or("valid")?,
            )?,
            cutoff: NaiveTime::from_hms_opt(7, 30, 0).ok_or("valid")?,
        };
        let wednesday = date(2026, 10, 21)?;
        let window = window_for(&schedule, wednesday)?;
        assert_eq!(window.start, datetime(2026, 10, 20, 17, 0)?);
        assert_eq!(window.end, datetime(2026, 10, 21, 7, 30)?);
        Ok(())
    }

    #[test]
    fn window_for_early_cutoff_still_yields_a_valid_window()
    -> Result<(), Box<dyn std::error::Error>> {
        // A cutoff near midnight still produces a valid (short) window -
        // start is always on the previous calendar day, so it cannot reach
        // past the cutoff. Pathological schedules are surfaced by the UI
        // preview, not rejected here.
        let schedule = BusinessSchedule {
            cutoff: NaiveTime::from_hms_opt(0, 0, 0).ok_or("valid")?,
            ..BusinessSchedule::default()
        };
        let monday = date(2026, 10, 19)?;
        let window = window_for(&schedule, monday)?;
        assert_eq!(window.start, datetime(2026, 10, 18, 16, 0)?);
        assert_eq!(window.end, datetime(2026, 10, 19, 0, 0)?);
        Ok(())
    }

    #[test]
    fn window_for_min_date_returns_date_too_early() {
        assert_eq!(
            window_for(&BusinessSchedule::default(), NaiveDate::MIN),
            Err(ScheduleError::DateTooEarly)
        );
    }
}
