//! Requisition dialogs: add a configured drug, explain a quantity change,
//! and confirm the print snapshot (DESIGN.md "print gate").
//!
//! Every manual change to the sheet carries a reason; the confirmation
//! dialog is the only way to enable printing, and the reason rows print with
//! the sheet.

use std::collections::HashSet;

use er_refill_core::{RequisitionError, add_configured_drug, adjust_requested};
use er_refill_models::{format_qty, format_thai_datetime_full};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::icons::{IconCheck, IconPlus, IconX};
use crate::state::{AppState, Banner, PendingEdit};

/// A configured drug that can still be added to the current sheet.
#[derive(Debug, Clone, PartialEq)]
struct AddCandidate {
    icode: String,
    name: String,
    strength_text: String,
    stock_text: String,
    category_name: String,
    max_requested: Option<f64>,
}

/// Thai rendering of a requisition mutation error.
fn requisition_error_th(err: &RequisitionError) -> String {
    match err {
        RequisitionError::DrugNotConfigured(_) => "รายการยานี้ไม่ได้อยู่ในบัญชีที่ตั้งค่าไว้".to_string(),
        RequisitionError::DrugAlreadyInReport(_) => "รายการยานี้อยู่ในใบเบิกแล้ว".to_string(),
        RequisitionError::RowNotFound(_) => "ไม่พบรายการยาในใบเบิก".to_string(),
        RequisitionError::MissingReason => "กรุณาระบุเหตุผล".to_string(),
        RequisitionError::InvalidRequested => "จำนวนเบิกต้องเป็นจำนวนเต็มและไม่ติดลบ".to_string(),
        RequisitionError::CategoryNotFound(_) => "ไม่พบหมวดของรายการยา".to_string(),
    }
}

/// `event_target_value` casts to an input element; reason fields are
/// textareas, so they need their own value reader.
fn text_area_value(event: &web_sys::Event) -> String {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|element| element.value())
        .unwrap_or_default()
}

/// The add/confirm/adjust dialog trio; only the active one renders.
#[component]
pub fn RequisitionDialogs(state: AppState) -> impl IntoView {
    let confirm_state = state.clone();
    let add_state = state.clone();
    let adjust_state = state;
    view! {
        {move || {
            confirm_state
                .confirm_open
                .get()
                .then(|| view! { <ConfirmRequisitionDialog state=confirm_state.clone() /> })
        }}
        {move || {
            add_state
                .add_drug_open
                .get()
                .then(|| view! { <AddDrugDialog state=add_state.clone() /> })
        }}
        {move || {
            adjust_state
                .pending_edit
                .get()
                .map(|pending| view! { <AdjustRequestDialog state=adjust_state.clone() pending=pending /> })
        }}
    }
}

