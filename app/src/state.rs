//! Shared reactive state for the ER-Refill frontend (Leptos 0.8, CSR).
//!
//! One [`AppState`] of cheap `RwSignal` handles is created in `App` and
//! passed by value into every component (the sibling-workbench pattern):
//! the signals clone cheaply because they are handles into the reactive
//! graph, not the values themselves.

use chrono::{Local, NaiveDate, NaiveDateTime};
use er_refill_models::{
    AppSettings, ConnectionSummary, ReportMeta, ReportSection, RequisitionWindow, SessionUser,
    WindowPreset,
};
use leptos::prelude::*;

/// Live reachability of HOSxP, mirrored from the backend's
/// `connection_health` (PascalCase on the wire, matching the Rust enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum ConnectionHealth {
    /// No stored settings yet.
    Unconfigured,
    /// A ping succeeded recently.
    Connected,
    /// A ping failed, or a query could not reach the database.
    Disconnected,
}

impl ConnectionHealth {
    /// Thai label for the status dot.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::Unconfigured => "ยังไม่ตั้งค่าการเชื่อมต่อ",
            Self::Connected => "เชื่อมต่อ HOSxP แล้ว",
            Self::Disconnected => "การเชื่อมต่อขัดข้อง",
        }
    }

    /// BEM modifier for `status-dot--*`.
    pub fn dot_class(self) -> &'static str {
        match self {
            Self::Unconfigured => "idle",
            Self::Connected => "ok",
            Self::Disconnected => "down",
        }
    }
}

/// Which settings tab is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    /// HOSxP connection credentials.
    Connection,
    /// Categories + drugs.
    Catalog,
    /// Printed header block.
    Report,
}

/// Banner tone - error red, attention amber, or neutral info.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    /// Something failed.
    Error,
    /// Something needs attention (degraded connection, stale data).
    Warn,
    /// Informational confirmation.
    Info,
}

/// A dismissible message strip shown under the top bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Banner {
    /// Tone selector.
    pub kind: BannerKind,
    /// Message text (Thai).
    pub message: String,
}

impl Banner {
    /// Builds an error banner.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Error,
            message: message.into(),
        }
    }

    /// Builds an attention banner.
    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Warn,
            message: message.into(),
        }
    }

    /// Builds an informational banner.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Info,
            message: message.into(),
        }
    }
}

/// The data set the operator confirmed before printing. Printing is gated on
/// this snapshot; any report edit clears it.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmedRequisition {
    /// Rows (including their reasons) exactly as confirmed.
    pub sections: Vec<ReportSection>,
    /// The requisition window shown on the sheet.
    pub window: RequisitionWindow,
    /// The printed header block at confirmation time.
    pub report: ReportMeta,
    /// When the operator pressed "ยืนยันข้อมูลเบิก".
    pub confirmed_at: NaiveDateTime,
}

/// A pending quantity edit waiting for its reason.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingEdit {
    /// HOSxP icode - the row identity.
    pub icode: String,
    /// Display name shown in the dialog.
    pub name: String,
    /// Strength label shown beside the name in the dialog.
    pub strength_text: String,
    /// The quantity before the edit.
    pub old_requested: f64,
    /// The quantity the operator wants.
    pub new_requested: f64,
    /// The stock ceiling carried by the row, if any.
    pub max_requested: Option<f64>,
}

