# AGENTS.md - ER-Fill

This file gives coding agents (and future contributors) the context needed to work on this repo correctly and safely. Read this **and** `docs/DESIGN.md` (visual/UI design system) before writing any code. `AGENTS-RUST.md` holds the Rust-specific style and workflow rules - this file covers product context, architecture, and process; `DESIGN.md` covers visual design only.

## 1. What this project is

ER-Fill ("ยาห้องฉุกเฉิน") is a desktop app for the pharmacy department of a community hospital. It generates the **daily emergency-room drug requisition form** (ใบเบิกยาประจำห้องฉุกเฉินรายวัน): it sums how much of each configured ER drug was used in HOSxP during the **out-of-hours window** since the last requisition, then prints the form for the ER to sign and the pharmacy to fill.

**Design goals:** simple, no unnecessary features; read-only against HOSxP (hospital MySQL/MariaDB); the requisition window is computed for the operator but always overridable; the printed A4 sheet must look like the hospital's real paper form.

**Non-goals:**
- Not a system for writing/editing/deleting anything in HOSxP.
- Not an inventory/depot system - it does not track ER stock on hand, only usage pulled from HOSxP.
- Not a stock-par / min-max alerting system (may come later).
- Does not handle in-hours (08:30-16:30 weekday / 08:00-16:00 weekend) usage - that flow stays with the pharmacy's normal process; this app covers **out-of-hours only**.
- Does not persist patient data to disk (the report lists drugs and quantities, never patients).

## 2. Hard rules (non-negotiable)

1. **Never write to the HOSxP database.** No `INSERT`/`UPDATE`/`DELETE`/`ALTER`/DDL of any kind, ever - not even in tests. All DB access goes through `crates/hosxp-connector`, and that crate must remain SELECT-only end to end (DB user grants, session mode, query guard, parameterized queries - see section 5).
2. **Never log secrets or PII.** Connection values and drug-usage parameters are never logged; log timing and error types only.
3. **Parameterized queries only.** Never build SQL by string concatenation. Dynamic `IN (?, ?, ...)` placeholder lists are built by repeating `?` only - values are always bound.
4. **No schema guessing without a flag.** HOSxP table/column names are best-effort until verified against the real instance. Query constants touching unverified tables/columns must carry a `// SCHEMA-UNVERIFIED:` comment until confirmed by the DBA (see section 6 - this includes the `opitemrece` naming question).
5. **Thai belongs to the edges.** Thai UI strings live in `app/` (frontend) and `src-tauri/commands.rs` (user-facing error messages). Crate code, comments, and docs stay in English.

## 3. Workspace layout (ports-lite)

```
er-fill/
├── src-tauri/            # Tauri 2 shell - thin command adapters only, no business logic, no SQL
├── crates/
│   ├── models/           # Domain types + Thai/BE formatting. No sqlx, no I/O.
│   ├── core/             # Window algorithm, catalog ops, report builder, repository trait + mock
│   └── hosxp-connector/  # The only crate allowed to talk to MySQL. Read-only guard lives here.
└── app/                  # Leptos 0.8 CSR/WASM frontend (separate wasm workspace, Trunk)
    └── style/            # CSS implementing DESIGN.md tokens
```

Dependency direction: `app` → `er-fill-core` → `er-fill-models`, and `er-fill-hosxp-connector` implements a repository trait defined in `er-fill-core`. `er-fill-core` must be testable with a mock repository, without a real database. The `app` workspace also depends on `er-fill-core`/`er-fill-models` directly (both are wasm-compatible - no tokio, no sqlx on their dependency path).

## 4. Domain model (`crates/models`)

- `BusinessSchedule { weekday: TimeRange, weekend: TimeRange, cutoff: NaiveTime }` - `TimeRange { start, end }`; defaults: weekday 08:30-16:30, weekend (Sat-Sun) 08:00-16:00, cutoff 08:00. Serialized on the wire with `"HH:MM"` times.
- `RequisitionWindow { start: NaiveDateTime, end: NaiveDateTime }` - the half-open window `[start, end)`.
- `Catalog { categories: Vec<DrugCategory>, drugs: Vec<ErDrug> }` - `DrugCategory { id, name, sort_order }`, `ErDrug { icode, name, strength_text, stock_text, category_id, sort_order }`. Seeded with the six categories of the approved ER list (see `docs/reference/er-drug-list.md`); drugs are added from HOSxP `drugitems` search, which prefills `strength_text` from `strength`/`units` (e.g. "500 mg") so same-name drugs stay distinguishable.
- `DrugItem { icode, name, strength, units }` - a `drugitems` search hit.
- `UsageRow { icode, qty }` - one grouped usage result from HOSxP.
- `ReportSection { category_id, category_name, rows: Vec<ReportRow> }`, `ReportRow { icode, name, strength_text, stock_text, usage, requested }` - the printable table; `requested` starts equal to `usage` and is editable in the UI before printing.
- `ReportMeta { hospital_name, document_title, department_name }` - the printed header block; defaults for Sra Bos Hospital.

