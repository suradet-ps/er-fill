# DESIGN.md - ER-Refill

## Overview

ER-Refill is used by a pharmacist once a day, in the morning, to produce one
paper: the ER drug requisition form. The interface therefore borrows two
things from the Mintlify language - **cinematic atmosphere on the rare empty
states** (gradient hero bands) and **documentation-grade density everywhere
work actually happens** - but re-anchors the palette on Thai clinical paper
work.

**The signature element is the sheet** - the A4 requisition itself, staged on
a soft desk surface as a live preview: hospital header, the window sentence
in full Thai ("วันอาทิตย์ที่ 18 ตุลาคม 2569 เวลา 16:00 น. ถึง วันจันทร์ที่ 19
ตุลาคม 2569 เวลา 08:30 น."), the grouped drug table with editable จำนวนเบิก
cells, and the ผู้เบิก/ผู้จ่าย signature blocks. The screen and the printer
share one DOM and one stylesheet - what the pharmacist verifies is exactly
what prints; there is no separate preview mode. The command bar above the
desk is the only control surface, and it states the window compactly
(mint = auto, amber = overridden - never silent).

**Key characteristics:**

- **Atmospheric gradient hero bands, used once**: the login gate gets the
  sky-gradient brand panel; that is the only marketing-grade surface in the
  app. Working surfaces are flat and dense.
- **Signature mint green** ({colors.brand-green}) reserved for accent CTAs,
  the auto window chip, active states, and feature confirmations - never body
  text or large surfaces.
- **Black-pill primary buttons** ({colors.primary} + `{rounded.full}`) for the
  dominant action (พิมพ์ใบเบิก); one mint accent CTA per viewport at most.
- **Inter for UI prose, IBM Plex Sans Thai for Thai text, IBM Plex Mono for
  codes and quantities** - numbers must align in columns.
- **Command bar + paper stage, not a sidebar**: the run controls (date,
  window, fetch, stats, print) live in one toolbar; the stage below shows
  the paper. The job is producing a document, not looking something up.
- **Tightly-controlled radius scale**: the sheet is paper (no radius); cards
  `{rounded.lg}` (12px), compact UI `{rounded.md}` (8px), all buttons and
  pills `{rounded.full}`.
- **One warm alert tone**: amber ({colors.warn-*}) for "this needs attention"
  (custom window, degraded connection, unmapped drug) - deliberately distinct
  from the error red used for required-field validation.

### Adaptation Notes - what changed from the Mintlify reference

| Reference token | ER-Refill token | Why |
|---|---|---|
| Mintlify mint `#00D4A4`-family | `{colors.brand-green}` `#00A87E` ("เขียวตราประทับ" seal green) | Same role; deepened one step so it holds WCAG AA on white for accent text and reads as ink-stamp green, not neon. |
| Inter + Geist Mono only | Inter + **IBM Plex Sans Thai** + IBM Plex Mono | The app is Thai-first; Inter has no Thai glyphs. Plex Thai pairs with Plex Mono and matches the sibling AllerX system. |
| Marketing scale (16px body, 72px hero) | Tool scale (15px body, 28px display) | This is a once-a-day work tool, not a landing page; density wins. The display scale exists **only** on the login brand headline (34px at desktop). |
| Testimonial orange | **Dropped** | No testimonial/marketing surfaces exist. The warm slot is filled by amber, which carries clinical meaning (attention), not emotion. |
| `rgba(0,212,164,.08) 0 8px 24px` featured glow | Kept, retinted | The mint glow marks exactly one thing: the single accent CTA on an empty state. |
| Photography/clouds/rockets | None | No illustrations. The only gradient art is the login brand panel. |

## Colors

### Brand & Accent

- **Seal Mint** ({colors.brand-green}): `#00A87E` - brand seal and accent CTAs,
  auto window chip border, green checkmarks in confirmation lists, active
  tab underline, sidebar active indicator. Never for body text.
- **Deep Mint** ({colors.brand-green-deep}): `#00795B` - pressed/active variant;
  success text on soft mint backgrounds.
- **Soft Mint** ({colors.brand-green-soft}): `#E4F7F0` - success tint, auto
  auto window chip background, the sheet window sentence tint.
- **Seal Glow** ({colors.brand-glow}): `rgba(0, 168, 126, 0.08)` - the single
  featured shadow, reserved for one brand accent; not used in the current UI.

### Semantic

- **Alert Red** ({colors.error}): `#D92D20` - required-field labels, validation
  messages, destructive confirmations. Never decorative.
- **Alert Soft** ({colors.error-soft}): `#FEECEB` - error field background tint.
- **Warn Amber** ({colors.warn-text} `#8A6420` on {colors.warn-bg} `#FFF8E1`,
  border {colors.warn-border} `#FFE082`) - system attention states: overridden
  run-window chip, degraded HOSxP connection banner, drug not yet mapped. This
  amber is a *system* warning; it never means "answer found/not found" (there
  is no such verdict in this app).
- **Info Blue** ({colors.tag-blue}): `#3772CF` on {colors.tag-bg}
  `rgba(55, 114, 207, 0.12)` - informational chips (icode, unit, "OPD" tag).

### Surface

- **Canvas** ({colors.canvas}): `#FFFFFF` - the printable sheet's color and
  the stage.
- **Surface** ({colors.surface}): `#F6F6F4` - section backgrounds, search
  pill rest, sheet table header rows, the command-bar stat strip.
- **Surface Soft** ({colors.surface-soft}): `#FAFAF8` - quieter section
  bands, zebra striping off (we keep rows flat; this is for framing).
- **Hairline** ({colors.hairline}): `#E6E6E3` - 1px borders and dividers.
- **Hairline Soft** ({colors.hairline-soft}): `#EFEFEC` - table row
  dividers, secondary breaks.
- **Canvas Dark** ({colors.canvas-dark}): `#17181C` - reserved (no dark
  surfaces in v1; kept named so a future variant doesn't rename tokens).

### Hero Atmospheric

- **Hero Sky From / To** ({colors.hero-sky-from} `#EAF4FF` → {colors.hero-sky-to}
  `#FFFBF2`): the login brand-panel gradient, 180° top-to-bottom.
- **Hero Dark From / To** ({colors.hero-dark-from} `#0E3D33` → {colors.hero-dark-to}
  `#12B58A`): reserved for the About/upgrade band if one ever ships; not used
  in v1 (documented so the token set is complete).

### Text

- **Ink** ({colors.ink}): `#111113` - headlines, primary text, table data.
- **Charcoal** ({colors.charcoal}): `#37373E` - body text.
- **Slate** ({colors.slate}): `#5E5E68` - secondary text, metadata.
- **Steel** ({colors.steel}): `#8B8B95` - tertiary text, table headers,
  sidebar inactive items.
- **Stone** ({colors.stone}): `#A9A9B2` - captions, placeholders.
- **Muted** ({colors.muted}): `#C7C7CE` - disabled text.
- **On Dark** ({colors.on-dark}) `#FFFFFF` / **On Dark Muted**
  ({colors.on-dark-muted} `rgba(255,255,255,0.72)`) - text on dark surfaces
  (reserved; see Canvas Dark).
- **On Primary** ({colors.on-primary}): `#FFFFFF` - text on black pills.

## Typography

### Font Family

- **Inter** (primary UI): body, headings, buttons, navigation. Fallbacks:
  `-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`.
- **IBM Plex Sans Thai** (Thai text): every Thai glyph, paired after Inter so
  Latin characters render in Inter and Thai in Plex Thai within the same
  string. Fallback: `"Leelawadee UI", Tahoma, sans-serif`.
- **IBM Plex Mono** (data): quantities, icodes, times, dates in tables -
  anything that benefits from column alignment (`font-variant-numeric:
  tabular-nums` is applied on top). Fallbacks: `"SF Mono", Menlo, Consolas,
  monospace`.

### Hierarchy

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|---|---|---|---|---|---|
| `{typography.display}` | 28px | 600 | 1.20 | -0.5px | Login brand headline only |
| `{typography.heading-1}` | 22px | 600 | 1.25 | 0 | Screen titles ("ใบเบิกยาประจำห้องฉุกเฉินรายวัน") |
| `{typography.heading-2}` | 18px | 600 | 1.30 | 0 | Category section headers |
| `{typography.heading-3}` | 16px | 600 | 1.35 | 0 | Card/section titles, modal titles |
| `{typography.sheet-window}` | 13px | 400 | 1.40 | 0 | The sheet window sentence (dates in mono) |
| `{typography.body-md}` | 15px | 400 | 1.50 | 0 | Primary body, table data |
| `{typography.body-md-medium}` | 15px | 500 | 1.50 | 0 | Emphasized body (drug name) |
| `{typography.body-sm}` | 14px | 400 | 1.50 | 0 | Secondary body, helper text |
| `{typography.label}` | 13px | 600 | 1.40 | 0 | Field labels |
| `{typography.caption}` | 12px | 400 | 1.40 | 0 | Metadata, table footnotes |
| `{typography.micro-uppercase}` | 11px | 600 | 1.40 | 0.5px | Command-bar chip labels, table column headers |
| `{typography.button}` | 14px | 500 | 1.30 | 0 | Button labels |
| `{typography.code-md}` | 14px | 400 | 1.50 | 0 | icodes, units (mono) |
| `{typography.code-sm}` | 13px | 400 | 1.40 | 0 | Table numbers, timestamps (mono) |

### Principles

- **Thai is first-class**: never render Thai in Inter-only stacks; never
  render Latin codes in Plex Thai (assign fonts by *content role*, not by
  luck of the fallback chain).
- **Weight carries emphasis**, not size: 400 reading, 500 emphasis, 600
  headings/buttons/labels. Only the login brand display goes above 22px.
- **Line height 1.40-1.50 on everything** - the person reading this is under
  time pressure in imperfect hospital lighting.
- **Numbers in tables are mono** with `tabular-nums` so columns of quantities
  align as columns, not as ragged text.

## Layout

### Spacing System

- Base unit 4px, primary increment 8px.
- Tokens: `{spacing.xxs}` (4px) · `{spacing.xs}` (8px) · `{spacing.sm}` (12px)
  · `{spacing.md}` (16px) · `{spacing.lg}` (20px) · `{spacing.xl}` (24px) ·
  `{spacing.xxl}` (32px) · `{spacing.xxxl}` (40px) · `{spacing.section-sm}`
  (48px) · `{spacing.section}` (64px).
- Working surfaces use `{spacing.xxl}` between sections; table rows pack to
  `{spacing.xs}`-`{spacing.sm}` vertical rhythm. `{spacing.section}` exists
  only for the login brand panel.

### Structure - Command Bar + Paper Stage (document-first)

Before the document exists, the full-height `login-gate` replaces the shell
until an `officer` signs in. Connection setup is not part of this surface:
the settings dialog opens from the gate and owns the HOSxP credentials. After
sign-in the shell below is revealed; the top bar adds the signed-in user and
the logout action beside the connection status.

```
┌──────────────────────────────────────────────────────────────────────┐
│ Top bar (48px: mark · ER-Refill · ระบบใบเบิกยาห้องฉุกเฉิน · status · ตั้งค่า) │
├──────────────────────────────────────────────────────────────────────┤
│ COMMAND BAR (the only control surface; two rows when editing)        │
│  วันที่เบิก [date] │ ช่วงเวลา 20 ต.ค. 20:30 → 21 ต.ค. 08:30 [อัตโนมัติ]   │
│  [แก้ไขช่วงเวลา]        รายการยา 69 · ใช้ 12 · เบิกรวม 34 [ดึงใหม่] [พิมพ์ใบเบิก] │
├──────────────────────────────────────────────────────────────────────┤
│ STAGE (desk surface, scrollable)                                     │
│                                                                      │
│        ┌─────────────────────────────┐  ← THE SHEET (A4 preview)    │
│        │  โรงพยาบาลสระโบสถ์           │    = the printed output:      │
│        │  ใบเบิกยาประจำห้องฉุกเฉินรายวัน │      one DOM, one CSS        │
│        │  ช่วงเวลาเบิก: ...            │                               │
│        │  [table: ลำดับ ชื่อยา+ความแรง │                               │
│        │   จำนวน/Stock จำนวนใช้ จำนวนเบิก]│                             │
│        │  ลงชื่อ ผู้เบิก │ ลงชื่อ ผู้จ่าย │                               │
│        └─────────────────────────────┘                              │
└──────────────────────────────────────────────────────────────────────┘
```

**Why document-first:** ER-Refill's job is not "look a value up" (the sidebar
pattern of the sibling AllerX tool); it is "verify a document, then print
it". Established document tools teach the same lesson: the print preview
must reuse the same DOM and CSS as the print output, or the preview lies.
So the sheet is rendered once, on screen, exactly as it prints - the
operator corrects จำนวนเบิก directly on the paper they will sign, and the
command bar above holds the few run controls. No second layout exists.

### Grid & Container

- Desktop window 1120×800 default, min 900×620 (a document tool; it does not
  need to survive phone widths).
- The sheet is a real A4: `min(210mm, 100%)` wide, `297mm` min-height,
  12mm internal padding, `{elevation.3}` on the `{colors.surface}` desk.
  On print, `@page` owns the margins and the sheet drops its padding.
- The table is the *only* multi-column surface; column widths are frozen:
  `46px` ลำดับ / `1fr` ชื่อยา (+ความแรง, +icode) / `108px` จำนวน/Stock /
  `64px` จำนวนใช้ / `76px` จำนวนเบิก.
- จำนวนเบิก starts as whole packs: `ceil(usage / units-per-pack)` from the
  จำนวน/Stock setting, capped at the stock pack count; the operator can
  adjust it on the sheet.

## Elevation & Depth

Predominantly flat with strategic atmosphere (same table as the reference,
re-scoped):

| Level | Treatment | Use |
|---|---|---|
| 0 (flat) | No shadow; `{colors.hairline}` border | Command bar, top bar, in-modal surfaces |
| 1 (subtle) | `rgba(0,0,0,0.04) 0 1px 2px` | Drug-picker result rows, small popovers |
| 2 (card) | `rgba(0,0,0,0.08) 0 4px 12px` | Toasts, elevated menu surfaces |
| 3 (sheet) | `rgba(0,0,0,0.12) 0 24px 48px -8px` | The A4 sheet on the desk - the paper look - and the settings modal |
| 4 (brand-tinted) | `{colors.brand-glow} 0 8px 24px` | Reserved for one brand accent; not used in the current UI |

The login brand panel needs no shadow - the gradient does the work. The
sheet's shadow is what makes it read as paper on a desk; it disappears in
print.

## Shapes

| Token | Value | Use |
|---|---|---|
| `{rounded.xs}` | 4px | Inline code chips (`icode`), the จำนวนเบิก input |
| `{rounded.sm}` | 6px | Category nav items, badges, status chips |
| `{rounded.md}` | 8px | Inputs, search pill, picker cards |
| `{rounded.lg}` | 12px | Settings modal, login card |
| `{rounded.xl}` | 16px | Reserved for oversized framed surfaces |
| `{rounded.full}` | 9999px | **All** buttons and pills, badges, status dot halo |

The scale is disciplined: no 10px corners; nothing between `{rounded.md}` and
`{rounded.lg}` for the same component family. Buttons are always pills; the
sheet is paper and takes no radius at all.

## Components

> Per the no-hover policy of the reference: states are documented as default /
> pressed / disabled / active; hover affordances are a flat background change
> only (see the Motion section below).

### Buttons

**`button-primary`** - Black pill primary CTA (the requisition action).
- Background `{colors.primary}` (`#111113`), text `{colors.on-primary}`,
  typography `{typography.button}`, padding `10px 20px`, rounded `{rounded.full}`.
- Pressed: background `{colors.charcoal}`. Disabled: background
  `{colors.hairline}`, text `{colors.muted}`, no pointer events.

**`button-accent-mint`** - Seal-mint pill, used **once per viewport** on an
empty-state or catalog CTA.
- Background `{colors.brand-green}`, text `#072A21`, typography
  `{typography.button}`, padding `10px 20px`, rounded `{rounded.full}`, plus
  an optional `{elevation.4}` glow.

**`button-secondary`** - Outlined pill: transparent background, text
`{colors.ink}`, border `1px solid {colors.hairline}`, padding `10px 20px`.

**`button-ghost`** - Quiet rectangular action (command bar "แก้ไขช่วงเวลา",
settings close): transparent, text `{colors.ink}`, padding `8px 12px`,
rounded `{rounded.md}`.

**`button-icon-circular`** - 32×32px circular utility (close, ↑/↓ in the
catalog list): background `{colors.canvas}`, border
`1px solid {colors.hairline}`, rounded `{rounded.full}`.

**`icon` set** - ER-Refill's own monoline SVGs: 24×24 viewBox, 1.8px stroke,
round caps and joins, `currentColor`. 16px inside buttons, 24px in the
settings rail. No filled or duotone variants; the line stays quiet beside
Thai text and quantities.

### Cards & Containers

**`card-base`** - Background `{colors.canvas}`, rounded `{rounded.lg}`,
padding `{spacing.xl}`, border `1px solid {colors.hairline}`.

**`card-feature`** - Section panel on `{colors.surface}`, rounded
`{rounded.lg}`, padding `{spacing.xxl}` (settings panels use it bare, without
a border).

### Inputs & Forms

**`text-input`** - Background `{colors.canvas}`, text `{colors.ink}`, border
`1px solid {colors.hairline}`, rounded `{rounded.md}`, padding
`{spacing.sm} {spacing.md}`, 40px height. Focus: `2px solid
{colors.brand-green}` (mint is the activation signal).

**`text-input-error`** - Border `1px solid {colors.error}`; helper line in
`{typography.caption}` + `{colors.error}`.

**`sheet-request-input`** - The editable จำนวนเบิก box *on the paper*: centered
`{typography.code-md}` in a borderless in-cell input, no spinners, focus
ring mint inset (`outline-offset: -2px`). It prints as a plain filled
number.

**`text-input--compact`** - 34px variant for the command bar (date and the
manual window editors).

**`search-pill`** - Drug-picker / search: background `{colors.surface}`, text
`{colors.steel}`, typography `{typography.body-sm}`, rounded `{rounded.md}`,
36px height, border `1px solid {colors.hairline}`.

### Settings navigation

**`settings-nav__item`** + **`settings-nav__item--active`** - The settings
modal's vertical navigation rail (icon + label per panel, on a
`{colors.surface-soft}` column): inactive text `{colors.steel}` with a stone
icon; active: `{colors.canvas}` background, ink text, and a 3px mint left
border. Below 960px the rail turns into a horizontal scrolling strip
(mint bottom border on the active item). The rail exists because the
catalog panel needs the modal's full width for its three-field drug rows.

