//! Shared reactive state for the ER-Refill frontend (Leptos 0.8, CSR).
//!
//! One [`AppState`] of cheap `RwSignal` handles is created in `App` and
//! passed by value into every component (the sibling-workbench pattern):
//! the signals clone cheaply because they are handles into the reactive
//! graph, not the values themselves.

use chrono::{Local, NaiveDate, NaiveDateTime};
use er_refill_models::{
    AppSettings, ConnectionSummary, ReportSection, RequisitionWindow, SessionUser,
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
    /// Business hours (the requisition window ends at opening time).
    Schedule,
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
    /// The run day the operator is requisitioning for (default: today).
    pub selected_date: RwSignal<NaiveDate>,
    /// The effective requisition window (`None` until settings load).
    pub window: RwSignal<Option<RequisitionWindow>>,
    /// True when the operator manually edited the window (amber state).
    pub window_custom: RwSignal<bool>,
    /// Whether the command bar's manual window editor row is open.
    pub window_edit_open: RwSignal<bool>,
    /// Built report sections (`None` before the first fetch / after edits
    /// that invalidate them).
    pub sections: RwSignal<Option<Vec<ReportSection>>>,
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
    /// Creates the state with today's date selected.
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
            selected_date: RwSignal::new(Local::now().date_naive()),
            window: RwSignal::new(None),
            window_custom: RwSignal::new(false),
            window_edit_open: RwSignal::new(false),
            sections: RwSignal::new(None),
            loading: RwSignal::new(false),
            banner: RwSignal::new(None),
            printed_at: RwSignal::new(None),
        }
    }

    /// Recomputes the automatic window from the loaded schedule and the
    /// selected date, and clears the custom flag. Call after settings load,
    /// after a schedule save, and on date change / "กลับเป็นอัตโนมัติ".
    pub fn recompute_window(&self) {
        let Some(settings) = self.settings.get() else {
            return;
        };
        let Ok(window) = er_refill_core::window_for(&settings.schedule, self.selected_date.get())
        else {
            return;
        };
        self.window.set(Some(window));
        self.window_custom.set(false);
    }

    /// Completes a successful login: stores the session, recomputes the
    /// window, and performs the morning fetch when the catalog is ready.
    pub fn finish_login(&self, user: SessionUser) {
        self.session.set(Some(user));
        self.recompute_window();
        if !self.catalog_is_empty() {
            self.fetch_usage_action();
        }
    }

    /// Clears the session UI (backend logout is performed by the caller).
    pub fn clear_session(&self) {
        self.session.set(None);
        self.sections.set(None);
        self.banner.set(None);
        self.settings_open.set(false);
        self.window_edit_open.set(false);
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
        let Some(window) = self.window.get() else {
            return;
        };
        let Some(settings) = self.settings.get() else {
            return;
        };
        let state = self.clone();
        state.loading.set(true);
        state.banner.set(None);
        leptos::task::spawn_local(async move {
            match crate::api::fetch_usage(&window).await {
                Ok(usage) => {
                    let sections = er_refill_core::build_report(&settings.catalog, &usage);
                    state.sections.set(Some(sections));
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
        assert_eq!(
            state.selected_date.get_untracked(),
            Local::now().date_naive()
        );
    }

    #[test]
    fn clear_session_resets_login_gated_ui() {
        let state = AppState::new();
        state.session.set(Some(SessionUser {
            officer_id: 7,
            login_name: "pharm".into(),
            display_name: "Pharmacist".into(),
            doctor_code: None,
            role: "STAFF".into(),
        }));
        state.settings_open.set(true);
        state.window_edit_open.set(true);
        state.clear_session();
        assert!(state.session.get_untracked().is_none());
        assert!(!state.settings_open.get_untracked());
        assert!(!state.window_edit_open.get_untracked());
    }

    #[test]
    fn recompute_window_uses_loaded_schedule() {
        let state = AppState::new();
        state.settings.set(Some(AppSettings::default()));
        // 2026-10-19 is a Monday: window = Sunday 16:00 → Monday 08:30.
        state.selected_date.set(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 19).expect("invariant: static test date"),
        );
        state.recompute_window();
        let window = state.window.get_untracked().expect("window computed");
        assert_eq!(window.start.to_string(), "2026-10-18 16:00:00");
        assert_eq!(window.end.to_string(), "2026-10-19 08:30:00");
        assert!(!state.window_custom.get_untracked());
    }
}
