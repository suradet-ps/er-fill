//! Domain types for ER-Refill - the daily emergency-room drug requisition app.
//!
//! Pure data + formatting only: no SQL, no I/O, no async. Every type here is
//! shared across the Tauri boundary and the Leptos frontend, so this crate
//! must compile for `wasm32-unknown-unknown` (it does - chrono is used with
//! its default-free feature set).
//!
//! Module map:
//! - `auth`: [`OfficerAccount`], [`SessionUser`], [`ConnectionSummary`].
//! - `thai`: Thai / Buddhist-era date, time, and quantity formatting.
//! - `window`: [`WindowPreset`], [`RequisitionWindow`] (half-open `[start, end)`).
//! - `catalog`: [`Catalog`], [`DrugCategory`], [`ErDrug`], [`PackStock`] (the ER drug list).
//! - `report`: [`DrugItem`], [`UsageRow`], [`ReportRow`], [`ReportSection`],
//!   [`ReportMeta`].
//! - `settings`: [`AppSettings`] (catalog + report header).

mod auth;
mod catalog;
mod report;
mod settings;
mod thai;
mod window;

pub use auth::{ConnectionSummary, OfficerAccount, SessionUser};
pub use catalog::{Catalog, DrugCategory, ErDrug, PackStock};
pub use report::{DrugItem, ReportMeta, ReportRow, ReportSection, UsageRow};
pub use settings::AppSettings;
pub use thai::{
    be_year, format_qty, format_thai_date, format_thai_date_abbr, format_thai_date_full,
    format_thai_datetime_abbr, format_thai_datetime_full, format_thai_datetime_short,
    format_time_hm, thai_month_abbr, thai_month_full, thai_weekday_abbr, thai_weekday_full,
};
pub use window::{RequisitionWindow, WindowError, WindowPreset};