### Badges & Status

**`badge-icode`** - icode chip: background `{colors.tag-bg}`, text
`{colors.tag-blue}`, typography `{typography.code-sm}`, rounded
`{rounded.xs}`, padding `2px 6px`.

**`badge-required`** - "REQUIRED" / "จำเป็น" label: background
`{colors.error}`, text `{colors.on-dark}`, `{typography.micro-uppercase}`,
rounded `{rounded.sm}`, `2px 6px`.

**`status-dot`** - 8px dot + text: `{colors.brand-green}` connected,
`{colors.warn-text}` degraded, `{colors.error}` disconnected, `{colors.stone}`
unconfigured. Applied with a soft halo (`box-shadow: 0 0 0 3px soft`).

**`run-window` card** - The command-bar window state: a compact card with a
"ช่วงเวลาเบิก" heading and badge ("อัตโนมัติ" mint, "แก้ไขเอง" amber) above
the two short Thai/BE datetimes (mono, tabular). The amber state is the
reason this card exists; the requisition never hides the window it covers.

### Signature Components

**`login-gate`** - Full-height split surface before the document exists:
- Left brand panel: the sky hero gradient, seal mark, the
  "ใบเบิกยาห้องฉุกเฉิน" headline, and a custom paper-and-pills illustration
  (`app/assets/login-illustration.svg`) - no marketing copy or proof points.
