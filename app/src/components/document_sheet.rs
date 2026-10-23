//! The requisition sheet: on screen it previews the A4 pages on the stage,
//! and in print media they are the only thing shown - one DOM, one CSS, no
//! second layout (DESIGN.md "sheet").
//!
//! Pagination is explicit (40 drug rows per page, see
//! `er_refill_core::paginate_sections`): every page repeats the document
//! header, the window sentence, and the column headers; a split category
//! continues with "(ต่อ)"; the signature blocks and print stamp render on
//! the last page only. 69 items therefore always print on two sheets.

use std::time::Duration;

use er_refill_core::{ROWS_PER_PAGE, SheetPage, paginate_sections};
use er_refill_models::{ReportSection, format_qty, format_thai_datetime_full};
use leptos::prelude::*;

use crate::components::icons::IconX;
use crate::components::top_bar::BrandMark;
use crate::state::{AppState, BannerKind, PendingEdit};

/// The printed sheet shows the strength only. HOSxP's `units` column (the
/// dispensing unit, e.g. "เม็ด (Cat B)") is appended to the catalog strength
/// label on older entries; hide that suffix while keeping the strength text.
fn printed_strength(text: &str) -> &str {
    let thai = |c: char| ('\u{0E00}'..='\u{0E7F}').contains(&c);
    let cut = text.find(thai).unwrap_or(text.len());
    let prefix = text[..cut].trim_end();
    if prefix.is_empty() || !prefix.chars().any(|c| c.is_ascii_digit()) {
        text.trim()
    } else {
        prefix
    }
}

/// The document: one `<article class="sheet">` per A4 page.
#[component]
pub fn DocumentSheet(state: AppState, sections: Vec<ReportSection>) -> impl IntoView {
    let meta = state
        .settings
        .get()
        .map(|settings| settings.report)
        .unwrap_or_default();
    let adjustments: Vec<(String, String)> = sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .filter(|row| row.has_adjustment())
        .map(|row| (row.name.clone(), row.reason.trim().to_string()))
        .collect();
    let mut pages = paginate_sections(&sections, ROWS_PER_PAGE);
    if pages.is_empty() {
        pages.push(SheetPage {
            number: 1,
            count: 1,
            is_last: true,
            sections: Vec::new(),
        });
    }

    view! {
        {pages
            .into_iter()
            .map(|page| {
                view! {
                    <SheetPageView
                        state=state.clone()
                        page=page
                        meta=meta.clone()
                        adjustments=adjustments.clone()
                    />
                }
            })
            .collect_view()}
    }
}