/// Pre-print gate: summarises the current sheet and snapshots it on confirm.
#[component]
fn ConfirmRequisitionDialog(state: AppState) -> impl IntoView {
    let close = {
        let state = state.clone();
        move |_| state.confirm_open.set(false)
    };
    let confirm = {
        let state = state.clone();
        move |_| {
            if state.confirm_requisition() {
                state.confirm_open.set(false);
            } else {
                state.banner.set(Some(Banner::error("ยังไม่พร้อมยืนยันข้อมูลเบิก")));
            }
        }
    };
    let summary_state = state.clone();
    let stop_propagation = move |ev: web_sys::MouseEvent| ev.stop_propagation();

    view! {
        <div class="modal-backdrop" on:click=close>
            <div
                class="dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="confirm-title"
                on:click=stop_propagation
            >
                <header class="dialog__header">
                    <h2 class="dialog__title" id="confirm-title">"ยืนยันข้อมูลก่อนพิมพ์"</h2>
                    <button class="button-icon-circular" aria-label="ปิด" on:click=close>
                        <IconX class="icon" />
                    </button>
                </header>
                <div class="dialog__body">
                    {move || {
                        let Some(sections) = summary_state.sections.get() else {
                            return view! {
                                <p class="dialog__note">"ยังไม่มีข้อมูลใบเบิกให้ยืนยัน"</p>
                            }
                                .into_any();
                        };
                        let rows: Vec<&er_refill_models::ReportRow> = sections
                            .iter()
                            .flat_map(|section| section.rows.iter())
                            .collect();
                        let total = rows.iter().map(|row| row.requested).sum::<f64>();
                        let adjusted = rows.iter().filter(|row| row.has_adjustment()).count();
                        let window = summary_state
                            .window
                            .get()
                            .map(|window| {
                                format!(
                                    "{} ถึง {}",
                                    format_thai_datetime_full(window.start),
                                    format_thai_datetime_full(window.end),
                                )
                            })
                            .unwrap_or_default();
                        view! {
                            <div class="dialog__summary">
                                <div class="dialog__summary-row">
                                    <span>"ช่วงเวลาเบิก"</span>
                                    <strong>{window}</strong>
                                </div>
                                <div class="dialog__summary-row">
                                    <span>"รายการในใบเบิก"</span>
                                    <strong>{format!("{} รายการ", rows.len())}</strong>
                                </div>
                                <div class="dialog__summary-row">
                                    <span>"เบิกรวม"</span>
                                    <strong>{format_qty(total)}</strong>
                                </div>
                                <div class="dialog__summary-row">
                                    <span>"ขอเพิ่ม/ขอแก้ไข"</span>
                                    <strong>{format!("{} รายการ", adjusted)}</strong>
                                </div>
                            </div>
                            <p class="dialog__note">
                                "หลังยืนยัน ปุ่มพิมพ์จะเปิดใช้งาน และสิ่งที่พิมพ์คือข้อมูลชุดนี้เท่านั้น หากแก้ไขข้อมูลภายหลัง ต้องยืนยันใหม่"
                            </p>
                        }
                            .into_any()
                    }}
                </div>
                <footer class="dialog__footer">
                    <button class="button-secondary" on:click=close>"ยกเลิก"</button>
                    <button
                        class="button-accent"
                        disabled=move || {
                            summary_state
                                .sections
                                .get()
                                .is_none_or(|sections| {
                                    sections.iter().all(|section| section.rows.is_empty())
                                })
                        }
                        on:click=confirm
                    >
                        <IconCheck class="icon" />
                        "ยืนยันข้อมูลเบิก"
                    </button>
                </footer>
            </div>
        </div>
    }
}