- Right panel: a single `rounded.lg` card focused on the `officer`
  username/password, with a "ตั้งค่า" button in the page toolbar. Until a
  connection is configured, a warn-toned notice inside the card points at the
  settings dialog; the card never embeds the connection form or connection
  summary.
- The connection password is a DB credential and lives only in the settings
  dialog; the login password is never persisted. Both use the standard field
  and button components.

**`command-bar`** - The only control surface, two rows tall when editing:
- Row 1: a labelled date input · a `run-window` card (heading + badge over
  the short Thai/BE times) · ghost "แก้ไขช่วงเวลา" on the left. On the right:
  one segmented `stat-strip` (รายการยา / มีการใช้ / เบิกรวม, mono numbers) ·
  ดึงข้อมูล (primary when the sheet is empty, secondary "ดึงใหม่" after) ·
  **พิมพ์ใบเบิก** (black pill, disabled until the sheet exists).
- Row 2 (collapsed by default): an inset surface card with the manual window
  editors (`datetime-local` × 2) + ghost "กลับเป็นอัตโนมัติ".
- Background `{colors.canvas}`, bottom hairline - chrome, not content.

**`sheet`** - The document itself: the A4 paper staged on the desk.
- Header: hospital name (`{typography.heading-2}` centered), document title,
  department line - all from `ReportMeta`.
