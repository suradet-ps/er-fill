//! Settings tab: the printed header block - hospital name, document title,
//! issuing department.

use er_fill_models::ReportMeta;
use leptos::prelude::*;

use crate::api;
use crate::components::settings_modal::{StatusLine, StatusMessage};
use crate::state::AppState;

/// The report-header form.
#[component]
pub fn ReportPanel(state: AppState) -> impl IntoView {
    let initial = state
        .settings
        .get_untracked()
        .map(|settings| settings.report)
        .unwrap_or_default();
    let hospital_name = RwSignal::new(initial.hospital_name);
    let document_title = RwSignal::new(initial.document_title);
    let department_name = RwSignal::new(initial.department_name);
    let status = RwSignal::new(None::<StatusMessage>);
    let busy = RwSignal::new(false);

    let on_save = {
        let state = state.clone();
        move |_| {
            let report = ReportMeta {
                hospital_name: hospital_name.get(),
                document_title: document_title.get(),
                department_name: department_name.get(),
            };
            busy.set(true);
            status.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::save_report(&report).await {
                    Ok(()) => {
                        state.settings.update(|settings| {
                            if let Some(settings) = settings {
                                settings.report = report;
                            }
                        });
                        status.set(Some(StatusMessage::ok("บันทึกหัวใบเบิกแล้ว")));
                    }
                    Err(err) => status.set(Some(StatusMessage::error(err.message))),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="panel">
            <h3 class="panel__subtitle">"ข้อความบนหัวใบเบิก"</h3>
            <label class="field">
                <span class="field__label">"ชื่อโรงพยาบาล"</span>
                <input
                    class="text-input"
                    type="text"
                    prop:value=move || hospital_name.get()
                    on:input=move |ev| hospital_name.set(event_target_value(&ev))
                />
            </label>
            <label class="field">
                <span class="field__label">"ชื่อเอกสาร"</span>
                <input
                    class="text-input"
                    type="text"
                    prop:value=move || document_title.get()
                    on:input=move |ev| document_title.set(event_target_value(&ev))
                />
            </label>
            <label class="field">
                <span class="field__label">"หน่วยงาน (บรรทัดใต้ชื่อเอกสาร)"</span>
                <input
                    class="text-input"
                    type="text"
                    prop:value=move || department_name.get()
                    on:input=move |ev| department_name.set(event_target_value(&ev))
                />
            </label>
            <div class="form-preview" aria-hidden="true">
                <p class="form-preview__hospital">{move || hospital_name.get()}</p>
                <p class="form-preview__title">{move || document_title.get()}</p>
                <p class="form-preview__dept">{move || department_name.get()}</p>
            </div>
            <p class="panel-note">"ตัวอย่างด้านบนคือหัวใบเบิกที่จะพิมพ์ออกมา"</p>
            <StatusLine message=status />
            <div class="panel__actions">
                <button class="button-primary" disabled=move || busy.get() on:click=on_save>
                    "บันทึกหัวใบเบิก"
                </button>
            </div>
        </div>
    }
}
