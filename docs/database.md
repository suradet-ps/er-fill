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
-- Verified against the live Sra Bos instance on 2026-10-21 and the 30-day
-- ER audit on 2026-10-22 (sections A.3/A.5).
SELECT o.icode AS icode, CAST(SUM(o.qty) AS DOUBLE) AS qty
FROM opitemrece o
JOIN drugitems d ON d.icode = o.icode
WHERE o.icode IN (?, ?, ...)          -- one ? per configured catalog icode
  AND o.qty > 0
  AND (o.an IS NULL OR o.an = '')
  AND o.dep_code = '009'              -- ordered by the ER department
  AND o.sub_type = '1'                -- drugitems only
  AND IFNULL(o.item_type, '') <> 'A'  -- drop automatic charges
  AND (o.rxdate > ? OR (o.rxdate = ? AND o.rxtime >= ?))
  AND (o.rxdate < ? OR (o.rxdate = ? AND o.rxtime < ?))
GROUP BY o.icode
ORDER BY o.icode
```

- The `drugitems` join plus `sub_type = '1'` and `item_type <> 'A'` keep
  non-drug and automatic charge lines out. The 30-day audit found 1,028
  automatic-charge lines (`item_type = 'A'`) in ER; their `rxtime` equals
  `vsttime`, so they would otherwise look like drug orders.
- ER orders are the ones ordered by the ER department (`dep_code = '009'`);
  the 30-day audit found 5,887 ER lines total and 1,901 drug lines. Do not
  anchor usage on `er_regist.vn` or `ovst.main_dep` - those describe the
  visit, not who ordered the drug. `ovst.spclty = '17'` had zero rows in 90
  days and must never identify ER.
- The window is timed by **`rxdate`/`rxtime`**, the order time written by the
  ER screen (`ERDetailEntryU.RxBtnClick` sets `rxdate := date` and
  `rxtime := now`). `vstdate`/`vsttime` is the visit/arrival time; only 1 of
  1,901 ER drug lines had `rxtime = vsttime` in the audit. Bounds are
  half-open `[start, end)` and decomposed into `(date, time)` pairs.
- `qty > 0` drops billing corrections.
- `an IS NULL OR an = ''` keeps inpatient-stay usage out of the ER
  requisition. On the surveyed instance 0 of 9,639 ER drug orders carry an
  `an`; if an ER patient is admitted, the lines move to the `an` side.
- `opitemrece_log.station = 'ER'` covers only 1,531 rows / 756 visits, and
  only 1,007 of 1,901 drug lines match a log within 60 seconds. It is a
  supplementary audit trail, not a primary filter.
- Do not use `last_modified` as the order time (it is the last edit time),
  `recetime` (NULL on 7,698/7,698 rows), `er_iv_list.iv_time`,
  `er_command.command_datetime`, `er_regist.triage_datetime`, or
  `er_regist.doctor_tx_time`/`finish_time` (auto-filled +5/+10 minutes).
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

### A.3 Verified surveys (Sra Bos, 2026-10-21 / 2026-10-22)

| Area | Observed | Consequence for ER-Refill |
|---|---|---|
| `er_regist` | 302,370 rows (2005-11-26 to 2026-10-21), PK `vn`, joins `ovst.vn` 100% | Visit registry only; not used to filter usage |
| `ovst.spclty = '17'` | 0 rows in 90 days | Never used to identify ER |
| `ovst.main_dep = '009'` | 4,086 / 4,966 ER visits (82%) | Visit-level hint only; not used to filter usage |
| `opitemrece.dep_code = '009'` | 5,887 / 7,698 ER lines (76%); 1,901 drug lines in 30 days | **Primary ER order filter** |
| `opitemrece.sub_type = '1'` | 1,901 drug lines in 30 days | Drug filter alongside the `drugitems` join |
| `opitemrece.item_type = 'A'` | 1,028 automatic charges in ER (30 days) | Exclude: not drugs |
| `opitemrece.rxtime` | Equals `vsttime` on only 1 / 1,901 ER drug lines | **Use `rxdate`/`rxtime` as order time** |
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

### A.5 30-day ER time audit (2026-10-22)

Verified against the live Sra Bos instance for 22 Sep - 22 Oct 2026, plus the
HOSxP source (`ERDetailEntryU.RxBtnClick`):

| Measurement | Result | Consequence |
|---|---|---|
| ER visits | 1,578 | - |
| `opitemrece` lines under ER visits | 7,698 | - |
| `dep_code = '009'` lines | 5,887 / 1,424 visits | Primary ER scope |
| Drug lines (`sub_type = '1'`) | 1,901 / 642 visits | Usage candidates |
| `rxtime = vsttime` on drug lines | 1 / 1,901 | Use `rxtime`, not `vsttime` |
| `opitemrece_log.station = 'ER'` | 1,531 rows / 756 visits; 1,007 / 1,901 drug lines within 60 s | Supplementary audit only |

- HOSxP source confirms the ER screen reads `vstdate`/`vsttime` from `ovst`
  (arrival) and writes `rxdate := date`, `rxtime := now` when the drug order
  is saved.
- `last_modified` is later than `rxtime` on 5,437 / 7,698 lines, so it is an
  edit timestamp, not the order time.

Fields/tables found unusable for this app:

| Field/table | Finding |
|---|---|
| `opitemrece.recetime` | NULL 100% (7,698/7,698) |
| `er_iv_list.iv_time` | 0 rows in 30 days |
| `er_command.command_datetime` | 0 rows |
| `er_regist.triage_datetime` | NULL 100% |
| `er_regist.doctor_tx_time` / `finish_time` | ~1,575/1,578 are `enter_er_time` + 5/10 minutes (auto-filled) |
| `nurse_confirm_rx`, `pq_rxorder` | 0 rows |
| `drugrequestsummary`, `drugrequestdetail` | 0 rows (no department requisition flow) |
| `rx_queue_dispense_log`, `opitemrece_sticker_log` | 0 rows in ER |

## B. Open questions for the DBA (block M1)

**B1. Table naming - resolved.** The live instance has `opitemrece`, not
`opitemrace`; `er_regist`, `ovst`, `drugitems`, and `opd_allergy` also exist
under those names (section A.3). The connector's constants are correct.

**B2. `vsttime` reliability - resolved.** The 30-day audit shows `rxtime`
differs from `vsttime` on 1,900/1,901 drug lines, so visit time is the wrong
clock. The query uses `rxdate`/`rxtime`.

**B3. Order time - resolved.** Use `rxdate`/`rxtime`, written by the ER order
screen. `vstdate`/`vsttime` is arrival time; `last_modified` is an edit
timestamp. `opi_dispense_dru` joins by `vn + item_no`, which is not unique, so
dispense reconciliation remains out of scope.

**B4. Indexes.** Index on `opitemrece(icode, rxdate, rxtime)` (or an
equivalent covering `dep_code`/`sub_type`) - daily-window queries must stay
< 300 ms. Creating an index is data-safe and can be requested separately.

**B5. Character encoding.** Older HOSxP instances may be TIS-620 rather than
UTF-8 - affects Thai name search in the drug picker.

**B6. Read-only account.** Confirm a dedicated `GRANT SELECT` user (e.g.
`errefill_ro`) for the app; this is the real security boundary (`AGENTS.md` section 5).

**B7. ER line scope - resolved.** The query counts drugs ordered by the ER
department (`dep_code = '009'`, `sub_type = '1'`, `item_type <> 'A'`), not
every line under an ER visit. The 30-day audit confirms this is the
order-source scope the pharmacist wants.

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
