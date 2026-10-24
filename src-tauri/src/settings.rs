//! Plain (unencrypted) application settings persistence: catalog and the
//! report header (AGENTS.md section 10). The [`AppSettings`] type itself lives
//! in `er-refill-models` (shared with the frontend); these are **not**
//! secrets - credentials live in the encrypted config file handled by
//! `er-refill-hosxp-connector`.

use std::path::Path;

pub use er_refill_models::AppSettings;
use serde::{Deserialize, Serialize};

/// File name of the settings file, inside the app config dir.
pub const SETTINGS_FILE_NAME: &str = "er-refill.settings.json";

/// Version of the on-disk settings format. Bump (with migration) if the
/// layout changes.
const SETTINGS_VERSION: u32 = 3;

/// Versions still readable. Their extra `schedule` field is ignored by serde:
/// the run date and time range are no longer configurable.
const LEGACY_SETTINGS_VERSIONS: [u32; 2] = [1, 2];

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
/// A missing file is **not** an error: the defaults (seeded catalog and
/// default report header) are returned - the first save writes the file.
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
    if file.version == SETTINGS_VERSION || LEGACY_SETTINGS_VERSIONS.contains(&file.version) {
        Ok(file.settings)
    } else {
        Err(SettingsError::Corrupt(format!(
            "unsupported settings version {}",
            file.version
        )))
    }
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
    use er_refill_models::ReportMeta;
    use tempfile::tempdir;

    #[tokio::test]
    async fn missing_file_returns_seeded_defaults() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = load(&path).await.expect("load succeeds");
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
    async fn legacy_schedule_file_still_loads() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        tokio::fs::write(
            &path,
            r#"{
                "version": 2,
                "schedule": {
                    "weekday": { "start": "08:30", "end": "20:30" },
                    "weekend": { "start": "08:00", "end": "16:00" }
                }
            }"#,
        )
        .await
        .expect("write in test");
        let settings = load(&path).await.expect("load succeeds");
        assert_eq!(settings, AppSettings::default());
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
