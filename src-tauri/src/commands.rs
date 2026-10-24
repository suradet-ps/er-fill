//! Tauri command adapters (AGENTS.md section 3): validate → delegate → translate.
//!
//! No SQL and no business logic live here - everything HOSxP-related goes
//! through `er-refill-hosxp-connector`, and this file is the boundary where
//! errors become Thai ([`crate::error::CommandError`]).

use std::path::Path;
use std::time::{Duration, Instant};

use er_refill_core::{HosxRepository, RepositoryError, verify_password};
use er_refill_hosxp_connector::config::{HosxConfig, load_vault};
use er_refill_hosxp_connector::{ConnectorRepository, MySqlPool, connect as connect_hosxp};
use er_refill_models::{
    Catalog, ConnectionSummary, DrugItem, ReportMeta, RequisitionWindow, SessionUser, UsageRow,
};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

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

/// Defensive cap on the exported settings payload (5 MiB).
const EXPORT_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Settings export payload produced by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettingsInput {
    /// The serialized export envelope.
    pub json: String,
    /// Suggested file name for the native Save As dialog.
    pub file_name: String,
}

/// Outcome of the native Save As flow.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettingsResult {
    /// False when the operator cancelled the dialog.
    pub saved: bool,
    /// Full path written when `saved` is true.
    pub path: Option<String>,
}

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

/// Username/password submitted from the login screen. Deliberately does not
/// derive `Debug` - the password must never be formatted into a log line.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    /// HOSxP `officer.officer_login_name`.
    pub username: String,
    /// Plaintext password, verified in memory against the stored MD5.
    pub password: String,
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
    require_session(&state).await?;
    input.validate()?;
    let config = input.into_config();

    let vault = load_vault().map_err(CommandError::from)?;
    er_refill_hosxp_connector::config::save_encrypted(&state.config_path(), &vault, &config)
        .await
        .map_err(CommandError::from)?;

    *state.pool.lock().await = None;
    *state.session.lock().await = None;
    let probe_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let health = probe_health(&probe_state).await;
        *probe_state.health.lock().await = health;
    });
    Ok(())
}

/// Returns the non-secret connection fields for the login screen, if a
/// config is stored.
#[tauri::command]
pub async fn connection_summary(
    state: State<'_, AppState>,
) -> Result<Option<ConnectionSummary>, CommandError> {
    if !state.config_path().exists() {
        return Ok(None);
    }
    let vault = load_vault().map_err(CommandError::from)?;
    let config = er_refill_hosxp_connector::config::load(&state.config_path(), &vault)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(CommandError::not_configured)?;
    Ok(Some(ConnectionSummary {
        host: config.host,
        port: config.port,
        database: config.database,
        user: config.user,
    }))
}