- Window sentence in full Thai, then the table.
- Table: header row `{typography.caption}` on `{colors.surface-soft}`,
  `1px solid {colors.hairline}` cells (print: `#999`); category rows bold,
  `{colors.surface-soft}`; name cell = name (600) + strength label
  (`{typography.code-sm}` `{colors.slate}`) + icode (`{typography.code-sm}`
  `{colors.steel}`) so same-name drugs never blur together; จำนวนใช้
  right-aligned mono, จำนวนเบิก the `sheet-request-input`.
- Signature blocks (ลงชื่อ ผู้เบิก / ผู้จ่าย + date lines) are visible on
  screen too - the preview is the paper, and they render on the **last
  page only**.
- **Pagination is explicit**: one `<article class="sheet">` per A4 page,
  at most **40 drug rows per page** (`er-refill-core::ROWS_PER_PAGE`), so the
  69-item pilot list always prints on two sheets. Every page repeats the
  header block, the window sentence, and the column headers; a split
  category continues with "(ต่อ)"; the footer carries หน้า X / Y (and the
  print stamp on the last page). Row metrics (11.5px text, `2px 6px` cell
  padding, 20px input) are tuned to this budget - do not loosen them
  without redoing the math.
- Ordering is the catalog order - never re-sorted by quantity.

