//! Login gate: connect to HOSxP first, then verify an `officer` account.
//!
//! The connection is tested against the live database before the login form
//! is enabled. Passwords are verified in the Rust backend against the stored
//! MD5 and are never persisted (docs/auth.md).

use er_fill_models::ConnectionSummary;
use leptos::prelude::*;

use crate::api::{self, ConnectionInput};
use crate::components::icons::{
    IconArrowRight, IconCheck, IconDatabase, IconLock, IconShieldCheck, IconUser,
};
use crate::components::top_bar::BrandMark;
use crate::state::{AppState, ConnectionHealth};

/// Full-screen login surface used until a HOSxP officer signs in.
#[component]
pub fn LoginScreen(state: AppState) -> impl IntoView {
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
    let db_user = RwSignal::new(
        summary
            .as_ref()
            .map(|summary| summary.user.clone())
            .unwrap_or_default(),
    );
    let db_password = RwSignal::new(String::new());
    let edit_connection = RwSignal::new(summary.is_none());
    let connection_busy = RwSignal::new(false);
    let connection_error = RwSignal::new(state.connection_error.get_untracked());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let login_busy = RwSignal::new(false);
    let login_error = RwSignal::new(None::<String>);

    let connected = Memo::new(move |_| {
        state.configured.get() && state.health.get() == ConnectionHealth::Connected
    });
    let show_connection_form = Memo::new(move |_| {
        !state.configured.get() || state.connection_summary.get().is_none() || edit_connection.get()
    });

    let build_input = move || -> Result<ConnectionInput, String> {
        let parsed_port = port
            .get()
            .trim()
            .parse::<u16>()
            .map_err(|_| "พอร์ตต้องเป็นตัวเลข 1-65535".to_string())?;
        if host.get().trim().is_empty() {
            return Err("กรุณากรอกที่อยู่เซิร์ฟเวอร์".to_string());
        }
        if database.get().trim().is_empty() {
            return Err("กรุณากรอกชื่อฐานข้อมูล".to_string());
        }
        if db_user.get().trim().is_empty() {
            return Err("กรุณากรอกผู้ใช้ฐานข้อมูล".to_string());
        }
        Ok(ConnectionInput {
            host: host.get().trim().to_string(),
            port: parsed_port,
            database: database.get().trim().to_string(),
            user: db_user.get().trim().to_string(),
            password: db_password.get(),
        })
    };

    let on_connect = {
        let state = state.clone();
        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            let input = match build_input() {
                Ok(input) => input,
                Err(message) => {
                    connection_error.set(Some(message));
                    return;
                }
            };
            connection_busy.set(true);
            connection_error.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::connect(&input).await {
                    Ok(_) => {
                        let summary = ConnectionSummary {
                            host: input.host.clone(),
                            port: input.port,
                            database: input.database.clone(),
                            user: input.user.clone(),
                        };
                        state.configured.set(true);
                        state.health.set(ConnectionHealth::Connected);
                        state.connection_summary.set(Some(summary));
                        state.connection_error.set(None);
                        connection_error.set(None);
                        db_password.set(String::new());
                        edit_connection.set(false);
                    }
                    Err(err) => {
                        state.health.set(ConnectionHealth::Disconnected);
                        state.connection_error.set(Some(err.message.clone()));
                        connection_error.set(Some(err.message));
                    }
                }
                connection_busy.set(false);
            });
        }
    };

    let on_reconnect = {
        let state = state.clone();
        move |_| {
            connection_busy.set(true);
            connection_error.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::reconnect().await {
                    Ok(_) => {
                        state.health.set(ConnectionHealth::Connected);
                        state.connection_error.set(None);
                        connection_error.set(None);
                        edit_connection.set(false);
                    }
                    Err(err) => {
                        state.health.set(ConnectionHealth::Disconnected);
                        state.connection_error.set(Some(err.message.clone()));
                        connection_error.set(Some(err.message));
                    }
                }
                connection_busy.set(false);
            });
        }
    };

    let on_change_connection = move |_| {
        connection_error.set(None);
        edit_connection.set(true);
    };
    let on_cancel_connection = move |_| {
        connection_error.set(None);
        edit_connection.set(false);
    };

    let on_login = {
        let state = state.clone();
        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            if !connected.get() {
                return;
            }
            let login_name = username.get();
            if login_name.trim().is_empty() {
                login_error.set(Some("กรุณากรอกชื่อผู้ใช้".to_string()));
                return;
            }
            let login_password = password.get();
            if login_password.is_empty() {
                login_error.set(Some("กรุณากรอกรหัสผ่าน".to_string()));
                return;
            }
            login_busy.set(true);
            login_error.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::login(login_name.trim(), &login_password).await {
                    Ok(user) => {
                        password.set(String::new());
                        state.finish_login(user);
                    }
                    Err(err) => {
                        if err.kind == api::ApiErrorKind::Connection {
                            state.health.set(ConnectionHealth::Disconnected);
                            state.connection_error.set(Some(err.message.clone()));
                            edit_connection.set(false);
                        }
                        login_error.set(Some(err.message));
                    }
                }
                login_busy.set(false);
            });
        }
    };

    view! {
        <div class="login">
            <aside class="login__brand">
                <div class="login__brand-top">
                    <BrandMark />
                    <div class="login__brand-titles">
                        <span class="login__brand-name">"ER-Fill"</span>
                        <span class="login__brand-sub">"ระบบใบเบิกยาห้องฉุกเฉิน"</span>
                    </div>
                </div>

                <div class="login__hero">
                    <span class="login__eyebrow">"สำหรับกลุ่มงานเภสัชกรรม"</span>
                    <h1 class="login__headline">
                        "ใบเบิกยาห้องฉุกเฉิน พร้อมพิมพ์ทุกเช้า"
                    </h1>
                    <p class="login__copy">
                        "ดึงยอดใช้ยานอกเวลาจาก HOSxP แบบอ่านอย่างเดียว แล้วสร้างใบเบิกที่ห้องฉุกเฉินเซ็นรับได้ทันที"
                    </p>
                    <ul class="login__points">
                        <li class="login__point">
                            <span class="login__point-icon">
                                <IconShieldCheck class="icon" />
                            </span>
                            <span>
                                <b>"Read-only"</b>
                                <small>"ไม่มีการเขียนข้อมูลกลับไปยัง HOSxP"</small>
                            </span>
                        </li>
                        <li class="login__point">
                            <span class="login__point-icon">
                                <IconUser class="icon" />
                            </span>
                            <span>
                                <b>"บัญชี HOSxP"</b>
                                <small>"ใช้ชื่อผู้ใช้และรหัสผ่านเดียวกับระบบเดิม"</small>
                            </span>
                        </li>
                        <li class="login__point">
                            <span class="login__point-icon">
                                <IconLock class="icon" />
                            </span>
                            <span>
                                <b>"ไม่เก็บรหัสผ่าน"</b>
                                <small>"ตรวจสอบในหน่วยความจำและไม่บันทึกลงเครื่อง"</small>
                            </span>
                        </li>
                    </ul>
                </div>

                <p class="login__brand-foot">
                    "โรงพยาบาลสระโบสถ์ · กลุ่มงานเภสัชกรรม"
                </p>
            </aside>

            <main class="login__main">
                <div class="login__card">
                    <ol class="login__steps">
                        <li class=move || {
                            if connected.get() {
                                "login__step login__step--done"
                            } else {
                                "login__step login__step--active"
                            }
                        }>
                            <span class="login__step-index">"01"</span>
                            "การเชื่อมต่อ"
                        </li>
                        <li class=move || {
                            if connected.get() {
                                "login__step login__step--active"
                            } else {
                                "login__step"
                            }
                        }>
                            <span class="login__step-index">"02"</span>
                            "เข้าสู่ระบบ"
                        </li>
                    </ol>

                    <section class="login__section">
                        <header class="login__section-head">
                            <div>
                                <h2 class="login__section-title">"เชื่อมต่อฐานข้อมูล HOSxP"</h2>
                                <p class="login__section-copy">
                                    "ตรวจสอบการเชื่อมต่อก่อน แล้วจึงเข้าสู่ระบบด้วยบัญชี officer"
                                </p>
                            </div>
                            {move || {
                                connected.get().then(|| {
                                    view! {
                                        <span class="login__status">
                                            <IconCheck class="icon" />
                                            "เชื่อมต่อแล้ว"
                                        </span>
                                    }
                                })
                            }}
                        </header>

                        {move || {
                            if show_connection_form.get() {
                                view! {
                                    <form class="login__form" on:submit=on_connect.clone()>
                                        <div class="login__grid">
                                            <label class="login__field login__grid-span-2">
                                                <span>"ที่อยู่เซิร์ฟเวอร์ HOSxP"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    placeholder="192.168.1.10"
                                                    prop:value=move || host.get()
                                                    on:input=move |ev| host.set(event_target_value(&ev))
                                                />
                                            </label>
                                            <label class="login__field">
                                                <span>"พอร์ต"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    inputmode="numeric"
                                                    prop:value=move || port.get()
                                                    on:input=move |ev| port.set(event_target_value(&ev))
                                                />
                                            </label>
                                            <label class="login__field">
                                                <span>"ชื่อฐานข้อมูล"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    placeholder="hos"
                                                    prop:value=move || database.get()
                                                    on:input=move |ev| database.set(event_target_value(&ev))
                                                />
                                            </label>
                                            <label class="login__field">
                                                <span>"ผู้ใช้ (อ่านอย่างเดียว)"</span>
                                                <input
                                                    class="text-input"
                                                    type="text"
                                                    placeholder="erfill_ro"
                                                    autocomplete="off"
                                                    prop:value=move || db_user.get()
                                                    on:input=move |ev| db_user.set(event_target_value(&ev))
                                                />
                                            </label>
                                            <label class="login__field">
                                                <span>"รหัสผ่านฐานข้อมูล"</span>
                                                <input
                                                    class="text-input"
                                                    type="password"
                                                    autocomplete="off"
                                                    prop:value=move || db_password.get()
                                                    on:input=move |ev| db_password.set(event_target_value(&ev))
                                                />
                                            </label>
                                        </div>
                                        <div class="login__actions">
                                            {(state.connection_summary.get().is_some())
                                                .then(|| {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="button-ghost"
                                                            disabled=move || connection_busy.get()
                                                            on:click=on_cancel_connection
                                                        >
                                                            "ยกเลิก"
                                                        </button>
                                                    }
                                                })}
                                            <button
                                                type="submit"
                                                class="button-primary"
                                                disabled=move || connection_busy.get()
                                            >
                                                <IconArrowRight class="icon" />
                                                {move || {
                                                    if connection_busy.get() {
                                                        "กำลังเชื่อมต่อ..."
                                                    } else {
                                                        "เชื่อมต่อและดำเนินการต่อ"
                                                    }
                                                }}
                                            </button>
                                        </div>
                                    </form>
                                }
                                .into_any()
                            } else {
                                view! {
                                    <div class="login__connection">
                                        <span class="login__connection-note">
                                            <IconShieldCheck class="icon" />
                                            "ใช้การเชื่อมต่อ HOSxP ที่บันทึกไว้บนเครื่องนี้"
                                        </span>
                                        <div class="login__connection-actions">
                                            <button
                                                class="button-ghost"
                                                disabled=move || connection_busy.get()
                                                on:click=on_change_connection
                                            >
                                                "เปลี่ยนการเชื่อมต่อ"
                                            </button>
                                            {(!connected.get())
                                                .then(|| {
                                                    view! {
                                                        <button
                                                            class="button-secondary"
                                                            disabled=move || connection_busy.get()
                                                            on:click=on_reconnect.clone()
                                                        >
                                                            "ลองเชื่อมต่ออีกครั้ง"
                                                        </button>
                                                    }
                                                })}
                                        </div>
                                    </div>
                                }
                                .into_any()
                            }
                        }}

                        {move || {
                            connection_error.get().map(|message| {
                                view! {
                                    <p class="login__error" role="alert">
                                        <IconDatabase class="icon" />
                                        {message}
                                    </p>
                                }
                            })
                        }}
                    </section>

                    <section class=move || {
                        if connected.get() {
                            "login__section"
                        } else {
                            "login__section login__section--muted"
                        }
                    }>
                        <header class="login__section-head">
                            <div>
                                <h2 class="login__section-title">"เข้าสู่ระบบ"</h2>
                                <p class="login__section-copy">
                                    "ใช้บัญชี officer เดียวกับ HOSxP"
                                </p>
                            </div>
                        </header>

                        <form class="login__form" on:submit=on_login>
                            <label class="login__field">
                                <span>"ชื่อผู้ใช้"</span>
                                <input
                                    class="text-input"
                                    type="text"
                                    autocomplete="username"
                                    prop:value=move || username.get()
                                    on:input=move |ev| username.set(event_target_value(&ev))
                                />
                            </label>
                            <label class="login__field">
                                <span>"รหัสผ่าน"</span>
                                <input
                                    class="text-input"
                                    type="password"
                                    autocomplete="current-password"
                                    prop:value=move || password.get()
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                />
                            </label>

                            {move || {
                                login_error.get().map(|message| {
                                    view! {
                                        <p class="login__error" role="alert">
                                            <IconLock class="icon" />
                                            {message}
                                        </p>
                                    }
                                })
                            }}

                            <div class="login__actions">
                                <button
                                    type="submit"
                                    class="button-primary login__submit"
                                    disabled=move || !connected.get() || login_busy.get()
                                >
                                    <IconArrowRight class="icon" />
                                    {move || {
                                        if login_busy.get() {
                                            "กำลังเข้าสู่ระบบ..."
                                        } else {
                                            "เข้าสู่ระบบ"
                                        }
                                    }}
                                </button>
                            </div>
                        </form>
                    </section>
                </div>

                <p class="login__foot">
                    "ER-Fill อ่านข้อมูลจาก HOSxP แบบ read-only · ไม่แสดงข้อมูลผู้ป่วย"
                </p>
            </main>
        </div>
    }
}

/// Minimal startup state while settings, connection, and session are probed.
#[component]
pub fn StartupSplash() -> impl IntoView {
    view! {
        <div class="startup">
            <BrandMark />
            <p>"กำลังเตรียมระบบ..."</p>
        </div>
    }
}
