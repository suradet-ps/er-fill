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

/// The bear mark: a nurse bear hugging a medical heart - a compact
/// version of `icon-master.svg` for the top bar and splash.
#[component]
pub fn BrandMark() -> impl IntoView {
    view! {
        <svg class="brand-mark" viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
            <defs>
                <linearGradient id="errefill-bg" x1="0" y1="0" x2="48" y2="48" gradientUnits="userSpaceOnUse">
                    <stop offset="0" stop-color="#FBCFE8" />
                    <stop offset="1" stop-color="#F472B6" />
                </linearGradient>
                <linearGradient id="errefill-bear" x1="24" y1="6" x2="24" y2="44" gradientUnits="userSpaceOnUse">
                    <stop offset="0" stop-color="#FDE047" />
                    <stop offset="0.5" stop-color="#F59E0B" />
                    <stop offset="1" stop-color="#B45309" />
                </linearGradient>
                <clipPath id="errefill-clip">
                    <rect width="48" height="48" rx="12.75" />
                </clipPath>
            </defs>
            <rect width="48" height="48" rx="12.75" fill="url(#errefill-bg)" />
            <g clip-path="url(#errefill-clip)">
                <circle cx="11.6" cy="13.1" r="8.1" fill="url(#errefill-bear)" />
                <circle cx="36.4" cy="13.1" r="8.1" fill="url(#errefill-bear)" />
                <circle cx="11.6" cy="13.1" r="4" fill="#FEF08A" />
                <circle cx="36.4" cy="13.1" r="4" fill="#FEF08A" />
                <circle cx="24" cy="24" r="16.5" fill="url(#errefill-bear)" />
                <path d="M15.4 10.9Q24 6.75 32.6 10.9L30.6 2.6Q24 0.94 17.4 2.6Z" fill="#FFFFFF" />
                <rect x="21.4" y="3.75" width="5.25" height="7.9" rx="1.3" fill="#F43F5E" />
                <rect x="19.1" y="5.6" width="9.75" height="3.9" rx="1.3" fill="#F43F5E" />
                <ellipse cx="24" cy="27.2" rx="6.75" ry="4.9" fill="#FEF3C7" />
                <ellipse cx="24" cy="25.7" rx="3" ry="2.06" fill="#451A03" />
                <path d="M21.2 28.9Q24 31.7 26.8 28.9" stroke="#451A03" stroke-width="1.5" fill="none" stroke-linecap="round" />
                <circle cx="16.1" cy="19.9" r="1.97" fill="#451A03" />
                <circle cx="31.9" cy="19.9" r="1.97" fill="#451A03" />
                <circle cx="12.75" cy="37.7" r="6.2" fill="url(#errefill-bear)" />
                <circle cx="35.25" cy="37.7" r="6.2" fill="url(#errefill-bear)" />
                <path d="M24 44.06C13.13 35.63 14.06 29.25 18.94 29.25C21.19 29.25 24 32.63 24 32.63C24 32.63 26.8 29.25 29.06 29.25C33.94 29.25 34.88 35.63 24 44.06Z" fill="#FFFFFF" />
                <path d="M24 35.4V40M21.7 37.7H26.3" stroke="#F43F5E" stroke-width="1.8" stroke-linecap="round" />
            </g>
        </svg>
    }
}
