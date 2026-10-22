//! Settings tab: HOSxP connection credentials. The plaintext is typed here
//! and handed to the backend, which encrypts it (AES-256-GCM + OS keychain)
//! before anything touches disk (AGENTS.md section 10). The panel remounts every
//! time the modal opens, so no credential lingers in a hidden DOM tree.

use leptos::prelude::*;

use crate::api::{self, ConnectionInput};
use crate::components::settings_modal::{StatusLine, StatusMessage};
use crate::state::{AppState, ConnectionHealth};

/// The connection form: host · port · database · user · password, with a
/// `SELECT 1` test and an encrypted save.
#[component]
pub fn ConnectionPanel(state: AppState) -> impl IntoView {
    let summary = state.connection_summary.get_untracked();
    let host = RwSignal::new(
        summary
            .as_ref()
            .map(|summary| summary.host.clone())
            .unwrap_or_default(),
    );
    let port = RwSignal::new(
        summary
            .as_ref()
            .map(|summary| summary.port.to_string())
            .unwrap_or_else(|| "3306".to_string()),
    );
    let database = RwSignal::new(
        summary
            .as_ref()
            .map(|summary| summary.database.clone())
            .unwrap_or_default(),
    );
    let user = RwSignal::new(
        summary
            .as_ref()
            .map(|summary| summary.user.clone())
            .unwrap_or_default(),
    );
    let password = RwSignal::new(String::new());
    let status = RwSignal::new(None::<StatusMessage>);
    let busy = RwSignal::new(false);

    let build_input = move || -> Result<ConnectionInput, String> {
        let port_value = port
            .get()
            .trim()
            .parse::<u16>()
            .map_err(|_| "พอร์ตต้องเป็นตัวเลข 1-65535".to_string())?;
        Ok(ConnectionInput {
            host: host.get().trim().to_string(),
            port: port_value,
            database: database.get().trim().to_string(),
            user: user.get().trim().to_string(),
            password: password.get(),
        })
    };

    let on_test = move |_| {
        let input = match build_input() {
            Ok(input) => input,
            Err(message) => {
                status.set(Some(StatusMessage::error(message)));
                return;
            }
        };
        busy.set(true);
        status.set(None);
        leptos::task::spawn_local(async move {
            match api::test_connection(&input).await {
                Ok(latency_ms) => status.set(Some(StatusMessage::ok(format!(
                    "เชื่อมต่อสำเร็จ ({latency_ms} ms)"
                )))),
                Err(err) => status.set(Some(StatusMessage::error(err.message))),
            }
            busy.set(false);
        });
    };

    let on_save = {
        let state = state.clone();
        move |_| {
            let input = match build_input() {
                Ok(input) => input,
                Err(message) => {
                    status.set(Some(StatusMessage::error(message)));
                    return;
                }
            };
            busy.set(true);
            status.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::connect(&input).await {
                    Ok(latency_ms) => {
                        state.configured.set(true);
                        state.health.set(ConnectionHealth::Connected);
                        state.connection_error.set(None);
                        state
                            .connection_summary
                            .set(Some(er_refill_models::ConnectionSummary {
                                host: input.host.clone(),
                                port: input.port,
                                database: input.database.clone(),
                                user: input.user.clone(),
                            }));
                        state.clear_session();
                        status.set(Some(StatusMessage::ok(format!(
                            "เชื่อมต่อและบันทึกการตั้งค่าแล้ว ({latency_ms} ms)"
                        ))));
                    }
                    Err(err) => {
                        state.health.set(ConnectionHealth::Disconnected);
                        state.connection_error.set(Some(err.message.clone()));
                        status.set(Some(StatusMessage::error(err.message)));
                    }
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="panel">
            <h3 class="panel__subtitle">"ฐานข้อมูล HOSxP"</h3>
            <div class="form-grid">
                <label class="field">
                    <span class="field__label">"ที่อยู่เซิร์ฟเวอร์ HOSxP" <span class="required">"จำเป็น"</span></span>
                    <input
                        class="text-input"
                        type="text"
                        placeholder="192.168.1.10"
                        prop:value=move || host.get()
                        on:input=move |ev| host.set(event_target_value(&ev))
                    />
                </label>
                <label class="field">
                    <span class="field__label">"พอร์ต"</span>
                    <input
                        class="text-input"
                        type="text"
                        inputmode="numeric"
                        prop:value=move || port.get()
                        on:input=move |ev| port.set(event_target_value(&ev))
                    />
                </label>
                <label class="field">
                    <span class="field__label">"ชื่อฐานข้อมูล" <span class="required">"จำเป็น"</span></span>
                    <input
                        class="text-input"
                        type="text"
                        placeholder="hos"
                        prop:value=move || database.get()
                        on:input=move |ev| database.set(event_target_value(&ev))
                    />
                </label>
                <label class="field">
                    <span class="field__label">"ผู้ใช้ (อ่านอย่างเดียว)" <span class="required">"จำเป็น"</span></span>
                    <input
                        class="text-input"
                        type="text"
                        placeholder="errefill_ro"
                        autocomplete="off"
                        prop:value=move || user.get()
                        on:input=move |ev| user.set(event_target_value(&ev))
                    />
                </label>
                <label class="field form-grid__span-2">
                    <span class="field__label">"รหัสผ่าน"</span>
                    <input
                        class="text-input"
                        type="password"
                        autocomplete="new-password"
                        prop:value=move || password.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                    />
                </label>
            </div>
            <StatusLine message=status />
            <div class="panel__actions">
                <button class="button-secondary" disabled=move || busy.get() on:click=on_test>
                    "ทดสอบการเชื่อมต่อ"
                </button>
                <button class="button-primary" disabled=move || busy.get() on:click=on_save>
                    "บันทึกการเชื่อมต่อ"
                </button>
            </div>
        </div>
    }
}
