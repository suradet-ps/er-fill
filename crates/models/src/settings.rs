//! The complete set of operator-configurable settings: schedule, catalog,
//! and report header - shared by the backend (persistence) and the frontend
//! (editing). Credentials are deliberately **not** here; they live in the
//! encrypted config file handled by `er-fill-hosxp-connector` (AGENTS.md section 10).

use serde::{Deserialize, Serialize};

use crate::catalog::Catalog;
use crate::report::ReportMeta;
use crate::schedule::BusinessSchedule;

/// Everything the operator configures besides the connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Business hours + requisition cutoff.
    pub schedule: BusinessSchedule,
    /// The ER drug list (categories + drugs).
    pub catalog: Catalog,
    /// Printed header block.
    pub report: ReportMeta,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schedule: BusinessSchedule::default(),
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
        assert_eq!(settings.schedule, BusinessSchedule::default());
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
}
