//! Tauri command adapters (AGENTS.md section 3): validate → delegate → translate.
//!
//! No SQL and no business logic live here - everything HOSxP-related goes
//! through `er-fill-hosxp-connector`, and this file is the boundary where
//! errors become Thai ([`crate::error::CommandError`]).

use std::time::{Duration, Instant};

use er_fill_core::{HosxRepository, RepositoryError};
use er_fill_hosxp_connector::config::{HosxConfig, load_vault};
use er_fill_hosxp_connector::{ConnectorRepository, MySqlPool, connect};
use er_fill_models::{
    BusinessSchedule, Catalog, DrugItem, ReportMeta, RequisitionWindow, UsageRow,
};
use secrecy::SecretBox;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::CommandError;
use crate::settings::{self, AppSettings};
use crate::state::{AppState, ConnectionHealth};

/// How often the health monitor pings HOSxP.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Minimum characters before a drug search hits the database.
const SEARCH_TERM_MIN_CHARS: usize = 2;

/// Upper bound on the operator's search term (defensive; the term is bound,
/// never interpolated).
const SEARCH_TERM_MAX_CHARS: usize = 100;

/// Plaintext connection settings typed by the operator in the settings
/// dialog. The plaintext lives in the webview's JS heap while the operator
/// types - a documented, unavoidable window (AGENTS.md section 10); the Rust side
/// encrypts it before anything touches disk.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl ConnectionInput {
    /// Rejects obviously unusable input before any network I/O.
    fn validate(&self) -> Result<(), CommandError> {
        if self.host.trim().is_empty() {
            return Err(CommandError::validation("กรุณากรอกที่อยู่เซิร์ฟเวอร์"));
        }
        if self.port == 0 {
            return Err(CommandError::validation("พอร์ตต้องอยู่ระหว่าง 1-65535"));
        }
        if self.database.trim().is_empty() {
            return Err(CommandError::validation("กรุณากรอกชื่อฐานข้อมูล"));
        }
        if self.user.trim().is_empty() {
            return Err(CommandError::validation("กรุณากรอกชื่อผู้ใช้"));
        }
        Ok(())
    }

    /// Moves the typed values into a [`HosxConfig`]; the password string is
    /// moved (not cloned) into the secret box.
    fn into_config(self) -> HosxConfig {
        let ConnectionInput {
            host,
            port,
            database,
            user,
            password,
        } = self;
        HosxConfig::new(
            host.trim().to_string(),
            port,
            database.trim().to_string(),
            user.trim().to_string(),
            SecretBox::from(password),
        )
    }
}

/// Result of the backend's `SELECT 1` smoke test.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
}

/// Whether encrypted HOSxP connection settings already exist on this
/// machine.
#[tauri::command]
pub async fn connection_status(state: State<'_, AppState>) -> Result<bool, CommandError> {
    Ok(state.config_path().exists())
}

/// Live reachability of HOSxP - the status-dot source.
#[tauri::command]
pub async fn connection_health(
    state: State<'_, AppState>,
) -> Result<ConnectionHealth, CommandError> {
    Ok(*state.health.lock().await)
}

/// Encrypts and saves HOSxP connection settings, drops the old pool, and
/// re-probes in the background.
#[tauri::command]
pub async fn configure_connection(
    input: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    input.validate()?;
    let config = input.into_config();

    let vault = load_vault().map_err(CommandError::from)?;
    er_fill_hosxp_connector::config::save_encrypted(&state.config_path(), &vault, &config)
        .await
        .map_err(CommandError::from)?;

    *state.pool.lock().await = None;
    let probe_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let health = probe_health(&probe_state).await;
        *probe_state.health.lock().await = health;
    });
    Ok(())
}

/// Runs the `SELECT 1` smoke test with the **typed** settings (not the saved
/// ones); keeps the pool on success.
#[tauri::command]
pub async fn test_connection(
    input: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, CommandError> {
    input.validate()?;
    let config = input.into_config();
    let started = Instant::now();

    let pool = match connect(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            *state.health.lock().await = ConnectionHealth::Disconnected;
            return Err(err.into());
        }
    };
    match ConnectorRepository::new(pool.clone()).ping().await {
        Ok(()) => {
            *state.pool.lock().await = Some(pool);
            *state.health.lock().await = ConnectionHealth::Connected;
            Ok(ConnectionTestResult {
                latency_ms: started.elapsed().as_millis() as u64,
            })
        }
        Err(err) => {
            note_query_outcome(&state, &err).await;
            Err(err.into())
        }
    }
}

