//! Errors from catalog operations ([`crate::catalog_ops`]).

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
