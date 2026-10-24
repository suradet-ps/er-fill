# Changelog

All notable changes to ER-Refill are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and versions follow SemVer.

## [Unreleased]

### Security

- Updated `ctutils` to 0.4.5, fixing RUSTSEC-2026-0385 (non-constant-time
  mask generation in AES-GCM's polyval path).

### Changed

- The requisition date is now fixed to today (read-only) and the configurable
  business-hours schedule was removed. The operator must choose one of four
  fixed overnight ranges before fetching: 20:30-08:00, 20:30-08:30,
  16:00-08:00, or 16:00-08:30 (previous day to run day). The command-bar date
  input, manual window editor, and summary stat cards were removed.
- Settings now hold only the catalog and report header (settings file and
  export version 3); legacy files with a `schedule` block still load.
- Polished the command bar: the locked-date caption is gone, the run date is
  now a larger card with a rose calendar mark, the four ranges are a compact
  segmented control, action buttons have stable widths, the stage reserves its
  scrollbar gutter, and messages now appear as a floating toast so the sheet
  never jumps when a banner appears or dismisses.
- The fixed run date now rolls over at midnight even when no range is
  selected. Clearing the requisition also invalidates any in-flight fetch, so
  switching A -> B -> A can not restore the old sheet from the first fetch.
- The printed sheet header now sits left-aligned beside the bear mark, with a
  large bordered box on the right showing the selected time range
  (e.g. 20:30–08:30).
- Simplified the top bar: settings now opens by clicking the bear mark, the
  connection state is a compact green/red dot with a readable tooltip beside
  the subtitle, and the right side shows only the signed-in name and a
  prominent logout pill.
- Redesigned the drug-catalog settings panel: category tabs on top, the
  HOSxP search next, and a scannable single-line drug table below. Search
  results disappear as soon as a drug is added.
- The login illustration now uses the master nurse-bear mascot from
  `icon-master.svg` instead of the older hand-drawn bear.
- ER usage is now scoped to `dep_code = '009'` drug orders (`sub_type = '1'`,
  excluding `item_type = 'A'` automatic charges) and the out-of-hours window
  is computed from `rxdate`/`rxtime` (ER order time) instead of
  `vstdate`/`vsttime` (visit time).
- The login gate now handles the `officer` sign-in only. HOSxP connection
  setup moved entirely into the settings dialog, opened from the login page;
  the connection form/status block is no longer embedded in the login card.
- Replaced the login brand copy and proof points with a custom requisition
  illustration, and trimmed the login/connection panels of the read-only and
  encryption explanatory paragraphs.
- Redrew the app icon and top-bar mark as a nurse bear with a nurse cap and a
  medical heart, regenerated every Tauri platform icon from `icon-master.svg`,
  and re-themed the UI to the rose/bear-gold palette. The printed sheet's
  nurse-bear mark drops the SVG emboss filter so it stays crisp vector art at
  the printer's native resolution.
- The nurse-bear SVG paint servers now live in one root-level defs block, so
  the gold/brown gradient still resolves on printed sheets when the top bar is
  hidden in print media.
- The top-bar name now falls back to `doctor.name` (via `officer_doctor_code`)
  when the officer name columns are blank, then to the login name.
- The printed requisition now fills the ผู้เบิก signature block with the
  signed-in officer's display name.
- The printed requisition shows the drug name + strength only (no icode or
  Thai dispensing-unit suffix); the settings picker keeps the icode and full
  strength label for identification.
- จำนวนเบิก is now derived as whole packs from the จำนวน/Stock setting
  (`units-per-pack/stock-packs`): `ceil(usage / units-per-pack)`, capped at
  the stock pack count; manual edits are rounded and clamped the same way.
- Narrowed the จำนวน/Stock, จำนวนใช้, and จำนวนเบิก columns on the sheet.
- The requisition now shows only drugs with usage in the window; configured
  drugs with no usage can be added manually (usage 0) from the catalog-only
  picker. Every add or จำนวนเบิก change requires a reason, printed in a final
  "ยาที่ขอเพิ่มหรือขอแก้ไขจำนวนเบิก" table (drug name + reason).
- Printing is gated by an explicit confirmation snapshot of the rows, reasons,
  window, and header. The print button stays disabled until confirmation, and
  any later edit invalidates it. The nurse-bear mark now sits at the top-left
  of every printed sheet page.
- Reworked the command bar: labelled date control, compact window card with
  short Thai/BE times and badge, segmented stat strip, a confirmation chip /
  add-drug action, and a deliberate two-row layout below 1280px.
- Redrew the UI icon set as a custom monoline family (1.8px stroke) with new
  pencil, refresh, printer, calendar, clipboard, file, trash, and database
  glyphs.
- Added a login gate: verify the HOSxP connection, then sign in with an
  `officer` account (MD5 check, active/auth-type checks). Sessions are
  in-memory; sensitive commands require a session; the top bar shows the
  signed-in user and a logout action.
- Added `docs/auth.md` with the read-only officer survey, hash-assumption
  caveat, and the deliberately uncreated `app_*` tables.

- Requisition window now ends at the run day's opening time (weekdays
  08:30, weekends 08:00) instead of a configurable cutoff.
