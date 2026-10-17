//! Tauri-managed application state.

use std::sync::Arc;

use er_fill_hosxp_connector::MySqlPool;
use er_fill_hosxp_connector::config::CONFIG_FILE_NAME;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::settings::{AppSettings, SETTINGS_FILE_NAME};

/// Live reachability of HOSxP. Driven by the startup warm-up, the periodic
/// health monitor, and query outcomes - never by "the config file exists".
/// This is the value behind the top-bar connection dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConnectionHealth {
    /// No stored settings - the settings dialog is the flow.
    Unconfigured,
    /// A ping succeeded recently (or a query succeeded).
    Connected,
    /// A ping failed, or a query could not reach the database.
    Disconnected,
}

/// Shared state for the command adapters.
///
/// `Arc<Mutex<...>>` (not `Mutex<...>` directly): `tokio::sync::Mutex` has no
/// `Clone` impl, and both the startup/monitor tasks and command closures
/// need cheap owned handles into the same state.
#[derive(Clone)]
pub struct AppState {
    /// Directory where the encrypted config and plain settings files live
    /// (app config dir).
    pub config_dir: std::path::PathBuf,
    /// Reusable read-only pool, created by the first successful connection
    /// test (or by the startup warm-up). `None` until then (and after
    /// reconfiguration).
    pub pool: Arc<Mutex<Option<MySqlPool>>>,
    /// Live reachability of HOSxP, kept fresh by the health monitor and
    /// every query outcome.
    pub health: Arc<Mutex<ConnectionHealth>>,
    /// Schedule + catalog + report header, mirrored from the settings file.
    pub settings: Arc<Mutex<AppSettings>>,
}

impl AppState {
    /// Creates the state with settings loaded at startup.
    pub fn new(config_dir: std::path::PathBuf, settings: AppSettings) -> Self {
        Self {
            config_dir,
            pool: Arc::new(Mutex::new(None)),
            health: Arc::new(Mutex::new(ConnectionHealth::Unconfigured)),
            settings: Arc::new(Mutex::new(settings)),
        }
    }

    /// Full path of the encrypted connection-config file.
    pub fn config_path(&self) -> std::path::PathBuf {
        self.config_dir.join(CONFIG_FILE_NAME)
    }

    /// Full path of the plain settings file.
    pub fn settings_path(&self) -> std::path::PathBuf {
        self.config_dir.join(SETTINGS_FILE_NAME)
    }
}
