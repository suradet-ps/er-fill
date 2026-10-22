//! Top bar: brand mark, titles, connection status dot, settings action
//! (DESIGN.md "Components / top-bar").

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{IconLogOut, IconSettings};
use crate::state::AppState;

/// The 44px header across the top of the window.
#[component]
pub fn TopBar(state: AppState) -> impl IntoView {
    let health = state.health;
    let open_settings = {
        let state = state.clone();
        move |_| state.settings_open.set(true)
    };
    let logout = {
        let state = state.clone();
        move |_| {
            let state = state.clone();
            leptos::task::spawn_local(async move {
                let _ = api::logout().await;
                state.clear_session();
            });
        }
    };

    view! {
        <header class="top-bar">
            <div class="top-bar__left">
                <BrandMark />
                <div class="top-bar__titles">
                    <span class="top-bar__title">"ER-Refill"</span>
                    <span class="top-bar__subtitle">"ระบบใบเบิกยาห้องฉุกเฉิน"</span>
                </div>
            </div>
            <div class="top-bar__right">
                <span
                    class=move || format!("status-dot status-dot--{}", health.get().dot_class())
                    role="status"
                >
                    <span class="status-dot__dot" aria-hidden="true"></span>
                    <span class="status-dot__label">{move || health.get().label_th()}</span>
                </span>
                {move || {
                    state
                        .session
                        .get()
                        .map(|user| {
                            let initial = user
                                .display_name
                                .chars()
                                .next()
                                .or_else(|| user.login_name.chars().next())
                                .map(|c| c.to_string())
                                .unwrap_or_default();
                            view! {
                                <span class="user-chip" title=user.login_name.clone()>
                                    <span class="user-chip__avatar">{initial}</span>
                                    <span class="user-chip__text">
                                        <span class="user-chip__name">{user.display_name}</span>
                                        <span class="user-chip__role">"เจ้าหน้าที่"</span>
                                    </span>
                                </span>
                            }
                        })
                }}
                <button class="button-ghost" title="ออกจากระบบ" on:click=logout>
                    <IconLogOut class="icon" />
                    "ออกจากระบบ"
                </button>
                <button class="button-ghost" title="ตั้งค่า" on:click=open_settings>
                    <IconSettings class="icon" />
                    "ตั้งค่า"
                </button>
            </div>
        </header>
    }
}

/// The seal mark: rounded square, capsule, cross - matching `icon-master.svg`.
#[component]
pub fn BrandMark() -> impl IntoView {
    view! {
        <svg class="brand-mark" viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
            <defs>
                <linearGradient id="errefill-seal" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0" stop-color="#00C08F" />
                    <stop offset="1" stop-color="#00795B" />
                </linearGradient>
            </defs>
            <rect x="2" y="2" width="44" height="44" rx="10" fill="url(#errefill-seal)" />
            <g fill="none" stroke="#FFFFFF" stroke-width="3.2" stroke-linecap="round">
                <rect
                    x="16.5"
                    y="11"
                    width="15"
                    height="26"
                    rx="7.5"
                    transform="rotate(24 24 24)"
                />
                <line x1="24" y1="18.5" x2="24" y2="29.5" />
                <line x1="18.5" y1="24" x2="29.5" y2="24" />
            </g>
        </svg>
    }
}