/// Returns the schedule + catalog + report header.
#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, CommandError> {
    Ok(state.settings.lock().await.clone())
}

/// Persists new business hours + requisition cutoff.
#[tauri::command]
pub async fn save_schedule(
    schedule: BusinessSchedule,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    validate_schedule(&schedule)?;
    update_settings(&state, |settings| settings.schedule = schedule).await
}

/// Persists the ER drug catalog (categories + drugs).
#[tauri::command]
pub async fn save_catalog(
    catalog: Catalog,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    update_settings(&state, |settings| settings.catalog = catalog).await
}

/// Persists the printed header block.
#[tauri::command]
pub async fn save_report(
    report: ReportMeta,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    if report.hospital_name.trim().is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อโรงพยาบาล"));
    }
    if report.document_title.trim().is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อเอกสาร"));
    }
    update_settings(&state, |settings| settings.report = report).await
}

/// Drug autocomplete from HOSxP `drugitems`.
#[tauri::command]
pub async fn search_drugs(
    term: String,
    state: State<'_, AppState>,
) -> Result<Vec<DrugItem>, CommandError> {
    let term = term.trim();
    let char_count = term.chars().count();
    if char_count < SEARCH_TERM_MIN_CHARS {
        return Err(CommandError::validation("พิมพ์อย่างน้อย 2 ตัวอักษรเพื่อค้นหายา"));
    }
    if char_count > SEARCH_TERM_MAX_CHARS {
        return Err(CommandError::validation("คำค้นหายาวเกินไป"));
    }

    let repository = acquire_repository(&state).await?;
    match repository.search_drugs(term).await {
        Ok(hits) => Ok(hits),
        Err(err) => {
            note_query_outcome(&state, &err).await;
            Err(err.into())
        }
    }
}

/// Summed usage of every configured catalog drug inside `window`.
#[tauri::command]
pub async fn fetch_usage(
    window: RequisitionWindow,
    state: State<'_, AppState>,
) -> Result<Vec<UsageRow>, CommandError> {
    validate_window(&window)?;
    let icodes = state.settings.lock().await.catalog.icodes();
    if icodes.is_empty() {
        return Ok(Vec::new());
    }

    let repository = acquire_repository(&state).await?;
    match repository.fetch_usage(&icodes, &window).await {
        Ok(rows) => {
            *state.health.lock().await = ConnectionHealth::Connected;
            Ok(rows)
        }
        Err(err) => {
            note_query_outcome(&state, &err).await;
            Err(err.into())
        }
    }
}

/// Warms the pool at startup, then refreshes health every 30 s for the
/// app's lifetime. Feeds `connection_health` (the status dot).
pub async fn run_health_monitor(state: AppState) {
    loop {
        let health = probe_health(&state).await;
        *state.health.lock().await = health;
        tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
    }
}

/// One health probe: unconfigured → disconnected → connected.
async fn probe_health(state: &AppState) -> ConnectionHealth {
    if !state.config_path().exists() {
        return ConnectionHealth::Unconfigured;
    }
    let Ok(vault) = load_vault() else {
        return ConnectionHealth::Disconnected;
    };
    let Ok(Some(config)) =
        er_fill_hosxp_connector::config::load(&state.config_path(), &vault).await
    else {
        return ConnectionHealth::Disconnected;
    };
    match ensure_pool(state, &config).await {
        Ok(pool) => match ConnectorRepository::new(pool).ping().await {
            Ok(()) => ConnectionHealth::Connected,
            Err(_) => {
                // Drop the pool so the next probe reconnects cleanly.
                *state.pool.lock().await = None;
                ConnectionHealth::Disconnected
            }
        },
        Err(_) => ConnectionHealth::Disconnected,
    }
}

/// Reuses the cached pool or opens a new one.
async fn ensure_pool(
    state: &AppState,
    config: &HosxConfig,
) -> Result<MySqlPool, er_fill_hosxp_connector::Error> {
    let mut guard = state.pool.lock().await;
    if let Some(pool) = guard.as_ref() {
        return Ok(pool.clone());
    }
    let pool = connect(config).await?;
    *guard = Some(pool.clone());
    Ok(pool)
}

/// Builds a repository for a command, or fails with the right Thai error.
async fn acquire_repository(state: &AppState) -> Result<ConnectorRepository, CommandError> {
    if !state.config_path().exists() {
        return Err(CommandError::not_configured());
    }
    let vault = load_vault().map_err(CommandError::from)?;
    let config = er_fill_hosxp_connector::config::load(&state.config_path(), &vault)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(CommandError::not_configured)?;
    let pool = ensure_pool(state, &config)
        .await
        .map_err(CommandError::from)?;
    Ok(ConnectorRepository::new(pool))
}

