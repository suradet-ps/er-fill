//! The repository contract between business logic and the HOSxP connector.
//!
//! `er-fill-hosxp-connector` implements [`HosxRepository`]; everything else
//! depends on this trait only, which keeps the default test suite DB-free
//! (AGENTS.md sections 3 and 13).

use async_trait::async_trait;
use er_fill_models::{DrugItem, RequisitionWindow, UsageRow};

/// Errors crossing the repository boundary. English and dev-facing - the
/// Thai user-facing translation happens only in `src-tauri/commands.rs`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// No connection settings stored yet.
    #[error("HOSxP connection is not configured")]
    NotConfigured,
    /// The database cannot be reached (network, credentials, server down).
    #[error("cannot reach the HOSxP database")]
    Connection,
    /// A statement failed on the server. The payload is the driver message
    /// - never parameter values (AGENTS.md section 2).
    #[error("query failed: {0}")]
    Query(String),
    /// The read-only guard rejected a statement (defense in depth).
    #[error("read-only guard rejected a statement")]
    Guard,
}

/// Read-only access to the HOSxP data ER-Fill needs.
#[async_trait]
pub trait HosxRepository: Send + Sync {
    /// `SELECT 1` smoke test against the configured database.
    ///
    /// # Errors
    ///
    /// [`RepositoryError::Connection`] when the database is unreachable.
    async fn ping(&self) -> Result<(), RepositoryError>;

    /// Contains-match search over `drugitems` by icode or name, at most 50
    /// hits, ordered by name.
    ///
    /// # Errors
    ///
    /// [`RepositoryError::Connection`] / [`RepositoryError::Query`].
    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError>;

    /// Summed usage (`SUM(qty)`) of each icode inside `window`, one row per
    /// icode with usage; icodes without usage are omitted.
    ///
    /// # Errors
    ///
    /// [`RepositoryError::Connection`] / [`RepositoryError::Query`], and
    /// [`RepositoryError::Guard`] when defensive layers reject the statement.
    async fn fetch_usage(
        &self,
        icodes: &[String],
        window: &RequisitionWindow,
    ) -> Result<Vec<UsageRow>, RepositoryError>;
}