Thai/Buddhist-era formatting (`be_year`, `format_thai_date*`, `format_qty`) lives in `models::thai` so the UI and the print sheet share one implementation.

## 5. Strict read-only - enforcement mechanism

Same layered approach as the sibling AllerX project:

1. **DB user level:** a MySQL user dedicated to ER-Fill, `GRANT SELECT` only. This is the real security boundary.
2. **Connection level:** `SET SESSION TRANSACTION READ ONLY;` immediately after connect (`after_connect` hook), plus a best-effort `SET SESSION max_execution_time = 5000` (SELECT timeout).
3. **Application level:** `readonly_guard.rs` rejects any outgoing statement that is not a single `SELECT`/`WITH` (comment-aware, semicolon-aware). Defense in depth, not the primary boundary.
4. **Queries:** sqlx with bound parameters only; SQL strings are compile-time constants (or assembled from constants with `?` repetition).
5. **Tauri capabilities:** `core:default` only - no fs/shell scopes.

## 6. HOSxP integration - schema notes (confirm against the live instance)

> **Naming note:** the original request referred to the table as `opitemrace`;
> every verified reference (HOSxP table lists, `hosxp-dash` in this
> workspace) uses **`opitemrece`** - "รายการสั่งยา ค่ารักษา ค่าบริการต่างๆ ทั้ง
> OPD, IPD". The connector is written against `opitemrece`; if the hospital
> genuinely has a custom `opitemrace` table/view, swap the constant behind a
> DBA check (docs/database.md B1). All usage queries carry
> `// SCHEMA-UNVERIFIED:` until the DBA confirms.