/// All shared UI state.
#[derive(Clone)]
pub struct AppState {
    /// Loaded settings (`None` until the initial `load_settings` resolves).
    pub settings: RwSignal<Option<AppSettings>>,
    /// Whether encrypted connection settings exist on this machine.
    pub configured: RwSignal<bool>,
    /// Non-secret connection fields for the login screen.
    pub connection_summary: RwSignal<Option<ConnectionSummary>>,
    /// Last connection failure to show on the login screen.
    pub connection_error: RwSignal<Option<String>>,
    /// Signed-in HOSxP officer, mirrored from the backend session.
    pub session: RwSignal<Option<SessionUser>>,
    /// True once the startup probe (settings, connection, session) finished.
    pub auth_checked: RwSignal<bool>,
    /// Status-dot source, polled every 30 s.
    pub health: RwSignal<ConnectionHealth>,
    /// Settings modal visibility.
    pub settings_open: RwSignal<bool>,
    /// Active settings tab.
    pub settings_tab: RwSignal<SettingsTab>,
    /// The fixed run day (today, refreshed whenever a preset is chosen).
    pub run_date: RwSignal<NaiveDate>,
    /// The operator's chosen overnight range, if any. Fetching is gated on
    /// this being set.
    pub window_preset: RwSignal<Option<WindowPreset>>,
    /// The effective requisition window (`None` until a preset is chosen).
    pub window: RwSignal<Option<RequisitionWindow>>,
    /// Monotonic token that invalidates in-flight fetches whenever the
    /// requisition is cleared (range switch, date rollover, logout, import).
    pub fetch_generation: RwSignal<u64>,
    /// Built report sections (`None` before the first fetch / after edits
    /// that invalidate them).
    pub sections: RwSignal<Option<Vec<ReportSection>>>,
    /// The confirmed print snapshot; `None` disables the print button.
    pub confirmed: RwSignal<Option<ConfirmedRequisition>>,
    /// A quantity edit waiting for its reason.
    pub pending_edit: RwSignal<Option<PendingEdit>>,
    /// The "add configured drug" dialog is open.
    pub add_drug_open: RwSignal<bool>,
    /// The pre-print confirmation dialog is open.
    pub confirm_open: RwSignal<bool>,
    /// A fetch is in flight.
    pub loading: RwSignal<bool>,
    /// Dismissible message strip.
    pub banner: RwSignal<Option<Banner>>,
    /// When the current sheet was last printed (print-sheet footer).
    pub printed_at: RwSignal<Option<NaiveDateTime>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Creates the state with today's date and no time range chosen.
    pub fn new() -> Self {
        Self {
            settings: RwSignal::new(None),
            configured: RwSignal::new(false),
            connection_summary: RwSignal::new(None),
            connection_error: RwSignal::new(None),
            session: RwSignal::new(None),
            auth_checked: RwSignal::new(false),
            health: RwSignal::new(ConnectionHealth::Unconfigured),
            settings_open: RwSignal::new(false),
            settings_tab: RwSignal::new(SettingsTab::Connection),
            run_date: RwSignal::new(Local::now().date_naive()),
            window_preset: RwSignal::new(None),
            window: RwSignal::new(None),
            fetch_generation: RwSignal::new(0),
            sections: RwSignal::new(None),
            confirmed: RwSignal::new(None),
            pending_edit: RwSignal::new(None),
            add_drug_open: RwSignal::new(false),
            confirm_open: RwSignal::new(false),
            loading: RwSignal::new(false),
            banner: RwSignal::new(None),
            printed_at: RwSignal::new(None),
        }
    }

    /// Applies the operator's chosen overnight range against today's date.
    ///
    /// The run date is never editable: it is refreshed to today here so a
    /// window that spans midnight still names the correct morning. Choosing
    /// the already-selected preset is a no-op (it must not discard a sheet).
    pub fn select_window_preset(&self, preset: WindowPreset) {
        if self.window_preset.get_untracked() == Some(preset) {
            return;
        }
        let today = Local::now().date_naive();
        let Ok(window) = er_refill_core::window_for(preset, today) else {
            self.banner
                .set(Some(Banner::error("คำนวณช่วงเวลาเบิกไม่สำเร็จ")));
            return;
        };
        self.run_date.set(today);
        self.window_preset.set(Some(preset));
        self.window.set(Some(window));
        self.banner.set(None);
        self.clear_requisition();
    }

