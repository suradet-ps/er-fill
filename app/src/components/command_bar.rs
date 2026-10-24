//! The command bar: the app's only control surface (DESIGN.md
//! "command-bar"). The run date is always today (read-only); the operator
//! picks one of four fixed overnight ranges before fetching. The document
//! actions sit on the right.

use std::time::Duration;

use er_refill_models::{WindowPreset, format_thai_date, format_thai_datetime_abbr, format_time_hm};
use leptos::prelude::*;

use crate::components::icons::{IconCalendar, IconCheck, IconPlus, IconPrinter, IconRefresh};
use crate::state::{AppState, Banner};

/// Buffer before `window.print()` so the sheet reflects the latest state
/// (the printed-at stamp) before the dialog takes over.
const PRINT_DELAY: Duration = Duration::from_millis(60);

/// Toolbar under the top bar: fixed date, range picker, fetch, print.
#[component]
pub fn CommandBar(state: AppState) -> impl IntoView {
    // Actions.
    let on_fetch = {
        let state = state.clone();
        move |_| state.fetch_usage_action()
    };
    let on_print = {
        let state = state.clone();
        move |_| {
            if !state.print_snapshot_matches() {
                state.confirmed.set(None);
                state.printed_at.set(None);
                state
                    .banner
                    .set(Some(Banner::warn("ข้อมูลเปลี่ยนแล้ว กรุณายืนยันข้อมูลเบิกใหม่ก่อนพิมพ์")));
                return;
            }
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
    let on_open_add = {
        let state = state.clone();
        move |_| {
            state.banner.set(None);
            state.add_drug_open.set(true);
        }
    };
    let on_open_confirm = {
        let state = state.clone();
        move |_| {
            state.banner.set(None);
            state.confirm_open.set(true);
        }
    };

    view! {
        <div class="command-bar">
            <div class="command-bar__main">
                <div class="command-bar__run">
                    <div class="run-date">
                        <span class="run-date__label">
                            <IconCalendar class="icon run-date__icon" />
                            "วันที่เบิก"
                        </span>
                        <span class="run-date__value">
                            {move || format_thai_date(state.run_date.get())}
                        </span>
                    </div>

                    <div class="range-picker">
                        <div class="range-picker__heading">
                            <span class="range-picker__label">"ช่วงเวลาเบิก"</span>
                            <span class=move || {
                                if state.window_preset.get().is_some() {
                                    "range-picker__status range-picker__status--ready"
                                } else {
                                    "range-picker__status"
                                }
                            }>
                                {move || {
                                    if state.window_preset.get().is_some() {
                                        "เลือกแล้ว"
                                    } else {
                                        "เลือก 1 ช่วง · เมื่อวาน → วันนี้"
                                    }
                                }}
                            </span>
                        </div>
                        <div class="range-picker__options" role="radiogroup" aria-label="ช่วงเวลาเบิก">
                            {WindowPreset::ALL
                                .into_iter()
                                .map(|preset| {
                                    let selected = Memo::new({
                                        let state = state.clone();
                                        move |_| state.window_preset.get() == Some(preset)
                                    });
                                    let on_pick = {
                                        let state = state.clone();
                                        move |_| state.select_window_preset(preset)
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            role="radio"
                                            aria-checked=move || selected.get().to_string()
                                            aria-label=format!(
                                                "ช่วงเวลา {} ถึง {}",
                                                format_time_hm(preset.start()),
                                                format_time_hm(preset.end()),
                                            )
                                            class=move || {
                                                if selected.get() {
                                                    "range-option range-option--selected"
                                                } else {
                                                    "range-option"
                                                }
                                            }
                                            on:click=on_pick
                                        >
                                            <span class="range-option__times">
                                                <span>{format_time_hm(preset.start())}</span>
                                                <span class="range-option__arrow">"→"</span>
                                                <span>{format_time_hm(preset.end())}</span>
                                            </span>
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <p
                            class=move || {
                                if state.window.get().is_some() {
                                    "range-picker__preview range-picker__preview--ready"
                                } else {
                                    "range-picker__preview"
                                }
                            }
                            aria-live="polite"
                        >
                            {move || {
                                state
                                    .window
                                    .get()
                                    .map(|window| {
                                        format!(
                                            "ดึงข้อมูล {} → {}",
                                            format_thai_datetime_abbr(window.start),
                                            format_thai_datetime_abbr(window.end),
                                        )
                                    })
                                    .unwrap_or_else(|| {
                                        "เลือกช่วงเวลาเพื่อดึงข้อมูลของเมื่อวานถึงวันนี้".to_string()
                                    })
                            }}
                        </p>
                    </div>
                </div>

                <div class="command-bar__actions">
                    <div class="command-bar__buttons">
                        <button
                            class="button-secondary"
                            disabled=move || state.sections.get().is_none() || state.loading.get()
                            on:click=on_open_add
                        >
                            <IconPlus class="icon" />
                            "เพิ่มรายการยา"
                        </button>
                        <button
                            class=move || {
                                if state.sections.get().is_none() {
                                    "button-primary command-bar__fetch"
                                } else {
                                    "button-secondary command-bar__fetch"
                                }
                            }
                            disabled=move || state.window.get().is_none() || state.loading.get()
                            title=move || {
                                if state.window.get().is_none() {
                                    "ต้องเลือกช่วงเวลาเบิกก่อน"
                                } else {
                                    ""
                                }
                            }
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
                        {move || {
                            state.sections.get()?;
                            if let Some(confirmed) = state.confirmed.get() {
                                Some(
                                    view! {
                                        <span class="confirm-chip" title="ยืนยันข้อมูลเบิกแล้ว">
                                            <IconCheck class="icon" />
                                            "ยืนยันแล้ว "
                                            {confirmed.confirmed_at.format("%H:%M").to_string()}
                                        </span>
                                    }
                                        .into_any(),
                                )
                            } else {
                                let has_rows = state
                                    .sections
                                    .get()
                                    .is_some_and(|sections| {
                                        sections.iter().any(|section| !section.rows.is_empty())
                                    });
                                let busy = state.loading.get();
                                Some(
                                    view! {
                                        <button
                                            class="button-accent command-bar__confirm"
                                            disabled=!has_rows || busy
                                            on:click=on_open_confirm
                                        >
                                            <IconCheck class="icon" />
                                            "ยืนยันข้อมูลเบิก"
                                        </button>
                                    }
                                        .into_any(),
                                )
                            }
                        }}
                        <button
                            class="button-primary command-bar__print"
                            disabled=move || {
                                state.sections.get().is_none()
                                    || state.confirmed.get().is_none()
                                    || state.loading.get()
                            }
                            title="ต้องยืนยันข้อมูลเบิกก่อนพิมพ์"
                            on:click=on_print
                        >
                            <IconPrinter class="icon" />
                            "พิมพ์ใบเบิก"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
