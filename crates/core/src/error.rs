//! Errors from catalog operations ([`crate::catalog_ops`]) and manual
//! requisition changes ([`crate::requisition`]).

/// A manual requisition change was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequisitionError {
    /// Only drugs configured in the catalog may be added to the sheet.
    #[error("drug {0} is not configured in the catalog")]
    DrugNotConfigured(String),
    /// The same icode cannot appear twice in the report.
    #[error("drug {0} is already in the requisition")]
    DrugAlreadyInReport(String),
    /// The row to edit no longer exists in the report.
    #[error("drug {0} was not found in the requisition")]
    RowNotFound(String),
    /// Every manual add or quantity change must explain itself.
    #[error("a reason is required for every manual change")]
    MissingReason,
    /// Requested quantities are whole, non-negative numbers.
    #[error("the requested quantity must be a non-negative whole number")]
    InvalidRequested,
    /// The configured drug points at a category that no longer exists.
    #[error("category {0} was not found")]
    CategoryNotFound(String),
}

/// A catalog mutation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogOpsError {
    /// A category or drug name was empty after trimming.
    #[error("name must not be empty")]
    EmptyName,
    /// The drug's icode was empty.
    #[error("drug icode must not be empty")]
    EmptyIcode,
    /// An icode can appear only once in the catalog.
    #[error("drug {0} is already in the catalog")]
    DuplicateDrug(String),
    /// The referenced category does not exist.
    #[error("category {0} was not found")]
    CategoryNotFound(String),
    /// The referenced drug does not exist.
    #[error("drug {0} was not found")]
    DrugNotFound(String),
    /// A category can be removed only after its drugs are moved or removed.
    #[error("category {0} still has drugs")]
    CategoryNotEmpty(String),
}
