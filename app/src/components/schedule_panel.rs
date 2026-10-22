//! Settings tab: business hours (AGENTS.md section 7), with a live preview
//! of the window the current inputs produce. The window ends at the opening
//! time of the run day - there is no separate cutoff.

use chrono::NaiveTime;
use er_refill_models::{BusinessSchedule, TimeRange, format_thai_datetime_abbr, format_time_hm};
use leptos::prelude::*;

use crate::api;
use crate::components::settings_modal::{StatusLine, StatusMessage};
use crate::state::AppState;

/// Parses `"HH:MM"` (also accepts `"HH:MM:SS"`).
fn parse_hm(raw: &str) -> Option<NaiveTime> {
    let raw = raw.trim();
    NaiveTime::parse_from_str(raw, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(raw, "%H:%M:%S"))
        .ok()
}

/// The schedule form: weekday range and weekend range.
#[component]
pub fn SchedulePanel(state: AppState) -> impl IntoView {
    let initial = state
        .settings
        .get_untracked()
        .map(|settings| settings.schedule)
        .unwrap_or_default();
    let weekday_start = RwSignal::new(format_time_hm(initial.weekday.start));
    let weekday_end = RwSignal::new(format_time_hm(initial.weekday.end));
    let weekend_start = RwSignal::new(format_time_hm(initial.weekend.start));
    let weekend_end = RwSignal::new(format_time_hm(initial.weekend.end));
    let status = RwSignal::new(None::<StatusMessage>);
    let busy = RwSignal::new(false);

    let build = move || -> Result<BusinessSchedule, String> {
        let weekday = TimeRange::new(
            parse_hm(&weekday_start.get()).ok_or("เวลาเริ่มวันจันทร์-ศุกร์ไม่ถูกต้อง")?,
            parse_hm(&weekday_end.get()).ok_or("เวลาเลิกวันจันทร์-ศุกร์ไม่ถูกต้อง")?,
        )
        .map_err(|_| "วันจันทร์-ศุกร์: เวลาเริ่มต้องก่อนเวลาเลิก".to_string())?;
        let weekend = TimeRange::new(
            parse_hm(&weekend_start.get()).ok_or("เวลาเริ่มเสาร์-อาทิตย์ไม่ถูกต้อง")?,
            parse_hm(&weekend_end.get()).ok_or("เวลาเลิกเสาร์-อาทิตย์ไม่ถูกต้อง")?,
        )
        .map_err(|_| "เสาร์-อาทิตย์: เวลาเริ่มต้องก่อนเวลาเลิก".to_string())?;
        Ok(BusinessSchedule { weekday, weekend })
    };

    let preview = move || {
        build()
            .ok()
            .and_then(|schedule| {
                er_refill_core::window_for(&schedule, state.selected_date.get()).ok()
            })
            .map(|window| {
                format!(
                    "ช่วงเวลาของวันที่เลือก: {} → {}",
                    format_thai_datetime_abbr(window.start),
                    format_thai_datetime_abbr(window.end),
                )
            })
    };

    let on_save = {
        let state = state.clone();
        move |_| {
            let schedule = match build() {
                Ok(schedule) => schedule,
                Err(message) => {
                    status.set(Some(StatusMessage::error(message)));
                    return;
                }
            };
            busy.set(true);
            status.set(None);
            let state = state.clone();
            leptos::task::spawn_local(async move {
                match api::save_schedule(&schedule).await {
                    Ok(()) => {
                        state.settings.update(|settings| {
                            if let Some(settings) = settings {
                                settings.schedule = schedule;
                            }
                        });
                        state.recompute_window();
                        state.sections.set(None);
                        status.set(Some(StatusMessage::ok("บันทึกเวลาทำการแล้ว")));
                    }
                    Err(err) => status.set(Some(StatusMessage::error(err.message))),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="panel">
            <h3 class="panel__subtitle">"เวลาทำการของแต่ละวัน"</h3>
            <div class="form-card">
                <span class="form-card__title">"วันจันทร์-ศุกร์"</span>
                <div class="form-card__fields">
                    <label class="field field--inline">
                        <span class="field__label">"เริ่ม"</span>
                        <input class="text-input" type="time" prop:value=move || weekday_start.get()
                            on:input=move |ev| weekday_start.set(event_target_value(&ev)) />
                    </label>
                    <span class="form-card__to">"ถึง"</span>
                    <label class="field field--inline">
                        <span class="field__label">"เลิก"</span>
                        <input class="text-input" type="time" prop:value=move || weekday_end.get()
                            on:input=move |ev| weekday_end.set(event_target_value(&ev)) />
                    </label>
                </div>
            </div>
            <div class="form-card">
                <span class="form-card__title">"วันเสาร์-อาทิตย์"</span>
                <div class="form-card__fields">
                    <label class="field field--inline">
                        <span class="field__label">"เริ่ม"</span>
                        <input class="text-input" type="time" prop:value=move || weekend_start.get()
                            on:input=move |ev| weekend_start.set(event_target_value(&ev)) />
                    </label>
                    <span class="form-card__to">"ถึง"</span>
                    <label class="field field--inline">
                        <span class="field__label">"เลิก"</span>
                        <input class="text-input" type="time" prop:value=move || weekend_end.get()
                            on:input=move |ev| weekend_end.set(event_target_value(&ev)) />
                    </label>
                </div>
            </div>
            <p class="panel-note">
                "ช่วงเบิกจะสิ้นสุดที่เวลาเปิดทำการของวันที่เบิกโดยอัตโนมัติ"
            </p>
            <p class="panel-note panel-note--preview">
                {move || preview().unwrap_or_else(|| "เวลาทำการไม่ถูกต้อง".to_string())}
            </p>
            <StatusLine message=status />
            <div class="panel__actions">
                <button class="button-primary" disabled=move || busy.get() on:click=on_save>
                    "บันทึกเวลาทำการ"
                </button>
            </div>
        </div>
    }
}
