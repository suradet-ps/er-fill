//! Every SQL statement the connector can execute (AGENTS-RUST.md section 14
//! override: compile-time constants only, `?`-placeholders only).
//!
//! Schema notes and the DBA open-questions list live in `docs/database.md`.

/// `SELECT 1` smoke test for the connection settings dialog and the health
/// monitor.
pub const PING_SQL: &str = "SELECT 1";

/// Drug search for the catalog picker (`docs/database.md` section A.1).
///
/// - `icode LIKE '1%'` - standard HOSxP drug-category prefix; confirm on the
///   instance (docs/database.md section B3).
/// - `strength`/`units` are joined along so the picker can show the drug's
///   strength (e.g. "500 mg"); both may be empty on some instances.
/// - The operator's term is escaped and wrapped `%...%` by the caller; both
///   bindings receive the same pattern.
// SCHEMA-UNVERIFIED: confirm the icode prefix convention and columns
// (`icode`, `name`, `strength`, `units`) against the live instance
// (docs/database.md section B).
pub const SEARCH_DRUGS_SQL: &str = "\
SELECT icode, COALESCE(name, '') AS name, COALESCE(strength, '') AS strength, COALESCE(units, '') AS units
FROM drugitems
WHERE icode LIKE '1%'
  AND (icode LIKE ? OR name LIKE ?)
ORDER BY name
LIMIT 50";

/// Head of the usage query; the `IN` list is `?` repeated by
/// [`usage_in_window_sql`].
const USAGE_SELECT: &str = "\
SELECT o.icode AS icode, CAST(SUM(o.qty) AS DOUBLE) AS qty
FROM opitemrece o
WHERE o.icode IN (";

/// Tail of the usage query.
///
/// - `qty > 0` drops billing corrections.
/// - `an IS NULL OR an = ''` keeps inpatient-stay usage out of the ER
///   requisition (ER visits are OPD visits) - pharmacist to confirm
///   (docs/database.md section A.2).
/// - The time predicate is the index-friendly decomposition of
///   `vstdate/vsttime ∈ [start, end)`.
const USAGE_FILTER: &str = "\
)
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND (o.vstdate > ? OR (o.vstdate = ? AND o.vsttime >= ?))
  AND (o.vstdate < ? OR (o.vstdate = ? AND o.vsttime < ?))
GROUP BY o.icode
ORDER BY o.icode";

/// Builds the usage query for `icodes_len` configured drugs: one `?` per
/// icode, then the window bounds (6 more). Values are always bound.
///
/// # Examples
///
/// ```
/// use er_fill_hosxp_connector::queries::usage_in_window_sql;
///
/// let sql = usage_in_window_sql(3);
/// assert!(sql.contains("IN (?, ?, ?)"));
/// assert!(sql.starts_with("SELECT"));
/// ```
// SCHEMA-UNVERIFIED: confirm `opitemrece` (not a custom `opitemrace`) and
// the `vstdate`/`vsttime`/`an` columns on the live instance
// (docs/database.md section B1-B3).
pub fn usage_in_window_sql(icodes_len: usize) -> String {
    let placeholders = std::iter::repeat_n("?", icodes_len)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{USAGE_SELECT}{placeholders}{USAGE_FILTER}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readonly_guard::assert_read_only;

    #[test]
    fn every_constant_passes_the_read_only_guard() {
        for sql in [PING_SQL, SEARCH_DRUGS_SQL] {
            assert!(assert_read_only(sql).is_ok(), "guard must accept: {sql}");
        }
        for count in [1_usize, 2, 7, 69] {
            let sql = usage_in_window_sql(count);
            assert!(assert_read_only(&sql).is_ok(), "guard must accept: {sql}");
        }
    }

    #[test]
    fn usage_query_binds_one_placeholder_per_icode_plus_six() {
        for count in [1_usize, 2, 5, 69] {
            let sql = usage_in_window_sql(count);
            let placeholders = sql.matches('?').count();
            assert_eq!(
                placeholders,
                count + 6,
                "expected {count} icodes + 6 window bounds in: {sql}"
            );
        }
    }

    #[test]
    fn usage_query_shape_is_stable() {
        let sql = usage_in_window_sql(2);
        assert!(sql.contains("FROM opitemrece o"));
        assert!(sql.contains("IN (?, ?)"));
        assert!(sql.contains("o.qty > 0"));
        assert!(sql.contains("(o.an IS NULL OR o.an = '')"));
        assert!(sql.contains("GROUP BY o.icode"));
    }

    #[test]
    fn search_query_shape_is_stable() {
        assert!(SEARCH_DRUGS_SQL.contains("FROM drugitems"));
        assert!(SEARCH_DRUGS_SQL.contains("icode LIKE '1%'"));
        assert!(SEARCH_DRUGS_SQL.contains("LIMIT 50"));
        assert!(!SEARCH_DRUGS_SQL.contains(';'));
    }
}
