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

/// Shared SVG paint servers for [`BrandMark`]. Rendered once at the app root
/// (not inside the top bar, which print hides) so every mark - including the
/// printed sheet - can resolve the bear gradient, emboss filter, and clip.
#[component]
pub fn BrandMarkDefs() -> impl IntoView {
    view! {
        <svg class="brand-mark-defs" width="0" height="0" aria-hidden="true">
            <defs>
                <linearGradient id="errefill-bear">
                    <stop offset="0" stop-color="#FDE047" />
                    <stop offset="0.5" stop-color="#F59E0B" />
                    <stop offset="1" stop-color="#B45309" />
                </linearGradient>
                <filter id="errefill-emboss" x="-30%" y="-30%" width="160%" height="160%">
                    <feDropShadow dx="0" dy="0.47" stdDeviation="0.56" flood-color="#000000" flood-opacity="0.3" />
                    <feDropShadow dx="0" dy="-0.28" stdDeviation="0.38" flood-color="#FFFFFF" flood-opacity="0.4" />
                </filter>
                <clipPath id="errefill-clip">
                    <rect width="48" height="48" rx="12.75" />
                </clipPath>
            </defs>
        </svg>
    }
}

/// The bear mark: a nurse bear hugging a medical heart - a compact
/// version of `icon-master.svg` for the top bar and splash.
#[component]
pub fn BrandMark() -> impl IntoView {
    view! {
        <svg class="brand-mark" viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
            <rect width="48" height="48" rx="12.75" fill="#FCE7F3" />
            <g clip-path="url(#errefill-clip)">
                <g fill="url(#errefill-bear)" filter="url(#errefill-emboss)">
                    <circle cx="11.625" cy="13.125" r="8.0625" />
                    <circle cx="36.375" cy="13.125" r="8.0625" />
                </g>
                <circle cx="11.625" cy="13.125" r="4.03125" fill="#FEF08A" />
                <circle cx="36.375" cy="13.125" r="4.03125" fill="#FEF08A" />
                <circle cx="24" cy="24" r="16.5" fill="url(#errefill-bear)" filter="url(#errefill-emboss)" />
                <g filter="url(#errefill-emboss)">
                    <path d="M15.375 10.875Q24 6.75 32.625 10.875L30.5625 2.625Q24 0.9375 17.4375 2.625Z" fill="#FFFFFF" />
                    <rect x="21.375" y="3.75" width="5.25" height="7.875" rx="1.3125" fill="#F43F5E" />
                    <rect x="19.125" y="5.625" width="9.75" height="3.9375" rx="1.3125" fill="#F43F5E" />
                </g>
                <ellipse cx="24" cy="27.1875" rx="6.75" ry="4.875" fill="#FEF3C7" />
                <ellipse cx="24" cy="25.6875" rx="3" ry="2.0625" fill="#451A03" />
                <path d="M21.1875 28.875Q24 31.6875 26.8125 28.875" stroke="#451A03" stroke-width="1.5" fill="none" stroke-linecap="round" />
                <circle cx="16.125" cy="19.875" r="1.96875" fill="#451A03" />
                <circle cx="31.875" cy="19.875" r="1.96875" fill="#451A03" />
                <circle cx="12.75" cy="37.6875" r="6.1875" fill="url(#errefill-bear)" filter="url(#errefill-emboss)" />
                <circle cx="35.25" cy="37.6875" r="6.1875" fill="url(#errefill-bear)" filter="url(#errefill-emboss)" />
                <g filter="url(#errefill-emboss)">
                    <path d="M24 44.0625C13.125 35.625 14.0625 29.25 18.9375 29.25C21.1875 29.25 24 32.625 24 32.625C24 32.625 26.8125 29.25 29.0625 29.25C33.9375 29.25 34.875 35.625 24 44.0625Z" fill="#FFFFFF" />
                </g>
                <path d="M24 34.3125V39.9375M20.8125 37.125H27.1875" stroke="#F43F5E" stroke-width="2.0625" stroke-linecap="round" />
            </g>
        </svg>
    }
}
