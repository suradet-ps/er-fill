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
                            let title = format!("{} · {}", user.display_name, user.login_name);
                            view! {
                                <span class="user-chip" title=title>
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

/// The seal mark: requisition sheet with a medical stamp - matching
/// `icon-master.svg`.
#[component]
pub fn BrandMark() -> impl IntoView {
    view! {
        <svg class="brand-mark" viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
            <defs>
                <linearGradient id="errefill-seal" x1="2" y1="2" x2="46" y2="46">
                    <stop offset="0" stop-color="#00C08F" />
                    <stop offset="1" stop-color="#00795B" />
                </linearGradient>
                <linearGradient id="errefill-pill" x1="24" y1="29" x2="40" y2="40">
                    <stop offset="0" stop-color="#00B889" />
                    <stop offset="1" stop-color="#00795B" />
                </linearGradient>
            </defs>
            <rect x="2" y="2" width="44" height="44" rx="10" fill="url(#errefill-seal)" />
            <rect x="13" y="7.5" width="22" height="33" rx="4.5" fill="#FFFFFF" />
            <path
                d="M13 12a4.5 4.5 0 0 1 4.5-4.5h13a4.5 4.5 0 0 1 4.5 4.5v5.5H13Z"
                fill="#00795B"
            />
            <path d="M17.5 9.2v4.6M15.2 11.5h4.6" stroke="#FFFFFF" stroke-width="1.4" stroke-linecap="round" />
            <rect x="21.5" y="10.2" width="9" height="1.6" rx="0.8" fill="#FFFFFF" fill-opacity="0.85" />
            <g fill="#00795B" fill-opacity="0.2">
                <rect x="16.5" y="22" width="12" height="1.7" rx="0.85" />
                <rect x="16.5" y="26" width="9" height="1.7" rx="0.85" />
                <rect x="16.5" y="30" width="11" height="1.7" rx="0.85" />
            </g>
            <g transform="rotate(-24 31.5 34.5)">
                <rect x="23" y="29.5" width="17" height="10.5" rx="5.25" fill="url(#errefill-pill)" stroke="#FFFFFF" stroke-width="1" />
                <path d="M31.5 32v5M29 34.5h5" stroke="#FFFFFF" stroke-width="1.5" stroke-linecap="round" />
            </g>
        </svg>
    }
}
