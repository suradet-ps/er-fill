//! Settings tab: the ER drug catalog - categories and their drugs, mapped
//! to HOSxP `drugitems` icodes.
//!
//! All edits land on a local draft; nothing is persisted until "บันทึก".
//! Mutations go through `er-refill-core::catalog_ops`, whose errors are
//! translated to Thai here (Thai strings stay in the frontend, AGENTS.md section 2).

use er_refill_core::CatalogOpsError;
use er_refill_models::{Catalog, DrugItem};
use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconChevronDown, IconChevronUp, IconPlus, IconSearch, IconTrash};
use crate::components::settings_modal::{StatusLine, StatusMessage};
use crate::state::AppState;

/// Thai rendering of a catalog operation error.
fn catalog_error_th(err: &CatalogOpsError) -> String {
    match err {
        CatalogOpsError::EmptyName => "กรุณากรอกชื่อ".to_string(),
        CatalogOpsError::EmptyIcode => "รหัสยาไม่ถูกต้อง".to_string(),
        CatalogOpsError::DuplicateDrug(icode) => format!("ยา {icode} อยู่ในบัญชีแล้ว"),
        CatalogOpsError::CategoryNotFound(_) => "ไม่พบหมวดที่เลือก".to_string(),
        CatalogOpsError::DrugNotFound(_) => "ไม่พบรายการยา".to_string(),
        CatalogOpsError::CategoryNotEmpty(_) => {
            "หมวดนี้ยังมีรายการยา กรุณาย้ายหรือลบรายการก่อน".to_string()
        }
    }
}