**`signature-block`** - Always visible at the sheet's foot (the screen
previews the paper): two columns "ลงชื่อ .................................. (ผู้เบิก)" /
"(ผู้จ่าย)" with a date line under each, `{typography.body-md}`.

**`@media print`** - The rule that keeps the preview honest: chrome
(`.top-bar`, `.command-bar`, `.banner`, modal) hides with
`display: none !important`; the stage loses its padding, background, and
overflow; the sheet drops its width cap, min-height, padding (the `@page`
A4 portrait 12mm margins own it now), and shadow; cell borders darken to
`#999`, header/category backgrounds go transparent, and the จำนวนเบิก input
prints its value plainly. Every sheet after the first starts a new printed
page (`break-before: page`), and rows are `break-inside: avoid` so nothing
is ever sliced. One DOM, one stylesheet - the preview cannot diverge from
the printout.

**`settings-modal`** - `{elevation.3}` dialog over a `rgba(17,17,19,0.4)`
backdrop, `{rounded.lg}`, max-width 960px. A 200px vertical `settings-nav`
rail (icons + การเชื่อมต่อ · เวลาทำการ · รายการยา · หัวใบเบิก) beside a
scrolling `settings-content` area. Each panel keeps its own action row
(mint accent or black pill for save). The catalog panel uses two-line drug
rows - identity line (name, icode chip, ↑/↓/remove) over a labeled field
line (ความแรง, จำนวน/Stock) - so the three inputs never fight for width.
The modal footer is a `{colors.surface-soft}` strip with
`ส่งออกการตั้งค่า` / `นำเข้าการตั้งค่า` secondary pills + a status line -
the exported JSON carries the schedule, catalog, and report header, never
connection credentials.

