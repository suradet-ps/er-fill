# AGENTS.md - ER-Refill

This file gives coding agents (and future contributors) the context needed to work on this repo correctly and safely. Read this **and** `docs/DESIGN.md` (visual/UI design system) before writing any code. `AGENTS-RUST.md` holds the Rust-specific style and workflow rules - this file covers product context, architecture, and process; `DESIGN.md` covers visual design only.

## 1. What this project is

ER-Refill ("ยาห้องฉุกเฉิน") is a desktop app for the pharmacy department of a community hospital. It generates the **daily emergency-room drug requisition form** (ใบเบิกยาประจำห้องฉุกเฉินรายวัน): it sums how much of each configured ER drug was used in HOSxP during an **overnight window**, then prints the form for the ER to sign and the pharmacy to fill. The run date is always **today** (read-only), and the operator must choose one of four fixed ranges before fetching: 20:30-08:00, 20:30-08:30, 16:00-08:00, or 16:00-08:30 (previous day to run day). The login gate is the first screen and handles only the HOSxP **`officer`** credentials; connection setup lives in the settings dialog. The session is in-memory only.

**Design goals:** simple, no unnecessary features; read-only against HOSxP (hospital MySQL/MariaDB); the date is fixed and the range choice explicit, so a fetch can never silently cover the wrong hours; the printed A4 sheet must look like the hospital's real paper form.

**Non-goals:**
- Not a system for writing/editing/deleting anything in HOSxP.
- Not an inventory/depot system - it does not track ER stock on hand, only usage pulled from HOSxP.
- Not a stock-par / min-max alerting system (may come later).
- Does not handle in-hours (08:30-20:30 weekday / 08:00-16:00 weekend) usage - that flow stays with the pharmacy's normal process; this app covers **out-of-hours only**.
- Does not persist patient data to disk (the report lists drugs and quantities, never patients).

## 2. Hard rules (non-negotiable)

1. **Never write to the HOSxP database.** No `INSERT`/`UPDATE`/`DELETE`/`ALTER`/DDL of any kind, ever - not even in tests. All DB access goes through `crates/hosxp-connector`, and that crate must remain SELECT-only end to end (DB user grants, session mode, query guard, parameterized queries - see section 5).
2. **Never log secrets or PII.** Connection values and drug-usage parameters are never logged; log timing and error types only.
3. **Parameterized queries only.** Never build SQL by string concatenation. Dynamic `IN (?, ?, ...)` placeholder lists are built by repeating `?` only - values are always bound.
4. **No schema guessing without a flag.** HOSxP table/column names are best-effort until verified against the real instance. Query constants touching unverified tables/columns must carry a `// SCHEMA-UNVERIFIED:` comment until confirmed by the DBA (see section 6 - this includes the `opitemrece` naming question).
5. **Thai belongs to the edges.** Thai UI strings live in `app/` (frontend) and `src-tauri/commands.rs` (user-facing error messages). Crate code, comments, and docs stay in English.
6. **Login reads `officer` only.** Never query `opduser` (gateway-blacklisted), never select `officer_login_password` (plaintext), never log usernames/passwords, and never persist a login log or token to HOSxP. The session lives only in the Tauri process; the MD5 assumption is documented in `docs/auth.md`.

## 3. Workspace layout (ports-lite)

```
er-refill/
├── src-tauri/            # Tauri 2 shell - thin command adapters only, no business logic, no SQL
├── crates/
│   ├── models/           # Domain types + Thai/BE formatting. No sqlx, no I/O.
│   ├── core/             # Window algorithm, catalog ops, report builder, repository trait + mock
│   └── hosxp-connector/  # The only crate allowed to talk to MySQL. Read-only guard lives here.
└── app/                  # Leptos 0.8 CSR/WASM frontend (separate wasm workspace, Trunk)
    └── style/            # CSS implementing DESIGN.md tokens
```