/// The catalog manager: category list on the left, drugs + HOSxP picker on
/// the right, save/cancel at the bottom.
#[component]
pub fn CatalogPanel(state: AppState) -> impl IntoView {
    let draft = RwSignal::new(
        state
            .settings
            .get_untracked()
            .map(|settings| settings.catalog)
            .unwrap_or_default(),
    );
    let selected = RwSignal::new(
        draft
            .get_untracked()
            .categories
            .first()
            .map(|category| category.id.clone()),
    );
    let new_category = RwSignal::new(String::new());
    let search_term = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<DrugItem>::new());
    let searching = RwSignal::new(false);
    let status = RwSignal::new(None::<StatusMessage>);
    let busy = RwSignal::new(false);
    let dirty = RwSignal::new(false);

    // Category actions.

    let on_add_category = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let name = new_category.get();
        let mut created = None;
        let mut error = None;
        draft.update(
            |catalog| match er_refill_core::add_category(catalog, &name) {
                Ok(id) => created = Some(id),
                Err(err) => error = Some(catalog_error_th(&err)),
            },
        );
        if let Some(id) = created {
            selected.set(Some(id));
            new_category.set(String::new());
            dirty.set(true);
            status.set(None);
        } else if let Some(message) = error {
            status.set(Some(StatusMessage::error(message)));
        }
    };

    let on_rename_category = move |ev| {
        let Some(category_id) = selected.get() else {
            return;
        };
        let name = event_target_value(&ev);
        let mut error = None;
        draft.update(|catalog| {
            if let Err(err) = er_refill_core::rename_category(catalog, &category_id, &name) {
                error = Some(catalog_error_th(&err));
            }
        });
        match error {
            Some(message) => status.set(Some(StatusMessage::error(message))),
            None => {
                dirty.set(true);
                status.set(None);
            }
        }
    };

    let on_remove_category = move |category_id: String| {
        let mut error = None;
        draft.update(|catalog| {
            if let Err(err) = er_refill_core::remove_category(catalog, &category_id) {
                error = Some(catalog_error_th(&err));
            }
        });
        match error {
            Some(message) => status.set(Some(StatusMessage::error(message))),
            None => {
                if selected.get().as_deref() == Some(category_id.as_str()) {
                    selected.set(
                        draft
                            .get_untracked()
                            .categories
                            .first()
                            .map(|category| category.id.clone()),
                    );
                }
                dirty.set(true);
                status.set(None);
            }
        }
    };

    // HOSxP drug search.

    let on_search = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let term = search_term.get();
        if term.trim().chars().count() < 2 {
            status.set(Some(StatusMessage::error(
                "พิมพ์อย่างน้อย 2 ตัวอักษรเพื่อค้นหายา".to_string(),
            )));
            return;
        }
        searching.set(true);
        status.set(None);
        leptos::task::spawn_local(async move {
            match api::search_drugs(&term).await {
                Ok(hits) => results.set(hits),
                Err(err) => {
                    results.set(Vec::new());
                    status.set(Some(StatusMessage::error(err.message)));
                }
            }
            searching.set(false);
        });
    };

    let on_add_drug = move |item: DrugItem| {
        let Some(category_id) = selected.get() else {
            status.set(Some(StatusMessage::error("เลือกหมวดก่อนเพิ่มยา".to_string())));
            return;
        };
        let mut error = None;
        draft.update(|catalog| {
            if let Err(err) = er_refill_core::add_drug(catalog, &category_id, &item, "") {
                error = Some(catalog_error_th(&err));
            }
        });
        match error {
            Some(message) => status.set(Some(StatusMessage::error(message))),
            None => {
                dirty.set(true);
                status.set(Some(StatusMessage::ok(format!(
                    "เพิ่ม {} แล้ว อย่าลืมกดบันทึก",
                    item.name
                ))));
            }
        }
    };

    // Drug list actions.

    let on_update_drug =
        move |icode: String, name: String, strength: String, stock_text: String| {
            let mut error = None;
            draft.update(|catalog| {
                if let Err(err) =
                    er_refill_core::update_drug(catalog, &icode, &name, &strength, &stock_text)
                {
                    error = Some(catalog_error_th(&err));
                }
            });
            match error {
                Some(message) => status.set(Some(StatusMessage::error(message))),
                None => {
                    dirty.set(true);
                    status.set(None);
                }
            }
        };

    let on_move_drug = move |icode: String, delta: i32| {
        let mut error = None;
        draft.update(|catalog| {
            if let Err(err) = er_refill_core::move_drug(catalog, &icode, delta) {
                error = Some(catalog_error_th(&err));
            }
        });
        match error {
            Some(message) => status.set(Some(StatusMessage::error(message))),
            None => {
                dirty.set(true);
                status.set(None);
            }
        }
    };

    let on_remove_drug = move |icode: String| {
        let mut error = None;
        draft.update(|catalog| {
            if let Err(err) = er_refill_core::remove_drug(catalog, &icode) {
                error = Some(catalog_error_th(&err));
            }
        });
        match error {
            Some(message) => status.set(Some(StatusMessage::error(message))),
            None => {
                dirty.set(true);
                status.set(None);
            }
        }
    };

    // Persist / reset.

    let on_save = {
        let state = state.clone();
        move |_| {
            let catalog: Catalog = draft.get();
            busy.set(true);
            status.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::save_catalog(&catalog).await {
                    Ok(()) => {
                        state.settings.update(|settings| {
                            if let Some(settings) = settings {
                                settings.catalog = catalog;
                            }
                        });
                        state.sections.set(None);
                        dirty.set(false);
                        status.set(Some(StatusMessage::ok("บันทึกรายการยาแล้ว")));
                    }
                    Err(err) => status.set(Some(StatusMessage::error(err.message))),
                }
                busy.set(false);
            });
        }
    };

    let on_reset = {
        let state = state.clone();
        move |_| {
            let catalog = state
                .settings
                .get_untracked()
                .map(|settings| settings.catalog)
                .unwrap_or_default();
            selected.set(
                catalog
                    .categories
                    .first()
                    .map(|category| category.id.clone()),
            );
            draft.set(catalog);
            dirty.set(false);
            results.set(Vec::new());
            status.set(None);
        }
    };

    view! {
        <div class="panel panel--catalog">
            <div class="catalog">
                <div class="catalog__sidebar">
                    <h3 class="catalog__subtitle">"หมวด"</h3>
                    <ul class="category-list">
                        {move || {
                            let catalog = draft.get();
                            let mut categories = catalog.categories.clone();
                            categories.sort_by_key(|category| category.sort_order);
                            categories
                                .into_iter()
                                .map(|category| {
                                    let category_id = category.id.clone();
                                    let is_selected = Memo::new({
                                        let category_id = category_id.clone();
                                        move |_| selected.get().as_deref() == Some(category_id.as_str())
                                    });
                                    let count = draft
                                        .get()
                                        .drugs
                                        .iter()
                                        .filter(|drug| drug.category_id == category_id)
                                        .count();
                                    let has_drugs = count > 0;
                                    let on_select = {
                                        let category_id = category_id.clone();
                                        move |_| selected.set(Some(category_id.clone()))
                                    };
                                    let on_remove = {
                                        let remove_id = category_id.clone();
                                        move |_: web_sys::MouseEvent| {
                                            on_remove_category(remove_id.clone())
                                        }
                                    };
                                    view! {
                                        <li class="category-list__item">
                                            <button
                                                class=move || if is_selected.get() {
                                                    "category-list__button category-list__button--active"
                                                } else {
                                                    "category-list__button"
                                                }
                                                on:click=on_select
                                            >
                                                <span class="category-list__name">{category.name}</span>
                                                <span class="category-list__count">{count}</span>
                                            </button>
                                            <button
                                                class="button-icon-circular button-icon-circular--small"
                                                aria-label="ลบหมวด"
                                                disabled=has_drugs
                                                on:click=on_remove
                                            >
                                                <IconTrash class="icon" />
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                    <form class="catalog__add" on:submit=on_add_category>
                        <input
                            class="text-input"
                            type="text"
                            placeholder="เพิ่มหมวดใหม่"
                            prop:value=move || new_category.get()
                            on:input=move |ev| new_category.set(event_target_value(&ev))
                        />
                        <button class="button-icon-circular" aria-label="เพิ่มหมวด" type="submit">
                            <IconPlus class="icon" />
                        </button>
                    </form>
                </div>

                <div class="catalog__main">
                    {move || {
                        let Some(category_id) = selected.get() else {
                            return view! {
                                <p class="rail__hint">"เพิ่มหมวดก่อน แล้วจึงเพิ่มรายการยา"</p>
                            }
                                .into_any();
                        };
                        let catalog = draft.get();
                        let Some(category) = catalog
                            .categories
                            .iter()
                            .find(|category| category.id == category_id)
                        else {
                            return view! { <p class="rail__hint">"ไม่พบหมวด"</p> }.into_any();
                        };
                        let category_name = category.name.clone();
                        let drugs = catalog.drugs_in_category(&category_id);
                        let drug_rows = drugs
                            .into_iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(|drug| {
                                let on_name_change = {
                                    let icode = drug.icode.clone();
                                    let strength = drug.strength_text.clone();
                                    let stock = drug.stock_text.clone();
                                    move |ev| {
                                        on_update_drug(
                                            icode.clone(),
                                            event_target_value(&ev),
                                            strength.clone(),
                                            stock.clone(),
                                        )
                                    }
                                };
                                let on_strength_change = {
                                    let icode = drug.icode.clone();
                                    let name = drug.name.clone();
                                    let stock = drug.stock_text.clone();
                                    move |ev| {
                                        on_update_drug(
                                            icode.clone(),
                                            name.clone(),
                                            event_target_value(&ev),
                                            stock.clone(),
                                        )
                                    }
                                };
                                let on_stock_change = {
                                    let icode = drug.icode.clone();
                                    let name = drug.name.clone();
                                    let strength = drug.strength_text.clone();
                                    move |ev| {
                                        on_update_drug(
                                            icode.clone(),
                                            name.clone(),
                                            strength.clone(),
                                            event_target_value(&ev),
                                        )
                                    }
                                };
                                let on_up = {
                                    let icode = drug.icode.clone();
                                    move |_| on_move_drug(icode.clone(), -1)
                                };
                                let on_down = {
                                    let icode = drug.icode.clone();
                                    move |_| on_move_drug(icode.clone(), 1)
                                };
                                let on_delete = {
                                    let icode = drug.icode.clone();
                                    move |_| on_remove_drug(icode.clone())
                                };
                                view! {
                                    <li class="drug-row">
                                        <div class="drug-row__line">
                                            <input
                                                class="text-input drug-row__name"
                                                type="text"
                                                value=drug.name.clone()
                                                aria-label="ชื่อยา"
                                                on:change=on_name_change
                                            />
                                            <span class="badge-icode">{drug.icode.clone()}</span>
                                            <div class="drug-row__actions">
                                                <button
                                                    class="button-icon-circular button-icon-circular--small"
                                                    aria-label="เลื่อนขึ้น"
                                                    on:click=on_up
                                                >
                                                    <IconChevronUp class="icon" />
                                                </button>
                                                <button
                                                    class="button-icon-circular button-icon-circular--small"
                                                    aria-label="เลื่อนลง"
                                                    on:click=on_down
                                                >
                                                    <IconChevronDown class="icon" />
                                                </button>
                                                <button
                                                    class="button-icon-circular button-icon-circular--small"
                                                    aria-label="ลบรายการยา"
                                                    on:click=on_delete
                                                >
                                                    <IconTrash class="icon" />
                                                </button>
                                            </div>
                                        </div>
                                        <div class="drug-row__line drug-row__line--fields">
                                            <label class="drug-row__cell">
                                                <span class="drug-row__cell-label">"ความแรง"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    placeholder="เช่น 500 mg"
                                                    value=drug.strength_text.clone()
                                                    on:change=on_strength_change
                                                />
                                            </label>
                                            <label class="drug-row__cell">
                                                <span class="drug-row__cell-label">"จำนวน/Stock (ต่อชุด/จำนวนชุด)"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    placeholder="เช่น 10/20"
                                                    value=drug.stock_text.clone()
                                                    on:change=on_stock_change
                                                />
                                            </label>
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view();
                        view! {
                            <label class="field">
                                <span class="field__label">"ชื่อหมวด"</span>
                                <input
                                    class="text-input"
                                    type="text"
                                    prop:value=category_name
                                    on:change=on_rename_category
                                />
                            </label>
                            <form class="picker" on:submit=on_search>
                                <div class="picker__search">
                                    <span class="picker__icon">
                                        <IconSearch class="icon" />
                                    </span>
                                    <input
                                        class="text-input"
                                        type="text"
                                        placeholder="ค้นหายาจากบัญชี HOSxP (อย่างน้อย 2 ตัวอักษร)"
                                        prop:value=move || search_term.get()
                                        on:input=move |ev| search_term.set(event_target_value(&ev))
                                    />
                                    <button class="button-secondary" type="submit" disabled=move || {
                                        searching.get()
                                    }>
                                        {move || if searching.get() { "กำลังค้นหา..." } else { "ค้นหา" }}
                                    </button>
                                </div>
                                <ul class="picker__results">
                                    {move || {
                                        results
                                            .get()
                                            .into_iter()
                                            .map(|item| {
                                                let add_item = item.clone();
                                                let strength_label = item.strength_label();
                                                view! {
                                                    <li class="picker__result">
                                                        <div class="picker__result-info">
                                                            <span class="picker__result-name">
                                                                {item.name.clone()}
                                                            </span>
                                                            <span class="picker__result-meta">
                                                                <span class="badge-icode">
                                                                    {item.icode.clone()}
                                                                </span>
                                                                {(!strength_label.is_empty())
                                                                    .then(move || {
                                                                        view! {
                                                                            <span class="picker__result-strength">
                                                                                {strength_label}
                                                                            </span>
                                                                        }
                                                                    })}
                                                            </span>
                                                        </div>
                                                        <button
                                                            class="button-ghost button-ghost--inline"
                                                            type="button"
                                                            on:click=move |_| on_add_drug(add_item.clone())
                                                        >
                                                            <IconPlus class="icon" />
                                                            "เพิ่ม"
                                                        </button>
                                                    </li>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </ul>
                            </form>
                            <ul class="drug-list">{drug_rows}</ul>
                        }
                            .into_any()
                    }}
                </div>
            </div>
            <StatusLine message=status />
            <div class="panel__actions">
                <span class="panel__dirty">
                    {move || dirty.get().then_some("มีการแก้ไขที่ยังไม่บันทึก")}
                </span>
                <button
                    class="button-secondary"
                    disabled=move || busy.get() || !dirty.get()
                    on:click=on_reset
                >
                    "ยกเลิกการแก้ไข"
                </button>
                <button
                    class="button-accent"
                    disabled=move || busy.get() || !dirty.get()
                    on:click=on_save
                >
                    "บันทึกรายการยา"
                </button>
            </div>
        </div>
    }
}