    /// Rolls the fixed run date over at midnight, even when no range has
    /// been chosen yet. Called from the recurring health timer.
    ///
    /// With a preset selected the window is recomputed for the new date and
    /// the current sheet is cleared, because it covers the previous day. If
    /// window computation fails (practically impossible), the old date and
    /// window are kept so the UI never shows a mismatched pair.
    pub fn refresh_date_for_today(&self) {
        let today = Local::now().date_naive();
        if self.run_date.get_untracked() == today {
            return;
        }
        let Some(preset) = self.window_preset.get_untracked() else {
            self.run_date.set(today);
            return;
        };
        let Ok(window) = er_refill_core::window_for(preset, today) else {
            self.banner
                .set(Some(Banner::error("คำนวณช่วงเวลาเบิกไม่สำเร็จ")));
            return;
        };
        self.run_date.set(today);
        self.window.set(Some(window));
        self.clear_requisition();
    }

    /// Completes a successful login. The range is not chosen yet, so the
    /// operator selects one before fetching (AGENTS.md section 8).
    pub fn finish_login(&self, user: SessionUser) {
        self.session.set(Some(user));
        self.run_date.set(Local::now().date_naive());
    }

    /// Clears the session UI (backend logout is performed by the caller).
    pub fn clear_session(&self) {
        self.session.set(None);
        self.window_preset.set(None);
        self.window.set(None);
        self.clear_requisition();
        self.banner.set(None);
        self.settings_open.set(false);
    }

    /// Drops the current document (rows, print confirmation, dialogs) because
    /// the date, window, or catalog changed and a new fetch is required.
    /// Bumps [`Self::fetch_generation`] so any in-flight response is discarded
    /// even if the selected window later returns to the same value.
    pub fn clear_requisition(&self) {
        self.fetch_generation.update(|generation| *generation += 1);
        self.sections.set(None);
        self.confirmed.set(None);
        self.pending_edit.set(None);
        self.add_drug_open.set(false);
        self.confirm_open.set(false);
        self.printed_at.set(None);
    }

    /// Invalidates the confirmed print snapshot without discarding the rows.
    /// Called after a manual add/quantity change; the operator must confirm
    /// again before printing.
    pub fn invalidate_confirmation(&self) {
        let had_confirmation = self.confirmed.get_untracked().is_some();
        self.confirmed.set(None);
        self.printed_at.set(None);
        self.confirm_open.set(false);
        if had_confirmation {
            self.banner
                .set(Some(Banner::warn("ข้อมูลเปลี่ยนแล้ว กรุณายืนยันข้อมูลเบิกใหม่ก่อนพิมพ์")));
        }
    }

    /// Snapshots the current rows, window, and header for printing.
    ///
    /// Returns `false` when there is no sheet or window to confirm.
    pub fn confirm_requisition(&self) -> bool {
        let Some(sections) = self.sections.get_untracked() else {
            return false;
        };
        let Some(window) = self.window.get_untracked() else {
            return false;
        };
        let Some(report) = self
            .settings
            .get_untracked()
            .map(|settings| settings.report)
        else {
            return false;
        };
        if sections.iter().all(|section| section.rows.is_empty()) {
            return false;
        }
        self.confirmed.set(Some(ConfirmedRequisition {
            sections,
            window,
            report,
            confirmed_at: Local::now().naive_local(),
        }));
        self.printed_at.set(None);
        self.set_info_banner("ยืนยันข้อมูลเบิกแล้ว พร้อมพิมพ์ใบเบิก");
        true
    }

    /// Shows a success/information banner. `SheetBanner` auto-dismisses info
    /// banners after a few seconds (warnings/errors stay until dismissed).
    pub fn set_info_banner(&self, message: impl Into<String>) {
        self.banner.set(Some(Banner::info(message)));
    }

    /// True when the current rows, window, and header are exactly the set the
    /// operator confirmed. The print action refuses to run otherwise.
    pub fn print_snapshot_matches(&self) -> bool {
        let Some(confirmed) = self.confirmed.get_untracked() else {
            return false;
        };
        let Some(sections) = self.sections.get_untracked() else {
            return false;
        };
        let Some(window) = self.window.get_untracked() else {
            return false;
        };
        let Some(report) = self
            .settings
            .get_untracked()
            .map(|settings| settings.report)
        else {
            return false;
        };
        sections == confirmed.sections && window == confirmed.window && report == confirmed.report
    }

