//! Canvas empty states: the quieter informational cards shown after sign-in.

use leptos::prelude::*;

use crate::components::icons::IconClipboardList;
use crate::state::{AppState, SettingsTab};

/// Catalog exists but has no drugs configured yet.
#[component]
pub fn CatalogEmpty(state: AppState) -> impl IntoView {
    let open_catalog = {
        let state = state.clone();
        move |_| {
            state.settings_tab.set(SettingsTab::Catalog);
            state.settings_open.set(true);
        }
    };
    view! {
        <div class="empty-card">
            <span class="empty-card__icon">
                <IconClipboardList class="icon icon--large" />
            </span>
            <h2 class="empty-card__title">"ยังไม่มีรายการยาในบัญชี"</h2>
            <p class="empty-card__text">
                "เพิ่มรายการยาประจำห้องฉุกเฉินแยกตามหมวดจากบัญชียา HOSxP เพื่อให้ระบบดึงยอดใช้ยาและสร้างใบเบิกได้"
            </p>
            <button class="button-accent" on:click=open_catalog>
                "ไปที่ตั้งค่ารายการยา"
            </button>
        </div>
    }
}

/// Configured, catalog ready, but nothing fetched for this window yet
/// (or the first fetch is still in flight - opening the app fetches
/// automatically, so this state is brief).
#[component]
pub fn NoDataHint(state: AppState) -> impl IntoView {
    let loading = state.loading;
    view! {
        <div class="empty-card empty-card--quiet">
            <span class="empty-card__icon">
                <IconClipboardList class="icon icon--large" />
            </span>
            <h2 class="empty-card__title">
                {move || {
                    if loading.get() { "กำลังดึงข้อมูล..." } else { "พร้อมสร้างใบเบิก" }
                }}
            </h2>
            <p class="empty-card__text">
                {move || {
                    if loading.get() {
                        "กำลังอ่านยอดใช้ยานอกเวลาราชการจาก HOSxP"
                    } else {
                        "กด \"ดึงข้อมูลการใช้ยา\" ที่แถบด้านบนเพื่อดึงยอดใช้ยาจาก HOSxP"
                    }
                }}
            </p>
        </div>
    }
}
