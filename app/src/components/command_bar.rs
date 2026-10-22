//! The command bar: the app's only control surface (DESIGN.md
//! "command-bar"). One row carries the run controls (date, window, fetch)
//! and the document actions (stats, refetch, print); a collapsible second
//! row holds the manual window editor.

use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use er_refill_models::{RequisitionWindow, format_qty, format_thai_datetime_short};
use leptos::prelude::*;

use crate::components::icons::{IconPencil, IconPrinter, IconRefresh, IconX};
use crate::state::{AppState, Banner};

/// Buffer before `window.print()` so the sheet reflects the latest state
/// (the printed-at stamp) before the dialog takes over.
const PRINT_DELAY: Duration = Duration::from_millis(60);

/// Parses an HTML `datetime-local` value (`"YYYY-MM-DDTHH:MM"`, optional
/// seconds).
fn parse_local_datetime(raw: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
        .ok()
}

/// Toolbar under the top bar: date, window, stats, fetch, print.
#[component]
pub fn CommandBar(state: AppState) -> impl IntoView {
    // Date input.
    let date_value = move || state.selected_date.get().format("%Y-%m-%d").to_string();
    let on_date_change = {
        let state = state.clone();
        move |ev| {
            let raw = event_target_value(&ev);
            if let Ok(date) = NaiveDate::parse_from_str(&raw, "%Y-%m-%d") {
                state.selected_date.set(date);
                state.recompute_window();
                state.sections.set(None);
            }
        }
    };

    // Manual window editing.
    let start_value = move || {
        state
            .window
            .get()
            .map(|window| window.start.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default()
    };
    let end_value = move || {
        state
            .window
            .get()
            .map(|window| window.end.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default()
    };
    let apply_manual = {
        let state = state.clone();
        move |start_raw: String, end_raw: String| {
            let (Some(start), Some(end)) = (
                parse_local_datetime(&start_raw),
                parse_local_datetime(&end_raw),
            ) else {
                state
                    .banner
                    .set(Some(Banner::error("ช่วงเวลาไม่ถูกต้อง กรุณาตรวจสอบวันและเวลา")));
                return;
            };
            match RequisitionWindow::new(start, end) {
                Ok(window) => {
                    state.window.set(Some(window));
                    state.window_custom.set(true);
                    state.sections.set(None);
                    state.banner.set(None);
                }
                Err(_) => state
                    .banner
                    .set(Some(Banner::error("ช่วงเวลาไม่ถูกต้อง: เวลาเริ่มต้องก่อนเวลาสิ้นสุด"))),
            }
        }
    };
    let on_start_change = {
        let state = state.clone();
        move |ev| {
            let raw = event_target_value(&ev);
            let end = state
                .window
                .get()
                .map(|window| window.end.format("%Y-%m-%dT%H:%M").to_string())
                .unwrap_or_default();
            apply_manual(raw, end);
        }
    };
    let on_end_change = {
        let state = state.clone();
        move |ev| {
            let raw = event_target_value(&ev);
            let start = state
                .window
                .get()
                .map(|window| window.start.format("%Y-%m-%dT%H:%M").to_string())
                .unwrap_or_default();
            apply_manual(start, raw);
        }
    };
    let reset_auto = {
        let state = state.clone();
        move |_| {
            state.recompute_window();
            state.sections.set(None);
        }
    };

    // Actions.
    let on_fetch = {
        let state = state.clone();
        move |_| state.fetch_usage_action()
    };
    let on_print = {
        let state = state.clone();
        move |_| {
            state
                .printed_at
                .set(Some(chrono::Local::now().naive_local()));
            let _ = set_timeout_with_handle(
                move || {
                    if let Some(window) = web_sys::window() {
                        let _ = window.print();
                    }
                },
                PRINT_DELAY,
            );
        }
    };
    let toggle_edit = {
        let state = state.clone();
        move |_| state.window_edit_open.update(|open| *open = !*open)
    };
    let stats_state = state.clone();

    view! {
        <div class="command-bar">
            <div class="command-bar__main">
                <div class="command-bar__run">
                    <label class="run-field">
                        <span class="run-field__label">"วันที่เบิก"</span>
                        <input
                            class="text-input text-input--compact run-field__input"
                            type="date"
                            prop:value=date_value
                            on:change=on_date_change
                        />
                    </label>

                    <div class=move || {
                        format!(
                            "run-window{}",
                            if state.window_custom.get() { " run-window--custom" } else { "" },
                        )
                    }>
                        <div class="run-window__heading">
                            <span class="run-window__label">"ช่วงเวลาเบิก"</span>
                            <span class=move || {
                                format!(
                                    "run-window__badge run-window__badge--{}",
                                    if state.window_custom.get() { "custom" } else { "auto" },
                                )
                            }>
                                {move || {
                                    if state.window_custom.get() { "แก้ไขเอง" } else { "อัตโนมัติ" }
                                }}
                            </span>
                        </div>
                        <div class="run-window__times">
                            <span class="run-window__time">
                                {move || {
                                    state
                                        .window
                                        .get()
                                        .map(|window| format_thai_datetime_short(window.start))
                                        .unwrap_or_default()
                                }}
                            </span>
                            <span class="run-window__arrow">"→"</span>
                            <span class="run-window__time">
                                {move || {
                                    state
                                        .window
                                        .get()
                                        .map(|window| format_thai_datetime_short(window.end))
                                        .unwrap_or_default()
                                }}
                            </span>
                        </div>
                    </div>

                    <button
                        class="button-ghost button-ghost--inline command-bar__edit-trigger"
                        on:click=toggle_edit
                    >
                        <IconPencil class="icon" />
                        "แก้ไขช่วงเวลา"
                    </button>
                </div>

                <div class="command-bar__actions">
                    {move || {
                        let drugs = stats_state.catalog_drug_count();
                        let (used, requested) = stats_state
                            .sections
                            .get()
                            .map(|sections| {
                                let used = sections
                                    .iter()
                                    .flat_map(|section| section.rows.iter())
                                    .filter(|row| row.usage > 0.0)
                                    .count();
                                let requested: f64 = sections
                                    .iter()
                                    .flat_map(|section| section.rows.iter())
                                    .map(|row| row.requested)
                                    .sum();
                                (used, requested)
                            })
                            .unwrap_or((0, 0.0));
                        view! {
                            <div class="stat-strip">
                                <span class="stat">
                                    <span class="stat__label">"รายการยา"</span>
                                    <span class="stat__value">{drugs}</span>
                                </span>
                                <span class="stat">
                                    <span class="stat__label">"มีการใช้"</span>
                                    <span class="stat__value">{used}</span>
                                </span>
                                <span class="stat">
                                    <span class="stat__label">"เบิกรวม"</span>
                                    <span class="stat__value">{format_qty(requested)}</span>
                                </span>
                            </div>
                        }
                    }}
                    <div class="command-bar__buttons">
                        <button
                            class=move || {
                                if state.sections.get().is_none() {
                                    "button-primary"
                                } else {
                                    "button-secondary"
                                }
                            }
                            disabled=move || state.window.get().is_none() || state.loading.get()
                            on:click=on_fetch
                        >
                            <IconRefresh class="icon" />
                            {move || {
                                if state.loading.get() {
                                    "กำลังดึงข้อมูล..."
                                } else if state.sections.get().is_some() {
                                    "ดึงใหม่"
                                } else {
                                    "ดึงข้อมูลการใช้ยา"
                                }
                            }}
                        </button>
                        <button
                            class="button-primary"
                            disabled=move || state.sections.get().is_none()
                            on:click=on_print
                        >
                            <IconPrinter class="icon" />
                            "พิมพ์ใบเบิก"
                        </button>
                    </div>
                </div>
            </div>

            {move || {
                state.window_edit_open.get().then(|| {
                    let reset_auto = reset_auto.clone();
                    view! {
                        <div class="command-bar__edit">
                            <span class="command-bar__edit-label">"กำหนดช่วงเวลาเอง"</span>
                            <label class="field field--inline">
                                <span class="field__label">"เริ่ม"</span>
                                <input
                                    class="text-input text-input--compact"
                                    type="datetime-local"
                                    prop:value=start_value
                                    on:change=on_start_change
                                />
                            </label>
                            <label class="field field--inline">
                                <span class="field__label">"สิ้นสุด"</span>
                                <input
                                    class="text-input text-input--compact"
                                    type="datetime-local"
                                    prop:value=end_value
                                    on:change=on_end_change
                                />
                            </label>
                            <button
                                class="button-ghost button-ghost--inline"
                                on:click=reset_auto
                            >
                                <IconX class="icon" />
                                "กลับเป็นอัตโนมัติ"
                            </button>
                        </div>
                    }
                })
            }}
        </div>
    }
}
