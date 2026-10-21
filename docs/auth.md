# Authentication - HOSxP officer login

ER-Fill signs operators in with their HOSxP **`officer`** account. The
HOSxP database remains read-only (AGENTS.md section 2); the login session
lives only in the Tauri process, and the app never writes an audit row or a
session token back to HOSxP.

## A. Source of truth

| Table | Observed on the Sra Bos instance | Use |
|---|---|---|
| `officer` | 104 accounts, 100% active; `officer_login_name` unique and non-empty; `officer_login_password_md5` 32 hex on all 104 | **The only login source** |
| `opduser` | 221 old (V3) accounts | **Never read** - the gateway blacklists it |
| `onlineuser` | 56 rows / 19 users | Optional online-status hint only, never identity |
| `officer_activity_log` | 131,415 rows | Existing HOSxP audit trail; ER-Fill does not write it |
| `user_jwt` | 1,868 rows | Never read |
| `emp` | 0 rows | Not used |

Additional verified facts:

- `officer_authentication_type_id` is `NULL` on all 104 rows; the query
  coalesces it to **1 = Local Database**. LDAP (2) is rejected for now.
- `officer_login_password` (plaintext) has only one populated row. It is
  **never selected**; only `officer_login_password_md5` is read.
- `officer_active` is `'Y'` on all current rows, but the login command still
  checks it before creating a session.
- `officer_group*`, `officer_module`, `officer_task`, and
  `officer_operation` are not a usable permission source (group tables are
  empty). The app assigns every signed-in officer the same `STAFF` role for
  now; app-specific roles are a later milestone.
- `officer_doctor_code` is present on 103/104 accounts and has one orphan
  (no matching `doctor` row). The query uses `NULLIF` and the UI must not
  assume a doctor record exists.
- `emp_id` is 0 on all accounts, so no employee-table join is possible.

## B. Flow

```
Launch
  │
  ├─ no encrypted connection config
  │     └─ connection form (host · port · database · read-only user · password)
  │           └─ connect: connect → SELECT 1 → encrypt & save → pool
  │
  ├─ config exists
  │     ├─ saved-connection confirmation (no host/database/user shown)
  │     ├─ reconnect: load vault → connect → SELECT 1
  │     └─ on failure: retry or replace the connection
  │
  └─ connection ready
        └─ officer login (username + password)
              ├─ fetch one officer row by login name
              ├─ check active = 'Y'
              ├─ check auth_type = 1 (Local Database)
              ├─ verify MD5(password) against officer_login_password_md5
              └─ store an in-memory SessionUser; UI opens
```

The connection must be verified before the login form is enabled. The
settings dialog can still replace the connection later; doing so clears the
backend session and returns the operator to the login gate.

## C. Password verification

The app computes `MD5(password)` in Rust and compares it with the stored
32-hex digest, case-insensitively. The password is moved into a
`secrecy::SecretString` for the verification window; it is never written to
disk, logged, or returned to the frontend.

> **Unverified assumption.** The 2026-10-21 survey confirmed the *shape*
> (MD5, 32 hex) but could not confirm from HOSxP source code what string is
> hashed (salt, encoding, or normalization). Before the pilot, test one
> known account with read-only access. If the real algorithm differs, login
> fails for every account and this assumption must be corrected first.
> Thai passwords would also require confirming the source encoding
> (UTF-8 vs TIS-620).

## D. Session and roles

- `SessionUser` is kept in `AppState.session` in the Tauri process only.
- Restarting the app requires signing in again.
- The frontend never receives the password hash.
- `fetch_usage`, `search_drugs`, and settings writes require a session at the
  command boundary; the login/connection commands do not.
- The session role is `STAFF` for everyone until app-specific roles are
  introduced. HOSxP group permissions are not consulted.

## E. Query

The only login query is `FETCH_OFFICER_SQL` in
`crates/hosxp-connector/src/queries.rs`:

```sql
SELECT CAST(o.officer_id AS SIGNED) AS officer_id,
       o.officer_login_name AS login_name,
       COALESCE(o.officer_login_password_md5, '') AS password_md5,
       COALESCE(o.officer_active, 'N') AS active,
       CAST(COALESCE(o.officer_authentication_type_id, 1) AS SIGNED) AS auth_type_id,
       NULLIF(o.officer_doctor_code, '') AS doctor_code,
       TRIM(CONCAT(COALESCE(o.officer_pname, ''), COALESCE(o.officer_fname, ''), ' ', COALESCE(o.officer_lname, ''))) AS display_name
FROM officer o
WHERE o.officer_login_name = ?
LIMIT 1
```

