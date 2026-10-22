//! [`er_refill_core::HosxRepository`] implementation over the read-only pool.

use chrono::NaiveDateTime;
use er_refill_core::{HosxRepository, RepositoryError};
use er_refill_models::{DrugItem, OfficerAccount, RequisitionWindow, UsageRow};
use sqlx::Row;
use sqlx::mysql::MySqlPool;

use crate::error::Error;
use crate::queries::{FETCH_OFFICER_SQL, PING_SQL, SEARCH_DRUGS_SQL, usage_in_window_sql};
use crate::readonly_guard::assert_read_only;

/// Read-only HOSxP access through the connector.
///
/// Thin by design: guard, bind, fetch, map. All SQL lives in
/// [`crate::queries`]; all schema decisions are documented in
/// `docs/database.md`.
#[derive(Debug, Clone)]
pub struct ConnectorRepository {
    pool: MySqlPool,
}

impl ConnectorRepository {
    /// Wraps an existing read-only pool (created by [`crate::connect`]).
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// The underlying pool, for the Tauri state layer.
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Runs the guard (defense in depth) before every statement.
    fn guard(sql: &str) -> Result<(), RepositoryError> {
        assert_read_only(sql).map_err(|_| Error::Guard.into())
    }
}

/// Escapes `\`, `%`, `_` in a `LIKE` pattern fragment. The operator's term
/// is data, never syntax.
fn escape_like(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len());
    for c in term.chars() {
        if matches!(c, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Decomposes the window into the six bounds the usage query binds:
/// `(start_date, start_date, start_time, end_date, end_date, end_time)`.
fn window_bounds(window: &RequisitionWindow) -> (NaiveDateTime, NaiveDateTime) {
    (window.start, window.end)
}

#[async_trait::async_trait]
impl HosxRepository for ConnectorRepository {
    async fn ping(&self) -> Result<(), RepositoryError> {
        Self::guard(PING_SQL)?;
        sqlx::query(PING_SQL)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| Error::Database(e).into())
    }

    async fn fetch_officer(
        &self,
        login_name: &str,
    ) -> Result<Option<OfficerAccount>, RepositoryError> {
        Self::guard(FETCH_OFFICER_SQL)?;
        let row = sqlx::query(FETCH_OFFICER_SQL)
            .bind(login_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Database)?;
        row.map(|row| -> Result<OfficerAccount, Error> {
            Ok(OfficerAccount {
                officer_id: row.try_get("officer_id").map_err(Error::Database)?,
                login_name: row.try_get("login_name").map_err(Error::Database)?,
                password_md5: row.try_get("password_md5").map_err(Error::Database)?,
                active: row.try_get("active").map_err(Error::Database)?,
                auth_type_id: row.try_get("auth_type_id").map_err(Error::Database)?,
                doctor_code: row.try_get("doctor_code").map_err(Error::Database)?,
                display_name: row.try_get("display_name").map_err(Error::Database)?,
            })
        })
        .transpose()
        .map_err(Into::into)
    }

    async fn search_drugs(&self, term: &str) -> Result<Vec<DrugItem>, RepositoryError> {
        Self::guard(SEARCH_DRUGS_SQL)?;
        let pattern = format!("%{}%", escape_like(term.trim()));
        let rows = sqlx::query(SEARCH_DRUGS_SQL)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;
        rows.into_iter()
            .map(|row| {
                Ok(DrugItem {
                    icode: row.try_get::<String, _>("icode").map_err(Error::Database)?,
                    name: row.try_get::<String, _>("name").map_err(Error::Database)?,
                    strength: row
                        .try_get::<String, _>("strength")
                        .map_err(Error::Database)?,
                    units: row.try_get::<String, _>("units").map_err(Error::Database)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
            .map_err(Into::into)
    }

    /// Sums drug lines only (`JOIN drugitems`, `sub_type = '1'`, excluding
    /// automatic `item_type = 'A'` charges) ordered by the ER department
    /// (`dep_code = '009'`) inside the `rxdate/rxtime` window.
    async fn fetch_usage(
        &self,
        icodes: &[String],
        window: &RequisitionWindow,
    ) -> Result<Vec<UsageRow>, RepositoryError> {
        if icodes.is_empty() {
            return Ok(Vec::new());
        }
        let sql = usage_in_window_sql(icodes.len());
        Self::guard(&sql)?;

        let (start, end) = window_bounds(window);
        let mut query = sqlx::query(&sql);
        for icode in icodes {
            query = query.bind(icode);
        }
        query = query
            .bind(start.date())
            .bind(start.date())
            .bind(start.time())
            .bind(end.date())
            .bind(end.date())
            .bind(end.time());

        let rows = query.fetch_all(&self.pool).await.map_err(Error::Database)?;
        rows.into_iter()
            .map(|row| {
                Ok(UsageRow {
                    icode: row.try_get::<String, _>("icode").map_err(Error::Database)?,
                    qty: row.try_get::<f64, _>("qty").map_err(Error::Database)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn escape_like_escapes_wildcards_and_backslash() {
        assert_eq!(escape_like("para"), "para");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("c:\\x"), "c:\\\\x");
        assert_eq!(escape_like("พารา"), "พารา");
    }

    #[test]
    fn window_bounds_returns_start_and_end() -> Result<(), Box<dyn std::error::Error>> {
        let start = NaiveDate::from_ymd_opt(2026, 10, 18)
            .and_then(|date| date.and_hms_opt(16, 0, 0))
            .ok_or("valid start")?;
        let end = NaiveDate::from_ymd_opt(2026, 10, 19)
            .and_then(|date| date.and_hms_opt(8, 0, 0))
            .ok_or("valid end")?;
        let window = RequisitionWindow::new(start, end)?;
        assert_eq!(window_bounds(&window), (start, end));
        Ok(())
    }
}