### 6.1 Drug search (`drugitems`)
- Fields used: `icode`, `name`, `strength`, `units`.
- Standard HOSxP convention: drug icodes start with `1` (`icode LIKE '1%'`) - filters out non-drug items that share the table family. Confirm on the instance.
- `strength` + `units` are shown in the picker as the drug's strength label (e.g. "500 mg", via `DrugItem::strength_label`); both may be empty on some instances - confirm the `strength` column with the DBA.
- Escape `\`, `%`, `_` in the operator's term before wrapping in `%...%` (contains match).

### 6.2 Usage window (`opitemrece`)
- Fields used: `icode`, `qty`, `vstdate` (date), `vsttime` (time), `an` (admission number - empty/NULL for OPD visits; ER visits are OPD).
- Every query must filter `qty > 0` (zero/negative rows are billing corrections, not dispensing events).
- Filter `(an IS NULL OR an = '')` - ER visits are OPD visits; this keeps inpatient-stay usage (e.g. ward NSS) out of an ER requisition. **Verify with the pharmacist** whether any ER usage is ever billed under an `an`.
- Time filtering is index-friendly decomposition, window `[start, end)`:
  `(vstdate > d1 OR (vstdate = d1 AND vsttime >= t1)) AND (vstdate < d2 OR (vstdate = d2 AND vsttime < t2))`.
- `vstdate`/`vsttime` (visit time) is used, consistent with AllerX; `rxdate`/`rxtime` (dispense time) is a candidate alternative - confirm with the DBA which better reflects "drug left the ER cupboard".

### 6.3 Open questions for the DBA (docs/database.md section B)
- Confirm `opitemrece` vs `opitemrace` naming on this instance.
- Confirm `vsttime` reliability (populated for all drug lines?).
- Confirm drug-type filter (`icode LIKE '1%'` vs `drugitems.istype`).
- Existing indexes on `opitemrece(icode, vstdate)` - daily window queries should stay < 300 ms.

## 7. Requisition window algorithm

Out-of-hours only. The window for a morning run on day `D` is
`[closing(day before D), cutoff(D))`:

- `closing(day)` = that day's end of business hours (Mon-Fri 16:30, Sat-Sun 16:00).
- `cutoff` = the daily requisition time, default **08:00** (configurable).
- The window **ends at the cutoff**, and the remainder until business opens
  (08:00-08:30 on weekdays) stays with the in-hours process - this matches the
  approved workflow.

Examples (defaults):

| Run morning | Window starts (previous closing) | Window ends |
|---|---|---|
| Monday | Sunday 16:00 | Monday 08:00 |
| Tuesday | Monday 16:30 | Tuesday 08:00 |
| Saturday | Friday 16:30 | Saturday 08:00 |
| Sunday | Saturday 16:00 | Sunday 08:00 |

The operator can always override both endpoints in the UI (`window_custom`
state); the printed form always shows the window actually used, and the UI
marks an overridden window in amber so it is never mistaken for the default.

## 8. Report flow

Document-first UI (DESIGN.md "Structure"): the command bar holds the run
controls; the stage shows the A4 sheet, which is one DOM/stylesheet shared
by screen and print.

1. Operator opens the app (default: today): the window is computed, and -
   the morning ritual - usage **auto-fetches once** when the connection and
   catalog are ready (the command bar can refetch any time).
2. "ดึงข้อมูล" reads usage for **all configured icodes at once** (one grouped
   query) for the window.
3. `er-fill-core::build_report` joins catalog + usage into category sections;
   drugs with no usage show `0` (the pharmacy sees they were not used);
   each row carries the drug's `strength_text` beside its name.
4. Operator may edit each row's "จำนวนเบิก" (defaults to usage) directly on
   the sheet.
5. "พิมพ์ใบเบิก" calls `window.print()` on the on-screen sheet - the preview
   IS the printed output (grouped by category, with the window sentence,
   hospital header, and ผู้เบิก/ผู้จ่าย signature blocks). The sheet
   paginates at **40 drug rows per A4 page** (`er-fill-core::paginate_sections`)
   so the 69-item pilot list prints on two sheets; split categories continue
   with "(ต่อ)" and the signatures render on the last page only.

## 9. Performance

- Pool size ~5 (`MySqlPoolOptions`), 5 s acquire timeout - a single-hospital
  desktop app.
- One grouped query per requisition run (not one per drug).
- Daily volume is small; correctness beats microseconds. Do not add caching.

## 10. Security & privacy

- Connection credentials: every field encrypted at rest with `encryptman`
  (AES-256-GCM), master key in the OS keychain via `encryptman-keyring`
  (service name `er-fill`). Password is `secrecy::SecretString` end to end in
  Rust, zeroized on drop, `Debug`-redacted. Documented residuals: the webview
  JS heap while typing, and sqlx's internal pool copy (needed for reconnects).
- The plain settings file (`er-fill.settings.json`) holds the schedule, catalog,
  and report header only - never credentials.
- The frontend never touches the database; it only calls Tauri commands.
- No patient data is ever displayed or stored - only aggregated quantities.

## 11. Milestones

| Milestone | Scope |
|---|---|
| **M0** | Workspace setup, AGENTS-RUST.md, Tauri 2 + Leptos 0.8 CSR scaffold, connection smoke test (`SELECT 1`) |
| **M1** | Schema verification with the DBA (docs/database.md section B), remove `SCHEMA-UNVERIFIED` markers |
| **M2** | Catalog management (categories + drug picker from `drugitems`) end-to-end |
| **M3** | Window computation + usage fetch + requisition table (+ overrides) |
| **M4** | Print sheet (A4) reviewed against the real paper form (ใบเบิกยา xlsx) |
| **M5** | Packaging (Tauri bundle), internal pilot with the pharmacy department, feedback |

## 12. Documentation-first workflow

1. If a task changes behavior, update this file and/or `docs/DESIGN.md` in the
   same commit as the code - don't let docs drift.
2. For anything touching HOSxP schema, verify against the real instance before
   writing the query and remove the `SCHEMA-UNVERIFIED` marker once confirmed.
3. Keep changes scoped to one milestone at a time.

## 13. Testing expectations

- `er-fill-core` + `er-fill-models`: the default suite (`cargo test` at root) -
  window algorithm (all four example days + custom hours + invalid input),
  catalog operations, report building, Thai formatting. No DB required.
- `er-fill-hosxp-connector`: unit tests for the guard and encrypted config run in
  the default suite; live-DB checks are gated behind
  `--features integration-tests` (never production).
- Every SQL constant gets a guard assertion test.
- `app/`: host tests for pure logic; `wasm-bindgen-test` for the IPC mock
  contract (separate workspace - run explicitly).

## 14. Style and conventions

- Rust style/error handling: follow `AGENTS-RUST.md`.
- **Error handling:** one typed `thiserror` error per crate boundary -
  `models::ScheduleError`, `core::CatalogOpsError`, `core::RepositoryError`,
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