/// One A4 page of the sheet.
#[component]
fn SheetPageView(
    state: AppState,
    page: SheetPage,
    meta: er_refill_models::ReportMeta,
    adjustments: Vec<(String, String)>,
) -> impl IntoView {
    let is_last = page.is_last;
    let page_label = format!("หน้า {} / {}", page.number, page.count);
    let sections = page.sections;
    let has_sections = !sections.is_empty();

    view! {
        <article class="sheet">
            <header class="sheet__header">
                <div class="sheet__logo">
                    <BrandMark />
                </div>
                <div class="sheet__heading">
                    <h1 class="sheet__hospital">{meta.hospital_name.clone()}</h1>
                    <h2 class="sheet__title">{meta.document_title.clone()}</h2>
                    <p class="sheet__department">{meta.department_name.clone()}</p>
                </div>
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
                                        let row_index = placed.row_index;
                                        let row = placed.row;
                                        let strength =
                                            printed_strength(&row.strength_text).to_string();
                                        let edit_icode = row.icode.clone();
                                        let edit_name = row.name.clone();
                                        let edit_strength = row.strength_text.clone();
                                        let edit_requested = row.requested;
                                        let edit_max = row.max_requested;
                                        let on_edit = {
                                            let state = state.clone();
                                            move |_| {
                                                state.pending_edit.set(Some(PendingEdit {
                                                    icode: edit_icode.clone(),
                                                    name: edit_name.clone(),
                                                    strength_text: edit_strength.clone(),
                                                    old_requested: edit_requested,
                                                    new_requested: edit_requested,
                                                    max_requested: edit_max,
                                                }));
                                            }
                                        };
                                        view! {
                                            <tr class="sheet__row">
                                                <td class="sheet__no">{row_index + 1}</td>
                                                <td class="sheet__name-cell">
                                                    <span class="sheet__drug">{row.name.clone()}</span>
                                                    {(!strength.is_empty())
                                                        .then(|| {
                                                            view! {
                                                                <span class="sheet__strength">
                                                                    {strength.clone()}
                                                                </span>
                                                            }
                                                        })}
                                                </td>
                                                <td class="sheet__stock-cell">{row.stock_text.clone()}</td>
                                                <td class="sheet__num">{format_qty(row.usage)}</td>
                                                <td class="sheet__request-cell">
                                                    <button
                                                        type="button"
                                                        class="sheet__request-button"
                                                        title="แก้ไขจำนวนเบิก (ต้องระบุเหตุผล)"
                                                        aria-label=format!("แก้ไขจำนวนเบิก {}", row.name)
                                                        on:click=on_edit
                                                    >
                                                        {format_qty(row.requested)}
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        }
                    })
                    .collect_view()}
                {(!has_sections)
                    .then(|| {
                        view! {
                            <tbody>
                                <tr class="sheet__empty-row">
                                    <td colspan="5">
                                        "ยังไม่มีรายการยาในช่วงเวลานี้ — กด \"เพิ่มรายการยา\" เพื่อขอเบิก"
                                    </td>
                                </tr>
                            </tbody>
                        }
                    })}
            </table>
            {is_last
                .then(|| {
                    view! {
                        {(!adjustments.is_empty())
                            .then(|| {
                                let rows = adjustments
                                    .into_iter()
                                    .map(|(name, reason)| {
                                        view! {
                                            <tr class="sheet__row">
                                                <td class="sheet__adjustment-name">{name}</td>
                                                <td class="sheet__adjustment-reason">{reason}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view();
                                view! {
                                    <h3 class="sheet__adjustments-title">
                                        "ยาที่ขอเพิ่มหรือขอแก้ไขจำนวนเบิก"
                                    </h3>
                                    <table class="sheet__table sheet__adjustments">
                                        <colgroup>
                                            <col class="sheet__col-adjust-name" />
                                            <col class="sheet__col-adjust-reason" />
                                        </colgroup>
                                        <thead>
                                            <tr>
                                                <th>"ชื่อยา"</th>
                                                <th>"เหตุผล"</th>
                                            </tr>
                                        </thead>
                                        <tbody>{rows}</tbody>
                                    </table>
                                }
                            })}
                        <div class="sheet__signatures">
                            <div class="sheet__signature">
                                <div class="sheet__signature-inner">
                                    <p>"ลงชื่อ .................................................. ผู้เบิก"</p>
                                    <p class="sheet__signer-name">
                                        "( "
                                        {move || {
                                            state
                                                .session
                                                .get()
                                                .map(|user| user.display_name.trim().to_string())
                                                .filter(|name| !name.is_empty())
                                                .unwrap_or_else(|| {
                                                    "..................................................".to_string()
                                                })
                                        }}
                                        " )"
                                    </p>
                                    <p>"วันที่ ......... / ......... / ........."</p>
                                </div>
                            </div>
                            <div class="sheet__signature">
                                <div class="sheet__signature-inner">
                                    <p>"ลงชื่อ .................................................. ผู้จ่าย"</p>
                                    <p>"( .................................................. )"</p>
                                    <p>"วันที่ ......... / ......... / ........."</p>
                                </div>
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

/// Small dismissible strip used above the sheet. Success/info strips dismiss
/// themselves; warnings and errors stay until the operator closes them.
#[component]
pub fn SheetBanner(state: AppState) -> impl IntoView {
    let banner_state = state.clone();
    let auto_dismiss_state = state.clone();
    Effect::new(move |_| {
        let Some(banner) = auto_dismiss_state.banner.get() else {
            return;
        };
        if banner.kind != BannerKind::Info {
            return;
        }
        let message = banner.message.clone();
        let state = auto_dismiss_state.clone();
        let _ = set_timeout_with_handle(
            move || {
                let still_same = state.banner.get_untracked().is_some_and(|current| {
                    current.kind == BannerKind::Info && current.message == message
                });
                if still_same {
                    state.banner.set(None);
                }
            },
            Duration::from_secs(5),
        );
    });

    view! {
        {move || {
            banner_state
                .banner
                .get()
                .map(|banner| {
                    let class = match banner.kind {
                        BannerKind::Error => "banner banner--error",
                        BannerKind::Warn => "banner banner--warn",
                        BannerKind::Info => "banner banner--info",
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

#[cfg(test)]
mod tests {
    use super::printed_strength;

    #[test]
    fn printed_strength_hides_appended_thai_units() {
        assert_eq!(
            printed_strength("500 mg. \u{0e40}\u{0e21}\u{0e47}\u{0e14} (Cat B)"),
            "500 mg."
        );
        assert_eq!(
            printed_strength("250 mg/5ml \u{0e40}\u{0e21}\u{0e47}\u{0e14} (60 ml.)"),
            "250 mg/5ml"
        );
    }

    #[test]
    fn printed_strength_keeps_plain_strength() {
        assert_eq!(printed_strength("500"), "500");
        assert_eq!(printed_strength(""), "");
        assert_eq!(
            printed_strength("\u{0e40}\u{0e21}\u{0e47}\u{0e14}\u{0e25}\u{0e30} 500 mg"),
            "\u{0e40}\u{0e21}\u{0e47}\u{0e14}\u{0e25}\u{0e30} 500 mg"
        );
    }
}
