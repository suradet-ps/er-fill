//! Plain (unencrypted) application settings persistence: schedule, catalog,
//! and the report header (AGENTS.md section 10). The [`AppSettings`] type itself
//! lives in `er-refill-models` (shared with the frontend); these are **not
//! secrets** - credentials live in the encrypted config file handled by
//! `er-refill-hosxp-connector`.

use std::path::Path;

use chrono::NaiveTime;
pub use er_refill_models::AppSettings;
use er_refill_models::{BusinessSchedule, TimeRange};
use serde::{Deserialize, Serialize};

/// File name of the settings file, inside the app config dir.
pub const SETTINGS_FILE_NAME: &str = "er-refill.settings.json";

/// Version of the on-disk settings format. Bump (with migration) if the
/// layout changes.
const SETTINGS_VERSION: u32 = 2;

/// Version 1 stored a `schedule.cutoff` and weekday hours ending at 16:30.
const PREVIOUS_SETTINGS_VERSION: u32 = 1;

/// Errors around reading and writing the settings file.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The file could not be read or written.
    #[error("settings file I/O failed: {0}")]
    Io(String),
    /// The file is not valid JSON or uses an unsupported version.
    #[error("settings file is corrupt: {0}")]
    Corrupt(String),
}

/// On-disk layout: a version tag next to the flattened settings.
#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    version: u32,
    #[serde(flatten)]
    settings: AppSettings,
}

/// Loads the settings file.
///
/// A missing file is **not** an error: the defaults (seeded catalog, default
/// schedule and report header) are returned - the first save writes the file.
///
/// # Errors
///
/// Returns [`SettingsError::Io`] for read failures and
/// [`SettingsError::Corrupt`] for invalid JSON or an unsupported version.
pub async fn load(path: &Path) -> Result<AppSettings, SettingsError> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| SettingsError::Io(e.to_string()))?;
    let file: SettingsFile =
        serde_json::from_str(&raw).map_err(|e| SettingsError::Corrupt(e.to_string()))?;
    match file.version {
        SETTINGS_VERSION => Ok(file.settings),
        PREVIOUS_SETTINGS_VERSION => Ok(migrate_v1(file.settings)),
        version => Err(SettingsError::Corrupt(format!(
            "unsupported settings version {version}"
        ))),
    }
}

/// Version 1 shipped weekday hours ending at 16:30. Upgrade only the exact
/// old default; an operator's custom schedule is preserved.
fn migrate_v1(mut settings: AppSettings) -> AppSettings {
    let old_default = BusinessSchedule {
        weekday: TimeRange {
            start: at(8, 30),
            end: at(16, 30),
        },
        weekend: TimeRange {
            start: at(8, 0),
            end: at(16, 0),
        },
    };
    if settings.schedule == old_default {
        settings.schedule = BusinessSchedule::default();
    }
    settings
}

/// Builds a [`NaiveTime`] from literal hours/minutes. The fallback is
/// unreachable for the compile-time-valid values used here; it keeps the
/// no-panic rule (AGENTS-RUST.md section 5.5).
fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN)
}

/// Writes the settings file (pretty JSON), creating parent directories.
///
/// # Errors
///
/// Returns [`SettingsError::Io`] on write failures and
/// [`SettingsError::Corrupt`] on serialization failures.
pub async fn save(path: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    let file = SettingsFile {
        version: SETTINGS_VERSION,
        settings: settings.clone(),
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| SettingsError::Corrupt(e.to_string()))?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| SettingsError::Io(e.to_string()))?;
    }
    tokio::fs::write(path, json)
        .await
        .map_err(|e| SettingsError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use er_refill_models::{BusinessSchedule, ReportMeta};
    use tempfile::tempdir;

    #[tokio::test]
    async fn missing_file_returns_seeded_defaults() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = load(&path).await.expect("load succeeds");
        assert_eq!(settings.schedule, BusinessSchedule::default());
        assert_eq!(settings.catalog.categories.len(), 6);
        assert_eq!(settings.report, ReportMeta::default());
    }

    #[tokio::test]
    async fn round_trip_preserves_settings() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let mut settings = AppSettings::default();
        settings.report.hospital_name = "รพ.ทดสอบ".into();
        save(&path, &settings).await.expect("save succeeds");
        let loaded = load(&path).await.expect("load succeeds");
        assert_eq!(loaded, settings);
    }

    #[tokio::test]
    async fn corrupt_file_is_rejected() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        tokio::fs::write(&path, "{ not json")
            .await
            .expect("write in test");
        assert!(matches!(load(&path).await, Err(SettingsError::Corrupt(_))));
    }

    #[tokio::test]
    async fn v1_legacy_default_schedule_is_upgraded() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        tokio::fs::write(
            &path,
            r#"{
                "version": 1,
                "schedule": {
                    "weekday": { "start": "08:30", "end": "16:30" },
                    "weekend": { "start": "08:00", "end": "16:00" },
                    "cutoff": "08:00"
                }
            }"#,
        )
        .await
        .expect("write in test");
        let settings = load(&path).await.expect("load succeeds");
        assert_eq!(settings.schedule, BusinessSchedule::default());
    }

    #[tokio::test]
    async fn v1_custom_schedule_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        tokio::fs::write(
            &path,
            r#"{
                "version": 1,
                "schedule": {
                    "weekday": { "start": "08:30", "end": "18:00" },
                    "weekend": { "start": "08:00", "end": "16:00" },
                    "cutoff": "08:00"
                }
            }"#,
        )
        .await
        .expect("write in test");
        let settings = load(&path).await.expect("load succeeds");
        assert_eq!(
            settings.schedule.weekday.end,
            chrono::NaiveTime::from_hms_opt(18, 0, 0).ok_or("valid")?
        );
        assert_eq!(
            settings.schedule.weekday.start,
            chrono::NaiveTime::from_hms_opt(8, 30, 0).ok_or("valid")?
        );
        Ok(())
    }

    #[tokio::test]
    async fn future_version_is_rejected() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        tokio::fs::write(&path, r#"{ "version": 99 }"#)
            .await
            .expect("write in test");
        assert!(matches!(load(&path).await, Err(SettingsError::Corrupt(_))));
    }

    #[tokio::test]
    async fn saved_file_contains_no_credentials_fields() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        save(&path, &AppSettings::default())
            .await
            .expect("save succeeds");
        let raw = tokio::fs::read_to_string(&path)
            .await
            .expect("read in test");
        for forbidden in ["password", "host", "user"] {
            assert!(
                !raw.contains(forbidden),
                "settings file must not carry connection fields ({forbidden:?})"
            );
        }
    }
}
