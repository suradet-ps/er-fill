//! Settings modal: a three-panel dialog (connection, catalog, report) with a
//! vertical navigation rail, per DESIGN.md "settings-modal". Panels live in
//! their own modules; this file owns the shell, the nav, and the shared
//! status line.

use leptos::prelude::*;

use crate::components::catalog_panel::CatalogPanel;
use crate::components::connection_panel::ConnectionPanel;
use crate::components::icons::{IconClipboardList, IconDatabase, IconFileText, IconX};
use crate::components::report_panel::ReportPanel;
use crate::state::{AppState, SettingsTab};

/// A success/failure line under a panel's actions.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    /// True renders in mint (`status-line--ok`), false in red.
    pub ok: bool,
    /// Thai message text.
    pub text: String,
}

impl StatusMessage {
    /// Success constructor.
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            ok: true,
            text: text.into(),
        }
    }

    /// Failure constructor.
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            ok: false,
            text: text.into(),
        }
    }
}

/// Renders a [`StatusMessage`] slot (`None` renders nothing).
#[component]
pub fn StatusLine(message: RwSignal<Option<StatusMessage>>) -> impl IntoView {
    view! {
        {move || {
            message
                .get()
                .map(|message| {
                    let class = if message.ok {
                        "status-line status-line--ok"
                    } else {
                        "status-line status-line--error"
                    };
                    view! { <p class=class>{message.text}</p> }
                })
        }}
    }
}

const TABS: [(SettingsTab, &str); 3] = [
    (SettingsTab::Connection, "การเชื่อมต่อ"),
    (SettingsTab::Catalog, "รายการยา"),
    (SettingsTab::Report, "หัวใบเบิก"),
];

/// The tab's icon, per the vertical navigation.
fn tab_icon(tab: SettingsTab) -> AnyView {
    match tab {
        SettingsTab::Connection => view! { <IconDatabase class="icon" /> }.into_any(),
        SettingsTab::Catalog => view! { <IconClipboardList class="icon" /> }.into_any(),
        SettingsTab::Report => view! { <IconFileText class="icon" /> }.into_any(),
    }
}

/// The modal shell - mounted always, rendered only while open. Panels
/// remount on every open, so typed passwords never linger in a hidden DOM
/// tree.
///
/// Layout: header / vertical settings nav + content / footer. The vertical
/// nav (instead of the old top tabs) gives the catalog manager the full
/// modal width it needs for the three-field drug rows.
#[component]
pub fn SettingsModal(state: AppState) -> impl IntoView {
    // Escape closes the dialog (handle kept alive for the component's life).
    let _escape = window_event_listener(leptos::ev::keydown, {
        let state = state.clone();
        move |ev| {
            if ev.key() == "Escape" {
                state.settings_open.set(false);
            }
        }
    });

    view! {
        {move || {
            if !state.settings_open.get() {
                return None;
            }
            let close_backdrop = {
                let state = state.clone();
                move |_| state.settings_open.set(false)
            };
            let close_button = {
                let state = state.clone();
                move |_| state.settings_open.set(false)
            };
            let stop_propagation = move |ev: web_sys::MouseEvent| ev.stop_propagation();
            let nav_state = state.clone();
            let body_state = state.clone();
            let footer_state = state.clone();
            let signed_in = Memo::new({
                let state = state.clone();
                move |_| state.session.get().is_some()
            });
            let nav_items = move || {
                TABS
                    .into_iter()
                    .filter(|(tab, _)| signed_in.get() || *tab == SettingsTab::Connection)
                    .map(|(tab, label)| {
                        let active = Memo::new({
                            let state = nav_state.clone();
                            move |_| state.settings_tab.get() == tab
                        });
                        let on_click = {
                            let state = nav_state.clone();
                            move |_| state.settings_tab.set(tab)
                        };
                        view! {
                            <button
                                class=move || if active.get() {
                                    "settings-nav__item settings-nav__item--active"
                                } else {
                                    "settings-nav__item"
                                }
                                role="tab"
                                aria-selected=move || active.get().to_string()
                                on:click=on_click
                            >
                                {tab_icon(tab)}
                                <span>{label}</span>
                            </button>
                        }
                    })
                    .collect_view()
            };
            Some(
                view! {
                    <div class="modal-backdrop" on:click=close_backdrop>
                        <div
                            class="settings-modal"
                            role="dialog"
                            aria-modal="true"
                            aria-label="ตั้งค่า"
                            on:click=stop_propagation
                        >
                            <header class="settings-modal__header">
                                <h2 class="settings-modal__title">
                                    {move || {
                                        if signed_in.get() {
                                            "ตั้งค่า"
                                        } else {
                                            "ตั้งค่าการเชื่อมต่อ"
                                        }
                                    }}
                                </h2>
                                <button
                                    class="button-icon-circular"
                                    aria-label="ปิด"
                                    on:click=close_button
                                >
                                    <IconX class="icon" />
                                </button>
                            </header>
                            <div class="settings-layout">
                                <nav class="settings-nav" role="tablist">
                                    {nav_items}
                                </nav>
                                <div class="settings-content">
                                    {move || {
                                        let tab = if body_state.session.get().is_some() {
                                            body_state.settings_tab.get()
                                        } else {
                                            SettingsTab::Connection
                                        };
                                        match tab {
                                            SettingsTab::Connection => {
                                                view! { <ConnectionPanel state=body_state.clone() /> }
                                                    .into_any()
                                            }
                                            SettingsTab::Catalog => {
                                                view! { <CatalogPanel state=body_state.clone() /> }
                                                    .into_any()
                                            }
                                            SettingsTab::Report => {
                                                view! { <ReportPanel state=body_state.clone() /> }
                                                    .into_any()
                                            }
                                        }
                                    }}
                                </div>
                            </div>
                            {move || {
                                footer_state
                                    .session
                                    .get()
                                    .is_some()
                                    .then(|| {
                                        view! { <SettingsFooter state=footer_state.clone() /> }
                                    })
                            }}
                        </div>
                    </div>
                },
            )
        }}
    }
}

