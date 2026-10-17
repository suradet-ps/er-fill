//! Business logic for ER-Fill - everything that can be tested without a
//! database and without a GUI (AGENTS.md section 3).
//!
//! Module map:
//! - `schedule`: [`window_for`], the out-of-hours requisition window.
//! - `catalog_ops`: add/rename/remove/reorder catalog entries.
//! - `requisition`: [`build_report`], join catalog + usage into sections.
//! - `pagination`: [`paginate_sections`], split the report into A4 pages.
//! - `repository`: the [`HosxRepository`] contract and [`RepositoryError`].
//! - `mock`: an in-memory [`MockRepository`] for tests.
//!
//! The frontend (`app/`) links this crate directly, so it must compile for
//! `wasm32-unknown-unknown` - keep tokio/sqlx out of the dependency path.

mod catalog_ops;
mod error;
mod mock;
mod pagination;
mod repository;
mod requisition;
mod schedule;

pub use catalog_ops::{
    add_category, add_drug, move_drug, remove_category, remove_drug, rename_category, update_drug,
};
pub use error::CatalogOpsError;
pub use mock::MockRepository;
pub use pagination::{PageRow, PageSection, ROWS_PER_PAGE, SheetPage, paginate_sections};
pub use repository::{HosxRepository, RepositoryError};
pub use requisition::build_report;
pub use schedule::window_for;
