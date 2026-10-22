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

/// Reads one local HOSxP officer for password verification.
///
/// Only `officer` is used: `opduser` is blacklisted by the gateway and
/// `officer_login_password` (the plaintext column) is never selected.
/// The display name comes from the officer name fields, falling back to
/// `doctor.name` through `officer_doctor_code` when they are blank.
/// Verified against the live Sra Bos instance on 2026-10-21; the hash
/// algorithm itself still needs a one-account pilot check (docs/auth.md).
///
/// SCHEMA-UNVERIFIED: `doctor.name` is the documented HOSxP display-name
/// column, but the DBA has not confirmed it on this instance yet.
pub const FETCH_OFFICER_SQL: &str = "\
SELECT CAST(o.officer_id AS SIGNED) AS officer_id,
       o.officer_login_name AS login_name,
       COALESCE(o.officer_login_password_md5, '') AS password_md5,
       COALESCE(o.officer_active, 'N') AS active,
       CAST(COALESCE(o.officer_authentication_type_id, 1) AS SIGNED) AS auth_type_id,
       NULLIF(o.officer_doctor_code, '') AS doctor_code,
       COALESCE(
           NULLIF(TRIM(CONCAT(COALESCE(o.officer_pname, ''), COALESCE(o.officer_fname, ''), ' ', COALESCE(o.officer_lname, ''))), ''),
           NULLIF(TRIM(d.name), ''),
           ''
       ) AS display_name
FROM officer o
LEFT JOIN doctor d ON d.code = NULLIF(o.officer_doctor_code, '')
WHERE o.officer_login_name = ?
LIMIT 1";

/// Head of the usage query; the `IN` list is `?` repeated by
/// [`usage_in_window_sql`].
///
/// `opitemrece` mixes drugs, automatic charges, and non-drug lines, so the
/// `drugitems` join and the `sub_type`/`item_type` filters are mandatory -
/// never count an `opitemrece` line without them.
const USAGE_SELECT: &str = "\
SELECT o.icode AS icode, CAST(SUM(o.qty) AS DOUBLE) AS qty
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.icode IN (";

/// Tail of the usage query.
///
/// - `qty > 0` drops billing corrections.
/// - `an IS NULL OR an = ''` keeps inpatient-stay usage out of the ER
///   requisition (0 of 9,639 ER drug orders carry an `an` on the surveyed
///   instance).
/// - ER orders are identified by the ordering department
///   (`o.dep_code = '009'`); the 30-day audit found 5,887 ER lines, of which
///   1,901 are drug lines.
/// - `o.sub_type = '1'` keeps drugitems rows; `item_type = 'A'` rows are
///   automatic charges (1,028 in the audit) and are excluded.
/// - The time predicate uses `rxdate/rxtime` - the order time written by the
///   ER screen - not `vstdate/vsttime` (the visit time; only 1 of 1,901 drug
///   lines shared the same value). The half-open window is `[start, end)`.
const USAGE_FILTER: &str = "\
)
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.dep_code = '009'
  AND o.sub_type = '1'
  AND IFNULL(o.item_type, '') <> 'A'
  AND (o.rxdate > ? OR (o.rxdate = ? AND o.rxtime >= ?))
  AND (o.rxdate < ? OR (o.rxdate = ? AND o.rxtime < ?))
GROUP BY o.icode
ORDER BY o.icode";

/// Builds the usage query for `icodes_len` configured drugs: one `?` per
/// icode, then the window bounds (6 more). Values are always bound.
///
/// # Examples
///
/// ```
/// use er_refill_hosxp_connector::queries::usage_in_window_sql;
///
/// let sql = usage_in_window_sql(3);
/// assert!(sql.contains("IN (?, ?, ?)"));
/// assert!(sql.starts_with("SELECT"));
/// ```
// Verified against the live Sra Bos instance on 2026-10-21 and the 30-day ER
// audit on 2026-10-22: `opitemrece` + `drugitems` are the real tables and the
// rxdate/rxtime, dep_code, sub_type, and item_type columns exist
// (docs/database.md sections A.2/A.5). The DBA sign-off for indexes and timing
// is still tracked in section B.
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
        for sql in [PING_SQL, SEARCH_DRUGS_SQL, FETCH_OFFICER_SQL] {
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
        assert!(sql.contains("o.dep_code = '009'"));
        assert!(sql.contains("o.sub_type = '1'"));
        assert!(sql.contains("IFNULL(o.item_type, '') <> 'A'"));
        assert!(sql.contains("o.rxdate"));
        assert!(sql.contains("o.rxtime"));
        assert!(sql.contains("GROUP BY o.icode"));
    }

    #[test]
    fn usage_query_requires_drug_master_and_er_department() {
        let sql = usage_in_window_sql(2);
        assert!(
            sql.contains("JOIN drugitems d ON d.icode = o.icode"),
            "opitemrece also holds non-drug charge lines: {sql}"
        );
        assert!(
            sql.contains("o.dep_code = '009'"),
            "ER lines must be the ones ordered by the ER department: {sql}"
        );
        assert!(
            !sql.contains("er_regist"),
            "ER usage must not be anchored on the visit registry: {sql}"
        );
        assert!(
            !sql.contains("main_dep"),
            "main_dep is not the ER usage scope: {sql}"
        );
        assert!(
            !sql.contains("spclty"),
            "spclty '17' had zero rows and must never identify ER: {sql}"
        );
        assert!(
            !sql.contains("vstdate") && !sql.contains("vsttime"),
            "usage must use the order time (rxdate/rxtime), not visit time: {sql}"
        );
    }

    #[test]
    fn search_query_shape_is_stable() {
        assert!(SEARCH_DRUGS_SQL.contains("FROM drugitems"));
        assert!(SEARCH_DRUGS_SQL.contains("icode LIKE '1%'"));
        assert!(SEARCH_DRUGS_SQL.contains("LIMIT 50"));
        assert!(!SEARCH_DRUGS_SQL.contains(';'));
    }

    #[test]
    fn officer_query_uses_only_the_safe_columns() {
        assert!(FETCH_OFFICER_SQL.contains("FROM officer o"));
        assert!(FETCH_OFFICER_SQL.contains("officer_login_password_md5"));
        assert!(!FETCH_OFFICER_SQL.contains("officer_login_password "));
        assert!(!FETCH_OFFICER_SQL.contains("opduser"));
        assert!(FETCH_OFFICER_SQL.contains("LEFT JOIN doctor d"));
        assert!(FETCH_OFFICER_SQL.contains("d.name"));
        assert!(FETCH_OFFICER_SQL.contains("LIMIT 1"));
        assert!(!FETCH_OFFICER_SQL.contains(';'));
    }
}