It is a single `SELECT` with one bound parameter; `opduser` and the
plaintext password column never appear.

## F. Why the proposed app_* tables are not created

The survey proposed `v_hosxp_user`, `v_hosxp_user_permission`,
`v_hosxp_online`, `app_user`, `app_session`, `app_login_log`, `app_role`,
and `app_role_permission`. None are created:

- `CREATE VIEW`/`CREATE TABLE` are writes, and ER-Fill must never write to
  HOSxP (AGENTS.md section 2).
- ER-Fill has no separate application database. Adding one is a product
  decision, not part of this milestone.
- A persistent login log would duplicate HOSxP `officer_activity_log` and
  would store staff identifiers locally; the current privacy rule is to
  avoid that until the hospital approves a local audit store.
- Online status remains an optional read (`onlineuser`) if it is ever needed
  in the UI; it is not identity.

If a local application database is approved later, the proposed app tables
can be introduced there without touching HOSxP. The design recorded for that
future decision is:

```sql
CREATE TABLE app_user (
  app_user_id       BIGINT AUTO_INCREMENT PRIMARY KEY,
  officer_id        BIGINT       NOT NULL,          -- officer.officer_id
  login_name        VARCHAR(100) NOT NULL,
  doctor_code       VARCHAR(15)  NULL,
  display_name      VARCHAR(200) NULL,
  role_code         VARCHAR(30)  NOT NULL DEFAULT 'STAFF',
  default_depcode   CHAR(3)      NULL,              -- '009' = ER
  is_active         CHAR(1)      NOT NULL DEFAULT 'Y',
  last_login_at     DATETIME     NULL,
  last_login_ip     VARCHAR(100) NULL,
  failed_count      INT          NOT NULL DEFAULT 0,
  locked_until      DATETIME     NULL,
  created_at        DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at        DATETIME     NULL ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_login (login_name),
  UNIQUE KEY uq_officer (officer_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE app_session (
  session_id     CHAR(36)      NOT NULL PRIMARY KEY,
  app_user_id    BIGINT        NOT NULL,
  login_name     VARCHAR(100)  NOT NULL,
  token_hash     VARCHAR(128)  NOT NULL,
  ip_address     VARCHAR(100)  NULL,
  user_agent     VARCHAR(250)  NULL,
  created_at     DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at   DATETIME      NULL,
  expires_at     DATETIME      NOT NULL,
  revoked_at     DATETIME      NULL,
  revoke_reason  VARCHAR(100)  NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE app_login_log (
  log_id       BIGINT AUTO_INCREMENT PRIMARY KEY,
  login_name   VARCHAR(100) NOT NULL,
  app_user_id  BIGINT       NULL,
  result       CHAR(1)      NOT NULL,   -- S/F/L/D
  fail_reason  VARCHAR(100) NULL,
  ip_address   VARCHAR(100) NULL,
  user_agent   VARCHAR(250) NULL,
  computername VARCHAR(100) NULL,
  login_at     DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE app_role (
  role_code   VARCHAR(30)  NOT NULL PRIMARY KEY,
  role_name   VARCHAR(100) NOT NULL,
  description VARCHAR(250) NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE app_role_permission (
  role_code    VARCHAR(30) NOT NULL,
  module_code  VARCHAR(30) NOT NULL,
  action_code  VARCHAR(20) NOT NULL,
  allow_flag   CHAR(1)     NOT NULL DEFAULT 'Y',
  PRIMARY KEY (role_code, module_code, action_code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

The read-model views (`v_hosxp_user`, `v_hosxp_user_permission`,
`v_hosxp_online`) are also deferred: they would require `CREATE VIEW` against
HOSxP. The app queries the base tables directly and inline.

## G. Acceptance checklist

- [ ] Verify `MD5(password)` on one real test account (read-only).
- [ ] Confirm the test account's password encoding (ASCII/UTF-8/TIS-620).
- [ ] Confirm `officer_authentication_type_id` really means 1 = Local
      Database on this instance.
- [ ] Confirm the officer name columns used for the display name
      (`officer_pname`, `officer_fname`, `officer_lname`).
- [ ] Confirm whether the 221 `opduser` users were migrated to `officer`;
      users who only exist in `opduser` cannot sign in.
- [ ] Test active/inactive and LDAP rejection paths with test data.
- [ ] Confirm the read-only MySQL user can `SELECT officer` (no write grant).
