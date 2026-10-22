# Database notes - HOSxP integration

ER-Refill reads from the hospital's HOSxP MySQL/MariaDB instance **read-only**
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
-- Verified against the live Sra Bos instance on 2026-10-21 (section A.3).
SELECT o.icode AS icode, CAST(SUM(o.qty) AS DOUBLE) AS qty
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.icode IN (?, ?, ...)          -- one ? per configured catalog icode
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND (
    EXISTS (SELECT 1 FROM er_regist e WHERE e.vn = o.vn)
    OR EXISTS (SELECT 1 FROM ovst v WHERE v.vn = o.vn AND v.main_dep = '009')
  )
  AND (o.vstdate > ? OR (o.vstdate = ? AND o.vsttime >= ?))
  AND (o.vstdate < ? OR (o.vstdate = ? AND o.vsttime < ?))
GROUP BY o.icode
ORDER BY o.icode
```

- The `drugitems` join is **mandatory**: `opitemrece` mixes drugs and
  non-drug charge lines (6,822,872 rows total, only 3,160,622 drugs on the
  surveyed instance). Without it the requisition quantities inflate ~2.2x.
- ER visits are anchored on `er_regist.vn` (302,370 rows, primary key `vn`,
  1:1 with `ovst`). `ovst.main_dep = '009'` ("ห้องอุบัติเหตุฉุกเฉิน") is a
  fallback only. `ovst.spclty = '17'` had zero rows in 90 days and must never
  be used to identify ER.
- `opitemrece.dep_code = '009'` appears on only 66% of ER drug orders; it is
  not used as a primary filter.
- Window is half-open `[start, end)`; bounds are decomposed into
  `(date, time)` pairs so `vstdate` index scans stay usable.
- `qty > 0` drops billing corrections.
- `an IS NULL OR an = ''` keeps inpatient-stay usage out of the ER
  requisition. On the surveyed instance 0 of 9,639 ER drug orders carry an
  `an`; if an ER patient is admitted, the lines move to the `an` side.
- Aggregation is by `icode`, so the `vn + item_no` uniqueness problem does
  not affect the summed quantity. A future line-level feature must use
  `opitemrece.hos_guid`, not `vn + item_no` (duplicate on 10,945 of 44,948
  rows in a 30-day sample).
- The query deliberately does **not** join `opi_dispense*`. Dispense rows
  join by `vn + item_no`, which is not unique, so a naive join would
  fan out and double-count. Dispense reconciliation remains a future
  read-only feature.
- The `IN (...)` placeholder list is built by repeating `?` - values are always
  bound, never interpolated.

### A.3 Verified survey (Sra Bos, 2026-10-21)

| Area | Observed | Consequence for ER-Refill |
|---|---|---|
| `er_regist` | 302,370 rows (2005-11-26 to 2026-10-21), PK `vn`, joins `ovst.vn` 100% | Anchor ER usage on `er_regist.vn` |
| `ovst.spclty = '17'` | 0 rows in 90 days | Never used to identify ER |
| `ovst.main_dep = '009'` | 4,086 / 4,966 ER visits (82%) | Fallback anchor only |
| `opitemrece.dep_code = '009'` | 6,402 / 9,637 ER drug orders (66%) | Supplemental signal, not a primary filter |
| `opitemrece` composition | 6,822,872 rows: 3,160,622 drugs, 3,661,892 non-drugs, 358 other | Always join `drugitems` |
| `opitemrece` line key | `vn + item_no` duplicated on 10,945 of 44,948 rows in 30 days (`item_no` is `tinyint`) | Use `hos_guid` for line identity |
| ER `an` | 0 / 9,639 ER drug orders have `an` | Keep the empty-`an` filter |
| Prescriber | `opitemrece.doctor` differs from `er_regist.er_doctor` on 5,132 / 9,639 (53%) | Two doctor concepts if line detail is ever added |
| Sig | `drugusage` populated 55,734 / 123,904 (45%); ER missing 1,042 / 9,639 (10.8%) | Require Sig at line level if a line-level module is added |
| Dispensing | `opi_dispense` 2,431,625 rows; `opi_dispense_dru` 948,498 rows, matches 935,057 / 948,498 orders (98.6%) | Evidence of actual dispensing; not safe to aggregate by `vn + item_no` |
| HOSxP stock module | `stock_department*`, `stock_dep_*`, `drugstock` all 0 rows | No dependency; ER-Refill does not track stock |
| Unused tables | `dispensing_drug`, `nurse_confirm_rx`, `drug_delivery`, `opitemrece_drug_control`, `er_iv_list`, `iptdrugorder` | Not used |
| Safety tables | `opd_allergy.agent` is a drug name (no icode); `drug_interaction` uses names | Patient-level checks are outside this aggregate app |

### A.4 What was deliberately not adopted

The survey also proposed a read model (`v_er_visit`, `v_er_drug_order`,
`v_er_drug_dispense`) and new requisition/stock tables. ER-Refill cannot adopt
them as DDL because the app is read-only (AGENTS.md section 2): `CREATE VIEW`
and `CREATE TABLE` are writes. The equivalent read model is the inline
`SELECT` in A.2, compiled into `crates/hosxp-connector/src/queries.rs`.

The `er_drug_store`, `er_drug_stock`, `er_drug_requisition*`,
`er_drug_stock_txn`, and `er_drug_safety_check` tables are not part of this
app: ER-Refill does not track stock, does not persist per-day requisitions,
and never handles patient-level data. They belong to a separate system if
the hospital decides to build one.

## B. Open questions for the DBA (block M1)

**B1. Table naming - resolved.** The live instance has `opitemrece`, not
`opitemrace`; `er_regist`, `ovst`, `drugitems`, and `opd_allergy` also exist
under those names (section A.3). The connector's constants are correct.

**B2. `vsttime` reliability.** Is `vsttime` populated for every drug line on
this instance? The 2026-10-21 survey did not test the null distribution. If
not, the window query needs a `rxtime` fallback strategy.

**B3. Visit time vs dispense time.** Which better reflects "drug left the ER
cupboard" - `vstdate/vsttime` (visit) or `rxdate/rxtime` (dispense)? Current
implementation uses visit time (consistent with AllerX); switching is a
constant swap. `opi_dispense_dru` matches 98.6% of orders but joins by
`vn + item_no`, which is not unique - any move to dispense time needs a safe
line key first.

**B4. Indexes.** Existing index on `opitemrece(icode, vstdate)` (or an
equivalent)? Daily-window queries must stay < 300 ms. Creating an index is
data-safe and can be requested separately.

**B5. Character encoding.** Older HOSxP instances may be TIS-620 rather than
UTF-8 - affects Thai name search in the drug picker.

**B6. Read-only account.** Confirm a dedicated `GRANT SELECT` user (e.g.
`errefill_ro`) for the app; this is the real security boundary (`AGENTS.md` section 5).

**B7. ER line scope.** The query counts every drug line under an ER visit,
not only `dep_code = '009'` (which covers 66% of ER drug orders). Confirm
with the pharmacist that this matches "what left the ER cupboard"; if not,
the next candidate is `dep_code = '009'` as an additional predicate.

## C. Access-control layers (code side)

1. `SET SESSION TRANSACTION READ ONLY` on every pooled connection.
2. Best-effort `SET SESSION max_execution_time = 5000` (server-side SELECT
   timeout; MariaDB ignores it, which is tolerated).
3. `readonly_guard::assert_read_only` rejects any statement that is not a
   single `SELECT`/`WITH`.
4. Constant SQL + bound parameters only.

## D. Integration test hook

Live-DB tests (never production) are gated behind the connector's
`integration-tests` feature and read `ER_REFILL_TEST_DATABASE_URL`:

```bash
cd crates/hosxp-connector
cargo test --features integration-tests
```
