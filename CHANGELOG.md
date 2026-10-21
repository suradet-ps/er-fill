# Changelog

All notable changes to ER-Fill are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and versions follow SemVer.

## [Unreleased]

### Changed

- Requisition window now ends at the run day's opening time (weekdays
  08:30, weekends 08:00) instead of a configurable cutoff.
- Weekday business hours extended to 08:30-20:30; weekends remain
  08:00-16:00.
- Settings export version 2 (the obsolete `schedule.cutoff` field is
  removed; version 1 files still import).
- On-disk settings version 2: a version 1 file with the old default weekday
  hours (closing 16:30) is upgraded to the new 20:30 closing; custom
  schedules are preserved.

## [0.1.0] - 2026-10-17

### Added

- Initial workspace: `er-fill-models`, `er-fill-core`, `er-fill-hosxp-connector`,
  `er-fill-tauri`, `er-fill-app`.
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
  cargo-deny, Miri on `er-fill-models`, a Tauri test build (plus a
  dispatch-only Windows installer job), and tagged releases via
  tauri-action.
- The sheet paginates explicitly at 40 drug rows per A4 page (69 items
  print on two sheets): repeated headers, "(ต่อ)" continuations for split
  categories, page labels, and signature blocks on the last page only.