**`banner`** - Degraded-connection / save-failure strip at the top of the
stage: amber pair (`{colors.warn-bg}`/`{colors.warn-text}`) with a dismiss
ghost button. Error banners use the red pair.

**`category-list` / `drug-picker`** - Catalog manager: left mini-list of
categories (`{typography.body-sm-medium}`), right list of drugs with
`button-icon-circular` ↑/↓/remove per row and three inline editors (name ·
ความแรง · จำนวน/Stock); `search-pill` above, results as `{elevation.2}`
picker rows with `+ เพิ่ม` ghost action. Each result shows the icode chip
plus the drug's strength label from `drugitems` (`{typography.code-sm}`,
e.g. "500 mg"), which is saved into the drug's `strengthText` when added.

## Do's and Don'ts

### Do

- Reserve `{colors.brand-green}` for accent CTAs, the auto window chip,
  active tab indicator, and confirmation checks - if it appears on a large
  generic surface, it did not earn that role.
- Use `{colors.primary}` black pills for the dominant action; exactly one mint
  accent CTA per viewport.
- Apply `{rounded.full}` to every button and pill; cards are always
  `{rounded.lg}`; the sheet is paper and stays square.
- Put every number that appears in a column into `{typography.code-*}` (mono,
  tabular) - quantities must align on the paper.