Dependency direction: `app` → `er-refill-core` → `er-refill-models`, and `er-refill-hosxp-connector` implements a repository trait defined in `er-refill-core`. `er-refill-core` must be testable with a mock repository, without a real database. The `app` workspace also depends on `er-refill-core`/`er-refill-models` directly (both are wasm-compatible - no tokio, no sqlx on their dependency path).

## 4. Domain model (`crates/models`)

- `WindowPreset` - the four fixed overnight ranges: 20:30-08:00, 20:30-08:30, 16:00-08:00, 16:00-08:30. Every preset starts on the previous day and ends on the run day; Thai labels live in `app/`.
- `RequisitionWindow { start: NaiveDateTime, end: NaiveDateTime }` - the half-open window `[start, end)`, always computed from today and the chosen preset (not user-editable).
- `Catalog { categories: Vec<DrugCategory>, drugs: Vec<ErDrug> }` - `DrugCategory { id, name, sort_order }`, `ErDrug { icode, name, strength_text, stock_text, category_id, sort_order }`. Seeded with the six categories of the approved ER list (see `docs/reference/er-drug-list.md`); drugs are added from HOSxP `drugitems` search, which prefills `strength_text` from `strength`/`units` (e.g. "500 mg") so same-name drugs stay distinguishable. The printed sheet hides the Thai unit suffix. `stock_text` is `units-per-pack/stock-packs` ("10/20" = 10 tablets per pack, 20 packs on hand); leading numbers are parsed, units after them are ignored.
- `DrugItem { icode, name, strength, units }` - a `drugitems` search hit.
- `UsageRow { icode, qty }` - one grouped usage result from HOSxP.
- `ReportSection { category_id, category_name, rows: Vec<ReportRow> }`, `ReportRow { icode, name, strength_text, stock_text, usage, requested, max_requested, added, reason }` - the printable table. `build_report` includes **only drugs with `usage > 0`**, so a quiet night does not print the whole catalog. `requested` starts as `ceil(usage / units-per-pack)`, capped at the stock pack count (`max_requested`); it is editable in whole packs before printing. `added` marks an operator-added configured drug (usage 0) and `reason` records why the row was added or its requested quantity changed; `has_adjustment()` identifies the rows that feed the printed reason table. A stock label without a slash falls back to whole usage with no cap.
- `ReportMeta { hospital_name, document_title, department_name }` - the printed header block; defaults for Sra Bos Hospital.
- `OfficerAccount` (backend-only) - one `officer` row: login name, MD5 hash, active flag, auth type, doctor code, display name. `SessionUser` - the sanitized login result (id, login name, display name, doctor code, role) sent over IPC. `ConnectionSummary` - host/port/database/user used to prefill the connection editor; never the password and never displayed on the login summary.

Thai/Buddhist-era formatting (`be_year`, `format_thai_date*`, `format_qty`) lives in `models::thai` so the UI and the print sheet share one implementation.

## 5. Strict read-only - enforcement mechanism

Same layered approach as the sibling AllerX project:

1. **DB user level:** a MySQL user dedicated to ER-Refill, `GRANT SELECT` only. This is the real security boundary.
2. **Connection level:** `SET SESSION TRANSACTION READ ONLY;` immediately after connect (`after_connect` hook), plus a best-effort `SET SESSION max_execution_time = 5000` (SELECT timeout).
3. **Application level:** `readonly_guard.rs` rejects any outgoing statement that is not a single `SELECT`/`WITH` (comment-aware, semicolon-aware). Defense in depth, not the primary boundary.
4. **Queries:** sqlx with bound parameters only; SQL strings are compile-time constants (or assembled from constants with `?` repetition).
5. **Tauri capabilities:** `core:default` only - no fs/shell scopes.

## 6. HOSxP integration - schema notes (confirm against the live instance)

> **Naming note:** the live Sra Bos instance uses **`opitemrece`** (not a
> custom `opitemrace`), confirmed in the 2026-10-21 schema survey
> (docs/database.md section A.3). The usage query is verified against that
> survey; the drug-search columns and the timing/index questions remain
> flagged or open in docs/database.md section B.

