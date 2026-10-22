//! The only crate allowed to talk to HOSxP MySQL (AGENTS.md section 5).
//!
//! Strictly SELECT-only, enforced in layers: the DB user's `GRANT SELECT`
//! (the real boundary), a read-only session per connection, and the
//! application-level guard in [`readonly_guard`] as defense in depth.
//! Connection settings are encrypted at rest (AES-256-GCM, OS keychain).
//!
//! Module map:
//! - [`config`]: encrypted connection settings (save/load).
//! - [`readonly_guard`]: rejects any statement that is not `SELECT`/`WITH`.
//! - [`ConnectorRepository`]: the [`er_refill_core::HosxRepository`] impl.
//! - [`connect`] / [`MySqlPool`]: pool creation with a read-only session.

pub mod config;
pub mod queries;
pub mod readonly_guard;

mod error;
mod pool;
mod repository;

pub use error::Error;
pub use pool::connect;
pub use repository::ConnectorRepository;
pub use sqlx::mysql::MySqlPool; // re-export so the Tauri layer never depends on sqlx directly
