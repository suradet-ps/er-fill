//! Live-database integration tests (AGENTS.md section 13).
//!
//! Gated behind `--features integration-tests` and pointed at a **test or
//! staging** HOSxP instance via `ER_FILL_TEST_DATABASE_URL` - never production.
//!
//! ```bash
//! cd crates/hosxp-connector
//! ER_FILL_TEST_DATABASE_URL='mysql://erfill_ro:pass@host:3306/hos' \
//!     cargo test --features integration-tests
//! ```

#![cfg(feature = "integration-tests")]

use sqlx::mysql::MySqlPoolOptions;

fn database_url() -> Result<String, Box<dyn std::error::Error>> {
    std::env::var("ER_FILL_TEST_DATABASE_URL")
        .map_err(|_| "ER_FILL_TEST_DATABASE_URL must be set for integration tests".into())
}

/// The smoke test must succeed against a reachable test instance.
#[tokio::test]
#[ignore = "requires ER_FILL_TEST_DATABASE_URL pointing at a staging instance"]
async fn ping_against_test_instance_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let url = database_url()?;
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
    assert_eq!(row.0, 1);
    Ok(())
}

/// A read-only session must reject DDL; the read-only account has no grants
/// for it either - either way the write fails while reads keep working.
#[tokio::test]
#[ignore = "requires ER_FILL_TEST_DATABASE_URL pointing at a staging instance"]
async fn read_only_session_rejects_ddl() -> Result<(), Box<dyn std::error::Error>> {
    let url = database_url()?;
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::query("SET SESSION TRANSACTION READ ONLY")
        .execute(&pool)
        .await?;
    let ddl = sqlx::query("CREATE TABLE erfill_ro_probe (id INT)")
        .execute(&pool)
        .await;
    assert!(ddl.is_err(), "read-only session must reject DDL");
    // Reads still work in the same session.
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
    assert_eq!(row.0, 1);
    Ok(())
}
