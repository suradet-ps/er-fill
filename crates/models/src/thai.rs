//! Thai-language and Buddhist-era formatting shared by the UI and the print
//! sheet (AGENTS.md section 4). All dates on screen and on paper are Buddhist-era
//! (พ.ศ. = ค.ศ. + 543) - HOSxP stores Gregorian dates, so conversion happens
//! at the display boundary only.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Full Thai month names, January first. Indexed by `month0()`.
pub const THAI_MONTHS: [&str; 12] = [
    "มกราคม",
    "กุมภาพันธ์",
    "มีนาคม",
    "เมษายน",
    "พฤษภาคม",
    "มิถุนายน",
    "กรกฎาคม",
    "สิงหาคม",
    "กันยายน",
    "ตุลาคม",
    "พฤศจิกายน",
    "ธันวาคม",
];

/// Abbreviated Thai month names ("ม.ค." style).
pub const THAI_MONTHS_ABBR: [&str; 12] = [
    "ม.ค.",
    "ก.พ.",
    "มี.ค.",
    "เม.ย.",
    "พ.ค.",
    "มิ.ย.",
    "ก.ค.",
    "ส.ค.",
    "ก.ย.",
    "ต.ค.",
    "พ.ย.",
    "ธ.ค.",
];

/// Thai weekday names, Sunday first. Indexed by `num_days_from_sunday()`.
pub const THAI_DAYS: [&str; 7] = ["อาทิตย์", "จันทร์", "อังคาร", "พุธ", "พฤหัสบดี", "ศุกร์", "เสาร์"];

/// Abbreviated Thai weekday names ("อา." style).
pub const THAI_DAYS_ABBR: [&str; 7] = ["อา.", "จ.", "อ.", "พ.", "พฤ.", "ศ.", "ส."];

/// Converts a Gregorian year (ค.ศ.) to a Buddhist-era year (พ.ศ.).
///
/// # Examples
///
/// ```
/// use er_refill_models::be_year;
///
/// assert_eq!(be_year(2026), 2569);
/// ```
pub fn be_year(ce_year: i32) -> i32 {
    ce_year + 543
}

/// Full Thai month name of `date` ("ตุลาคม").
pub fn thai_month_full(date: NaiveDate) -> &'static str {
    THAI_MONTHS
        .get(date.month0() as usize)
        .copied()
        .unwrap_or("")
}

/// Abbreviated Thai month name of `date` ("ต.ค.").
pub fn thai_month_abbr(date: NaiveDate) -> &'static str {
    THAI_MONTHS_ABBR
        .get(date.month0() as usize)
        .copied()
        .unwrap_or("")
}

/// Full Thai weekday name of `date` ("อาทิตย์").
pub fn thai_weekday_full(date: NaiveDate) -> &'static str {
    // Fully-qualified `Datelike::weekday`: chrono 0.4.45 added a private
    // inherent `weekday` that would otherwise shadow the trait method.
    THAI_DAYS
        .get(Datelike::weekday(&date).num_days_from_sunday() as usize)
        .copied()
        .unwrap_or("")
}

/// Abbreviated Thai weekday name of `date` ("อา.").
pub fn thai_weekday_abbr(date: NaiveDate) -> &'static str {
    THAI_DAYS_ABBR
        .get(Datelike::weekday(&date).num_days_from_sunday() as usize)
        .copied()
        .unwrap_or("")
}

/// "18 ตุลาคม 2569" - day, full month, Buddhist year.
pub fn format_thai_date(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        thai_month_full(date),
        be_year(date.year())
    )
}

/// "วันอาทิตย์ที่ 18 ตุลาคม 2569" - the long form for print headings.
pub fn format_thai_date_full(date: NaiveDate) -> String {
    format!(
        "วัน{}ที่ {} {} {}",
        thai_weekday_full(date),
        date.day(),
        thai_month_full(date),
        be_year(date.year())
    )
}

/// "อา. 18 ต.ค. 2569" - the compact form for rail labels and table cells.
pub fn format_thai_date_abbr(date: NaiveDate) -> String {
    format!(
        "{} {} {} {}",
        thai_weekday_abbr(date),
        date.day(),
        thai_month_abbr(date),
        be_year(date.year())
    )
}

/// "18 / 10 / 2569" - the numeric Buddhist-era date used on signature lines.
pub fn format_thai_date_slash(date: NaiveDate) -> String {
    format!(
        "{:02} / {:02} / {}",
        date.day(),
        date.month(),
        be_year(date.year())
    )
}