/// Tests typed connection settings, saves them only after a successful
/// `SELECT 1`, and keeps the resulting pool. Used by the settings dialog's
/// connection panel, including before sign-in.
#[tauri::command]
pub async fn connect(
    input: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, CommandError> {
    input.validate()?;
    let config = input.into_config();
    let started = Instant::now();
    let pool = match connect_hosxp(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            *state.health.lock().await = ConnectionHealth::Disconnected;
            return Err(err.into());
        }
    };
    if let Err(err) = ConnectorRepository::new(pool.clone()).ping().await {
        *state.pool.lock().await = None;
        *state.health.lock().await = ConnectionHealth::Disconnected;
        return Err(err.into());
    }

    let vault = load_vault().map_err(CommandError::from)?;
    er_refill_hosxp_connector::config::save_encrypted(&state.config_path(), &vault, &config)
        .await
        .map_err(CommandError::from)?;
    *state.pool.lock().await = Some(pool);
    *state.health.lock().await = ConnectionHealth::Connected;
    *state.session.lock().await = None;
    Ok(ConnectionTestResult {
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

/// Re-tests the saved connection (encrypted config) and keeps the pool.
#[tauri::command]
pub async fn reconnect(state: State<'_, AppState>) -> Result<ConnectionTestResult, CommandError> {
    let started = Instant::now();
    let repository = acquire_repository(&state).await?;
    if let Err(err) = repository.ping().await {
        *state.pool.lock().await = None;
        *state.health.lock().await = ConnectionHealth::Disconnected;
        return Err(err.into());
    }
    *state.health.lock().await = ConnectionHealth::Connected;
    Ok(ConnectionTestResult {
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

/// Verifies a HOSxP officer password and starts an in-memory session.
///
/// The plaintext password is moved into a [`SecretString`] for the
/// verification window; the stored hash never leaves this process.
#[tauri::command]
pub async fn login(
    input: LoginInput,
    state: State<'_, AppState>,
) -> Result<SessionUser, CommandError> {
    let username = input.username.trim();
    if username.is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อผู้ใช้"));
    }
    if username.chars().count() > 100 {
        return Err(CommandError::validation("ชื่อผู้ใช้ยาวเกินไป"));
    }
    let password = SecretString::from(input.password);
    if password.expose_secret().is_empty() {
        return Err(CommandError::validation("กรุณากรอกรหัสผ่าน"));
    }
    if password.expose_secret().chars().count() > 200 {
        return Err(CommandError::validation("รหัสผ่านยาวเกินไป"));
    }

    *state.session.lock().await = None;
    let repository = acquire_repository(&state).await?;
    let account = repository.fetch_officer(username).await?;
    let Some(account) = account else {
        return Err(CommandError::auth("ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง"));
    };
    if account.auth_type_id != 1 {
        return Err(CommandError::auth(
            "บัญชีนี้ใช้การยืนยันตัวตนแบบ LDAP ซึ่งยังไม่เปิดใช้งาน",
        ));
    }
    if !account.active.eq_ignore_ascii_case("Y") {
        return Err(CommandError::auth("บัญชีนี้ถูกปิดใช้งาน กรุณาติดต่อผู้ดูแลระบบ"));
    }
    if !verify_password(password.expose_secret(), &account.password_md5) {
        return Err(CommandError::auth("ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง"));
    }

    let display_name = if account.display_name.trim().is_empty() {
        account.login_name.clone()
    } else {
        account.display_name
    };
    let user = SessionUser {
        officer_id: account.officer_id,
        login_name: account.login_name,
        display_name,
        doctor_code: account.doctor_code,
        role: "STAFF".to_string(),
    };
    *state.session.lock().await = Some(user.clone());
    Ok(user)
}

/// Clears the in-memory session.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), CommandError> {
    *state.session.lock().await = None;
    Ok(())
}

/// Returns the current session, if any (memory only).
#[tauri::command]
pub async fn current_session(
    state: State<'_, AppState>,
) -> Result<Option<SessionUser>, CommandError> {
    Ok(state.session.lock().await.clone())
}

/// Runs the `SELECT 1` smoke test with the **typed** settings (not the saved
/// ones). The probe pool is dropped afterwards; only `connect`/`reconnect`
/// may replace the app's active pool.
#[tauri::command]
pub async fn test_connection(input: ConnectionInput) -> Result<ConnectionTestResult, CommandError> {
    input.validate()?;
    let config = input.into_config();
    let started = Instant::now();
    let pool = connect_hosxp(&config).await.map_err(CommandError::from)?;
    ConnectorRepository::new(pool)
        .ping()
        .await
        .map_err(CommandError::from)?;
    Ok(ConnectionTestResult {
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

/// Returns the catalog + report header.
#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, CommandError> {
    Ok(state.settings.lock().await.clone())
}

/// Persists the ER drug catalog (categories + drugs).
#[tauri::command]
pub async fn save_catalog(
    catalog: Catalog,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    require_session(&state).await?;
    update_settings(&state, |settings| settings.catalog = catalog).await
}

/// Persists the printed header block.
#[tauri::command]
pub async fn save_report(
    report: ReportMeta,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    require_session(&state).await?;
    if report.hospital_name.trim().is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อโรงพยาบาล"));
    }
    if report.document_title.trim().is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อเอกสาร"));
    }
    update_settings(&state, |settings| settings.report = report).await
}

/// Writes the settings export to a path chosen in the native Save As dialog.
///
/// The webview supplies the already-serialized export; the backend validates
/// its envelope and size before opening the dialog, then writes the exact
/// bytes to the chosen path. Frontend filesystem scopes stay closed.
#[tauri::command]
pub async fn export_settings_file(
    input: ExportSettingsInput,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<ExportSettingsResult, CommandError> {
    require_session(&state).await?;
    validate_export_payload(&input.json)?;

    let Some(picked) = app
        .dialog()
        .file()
        .set_title("ส่งออกการตั้งค่า")
        .set_file_name(sanitize_export_file_name(&input.file_name))
        .add_filter("ER-Refill settings", &["json"])
        .set_parent(&window)
        .blocking_save_file()
    else {
        return Ok(ExportSettingsResult {
            saved: false,
            path: None,
        });
    };

    let path = picked
        .into_path()
        .map_err(|_| CommandError::storage("invalid export path"))?;
    tokio::fs::write(&path, input.json.as_bytes())
        .await
        .map_err(|err| CommandError::storage(err.to_string()))?;
    Ok(ExportSettingsResult {
        saved: true,
        path: Some(path.display().to_string()),
    })
}

/// Drug autocomplete from HOSxP `drugitems`.
#[tauri::command]
pub async fn search_drugs(
    term: String,
    state: State<'_, AppState>,
) -> Result<Vec<DrugItem>, CommandError> {
    require_session(&state).await?;
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
    require_session(&state).await?;
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
        er_refill_hosxp_connector::config::load(&state.config_path(), &vault).await
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
) -> Result<MySqlPool, er_refill_hosxp_connector::Error> {
    let mut guard = state.pool.lock().await;
    if let Some(pool) = guard.as_ref() {
        return Ok(pool.clone());
    }
    let pool = connect_hosxp(config).await?;
    *guard = Some(pool.clone());
    Ok(pool)
}

/// Rejects a command that requires a signed-in operator.
async fn require_session(state: &AppState) -> Result<SessionUser, CommandError> {
    state
        .session
        .lock()
        .await
        .clone()
        .ok_or_else(CommandError::unauthorized)
}

/// Builds a repository for a command, or fails with the right Thai error.
async fn acquire_repository(state: &AppState) -> Result<ConnectorRepository, CommandError> {
    if !state.config_path().exists() {
        return Err(CommandError::not_configured());
    }
    let vault = load_vault().map_err(CommandError::from)?;
    let config = er_refill_hosxp_connector::config::load(&state.config_path(), &vault)
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

/// Accepts only a bounded JSON envelope produced by the settings exporter.
fn validate_export_payload(json: &str) -> Result<(), CommandError> {
    if json.len() > EXPORT_MAX_BYTES {
        return Err(CommandError::validation("ไฟล์ส่งออกใหญ่เกินไป"));
    }
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| CommandError::validation("ข้อมูลส่งออกไม่ถูกต้อง"))?;
    if value.get("format").and_then(serde_json::Value::as_str) != Some("er-refill-settings") {
        return Err(CommandError::validation("ข้อมูลส่งออกไม่ถูกต้อง"));
    }
    Ok(())
}

/// Keeps only the file name and guarantees a `.json` extension.
fn sanitize_export_file_name(raw: &str) -> String {
    let name = Path::new(raw)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("er-refill-settings.json");
    if name.to_ascii_lowercase().ends_with(".json") {
        name.to_string()
    } else {
        format!("{name}.json")
    }
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
    use crate::error::CommandErrorKind;

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
            input("10.0.0.1", 3306, "hos", "errefill_ro")
                .validate()
                .is_ok()
        );
        assert!(input("   ", 3306, "hos", "errefill_ro").validate().is_err());
        assert!(
            input("10.0.0.1", 0, "hos", "errefill_ro")
                .validate()
                .is_err()
        );
        assert!(
            input("10.0.0.1", 3306, "  ", "errefill_ro")
                .validate()
                .is_err()
        );
        assert!(input("10.0.0.1", 3306, "hos", "  ").validate().is_err());
    }

    #[test]
    fn into_config_trims_every_field() {
        let config = input(" 10.0.0.1 ", 3306, " hos ", " errefill_ro ").into_config();
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.database, "hos");
        assert_eq!(config.user, "errefill_ro");
    }

    #[tokio::test]
    async fn require_session_rejects_without_a_session() {
        let dir = tempfile::tempdir().expect("tempdir in test");
        let state = AppState::new(dir.path().to_path_buf(), AppSettings::default());
        let error = require_session(&state).await.expect_err("must reject");
        assert_eq!(error.kind, CommandErrorKind::Auth);

        let user = SessionUser {
            officer_id: 7,
            login_name: "pharm".into(),
            display_name: "Pharmacist".into(),
            doctor_code: None,
            role: "STAFF".into(),
        };
        *state.session.lock().await = Some(user.clone());
        assert_eq!(require_session(&state).await.expect("session"), user);
    }

    #[test]
    fn sanitize_export_file_name_strips_paths_and_appends_json() {
        assert_eq!(sanitize_export_file_name("../../evil.txt"), "evil.txt.json");
        assert_eq!(
            sanitize_export_file_name("er-refill-settings-20261024.json"),
            "er-refill-settings-20261024.json"
        );
        assert_eq!(sanitize_export_file_name("   "), "er-refill-settings.json");
    }

    #[test]
    fn validate_export_payload_accepts_only_the_export_envelope() {
        assert!(validate_export_payload(r#"{"format":"er-refill-settings"}"#).is_ok());
        assert!(validate_export_payload(r#"{"format":"other"}"#).is_err());
        assert!(validate_export_payload("not json").is_err());
    }

    #[test]
    fn validate_window_rejects_reversed() -> Result<(), Box<dyn std::error::Error>> {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
            .and_then(|date| date.and_hms_opt(16, 0, 0))
            .ok_or("valid")?;
        let end = chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
            .and_then(|date| date.and_hms_opt(8, 30, 0))
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