/// Updates the in-memory settings only after the file write succeeds.
async fn update_settings(
    state: &AppState,
    mutate: impl FnOnce(&mut AppSettings),
) -> Result<(), CommandError> {
    let mut guard = state.settings.lock().await;
    let mut updated = guard.clone();
    mutate(&mut updated);
    settings::save(&state.settings_path(), &updated)
        .await
        .map_err(CommandError::from)?;
    *guard = updated;
    Ok(())
}

/// A failed query may mean the connection died - reflect that in the health
/// state so the status dot and banner react without waiting for the poll.
async fn note_query_outcome(state: &AppState, err: &RepositoryError) {
    if matches!(err, RepositoryError::Connection) {
        *state.pool.lock().await = None;
        *state.health.lock().await = ConnectionHealth::Disconnected;
    }
}

/// Rejects reversed weekday/weekend ranges before persisting.
fn validate_schedule(schedule: &BusinessSchedule) -> Result<(), CommandError> {
    if schedule.weekday.start >= schedule.weekday.end {
        return Err(CommandError::validation(
            "เวลาทำการวันจันทร์-ศุกร์: เวลาเริ่มต้องก่อนเวลาเลิก",
        ));
    }
    if schedule.weekend.start >= schedule.weekend.end {
        return Err(CommandError::validation(
            "เวลาทำการเสาร์-อาทิตย์: เวลาเริ่มต้องก่อนเวลาเลิก",
        ));
    }
    Ok(())
}

/// Rejects a reversed requisition window before touching the database.
fn validate_window(window: &RequisitionWindow) -> Result<(), CommandError> {
    RequisitionWindow::new(window.start, window.end)
        .map_err(|_| CommandError::validation("ช่วงเวลาเบิก: เวลาเริ่มต้องก่อนเวลาสิ้นสุด"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(host: &str, port: u16, database: &str, user: &str) -> ConnectionInput {
        ConnectionInput {
            host: host.to_string(),
            port,
            database: database.to_string(),
            user: user.to_string(),
            password: "pw".to_string(),
        }
    }

    #[test]
    fn connection_input_accepts_valid_and_rejects_blank_fields() {
        assert!(
            input("10.0.0.1", 3306, "hos", "erfill_ro")
                .validate()
                .is_ok()
        );
        assert!(input("   ", 3306, "hos", "erfill_ro").validate().is_err());
        assert!(input("10.0.0.1", 0, "hos", "erfill_ro").validate().is_err());
        assert!(
            input("10.0.0.1", 3306, "  ", "erfill_ro")
                .validate()
                .is_err()
        );
        assert!(input("10.0.0.1", 3306, "hos", "  ").validate().is_err());
    }

    #[test]
    fn into_config_trims_every_field() {
        let config = input(" 10.0.0.1 ", 3306, " hos ", " erfill_ro ").into_config();
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.database, "hos");
        assert_eq!(config.user, "erfill_ro");
    }

    #[test]
    fn validate_schedule_rejects_reversed_ranges() -> Result<(), Box<dyn std::error::Error>> {
        let mut schedule = BusinessSchedule::default();
        schedule.weekday.start = chrono::NaiveTime::from_hms_opt(17, 0, 0).ok_or("valid")?;
        assert!(validate_schedule(&schedule).is_err());

        let mut schedule = BusinessSchedule::default();
        schedule.weekend.end = chrono::NaiveTime::from_hms_opt(7, 0, 0).ok_or("valid")?;
        assert!(validate_schedule(&schedule).is_err());

        assert!(validate_schedule(&BusinessSchedule::default()).is_ok());
        Ok(())
    }

    #[test]
    fn validate_window_rejects_reversed() -> Result<(), Box<dyn std::error::Error>> {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
            .and_then(|date| date.and_hms_opt(16, 0, 0))
            .ok_or("valid")?;
        let end = chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
            .and_then(|date| date.and_hms_opt(8, 0, 0))
            .ok_or("valid")?;
        let valid = RequisitionWindow { start, end };
        assert!(validate_window(&valid).is_ok());
        let reversed = RequisitionWindow {
            start: end,
            end: start,
        };
        assert!(validate_window(&reversed).is_err());
        Ok(())
    }
}
