# Database notes - HOSxP integration

ER-Fill reads from the hospital's HOSxP MySQL/MariaDB instance **read-only**
(`AGENTS.md` sections 2 and 5). This file documents what the connector touches, what is
verified, and what the DBA must confirm.

## A. Current access surface

### A.1 `drugitems` (drug search)

```sql
-- SCHEMA-UNVERIFIED: confirm columns on the instance before pilot.
SELECT icode, COALESCE(name, '') AS name,
       COALESCE(strength, '') AS strength, COALESCE(units, '') AS units
FROM drugitems
WHERE icode LIKE '1%'
  AND (icode LIKE ? OR name LIKE ?)
ORDER BY name
LIMIT 50
```

- `icode LIKE '1%'` - standard HOSxP drug-category prefix (non-drug items live
  in `nondrugitems`/other prefixes). Confirm.
- `strength` and `units` join along for the picker's strength label (e.g.
  "500 mg"). **Confirm the `strength` column exists** on this instance; both
  fields may be empty.
- Operator term is escaped (`\`, `%`, `_`) then wrapped `%...%`; contains-match
  on name for Thai partial words.

### A.2 `opitemrece` (usage in window)

```sql
-- SCHEMA-UNVERIFIED: confirm column names + the opitemrace question (section B1).
SELECT o.icode AS icode, CAST(SUM(o.qty) AS DOUBLE) AS qty
FROM opitemrece o
WHERE o.icode IN (?, ?, ...)          -- one ? per configured catalog icode
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND (o.vstdate > ? OR (o.vstdate = ? AND o.vsttime >= ?))
  AND (o.vstdate < ? OR (o.vstdate = ? AND o.vsttime < ?))
GROUP BY o.icode
ORDER BY o.icode
```

- Window is half-open `[start, end)`; bounds are decomposed into
  `(date, time)` pairs so `vstdate` index scans stay usable.
- `qty > 0` drops billing corrections.
- `an IS NULL OR an = ''` keeps inpatient-stay usage out of the ER
  requisition (ER visits are OPD visits). **Pharmacist to confirm** no ER
  usage is ever billed under an `an`.
- The `IN (...)` placeholder list is built by repeating `?` - values are always
  bound, never interpolated.

## B. Open questions for the DBA (block M1)

**B1. Table naming - `opitemrece` vs `opitemrace`.**
The original request referred to the table as `opitemrace`. All verified
references (published HOSxP table lists; the `hosxp-dash` project in this
workspace) use `opitemrece` - "รายการสั่งยา ค่ารักษา ค่าบริการต่างๆ ทั้ง OPD,
IPD". Run:

```sql
SHOW TABLES LIKE 'opitem%';
SHOW COLUMNS FROM opitemrece;
```

If a custom `opitemrace` view/table genuinely exists, tell the developers -
it is a one-constant change in `crates/hosxp-connector/src/queries.rs`.

**B2. `vsttime` reliability.** Is `vsttime` populated for every drug line on
this instance? (If not, the window query needs a `rxtime` fallback strategy.)

**B3. Visit time vs dispense time.** Which better reflects "drug left the ER
cupboard" - `vstdate/vsttime` (visit) or `rxdate/rxtime` (dispense)? Current
implementation uses visit time (consistent with AllerX); switching is a
constant swap.

**B4. Indexes.** Existing index on `opitemrece(icode, vstdate)` (or an
equivalent)? Daily-window queries must stay < 300 ms. Creating an index is
data-safe and can be requested separately.

**B5. Character encoding.** Older HOSxP instances may be TIS-620 rather than
UTF-8 - affects Thai name search in the drug picker.

**B6. Read-only account.** Confirm a dedicated `GRANT SELECT` user (e.g.
`erfill_ro`) for the app; this is the real security boundary (`AGENTS.md` section 5).

## C. Access-control layers (code side)

1. `SET SESSION TRANSACTION READ ONLY` on every pooled connection.
2. Best-effort `SET SESSION max_execution_time = 5000` (server-side SELECT
   timeout; MariaDB ignores it, which is tolerated).
3. `readonly_guard::assert_read_only` rejects any statement that is not a
   single `SELECT`/`WITH`.
4. Constant SQL + bound parameters only.

## D. Integration test hook

Live-DB tests (never production) are gated behind the connector's
`integration-tests` feature and read `ER_FILL_TEST_DATABASE_URL`:

```bash
cd crates/hosxp-connector
cargo test --features integration-tests
```
