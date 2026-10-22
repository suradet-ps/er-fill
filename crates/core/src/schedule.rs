//! The out-of-hours requisition window algorithm (AGENTS.md section 7).
//!
//! The window for a morning run on day `D` is `[closing(D - 1), opening(D))`.
//! Every day has a closing time (weekday or weekend hours), so consecutive
//! windows chain correctly: Monday's window starts at Sunday 16:00, Satur-
//! day's at Friday 20:30, and so on. Usage during business hours stays with
//! the pharmacy's in-hours process - this app covers out-of-hours only.

use chrono::{Days, NaiveDate, NaiveDateTime};
use er_refill_models::{BusinessSchedule, RequisitionWindow, ScheduleError};

/// Computes the requisition window for a run on `today`.
///
/// - Start: `today - 1 day` at that day's closing time.
/// - End: `today` at that day's opening time.
///
/// # Errors
///
/// - [`ScheduleError::DateTooEarly`] when `today` is the earliest
///   representable date (no previous day exists).
/// - [`ScheduleError::InvalidWindow`] when the resulting window would be
///   empty or reversed.
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use er_refill_core::window_for;
/// use er_refill_models::BusinessSchedule;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let monday = NaiveDate::from_ymd_opt(2026, 10, 19).ok_or("valid date")?;
/// let window = window_for(&BusinessSchedule::default(), monday)?;
/// assert_eq!(window.start.to_string(), "2026-10-18 16:00:00"); // Sunday closing
/// assert_eq!(window.end.to_string(), "2026-10-19 08:30:00");   // Monday opening
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
    let end: NaiveDateTime = today.and_time(schedule.opening_time(today));
    RequisitionWindow::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use er_refill_models::TimeRange;

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
    fn window_for_all_seven_days_matches_the_requisition_table()
    -> Result<(), Box<dyn std::error::Error>> {
        // Week of Mon 2026-10-19 through Sun 2026-10-25 (the pharmacy's
        // weekday closing is 20:30; the weekend closes at 16:00).
        let cases = [
            (
                date(2026, 10, 19)?,
                datetime(2026, 10, 18, 16, 0)?,
                datetime(2026, 10, 19, 8, 30)?,
            ),
            (
                date(2026, 10, 20)?,
                datetime(2026, 10, 19, 20, 30)?,
                datetime(2026, 10, 20, 8, 30)?,
            ),
            (
                date(2026, 10, 21)?,
                datetime(2026, 10, 20, 20, 30)?,
                datetime(2026, 10, 21, 8, 30)?,
            ),
            (
                date(2026, 10, 22)?,
                datetime(2026, 10, 21, 20, 30)?,
                datetime(2026, 10, 22, 8, 30)?,
            ),
            (
                date(2026, 10, 23)?,
                datetime(2026, 10, 22, 20, 30)?,
                datetime(2026, 10, 23, 8, 30)?,
            ),
            (
                date(2026, 10, 24)?,
                datetime(2026, 10, 23, 20, 30)?,
                datetime(2026, 10, 24, 8, 0)?,
            ),
            (
                date(2026, 10, 25)?,
                datetime(2026, 10, 24, 16, 0)?,
                datetime(2026, 10, 25, 8, 0)?,
            ),
        ];
        for (today, expected_start, expected_end) in cases {
            let window = window_for(&BusinessSchedule::default(), today)?;
            assert_eq!(window.start, expected_start, "start for {today}");
            assert_eq!(window.end, expected_end, "end for {today}");
        }
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
        };
        let wednesday = date(2026, 10, 21)?;
        let window = window_for(&schedule, wednesday)?;
        assert_eq!(window.start, datetime(2026, 10, 20, 17, 0)?);
        assert_eq!(window.end, datetime(2026, 10, 21, 9, 0)?);
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
