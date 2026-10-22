//! In-memory [`MockRepository`] - the default test suite runs against this,
//! never a real database (AGENTS.md section 13).

use std::collections::HashMap;

use async_trait::async_trait;
use er_refill_models::{DrugItem, OfficerAccount, RequisitionWindow, UsageRow};

use crate::repository::{HosxRepository, RepositoryError};

/// Scripted repository: canned drugs and usage amounts, plus an optional
/// forced error so failure paths are testable without a server.
#[derive(Debug, Clone, Default)]
pub struct MockRepository {
    /// Hits returned by [`HosxRepository::search_drugs`] (substring match on
    /// name or icode, mirroring the intent of the live query).
    pub drugs: Vec<DrugItem>,
    /// Usage by icode returned by [`HosxRepository::fetch_usage`].
    pub usage: HashMap<String, f64>,
    /// Officer accounts by login name returned by
    /// [`HosxRepository::fetch_officer`].
    pub officers: HashMap<String, OfficerAccount>,
    /// When set, every call fails with this error.
    pub fail: Option<RepositoryError>,
}

impl MockRepository {
    fn check_fail(&self) -> Result<(), RepositoryError> {
        match &self.fail {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl HosxRepository for MockRepository {
    async fn ping(&self) -> Result<(), RepositoryError> {
        self.check_fail()
    }

    async fn fetch_officer(
        &self,
        login_name: &str,
    ) -> Result<Option<OfficerAccount>, RepositoryError> {
        self.check_fail()?;
        Ok(self.officers.get(login_name).cloned())
    }

    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError> {
        self.check_fail()?;
        Ok(self
            .drugs
            .iter()
            .filter(|drug| drug.name.contains(term) || drug.icode.contains(term))
            .cloned()
            .collect())
    }

    async fn fetch_usage(
        &self,
        icodes: &[String],
        _window: &RequisitionWindow,
    ) -> Result<Vec<UsageRow>, RepositoryError> {
        self.check_fail()?;
        Ok(icodes
            .iter()
            .filter_map(|icode| {
                self.usage.get(icode).map(|qty| UsageRow {
                    icode: icode.clone(),
                    qty: *qty,
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use er_refill_models::RequisitionWindow;

    fn window() -> Result<RequisitionWindow, Box<dyn std::error::Error>> {
        let start = NaiveDate::from_ymd_opt(2026, 10, 18)
            .and_then(|date| date.and_hms_opt(16, 0, 0))
            .ok_or("valid start")?;
        let end = NaiveDate::from_ymd_opt(2026, 10, 19)
            .and_then(|date| date.and_hms_opt(8, 0, 0))
            .ok_or("valid end")?;
        Ok(RequisitionWindow::new(start, end)?)
    }

    fn sample() -> MockRepository {
        let mut usage = HashMap::new();
        usage.insert("1001".to_string(), 6.0);
        MockRepository {
            drugs: vec![
                DrugItem {
                    icode: "1001".into(),
                    name: "Paracetamol".into(),
                    strength: "500".into(),
                    units: "mg".into(),
                },
                DrugItem {
                    icode: "1002".into(),
                    name: "CPM".into(),
                    strength: "4".into(),
                    units: "mg".into(),
                },
            ],
            usage,
            officers: HashMap::new(),
            fail: None,
        }
    }

    #[tokio::test]
    async fn search_drugs_matches_name_or_icode() -> Result<(), Box<dyn std::error::Error>> {
        let repo = sample();
        // The mock matches case-sensitively (the live SQL relies on MySQL's
        // case-insensitive collation instead).
        let hits = repo.search_drugs("Para").await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].icode, "1001");
        assert_eq!(repo.search_drugs("1002").await?.len(), 1);
        assert_eq!(repo.search_drugs("zzz").await?.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn fetch_usage_returns_only_known_icodes() -> Result<(), Box<dyn std::error::Error>> {
        let repo = sample();
        let usage = repo
            .fetch_usage(&["1001".to_string(), "9999".to_string()], &window()?)
            .await?;
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].qty, 6.0);
        Ok(())
    }

    #[tokio::test]
    async fn fetch_officer_returns_the_scripted_account() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut repo = sample();
        let account = OfficerAccount {
            officer_id: 7,
            login_name: "pharm".into(),
            password_md5: "5f4dcc3b5aa765d61d8327deb882cf99".into(),
            active: "Y".into(),
            auth_type_id: 1,
            doctor_code: None,
            display_name: "Pharmacist".into(),
        };
        repo.officers.insert("pharm".into(), account.clone());
        assert_eq!(repo.fetch_officer("pharm").await?, Some(account));
        assert_eq!(repo.fetch_officer("missing").await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn forced_failure_propagates() -> Result<(), Box<dyn std::error::Error>> {
        let repo = MockRepository {
            fail: Some(RepositoryError::Connection),
            ..MockRepository::default()
        };
        assert_eq!(repo.ping().await, Err(RepositoryError::Connection));
        assert!(repo.fetch_officer("x").await.is_err());
        assert!(repo.search_drugs("x").await.is_err());
        assert!(repo.fetch_usage(&[], &window()?).await.is_err());
        Ok(())
    }
}
