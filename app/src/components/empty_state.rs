//! Canvas empty states: the first-run hero (the only atmospheric surface,
//! DESIGN.md "hero-band-sky") and the quieter informational cards.

use leptos::prelude::*;

use crate::components::icons::IconClipboardList;
use crate::state::{AppState, SettingsTab};

/// First run: no HOSxP connection configured yet - the one place the sky
/// gradient and the mint accent CTA live.
#[component]
pub fn FirstRunHero(state: AppState) -> impl IntoView {
    let open_connection_settings = {
        let state = state.clone();
        move |_| {
            state.settings_tab.set(SettingsTab::Connection);
            state.settings_open.set(true);
        }
    };
    view! {
        <div class="empty-hero">
            <div class="empty-hero__content">
                <h1 class="empty-hero__headline">"พิมพ์ใบเบิกยาห้องฉุกเฉินได้ในคลิกเดียว"</h1>
                <p class="empty-hero__subtitle">
                    "ระบบดึงยอดใช้ยานอกเวลาราชการจาก HOSxP ตั้งแต่เวลาเลิกทำการของวันก่อนหน้า ถึงเวลาเบิกตอนเช้าของวันนี้ แล้วพิมพ์ใบเบิกให้ห้องยาเตรียมยาให้ห้องฉุกเฉิน"
                </p>
                <button class="button-accent-mint" on:click=open_connection_settings>
                    "เริ่มตั้งค่าการเชื่อมต่อ"
                </button>
            </div>
            <div class="empty-hero__mock" aria-hidden="true">
                <div class="mock-table">
                    <div class="mock-table__head">
                        <span>"ลำดับ"</span>
                        <span>"ชื่อยา"</span>
                        <span>"จำนวนใช้"</span>
                        <span>"จำนวนเบิก"</span>
                    </div>
                    <div class="mock-table__band">"ยาเม็ด"</div>
                    <div class="mock-table__row">
                        <span>"1"</span>
                        <span>"Paracetamol 500 mg"</span>
                        <span>"6"</span>
                        <span>"6"</span>
                    </div>
                    <div class="mock-table__row">
                        <span>"2"</span>
                        <span>"CPM 4 mg"</span>
                        <span>"0"</span>
                        <span>"0"</span>
                    </div>
                    <div class="mock-table__band">"ยาฉีด"</div>
                    <div class="mock-table__row">
                        <span>"1"</span>
                        <span>"Ceftriaxone 1 g"</span>
                        <span>"2"</span>
                        <span>"2"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

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
                "เพิ่มรายการยาประจำห้องฉุกเฉินแยกตามหมวดจากบัญชียา HOSxP เพื่อให้ระบบดึงยอดใช้ยาและสร้างใบเบิกได้ (รายการอ้างอิง 69 รายการจากบัญชีที่อนุมัติอยู่ใน docs/reference)"
            </p>
            <button class="button-accent-mint" on:click=open_catalog>
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
