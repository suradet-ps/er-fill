//! The complete set of operator-configurable settings: catalog and report
//! header - shared by the backend (persistence) and the frontend (editing).
//! The run date and time range are deliberately **not** configurable:
//! the app always uses today and one of the four fixed overnight ranges.
//! Credentials are not here either; they live in the encrypted config file
//! handled by `er-refill-hosxp-connector` (AGENTS.md section 10).

use serde::{Deserialize, Serialize};

use crate::catalog::Catalog;
use crate::report::ReportMeta;

/// Everything the operator configures besides the connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// The ER drug list (categories + drugs).
    pub catalog: Catalog,
    /// Printed header block.
    pub report: ReportMeta,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            catalog: Catalog::seed_default(),
            report: ReportMeta::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seeds_the_six_categories() {
        let settings = AppSettings::default();
        assert_eq!(settings.catalog.categories.len(), 6);
        assert!(settings.catalog.is_empty());
        assert_eq!(settings.report, ReportMeta::default());
    }

    #[test]
    fn serde_round_trip_and_missing_fields_default() -> Result<(), Box<dyn std::error::Error>> {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings)?;
        let back: AppSettings = serde_json::from_str(&json)?;
        assert_eq!(back, settings);

        let partial: AppSettings = serde_json::from_str("{}")?;
        assert_eq!(partial, AppSettings::default());
        Ok(())
    }

    #[test]
    fn legacy_schedule_field_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "schedule": {
                "weekday": { "start": "08:30", "end": "20:30" },
                "weekend": { "start": "08:00", "end": "16:00" }
            }
        }"#;
        let settings: AppSettings = serde_json::from_str(json)?;
        assert_eq!(settings, AppSettings::default());
        Ok(())
    }
}
