# ER-Fill

[![CI](https://github.com/suradet-ps/er-fill/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/suradet-ps/er-fill/actions/workflows/ci.yml)
[![Rust Safety Check](https://github.com/suradet-ps/er-fill/actions/workflows/rust-safety.yml/badge.svg?branch=main)](https://github.com/suradet-ps/er-fill/actions/workflows/rust-safety.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust: stable](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![Leptos 0.8](https://img.shields.io/badge/Leptos-0.8-blue.svg)](https://leptos.dev)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/suradet-ps/er-fill/issues)

---

## ◆ PULSE

The ER cupboard empties at night; the requisition must be ready by morning.
ER-Fill reads the hospital's [HOSxP](https://hosxp.org/) record, strictly
read-only, and sums how much of the configured emergency drug list was used
**outside business hours** - Sunday 16:00 to Monday 08:30 for a Monday run -
then prints the Thai requisition sheet (ใบเบิกยาประจำห้องฉุกเฉินรายวัน) for
the ผู้เบิก / ผู้จ่าย signatures. The window is computed for the operator,
stated on the paper, and overridable; HOSxP is never written, nothing is
cached, and no patient appears anywhere - only aggregated quantities.

| M0 scaffold ▣ | M1 schema ☐ | M2 catalog ▣ | M3 usage ▣ | M4 print ◐ | M5 pilot ☐ |
|---|---|---|---|---|---|

*The requisition pipeline - window, catalog, usage read, printable sheet -
is sealed. The paper-form review against the real xlsx, the DBA sign-off,
and the pharmacy department pilot wait ahead.*

> Built with Tauri 2 + Leptos 0.8, decided by `er-fill-core`, read from
> HOSxP `opitemrece` by `er-fill-hosxp-connector` - never a write, never a
> silent guess.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

Two installs, one test, one launch.

```
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked
⟫ cargo test            # DB-free: er-fill-core runs against a mock repository
⟫ cargo tauri dev       # desktop app; trunk serves itself
```

The release artifact: `⟫ cargo tauri build` - output lands in
`src-tauri/target/release/bundle`.

<details>
<summary>Notes for clinic PCs</summary>

- Windows needs the WebView2 Runtime - preinstalled on updated Windows 10/11;
  offline machines should take Microsoft's Evergreen offline installer first.
- Installers are **unsigned for v0.x**: expect the SmartScreen prompt
  (*More info -> Run anyway*) and coordinate AV whitelisting with hospital IT.
- The catalog is configured once through the `drugitems` search (the approved
  69 items across 6 categories are the checklist in
  `docs/reference/er-drug-list.md`) and travels to other machines from the
  settings modal: **ส่งออกการตั้งค่า / นำเข้าการตั้งค่า** as a JSON file.
- Connection settings never travel: the password rests encrypted per machine
  in the OS keychain - each PC enters its own once.

</details>

---

## ◆ ANATOMY

Four crates, one hard law: HOSxP is read-only, without exceptions.

- **Decides** - `er-fill-core` is pure Rust with the morning ritual in it:
  the out-of-hours window algorithm (previous closing to today's opening),
  catalog operations, and the report builder that joins the configured drug
  list with usage into category sections. Tested against a mock repository -
  no live database in the default suite, ever.
- **Reads** - `er-fill-hosxp-connector` talks to MySQL/MariaDB through
  `sqlx`: one grouped `SELECT` sums `opitemrece.qty` inside the window for
  every configured icode, joined to `drugitems` (so non-drug charge lines
  never inflate the count) and anchored to `er_regist.vn` (with
  `ovst.main_dep = '009'` as fallback). The drug picker searches `drugitems`
  and brings `strength`/`units` (e.g. "500 mg") along. Every SQL constant
  carries a guard test asserting it is a single read statement - no writes
  of any kind.
- **Prints** - `app` is the Leptos frontend, document-first: the command bar
  holds the run controls, and the stage shows the A4 sheet itself - the
  same DOM and stylesheet that print. The sheet states the window as a Thai
  sentence (Buddhist-era), shows จำนวนใช้ next to an editable จำนวนเบิก, and
  carries the ผู้เบิก/ผู้จ่าย signature blocks; พิมพ์ใบเบิก just calls
  `window.print()` on what is already on screen.
- **Seals** - read-only is enforced in layers: a dedicated user with
  `GRANT SELECT` only, `SET SESSION TRANSACTION READ ONLY` on every pooled
  connection, an application-level guard rejecting anything but a single
  read statement, parameterized queries everywhere. Credentials rest
  AES-256-GCM-encrypted with the key in the OS keychain; the password is a
  `secrecy::SecretString` end to end, zeroized on drop, `Debug`-redacted.
  The rest of the settings - schedule, catalog, report header - live in a
  plain JSON file and leave the machine only when the operator exports them.
- **Labels** - Thai/Buddhist-era formatting lives in `er-fill-models`
  (`วันอาทิตย์ที่ 18 ตุลาคม 2569 เวลา 16:00 น.`), one implementation shared by
  the screen and the paper.

---

## ◆ RITUALS

**The core ceremony** - the morning requisition:

1. Open the app on the run morning: the window is computed and the usage
   fetches itself - Sunday 16:00 → Monday 08:30 for a Monday, with the
   "อัตโนมัติ" chip on the window.
2. The sheet is already staged: one grouped read returned every configured
   drug's out-of-hours usage, with strength beside each name.
3. Review the paper grouped by หมวด; where reality differs from the record,
   adjust จำนวนเบิก directly in its box - the preview is exactly what prints.
4. กด "พิมพ์ใบเบิก": the A4 pages (40 items per page, so the 69-item list
   prints on two sheets, "(ต่อ)" where a category continues) go to the ER
   to sign and back to the pharmacy to fill.

**The ceremony of the honest window** - the printed form always shows the
window it was built from, as a full Thai sentence. A manual override turns
the command-bar chip amber; the requisition never hides which hours it
covers.

**The ceremony of restraint** - the app reads a record and prints a paper.
It stores no patient data and keeps no per-day history that could become a
second version of the truth. The signed paper is the record of the morning.

---

## ◆ ECHOES

**Where this artifact is heading**

```
window  ▸ out-of-hours algorithm, overridable, amber-marked ────── ▸ sealed
catalog ▸ 6 หมวด + drugitems picker (strength joined) ───────────── ▸ sealed
usage   ▸ one grouped, ER-anchored opitemrece read per run ──────── ▸ sealed
print   ▸ A4 ใบเบิก + Thai/BE labels + signatures ───────────────── ▸ sealed
travel  ▸ JSON export/import of schedule, catalog, report ──────── ▸ sealed
schema  ▸ DBA sign-off: opitemrece columns, vsttime, indexes ────── ▸ ahead
pilot   ▸ pharmacy department, real mornings, feedback ─────────── ▸ ahead
```

**Raising the artifact** - read `AGENTS.md` first, especially the hard
rules: HOSxP read-only without exceptions, no secrets or PII in logs,
parameterized queries only, `SCHEMA-UNVERIFIED` markers until the DBA
confirms. The visual system is `docs/DESIGN.md`; the DBA checklist lives in
`docs/database.md` section B; the approved 69-item ER list in
`docs/reference/er-drug-list.md`.

**Status** - every change runs fmt, `clippy -D warnings`, the DB-free test
suite, `cargo doc`, `cargo-deny`, and a `trunk build` of the wasm frontend.

> ER-Fill prints what HOSxP recorded. The pharmacist counts, signs, and
> dispenses - the paper is the contract.

---

```
  ─────────────────────────────────────────
   A drug missing from the ER cupboard is
   not a statistic. It is tomorrow's first
   requisition.
  ─────────────────────────────────────────
```

ER-Fill is distributed under the [MIT License](LICENSE).