    /// True when the catalog exists but has no drugs configured.
    pub fn catalog_is_empty(&self) -> bool {
        self.settings
            .get()
            .is_some_and(|settings| settings.catalog.is_empty())
    }

    /// Total number of configured drugs.
    pub fn catalog_drug_count(&self) -> usize {
        self.settings
            .get()
            .map(|settings| settings.catalog.drugs.len())
            .unwrap_or(0)
    }

    /// Fetches usage for the current window and builds the report sections.
    ///
    /// Reads the catalog from `settings` for the join; a missing window or
    /// settings is a no-op (the fetch button is disabled in that state).
    /// On failure the banner explains it in Thai and the kind decides the
    /// tone; "not configured" also steers the operator to the right tab.
    pub fn fetch_usage_action(&self) {
        let Some(preset) = self.window_preset.get_untracked() else {
            return;
        };
        // Recompute against today so an app left open past midnight can not
        // fetch yesterday's window.
        let today = Local::now().date_naive();
        let Ok(window) = er_refill_core::window_for(preset, today) else {
            self.banner
                .set(Some(Banner::error("คำนวณช่วงเวลาเบิกไม่สำเร็จ")));
            return;
        };
        if self.run_date.get_untracked() != today {
            self.run_date.set(today);
            self.window.set(Some(window));
            self.clear_requisition();
        }
        let Some(settings) = self.settings.get() else {
            return;
        };
        let state = self.clone();
        let generation = state.fetch_generation.get_untracked();
        state.loading.set(true);
        state.banner.set(None);
        state.confirmed.set(None);
        state.printed_at.set(None);
        leptos::task::spawn_local(async move {
            let result = crate::api::fetch_usage(&window).await;
            // The operator may have switched ranges while this query ran;
            // a stale response must not replace the newly chosen window's
            // sheet. The generation guard also covers A -> B -> A, where the
            // window matches again but an intervening clear invalidated it.
            if state.fetch_generation.get_untracked() != generation
                || state.window.get_untracked() != Some(window)
            {
                state.loading.set(false);
                return;
            }
            match result {
                Ok(usage) => {
                    let sections = er_refill_core::build_report(&settings.catalog, &usage);
                    state.sections.set(Some(sections));
                    state.confirmed.set(None);
                    state.printed_at.set(None);
                }
                Err(err) => {
                    let banner = if err.kind == crate::api::ApiErrorKind::NotConfigured {
                        state.settings_tab.set(SettingsTab::Connection);
                        Banner::warn(err.message)
                    } else {
                        Banner::error(err.message)
                    };
                    state.banner.set(Some(banner));
                }
            }
            state.loading.set(false);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_labels_and_classes_are_stable() {
        assert_eq!(ConnectionHealth::Connected.label_th(), "เชื่อมต่อ HOSxP แล้ว");
        assert_eq!(ConnectionHealth::Connected.dot_class(), "ok");
        assert_eq!(ConnectionHealth::Disconnected.dot_class(), "down");
        assert_eq!(ConnectionHealth::Unconfigured.dot_class(), "idle");
    }

    #[test]
    fn new_state_starts_unconfigured_with_todays_date() {
        let state = AppState::new();
        assert_eq!(state.health.get_untracked(), ConnectionHealth::Unconfigured);
        assert!(!state.configured.get_untracked());
        assert!(state.session.get_untracked().is_none());
        assert!(!state.auth_checked.get_untracked());
        assert!(state.window.get_untracked().is_none());
        assert!(state.window_preset.get_untracked().is_none());
        assert_eq!(state.run_date.get_untracked(), Local::now().date_naive());
    }

    #[test]
    fn clear_session_resets_login_gated_ui_and_range() {
        let state = AppState::new();
        state.session.set(Some(SessionUser {
            officer_id: 7,
            login_name: "pharm".into(),
            display_name: "Pharmacist".into(),
            doctor_code: None,
            role: "STAFF".into(),
        }));
        state.settings_open.set(true);
        state.select_window_preset(WindowPreset::SixteenToEight);
        state.clear_session();
        assert!(state.session.get_untracked().is_none());
        assert!(!state.settings_open.get_untracked());
        assert!(state.window_preset.get_untracked().is_none());
        assert!(state.window.get_untracked().is_none());
    }

    #[test]
    fn selecting_a_preset_uses_today_and_previous_day() {
        let state = AppState::new();
        state.select_window_preset(WindowPreset::TwentyThirtyToEightThirty);
        let today = Local::now().date_naive();
        let window = state.window.get_untracked().expect("window computed");
        assert_eq!(
            window.start.date(),
            today
                .checked_sub_days(chrono::Days::new(1))
                .expect("today has a previous day")
        );
        assert_eq!(window.end.date(), today);
        assert_eq!(
            window.start.time(),
            chrono::NaiveTime::from_hms_opt(20, 30, 0).expect("valid time")
        );
        assert_eq!(
            window.end.time(),
            chrono::NaiveTime::from_hms_opt(8, 30, 0).expect("valid time")
        );
        assert_eq!(
            state.window_preset.get_untracked(),
            Some(WindowPreset::TwentyThirtyToEightThirty)
        );

        // Re-selecting the same preset must not discard the current sheet.
        state.sections.set(Some(Vec::new()));
        state.select_window_preset(WindowPreset::TwentyThirtyToEightThirty);
        assert!(state.sections.get_untracked().is_some());
    }

    #[test]
    fn date_rollover_refreshes_the_date_and_invalidates_the_sheet() {
        let state = AppState::new();
        let today = Local::now().date_naive();
        let yesterday = today
            .checked_sub_days(chrono::Days::new(1))
            .expect("today has a previous day");

        // Without a preset the date still rolls over; nothing else changes.
        state.run_date.set(yesterday);
        state.refresh_date_for_today();
        assert_eq!(state.run_date.get_untracked(), today);
        assert!(state.window.get_untracked().is_none());

        // With a preset the window follows the new date and the old sheet
        // plus any in-flight fetch are invalidated.
        state.select_window_preset(WindowPreset::SixteenToEight);
        state.sections.set(Some(Vec::new()));
        let generation = state.fetch_generation.get_untracked();
        state.run_date.set(yesterday);
        state.refresh_date_for_today();
        assert_eq!(state.run_date.get_untracked(), today);
        assert_eq!(
            state
                .window
                .get_untracked()
                .expect("window recomputed")
                .end
                .date(),
            today
        );
        assert!(state.sections.get_untracked().is_none());
        assert_eq!(state.fetch_generation.get_untracked(), generation + 1);
    }

    #[test]
    fn confirmation_snapshots_the_sheet_and_edits_invalidate_it() {
        let state = AppState::new();
        state.settings.set(Some(AppSettings::default()));
        state.window.set(Some(
            RequisitionWindow::new(
                chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
                    .and_then(|date| date.and_hms_opt(16, 0, 0))
                    .expect("invariant: static test date"),
                chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
                    .and_then(|date| date.and_hms_opt(8, 30, 0))
                    .expect("invariant: static test date"),
            )
            .expect("valid window"),
        ));
        state.sections.set(Some(vec![ReportSection {
            category_id: "cat-1".into(),
            category_name: "ยาเม็ด".into(),
            rows: vec![er_refill_models::ReportRow {
                icode: "1001".into(),
                name: "Paracetamol".into(),
                strength_text: "500 mg".into(),
                stock_text: "10/20".into(),
                usage: 6.0,
                requested: 1.0,
                max_requested: Some(20.0),
                added: false,
                reason: String::new(),
            }],
        }]));

        assert!(state.confirm_requisition());
        assert!(state.confirmed.get_untracked().is_some());
        assert!(state.print_snapshot_matches());

        let mut edited = state.sections.get_untracked().expect("sections");
        edited[0].rows[0].requested = 2.0;
        state.sections.set(Some(edited));
        assert!(!state.print_snapshot_matches());

        state.invalidate_confirmation();
        assert!(state.confirmed.get_untracked().is_none());
    }
}