- Keep the run-window chip's two states visually distinct at a glance (mint =
  auto, amber = overridden); never make the override silent.
- Render strength beside the drug name everywhere (catalog, sheet, paper) so
  same-name drugs (Paracetamol 325 vs 500) never blur.
- Use `150-200ms ease` for state transitions; `{prefers-reduced-motion}`
  disables them.

### Don't

- Don't use `{colors.brand-green}` on body text or large surfaces.
- Don't add a second accent hue beyond mint, info-blue, warn-amber, and error
  red.
- Don't use amber for "error" or red for "attention" - they are different
  signals and the pharmacist must distinguish them instantly.
- Don't add gradients anywhere except the login brand panel - the desk and
  the sheet stay flat.
- Don't build a second layout for printing: the stage sheet IS the print
  output (same DOM, same stylesheet); a separate hidden sheet is a bug
  waiting to happen.
- Don't re-sort the requisition table by usage - the order mirrors the
  physical form and the pharmacy's pick route.
- Don't render Thai text in a Latin-only stack or Latin codes in the Thai
  face - the assignment is the brand voice.
- Don't reduce table text below 12.5px - legibility at arm's length beats
  fitting one more row.

## Responsive Behavior

A fixed-purpose desktop window (Tauri), 1120×800 default, min 900×620:

| Window width | Behavior |
|---|---|
| < 1100px | Command bar stacks into two intentional rows (run context, then stats + actions); the sheet goes full-width with reduced padding; the settings nav rail turns horizontal and the catalog collapses to one column. |
| 1100-1279px | Command bar on one row; the sheet stays at `min(210mm, 100%)`. |
| ≥ 1280px | Full command bar; the sheet centered on the desk with breathing room. |

Touch targets: buttons 40px effective height, icon-circular 32px desktop.
This is a mouse-class desktop app; there is no mobile layout.

## Motion

- State transitions: `background-color 150ms ease-out` (chips, buttons), focus
  borders 150ms.
- The run-window chip never animates between mint and amber - the color *is*
  the message; a cross-fade would slow reading it.
- No bounce/elastic easing anywhere; `prefers-reduced-motion` disables all
  transitions.

## Iteration Guide

1. Any new component declares its tokens from this file - no ad-hoc hex
   values in `app/style/main.css`.
2. Default body is `{typography.body-md}`; anything numeric-in-a-column is
   `{typography.code-*}`.
3. New states should first be expressible as weight/border/spacing changes;
   new named colors need a semantic role (auto/custom/attention/error) to
   exist at all.
4. Re-check every new surface against "quiet except for the sheet and its window sentence".
5. Keep screen table and print table column-order identical - the screen is a
   proof of what prints.

## Known Gaps

- Dark mode is out of scope (no dark values defined; token names support it).
- The animated cloud/rocket aesthetic of the reference is deliberately not
  ported - only its gradient treatment survives, once, on the login brand panel.
- Print-color fidelity is assumed (mint/amber bands print as ink-saving text
  only; the sheet is black-on-white by design).