/// Search the configured catalog for a zero-usage drug, require a reason,
/// and add it to the sheet.
#[component]
fn AddDrugDialog(state: AppState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new(None::<AddCandidate>);
    let requested_text = RwSignal::new("1".to_string());
    let reason = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let candidates = Memo::new({
        let state = state.clone();
        move |_| {
            let Some(settings) = state.settings.get() else {
                return Vec::new();
            };
            let present: HashSet<String> = state
                .sections
                .get()
                .unwrap_or_default()
                .iter()
                .flat_map(|section| section.rows.iter())
                .map(|row| row.icode.clone())
                .collect();
            let mut categories = settings.catalog.categories.clone();
            categories.sort_by_key(|category| category.sort_order);
            let mut out = Vec::new();
            for category in categories {
                for drug in settings.catalog.drugs_in_category(&category.id) {
                    if present.contains(&drug.icode) {
                        continue;
                    }
                    out.push(AddCandidate {
                        icode: drug.icode.clone(),
                        name: drug.name.clone(),
                        strength_text: drug.strength_text.clone(),
                        stock_text: drug.stock_text.clone(),
                        category_name: category.name.clone(),
                        max_requested: drug.pack_stock().map(|pack| pack.max_requested()),
                    });
                }
            }
            out
        }
    });

    let filtered = Memo::new(move |_| {
        let needle = query.get().trim().to_lowercase();
        candidates
            .get()
            .into_iter()
            .filter(|candidate| {
                needle.is_empty()
                    || candidate.name.to_lowercase().contains(&needle)
                    || candidate.icode.to_lowercase().contains(&needle)
                    || candidate.category_name.to_lowercase().contains(&needle)
                    || candidate.strength_text.to_lowercase().contains(&needle)
            })
            .collect::<Vec<_>>()
    });

    let select_candidate = move |candidate: AddCandidate| {
        let default = candidate
            .max_requested
            .filter(|max| *max >= 1.0)
            .map(|_| "1".to_string())
            .unwrap_or_else(|| {
                candidate
                    .max_requested
                    .map(|max| format_qty(max.max(0.0)))
                    .unwrap_or_else(|| "1".to_string())
            });
        requested_text.set(default);
        reason.set(String::new());
        error.set(None);
        selected.set(Some(candidate));
    };

    let close = {
        let state = state.clone();
        move |_| state.add_drug_open.set(false)
    };
    let on_confirm = {
        let state = state.clone();
        move |_| {
            let Some(candidate) = selected.get_untracked() else {
                return;
            };
            let raw = requested_text.get_untracked();
            let Ok(requested) = raw.trim().parse::<f64>() else {
                error.set(Some("กรุณากรอกจำนวนเบิกเป็นตัวเลข".to_string()));
                return;
            };
            let reason_text = reason.get_untracked().trim().to_string();
            if reason_text.is_empty() {
                error.set(Some("กรุณาระบุเหตุผลที่ขอเพิ่ม".to_string()));
                return;
            }
            let Some(settings) = state.settings.get_untracked() else {
                error.set(Some("ยังโหลดการตั้งค่าไม่เสร็จ".to_string()));
                return;
            };
            let Some(mut sections) = state.sections.get_untracked() else {
                error.set(Some("ยังไม่มีใบเบิกให้เพิ่มรายการ".to_string()));
                return;
            };
            match add_configured_drug(
                &settings.catalog,
                &mut sections,
                &candidate.icode,
                requested,
                &reason_text,
            ) {
                Ok(()) => {
                    state.add_drug_open.set(false);
                    state.sections.set(Some(sections));
                    state.invalidate_confirmation();
                    state.set_info_banner(format!("เพิ่มรายการยา {} แล้ว", candidate.name));
                }
                Err(err) => error.set(Some(requisition_error_th(&err))),
            }
        }
    };

    let can_confirm = move || {
        let Some(candidate) = selected.get() else {
            return false;
        };
        let Ok(requested) = requested_text.get().trim().parse::<f64>() else {
            return false;
        };
        requested >= 1.0
            && requested.fract() == 0.0
            && candidate.max_requested.is_none_or(|max| requested <= max)
            && !reason.get().trim().is_empty()
    };
    let stop_propagation = move |ev: web_sys::MouseEvent| ev.stop_propagation();

    view! {
        <div class="modal-backdrop" on:click=close>
            <div
                class="dialog dialog--wide"
                role="dialog"
                aria-modal="true"
                aria-labelledby="add-drug-title"
                on:click=stop_propagation
            >
                <header class="dialog__header">
                    <h2 class="dialog__title" id="add-drug-title">"เพิ่มรายการยาในใบเบิก"</h2>
                    <button class="button-icon-circular" aria-label="ปิด" on:click=close>
                        <IconX class="icon" />
                    </button>
                </header>
                <div class="dialog__body">
                    <p class="dialog__note">
                        "เลือกได้เฉพาะรายการยาที่ตั้งค่าไว้ในบัญชี รายการที่เพิ่มจะมียอดใช้ 0 และต้องระบุเหตุผล"
                    </p>
                    <input
                        class="text-input"
                        type="search"
                        placeholder="ค้นหาชื่อยา รหัสยา หรือหมวด"
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                    <div class="add-drug__list">
                        {move || {
                            let items = filtered.get();
                            if items.is_empty() {
                                return view! {
                                    <p class="dialog__empty">
                                        "ไม่พบรายการยาที่เพิ่มได้ (รายการที่ตั้งค่าไว้อาจอยู่ในใบเบิกแล้ว)"
                                    </p>
                                }
                                    .into_any();
                            }
                            items
                                .into_iter()
                                .map(|candidate| {
                                    let candidate_for_click = candidate.clone();
                                    let candidate_icode = candidate.icode.clone();
                                    let is_selected = move || {
                                        selected
                                            .get()
                                            .is_some_and(|selected| selected.icode == candidate_icode)
                                    };
                                    let on_select = move |_| {
                                        select_candidate(candidate_for_click.clone());
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if is_selected() {
                                                    "picker__result add-drug__result add-drug__result--selected"
                                                } else {
                                                    "picker__result add-drug__result"
                                                }
                                            }
                                            on:click=on_select
                                        >
                                            <span class="picker__result-info">
                                                <span class="picker__result-name">
                                                    {candidate.name.clone()}
                                                </span>
                                                <span class="picker__result-meta">
                                                    <span class="badge-icode">
                                                        {candidate.icode.clone()}
                                                    </span>
                                                    <span class="picker__result-strength">
                                                        {candidate.category_name.clone()}
                                                    </span>
                                                    <span class="picker__result-strength">
                                                        {candidate.strength_text.clone()}
                                                    </span>
                                                    <span class="picker__result-strength">
                                                        {format!("Stock {}", candidate.stock_text)}
                                                    </span>
                                                </span>
                                            </span>
                                            <span class="add-drug__pick">"เลือก"</span>
                                        </button>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>

                    {move || {
                        selected
                            .get()
                            .map(|candidate| {
                                view! {
                                    <div class="add-drug__form">
                                        <div class="dialog__summary">
                                            <div class="dialog__summary-row">
                                                <span>"รายการที่เลือก"</span>
                                                <strong>
                                                    {candidate.name.clone()}
                                                    {(!candidate.strength_text.is_empty())
                                                        .then(|| {
                                                            format!(" ({})", candidate.strength_text)
                                                        })}
                                                </strong>
                                            </div>
                                            <div class="dialog__summary-row">
                                                <span>"หมวด / Stock"</span>
                                                <strong>
                                                    {format!(
                                                        "{} · {}",
                                                        candidate.category_name,
                                                        candidate.stock_text,
                                                    )}
                                                </strong>
                                            </div>
                                        </div>
                                        <label class="field">
                                            <span class="field__label">"จำนวนเบิก (แพ็ค)"</span>
                                            <input
                                                class="text-input"
                                                type="number"
                                                min="1"
                                                step="1"
                                                max=candidate
                                                    .max_requested
                                                    .map(format_qty)
                                                    .unwrap_or_default()
                                                prop:value=move || requested_text.get()
                                                on:input=move |ev| {
                                                    requested_text.set(event_target_value(&ev))
                                                }
                                            />
                                        </label>
                                        <label class="field">
                                            <span class="field__label">"เหตุผลที่ขอเพิ่ม"</span>
                                            <textarea
                                                class="text-area"
                                                rows="3"
                                                prop:value=move || reason.get()
                                                on:input=move |ev| reason.set(text_area_value(&ev))
                                            ></textarea>
                                        </label>
                                    </div>
                                }
                            })
                    }}

                    {move || {
                        error
                            .get()
                            .map(|message| {
                                view! {
                                    <p class="dialog__error" role="alert">{message}</p>
                                }
                            })
                    }}
                </div>
                <footer class="dialog__footer">
                    <button class="button-secondary" on:click=close>"ยกเลิก"</button>
                    <button class="button-accent" disabled=move || !can_confirm() on:click=on_confirm>
                        <IconPlus class="icon" />
                        "เพิ่มรายการยา"
                    </button>
                </footer>
            </div>
        </div>
    }
}

/// Quantity editor that requires a reason before applying the change.
#[component]
fn AdjustRequestDialog(state: AppState, pending: PendingEdit) -> impl IntoView {
    let requested_text = RwSignal::new(format_qty(pending.new_requested));
    let reason = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let old_requested = pending.old_requested;
    let max_requested = pending.max_requested;
    let icode = pending.icode.clone();
    let name = pending.name.clone();
    let banner_name = name.clone();
    let strength_text = pending.strength_text.clone();

    let close = {
        let state = state.clone();
        move |_| state.pending_edit.set(None)
    };
    let on_confirm = {
        let state = state.clone();
        move |_| {
            let raw = requested_text.get_untracked();
            let Ok(requested) = raw.trim().parse::<f64>() else {
                error.set(Some("กรุณากรอกจำนวนเบิกเป็นตัวเลข".to_string()));
                return;
            };
            let reason_text = reason.get_untracked().trim().to_string();
            if reason_text.is_empty() {
                error.set(Some("กรุณาระบุเหตุผลที่แก้ไขจำนวนเบิก".to_string()));
                return;
            }
            let Some(mut sections) = state.sections.get_untracked() else {
                error.set(Some("ไม่พบใบเบิก".to_string()));
                return;
            };
            match adjust_requested(&mut sections, &icode, requested, &reason_text) {
                Ok(()) => {
                    state.pending_edit.set(None);
                    state.sections.set(Some(sections));
                    state.invalidate_confirmation();
                    state.set_info_banner(format!("บันทึกการแก้ไขจำนวนเบิก {} แล้ว", banner_name));
                }
                Err(err) => error.set(Some(requisition_error_th(&err))),
            }
        }
    };
    let can_confirm = move || {
        let Ok(requested) = requested_text.get().trim().parse::<f64>() else {
            return false;
        };
        requested >= 0.0
            && requested.fract() == 0.0
            && requested != old_requested
            && max_requested.is_none_or(|max| requested <= max)
            && !reason.get().trim().is_empty()
    };
    let stop_propagation = move |ev: web_sys::MouseEvent| ev.stop_propagation();

    view! {
        <div class="modal-backdrop" on:click=close>
            <div
                class="dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="adjust-title"
                on:click=stop_propagation
            >
                <header class="dialog__header">
                    <h2 class="dialog__title" id="adjust-title">
                        "ระบุเหตุผลการแก้ไขจำนวนเบิก"
                    </h2>
                    <button class="button-icon-circular" aria-label="ปิด" on:click=close>
                        <IconX class="icon" />
                    </button>
                </header>
                <div class="dialog__body">
                    <div class="dialog__summary">
                        <div class="dialog__summary-row">
                            <span>"รายการยา"</span>
                            <strong>
                                {name.clone()}
                                {(!strength_text.is_empty())
                                    .then(|| format!(" ({})", strength_text))}
                            </strong>
                        </div>
                        <div class="dialog__summary-row">
                            <span>"จำนวนเดิม → ใหม่"</span>
                            <strong>
                                {move || {
                                    format!(
                                        "{} → {}",
                                        format_qty(old_requested),
                                        requested_text.get(),
                                    )
                                }}
                            </strong>
                        </div>
                    </div>
                    <label class="field">
                        <span class="field__label">"จำนวนเบิกใหม่ (แพ็ค)"</span>
                        <input
                            class="text-input"
                            type="number"
                            min="0"
                            step="1"
                            max=max_requested.map(format_qty).unwrap_or_default()
                            prop:value=move || requested_text.get()
                            on:input=move |ev| requested_text.set(event_target_value(&ev))
                        />
                    </label>
                    <label class="field">
                        <span class="field__label">"เหตุผลที่แก้ไข"</span>
                        <textarea
                            class="text-area"
                            rows="3"
                            prop:value=move || reason.get()
                            on:input=move |ev| reason.set(text_area_value(&ev))
                        ></textarea>
                    </label>
                    {move || {
                        error
                            .get()
                            .map(|message| {
                                view! {
                                    <p class="dialog__error" role="alert">{message}</p>
                                }
                            })
                    }}
                </div>
                <footer class="dialog__footer">
                    <button class="button-secondary" on:click=close>"ยกเลิก"</button>
                    <button class="button-accent" disabled=move || !can_confirm() on:click=on_confirm>
                        "ยืนยันการแก้ไข"
                    </button>
                </footer>
            </div>
        </div>
    }
}