/// Modal footer: portable settings export/import. Recreated on every open,
/// so its status line never lingers across sessions.
///
/// The exported JSON carries the catalog and report header. Connection
/// credentials are deliberately excluded: they are encrypted with this
/// machine's OS-keychain key (AGENTS.md section 10), so they can not travel.
#[component]
fn SettingsFooter(state: AppState) -> impl IntoView {
    let status = RwSignal::new(None::<StatusMessage>);
    let file_input = NodeRef::<leptos::html::Input>::new();
    let export_busy = RwSignal::new(false);

    let on_export = {
        let state = state.clone();
        move |_| {
            if export_busy.get_untracked() {
                return;
            }
            let Some(settings) = state.settings.get_untracked() else {
                return;
            };
            let json = match crate::settings_io::export_json(&settings) {
                Ok(json) => json,
                Err(message) => {
                    status.set(Some(StatusMessage::error(message)));
                    return;
                }
            };
            let file_name = crate::settings_io::export_file_name();
            export_busy.set(true);
            status.set(None);
            let status = status;
            leptos::task::spawn_local(async move {
                let message = match crate::api::export_settings_file(&json, &file_name).await {
                    Ok(result) if result.saved => {
                        let text = result
                            .path
                            .map(|path| format!("บันทึกไฟล์แล้ว: {path}"))
                            .unwrap_or_else(|| "บันทึกไฟล์แล้ว".to_string());
                        Some(StatusMessage::ok(text))
                    }
                    Ok(_) => None,
                    Err(err) => Some(StatusMessage::error(err.message)),
                };
                status.set(message);
                export_busy.set(false);
            });
        }
    };

    let open_file_picker = move |_: web_sys::MouseEvent| {
        if let Some(input) = file_input.get() {
            input.click();
        }
    };

    let on_import_file = {
        let state = state.clone();
        move |ev| {
            let input = event_target::<web_sys::HtmlInputElement>(&ev);
            let Some(file) = input.files().and_then(|files| files.get(0)) else {
                return;
            };
            status.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                let message = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                    Ok(value) => match value.as_string() {
                        Some(raw) => match crate::settings_io::parse_import(&raw) {
                            Ok(settings) => match persist_import(&state, &settings).await {
                                Ok(summary) => StatusMessage::ok(summary),
                                Err(message) => StatusMessage::error(message),
                            },
                            Err(message) => StatusMessage::error(message),
                        },
                        None => StatusMessage::error("อ่านไฟล์ไม่สำเร็จ"),
                    },
                    Err(_) => StatusMessage::error("อ่านไฟล์ไม่สำเร็จ"),
                };
                status.set(Some(message));
                input.set_value("");
            });
        }
    };

    view! {
        <footer class="settings-modal__footer">
            <div class="settings-modal__io">
                <button
                    class="button-secondary"
                    disabled=move || export_busy.get()
                    on:click=on_export
                >
                    {move || {
                        if export_busy.get() {
                            "กำลังเปิดหน้าต่างบันทึก..."
                        } else {
                            "ส่งออกการตั้งค่า"
                        }
                    }}
                </button>
                <button class="button-secondary" on:click=open_file_picker>
                    "นำเข้าการตั้งค่า"
                </button>
                <input
                    node_ref=file_input
                    class="settings-modal__file"
                    type="file"
                    accept="application/json,.json"
                    on:change=on_import_file
                    tabindex="-1"
                />
            </div>
            <p class="settings-modal__io-note">
                "ไฟล์ส่งออกเป็น JSON (รายการยาและหัวใบเบิก) สำหรับใช้กับเครื่องอื่น เมื่อกดส่งออก ระบบจะเปิดหน้าต่างให้เลือกโฟลเดอร์และชื่อไฟล์เอง ส่วนการเชื่อมต่อและรหัสผ่านต้องตั้งค่าในแต่ละเครื่อง"
            </p>
            <StatusLine message=status />
        </footer>
    }
}

/// Saves an imported bundle through the ordinary commands, then refreshes
/// the in-memory state; returns the Thai success summary.
async fn persist_import(
    state: &AppState,
    settings: &er_refill_models::AppSettings,
) -> Result<String, String> {
    crate::api::save_catalog(&settings.catalog)
        .await
        .map_err(|err| err.message)?;
    crate::api::save_report(&settings.report)
        .await
        .map_err(|err| err.message)?;
    state.settings.set(Some(settings.clone()));
    state.clear_requisition();
    Ok(format!(
        "นำเข้าการตั้งค่าแล้ว: {} หมวด, {} รายการยา",
        settings.catalog.categories.len(),
        settings.catalog.drugs.len()
    ))
}