- Weekday business hours extended to 08:30-20:30; weekends remain
  08:00-16:00.
- Settings export version 2 (the obsolete `schedule.cutoff` field is
  removed; version 1 files still import).
- On-disk settings version 2: a version 1 file with the old default weekday
  hours (closing 16:30) is upgraded to the new 20:30 closing; custom
  schedules are preserved.

### Fixed

- Saving an edited จำนวนเบิก now closes the reason dialog before the sheet
  refresh, and success/info strips auto-dismiss after 5 seconds, so a saved
  edit no longer leaves a lingering popup/banner.
- Printed sheets no longer carry the browser's automatic title/URL header and
  footer: empty `@page` margin boxes suppress them while the A4 12mm margins
  remain.
- The top-bar officer name no longer clips Thai ascenders, and long names get
  more room before ellipsizing. The command-bar date/window labels also keep
  their full glyph height.

## [0.1.0] - 2026-10-17

### Added

- Initial workspace: `er-refill-models`, `er-refill-core`, `er-refill-hosxp-connector`,
  `er-refill-tauri`, `er-refill-app`.
- Out-of-hours requisition window algorithm (weekday 08:30-16:30, weekend
  08:00-16:00, cutoff 08:00) with Thai/Buddhist-era labels.
- ER drug catalog (categories + drugs) with defaults seeded from the approved
  6-category ER drug list of Sra Bos Hospital.
- HOSxP connector: encrypted connection settings (AES-256-GCM, OS keychain),
  read-only pool, SQL guard, `drugitems` search + `opitemrece` usage query.
- Tauri commands for settings, drug search, and window usage.
- Leptos UI: window panel, requisition table with editable requested
  quantities, settings modal (connection/schedule/catalog/report), printable
  A4 requisition sheet (`window.print()`).
- Portable settings export/import (JSON) in the settings modal footer, for
  configuring one machine and importing the rest; connection credentials are
  intentionally excluded (encrypted per machine).
- Drug search joins `drugitems.strength` and shows the strength label
  (e.g. "500 mg") in the catalog picker.
- Strength propagates everywhere: `ErDrug.strength_text` (prefilled from the
  picker, editable), the catalog drug list, and the requisition sheet/print
  beside the drug name, so same-name drugs stay distinguishable.
- UI redesigned document-first: a command bar (date, window chips, stats,
  fetch, print) over a paper stage; the A4 sheet previews exactly what
  prints (one DOM, one stylesheet), and usage auto-fetches once on launch.
- CI workflows: root and wasm check suites, headless-Chrome wasm tests,
  cargo-deny, Miri on `er-refill-models`, a Tauri test build (plus a
  dispatch-only Windows installer job), and tagged releases via
  tauri-action.
- The sheet paginates explicitly at 40 drug rows per A4 page (69 items
  print on two sheets): repeated headers, "(ต่อ)" continuations for split
  categories, page labels, and signature blocks on the last page only.