### 6.1 Drug search (`drugitems`)
- Fields used: `icode`, `name`, `strength`, `units`.
- Standard HOSxP convention: drug icodes start with `1` (`icode LIKE '1%'`) - filters out non-drug items that share the table family. Confirm on the instance.
- The picker keeps the icode chip and shows the full strength label (`strength` + `units`, e.g. "500 mg") so operators can identify the item; the printed sheet hides the Thai dispensing-unit suffix and the icode. Both columns may be empty on some instances - confirm the `strength` column with the DBA.
- Escape `\`, `%`, `_` in the operator's term before wrapping in `%...%` (contains match).

### 6.2 Usage window (`opitemrece`)
- Fields used: `icode`, `qty`, `rxdate` (date), `rxtime` (time), `dep_code`, `sub_type`, `item_type`, `an` (admission number - empty/NULL for OPD visits; ER visits are OPD).
- Always `JOIN drugitems` - `opitemrece` also stores non-drug charge lines; the join plus `sub_type = '1'` and `IFNULL(item_type, '') <> 'A'` keep automatic charges out.
- ER lines are the ones ordered by the ER department: `dep_code = '009'`. Do not anchor usage on `er_regist.vn`/`ovst.main_dep`, and never use `ovst.spclty = '17'` (zero rows in 90 days).
- Every query must filter `qty > 0` (zero/negative rows are billing corrections, not dispensing events).
- Filter `(an IS NULL OR an = '')` - ER visits are OPD visits. On the surveyed instance 0 of 9,639 ER drug orders carry an `an`.
- Time filtering is index-friendly decomposition, window `[start, end)`:
  `(rxdate > d1 OR (rxdate = d1 AND rxtime >= t1)) AND (rxdate < d2 OR (rxdate = d2 AND rxtime < t2))`.
- `rxdate`/`rxtime` is the **order time** written by the ER screen (`RxBtnClick`); `vstdate`/`vsttime` is the visit time. The 30-day audit found only 1 of 1,901 ER drug lines with `rxtime = vsttime`, so never use visit time for the usage window.
- `opitemrece_log.station = 'ER'` covers only part of the orders (1,531 log rows / 756 visits; 1,007 of 1,901 drug lines within 60 s) - use it as a supplementary audit trail, never as the primary filter.
- `last_modified` is not the order time (it changes on later edits); `recetime` is NULL 100%; `er_iv_list.iv_time` and `er_command.command_datetime` are empty - do not use them.
- Use `hos_guid` for line identity if a line-level feature is ever added - `vn + item_no` is not unique (`item_no` is `tinyint`).

### 6.3 Open questions for the DBA (docs/database.md section B)
- ~~Confirm `opitemrece` vs `opitemrace` naming~~ - resolved: the live instance uses `opitemrece`.
- ~~Confirm visit time vs order time~~ - resolved: use `rxdate`/`rxtime` (order time), not `vstdate`/`vsttime`.
- ~~Confirm the ER line scope~~ - resolved: `dep_code = '009'` plus `sub_type = '1'` and `item_type <> 'A'` (30-day audit).
- Confirm drug-type filter for the picker (`icode LIKE '1%'` vs `drugitems.istype`).
- Index on `opitemrece(icode, rxdate, rxtime)` (or equivalent) - daily window queries should stay < 300 ms.

### 6.4 Authentication (`officer`)
- Read one row by `officer_login_name`; check `officer_active = 'Y'`, `IFNULL(officer_authentication_type_id, 1) = 1`, and `MD5(password) = officer_login_password_md5` (case-insensitive).
- Never select `officer_login_password`; never read `opduser`; never write a session or audit row to HOSxP.
- Display name: `officer_pname`/`fname`/`lname`, falling back to `doctor.name` through `officer_doctor_code`, then to the login name.
- `officer_group*` is empty, so the app assigns `role = "STAFF"`; app-specific roles are a later milestone.
- The MD5 assumption is **not** source-confirmed; test one real account before the pilot. Full design and caveats: `docs/auth.md`.

## 7. Requisition window algorithm

The run date is **always today** and is not editable. The operator chooses one
of four fixed ranges; each starts on the previous day and ends on the run day,
window `[start, end)`:

| Preset | Example: run on Sat 24 Oct 2026 |
|---|---|
| 20:30-08:00 | Fri 23 Oct 20:30 → Sat 24 Oct 08:00 |
| 20:30-08:30 | Fri 23 Oct 20:30 → Sat 24 Oct 08:30 |
| 16:00-08:00 | Fri 23 Oct 16:00 → Sat 24 Oct 08:00 |
| 16:00-08:30 | Fri 23 Oct 16:00 → Sat 24 Oct 08:30 |

There are no custom hours, no manual date picker, and no override: fetching
stays disabled until a range is selected. Changing the selected range clears
the current sheet and the print confirmation. A 30-second UI timer also rolls
the date over at midnight: without a selected range only the date changes;
with one, the window is recomputed and the old sheet is cleared. The printed
form always shows the actual window it was built from.

## 8. Report flow

Document-first UI (DESIGN.md "Structure"): the command bar holds the run
controls; the stage shows the A4 sheet, which is one DOM/stylesheet shared
by screen and print.

1. Operator opens the app on the login gate and verifies the `officer`
   username/password (the saved HOSxP connection is re-probed at startup;
   connection setup lives in the settings dialog, not on the login card).
   After sign-in the command bar shows **today's date (read-only)** and the
   four range options; no fetch happens until the operator selects one.
2. "ดึงข้อมูล" reads usage for **all configured icodes at once** (one grouped
   query) for the selected window. Switching the range clears the sheet and
   the confirmation, and a stale in-flight response is discarded.
3. `er-refill-core::build_report` joins catalog + usage into category sections;
   **only drugs with `usage > 0` appear** (items that were not used do not
   clutter the paper); each row carries the drug's `strength_text` beside its
   name and derives "จำนวนเบิก" as whole packs (`ceil(usage / units-per-pack)`,
   capped at the stock pack count).
4. The operator can add configured catalog drugs that had no usage (a row with
   usage 0) and may change any row's "จำนวนเบิก". **Every manual add or
   quantity change requires a reason** (`er-refill-core::add_configured_drug`
   / `adjust_requested`); the values are rounded to whole packs and clamped to
   the same stock cap. The sheet prints a final
   **"ยาที่ขอเพิ่มหรือขอแก้ไขจำนวนเบิก"** table (drug name + reason only) on the
   last page, after the main table and before the signatures.
5. Printing is gated: "ยืนยันข้อมูลเบิก" opens a confirmation dialog and
   snapshots the rows, reasons, window, and printed header. The print button
   stays disabled until that snapshot exists, and the print action re-checks
   that the current data is identical. "พิมพ์ใบเบิก" then calls
   `window.print()` on the on-screen sheet - the preview IS the printed output
   (bear mark at the top-left, grouped by category, with the window sentence,
   hospital header, and ผู้เบิก/ผู้จ่าย signature blocks). The ผู้เบิก
   parentheses use the signed-in officer's display name; the ผู้จ่าย line is
   left blank for the pharmacy. The sheet paginates at **40 drug rows per A4
   page** (`er-refill-core::paginate_sections`); split categories continue with
   "(ต่อ)" and the signatures/adjustment table render on the last page only.
   Any later edit invalidates the confirmation and disables printing until the
   operator confirms again.

## 9. Performance

- Pool size ~5 (`MySqlPoolOptions`), 5 s acquire timeout - a single-hospital
  desktop app.
- One grouped query per requisition run (not one per drug).
- Daily volume is small; correctness beats microseconds. Do not add caching.

## 10. Security & privacy

- Connection credentials: every field encrypted at rest with `encryptman`
  (AES-256-GCM), master key in the OS keychain via `encryptman-keyring`
  (service name `er-refill`). Password is `secrecy::SecretString` end to end in
  Rust, zeroized on drop, `Debug`-redacted. Documented residuals: the webview
  JS heap while typing, and sqlx's internal pool copy (needed for reconnects).
- The plain settings file (`er-refill.settings.json`) holds the catalog and
  report header only - never credentials. The run date and range are UI state,
  not settings.
- Login passwords are sent to the Rust backend, moved into a `SecretString`
  for the MD5 check, and dropped; they are never stored or logged. The
  backend session is memory-only and cleared on logout, restart, or a
  connection change.
- The frontend never touches the database; it only calls Tauri commands.
  `fetch_usage`, `search_drugs`, and settings writes require a backend
  session; login/connection commands do not.
- No patient data is ever displayed or stored - only aggregated quantities.

## 11. Milestones

| Milestone | Scope |
|---|---|
| **M0** | Workspace setup, AGENTS-RUST.md, Tauri 2 + Leptos 0.8 CSR scaffold, connection smoke test (`SELECT 1`) |
| **M1** | Schema verification with the DBA (docs/database.md section B), remove `SCHEMA-UNVERIFIED` markers |
| **M2** | Catalog management (categories + drug picker from `drugitems`) end-to-end |
| **M3** | Fixed-date range picker + usage fetch + requisition table |
| **M4** | Print sheet (A4) reviewed against the real paper form (ใบเบิกยา xlsx) |
| **M5** | Packaging (Tauri bundle), internal pilot with the pharmacy department, feedback |

## 12. Documentation-first workflow

1. If a task changes behavior, update this file and/or `docs/DESIGN.md` in the
   same commit as the code - don't let docs drift.
2. For anything touching HOSxP schema, verify against the real instance before
   writing the query and remove the `SCHEMA-UNVERIFIED` marker once confirmed.
3. Keep changes scoped to one milestone at a time.

## 13. Testing expectations

- `er-refill-core` + `er-refill-models`: the default suite (`cargo test` at root) -
  window algorithm (all four presets, previous-day start, date-too-early),
  catalog operations, report building (usage-only rows, catalog-order additions,
  required reasons, stock caps), Thai formatting, MD5 verification.
  No DB required.
- `er-refill-hosxp-connector`: unit tests for the guard and encrypted config run in
  the default suite; live-DB checks are gated behind
  `--features integration-tests` (never production).
- Every SQL constant gets a guard assertion test.
- `app/`: host tests for pure logic and the print-confirmation snapshot
  (`state.rs`); `wasm-bindgen-test` for the IPC mock contract (separate
  workspace - run explicitly).

## 14. Style and conventions

- Rust style/error handling: follow `AGENTS-RUST.md`.
- **Error handling:** one typed `thiserror` error per crate boundary -
  `models::WindowError`, `core::CatalogOpsError`, `core::RepositoryError`,
  connector `Error`, and the Tauri `CommandError` (kind + Thai message).
  Conversion happens only at each boundary (`impl From`), and Thai user-facing
  translation happens **only** in `src-tauri/commands.rs`.
- CSS: plain CSS implementing `DESIGN.md` tokens, no framework. Lives in
  `app/style/main.css`.
- Prefer small, explicit functions over clever abstractions - this codebase
  should be easy for a pharmacist-developer to re-read six months later.

## 15. What NOT to do

- Don't add a full ORM or heavy query builder - `sqlx::query_as` with explicit
  structs is the standard here.
- Don't introduce a second way to reach the database. `hosxp-connector` is the
  only door.
- Don't add write-capable Tauri fs/shell scopes "just in case."
- Don't add PDF-generation libraries: printing is `window.print()` with a
  hidden A4 sheet (the OS print dialog can save PDF) - keep it dependency-free.
- Don't store per-day requisition history in v1 - the HOSxP data is the record
  of truth; a local audit trail is a later milestone if requested.