/// "16:00" - 24-hour clock, zero-padded (callers append " น." if idiomatic).
pub fn format_time_hm(time: NaiveTime) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// "วันอาทิตย์ที่ 18 ตุลาคม 2569 เวลา 16:00 น." - the window-band sentence.
pub fn format_thai_datetime_full(datetime: NaiveDateTime) -> String {
    format!(
        "{} เวลา {} น.",
        format_thai_date_full(datetime.date()),
        format_time_hm(datetime.time())
    )
}

/// "อา. 18 ต.ค. 2569 16:00".
pub fn format_thai_datetime_abbr(datetime: NaiveDateTime) -> String {
    format!(
        "{} {}",
        format_thai_date_abbr(datetime.date()),
        format_time_hm(datetime.time())
    )
}

/// "18/10/2569 16:00" - the densest form (table metadata).
pub fn format_thai_datetime_short(datetime: NaiveDateTime) -> String {
    format!(
        "{:02}/{:02}/{} {}",
        datetime.day(),
        datetime.month(),
        be_year(datetime.year()),
        format_time_hm(datetime.time())
    )
}

/// Formats a drug quantity without trailing zeros: `2.0 → "2"`, `2.5 → "2.5"`.
///
/// Rounds to two decimals first, so float sums like `0.1 + 0.2` display as
/// `0.3`. Quantities from HOSxP are small non-negative counts; values beyond
/// `i64` do not occur in practice and would saturate.
pub fn format_qty(qty: f64) -> String {
    let rounded = (qty * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> Result<NaiveDate, Box<dyn std::error::Error>> {
        NaiveDate::from_ymd_opt(y, m, d).ok_or_else(|| "valid test date".into())
    }

    #[test]
    fn be_year_adds_543() {
        assert_eq!(be_year(2026), 2569);
        assert_eq!(be_year(1990), 2533);
    }

    #[test]
    fn format_thai_date_uses_buddhist_year_and_thai_month() {
        let d = date(2026, 10, 18).expect("valid date");
        assert_eq!(format_thai_date(d), "18 ตุลาคม 2569");
    }

    #[test]
    fn format_thai_date_full_prefixes_weekday() {
        let d = date(2026, 10, 18).expect("valid date");
        assert_eq!(format_thai_date_full(d), "วันอาทิตย์ที่ 18 ตุลาคม 2569");
    }

    #[test]
    fn format_thai_date_abbr_is_compact() {
        let d = date(2026, 10, 18).expect("valid date");
        assert_eq!(format_thai_date_abbr(d), "อา. 18 ต.ค. 2569");
    }

    #[test]
    fn format_thai_date_slash_is_numeric_buddhist_era() {
        let d = date(2026, 1, 5).expect("valid date");
        assert_eq!(format_thai_date_slash(d), "05 / 01 / 2569");
        let d = date(2026, 10, 18).expect("valid date");
        assert_eq!(format_thai_date_slash(d), "18 / 10 / 2569");
    }

    #[test]
    fn format_thai_datetime_full_is_the_window_sentence() {
        let d = date(2026, 10, 18).expect("valid date");
        let dt = d.and_hms_opt(16, 0, 0).expect("valid time");
        assert_eq!(
            format_thai_datetime_full(dt),
            "วันอาทิตย์ที่ 18 ตุลาคม 2569 เวลา 16:00 น."
        );
    }

    #[test]
    fn format_thai_datetime_abbr_and_short() {
        let d = date(2026, 10, 19).expect("valid date");
        let dt = d.and_hms_opt(8, 0, 0).expect("valid time");
        assert_eq!(format_thai_datetime_abbr(dt), "จ. 19 ต.ค. 2569 08:00");
        assert_eq!(format_thai_datetime_short(dt), "19/10/2569 08:00");
    }

    #[test]
    fn format_time_hm_zero_pads() {
        let t = NaiveTime::from_hms_opt(8, 5, 0).expect("valid time");
        assert_eq!(format_time_hm(t), "08:05");
    }

    #[test]
    fn format_qty_trims_trailing_zeros() {
        assert_eq!(format_qty(2.0), "2");
        assert_eq!(format_qty(2.5), "2.5");
        assert_eq!(format_qty(0.0), "0");
    }

    #[test]
    fn format_qty_rounds_float_noise() {
        assert_eq!(format_qty(0.1 + 0.2), "0.3");
        assert_eq!(format_qty(2.5000001), "2.5");
    }
}
