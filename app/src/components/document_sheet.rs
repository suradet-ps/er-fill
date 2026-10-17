//! The requisition sheet: on screen it previews the A4 pages on the stage,
//! and in print media they are the only thing shown - one DOM, one CSS, no
//! second layout (DESIGN.md "sheet").
//!
//! Pagination is explicit (40 drug rows per page, see
//! `er_fill_core::paginate_sections`): every page repeats the document
//! header, the window sentence, and the column headers; a split category
//! continues with "(ต่อ)"; the signature blocks and print stamp render on
//! the last page only. 69 items therefore always print on two sheets.

use er_fill_core::{ROWS_PER_PAGE, SheetPage, paginate_sections};
use er_fill_models::{ReportSection, format_qty, format_thai_datetime_full};
use leptos::prelude::*;

use crate::components::icons::IconX;
use crate::state::AppState;

/// The document: one `<article class="sheet">` per A4 page.
#[component]
pub fn DocumentSheet(state: AppState, sections: Vec<ReportSection>) -> impl IntoView {
    let meta = state
        .settings
        .get()
        .map(|settings| settings.report)
        .unwrap_or_default();
    let pages = paginate_sections(&sections, ROWS_PER_PAGE);

    view! {
        {pages
            .into_iter()
            .map(|page| {
                view! { <SheetPageView state=state.clone() page=page meta=meta.clone() /> }
            })
            .collect_view()}
    }
}

/// One A4 page of the sheet.
#[component]
fn SheetPageView(
    state: AppState,
    page: SheetPage,
    meta: er_fill_models::ReportMeta,
) -> impl IntoView {
    let is_last = page.is_last;
    let page_label = format!("หน้า {} / {}", page.number, page.count);
    let sections = page.sections;

    view! {
        <article class="sheet">
            <header class="sheet__header">
                <h1 class="sheet__hospital">{meta.hospital_name.clone()}</h1>
                <h2 class="sheet__title">{meta.document_title.clone()}</h2>
                <p class="sheet__department">{meta.department_name.clone()}</p>
            </header>
            <p class="sheet__window">
                "ช่วงเวลาเบิก: "
                {move || {
                    state
                        .window
                        .get()
                        .map(|window| {
                            format!(
                                "{} ถึง {}",
                                format_thai_datetime_full(window.start),
                                format_thai_datetime_full(window.end),
                            )
                        })
                        .unwrap_or_default()
                }}
            </p>
            <table class="sheet__table">
                <colgroup>
                    <col class="sheet__col-no" />
                    <col class="sheet__col-name" />
                    <col class="sheet__col-stock" />
                    <col class="sheet__col-usage" />
                    <col class="sheet__col-request" />
                </colgroup>
                <thead>
                    <tr>
                        <th>"ลำดับ"</th>
                        <th>"ชื่อยา"</th>
                        <th>"จำนวน/Stock"</th>
                        <th class="sheet__th-num">"จำนวนใช้"</th>
                        <th class="sheet__th-num">"จำนวนเบิก"</th>
                    </tr>
                </thead>
                {sections
                    .into_iter()
                    .map(|section| {
                        let category_label = if section.continued {
                            format!("{} (ต่อ)", section.category_name)
                        } else {
                            section.category_name.clone()
                        };
                        view! {
                            <tbody>
                                <tr class="sheet__category">
                                    <td colspan="5">{category_label}</td>
                                </tr>
                                {section
                                    .rows
                                    .into_iter()
                                    .map(|placed| {
                                        let section_index = placed.section_index;
                                        let row_index = placed.row_index;
                                        let row = placed.row;
                                        let on_request = {
                                            let state = state.clone();
                                            move |ev| {
                                                let raw = event_target_value(&ev);
                                                let Ok(value) = raw.parse::<f64>() else {
                                                    return;
                                                };
                                                if !(value.is_finite() && value >= 0.0) {
                                                    return;
                                                }
                                                state.sections.update(|sections| {
                                                    if let Some(row) = sections
                                                        .as_mut()
                                                        .and_then(|sections| sections.get_mut(section_index))
                                                        .and_then(|section| section.rows.get_mut(row_index))
                                                    {
                                                        row.requested = value;
                                                    }
                                                });
                                            }
                                        };
                                        view! {
                                            <tr class="sheet__row">
                                                <td class="sheet__no">{row_index + 1}</td>
                                                <td class="sheet__name-cell">
                                                    <span class="sheet__drug">{row.name.clone()}</span>
                                                    {(!row.strength_text.is_empty())
                                                        .then(|| {
                                                            view! {
                                                                <span class="sheet__strength">
                                                                    {row.strength_text.clone()}
                                                                </span>
                                                            }
                                                        })}
                                                    <span class="sheet__icode">{row.icode.clone()}</span>
                                                </td>
                                                <td class="sheet__stock-cell">{row.stock_text.clone()}</td>
                                                <td class="sheet__num">{format_qty(row.usage)}</td>
                                                <td class="sheet__request-cell">
                                                    <input
                                                        class="sheet__request-input"
                                                        type="number"
                                                        min="0"
                                                        step="any"
                                                        value=format_qty(row.requested)
                                                        on:change=on_request
                                                        aria-label=format!("จำนวนเบิก {}", row.name)
                                                    />
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        }
                    })
                    .collect_view()}
            </table>
            {is_last
                .then(|| {
                    view! {
                        <div class="sheet__signatures">
                            <div class="sheet__signature">
                                <p>"ลงชื่อ .................................................. ผู้เบิก"</p>
                                <p>"( .................................................. )"</p>
                                <p>"วันที่ ......... / ......... / ........."</p>
                            </div>
                            <div class="sheet__signature">
                                <p>"ลงชื่อ .................................................. ผู้จ่าย"</p>
                                <p>"( .................................................. )"</p>
                                <p>"วันที่ ......... / ......... / ........."</p>
                            </div>
                        </div>
                    }
                })}
            <footer class="sheet__footer">
                <span>{page_label}</span>
                {is_last
                    .then_some(move || {
                        state
                            .printed_at
                            .get()
                            .map(|printed_at| {
                                format!("พิมพ์เมื่อ {}", format_thai_datetime_full(printed_at))
                            })
                            .unwrap_or_default()
                    })}
            </footer>
        </article>
    }
}

/// Small dismissible strip used above the sheet (errors, warnings).
#[component]
pub fn SheetBanner(state: AppState) -> impl IntoView {
    let banner_state = state.clone();
    view! {
        {move || {
            banner_state
                .banner
                .get()
                .map(|banner| {
                    let class = match banner.kind {
                        crate::state::BannerKind::Error => "banner banner--error",
                        crate::state::BannerKind::Warn => "banner banner--warn",
                        crate::state::BannerKind::Info => "banner banner--info",
                    };
                    let close_state = banner_state.clone();
                    view! {
                        <div class=class role="status">
                            <span class="banner__text">{banner.message}</span>
                            <button
                                class="banner__close"
                                aria-label="ปิด"
                                on:click=move |_| close_state.banner.set(None)
                            >
                                <IconX class="icon" />
                            </button>
                        </div>
                    }
                })
        }}
    }
}
