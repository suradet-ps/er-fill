//! Wasm tests for the IPC contract (AGENTS.md section 13): the frontend API layer
//! against a mock `invoke`, in a real browser DOM.
//!
//! Run explicitly (needs a browser runner):
//! `cd app && cargo test --target wasm32-unknown-unknown` with
//! `CHROMEDRIVER` set / `wasm-bindgen-test-runner`.

#![cfg(target_arch = "wasm32")]

use er_fill_app::api;
use er_fill_models::AppSettings;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Resolves a promise with a serialized command result.
fn js_ok(value: JsValue) -> js_sys::Promise {
    js_sys::Promise::resolve(&value)
}

/// Rejects a promise with a serialized [`api::ApiError`].
fn js_reject(value: JsValue) -> js_sys::Promise {
    js_sys::Promise::reject(&value)
}

fn to_js(value: &impl serde::Serialize) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::UNDEFINED)
}

/// Installs the standard mock used by most tests.
fn install_default_mock() {
    api::install_mock_invoke(|cmd, _args| match cmd {
        "connection_status" => js_ok(JsValue::from_bool(true)),
        "connection_health" => js_ok(JsValue::from_str("Connected")),
        "load_settings" => js_ok(to_js(&AppSettings::default())),
        "fetch_usage" => js_ok(to_js(&[er_fill_models::UsageRow {
            icode: "1001".into(),
            qty: 6.0,
        }])),
        other => js_reject(to_js(&api::ApiError {
            kind: api::ApiErrorKind::Query,
            message: format!("unexpected command {other}"),
        })),
    });
}

#[wasm_bindgen_test]
async fn connection_status_reads_bool_from_backend() {
    install_default_mock();
    assert!(api::connection_status().await);
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn load_settings_round_trips_the_defaults() {
    install_default_mock();
    let settings = api::load_settings().await.expect("mock resolves settings");
    assert_eq!(settings.catalog.categories.len(), 6);
    assert!(settings.catalog.drugs.is_empty());
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn fetch_usage_returns_rows() {
    install_default_mock();
    let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
        .and_then(|date| date.and_hms_opt(16, 0, 0))
        .expect("valid test datetime");
    let end = chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
        .and_then(|date| date.and_hms_opt(8, 0, 0))
        .expect("valid test datetime");
    let window = er_fill_models::RequisitionWindow::new(start, end).expect("valid window");
    let usage = api::fetch_usage(&window)
        .await
        .expect("mock resolves usage");
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].icode, "1001");
    assert_eq!(usage[0].qty, 6.0);
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn rejection_maps_to_typed_api_error() {
    api::install_mock_invoke(|_cmd, _args| {
        js_reject(to_js(&api::ApiError {
            kind: api::ApiErrorKind::Connection,
            message: "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ".to_string(),
        }))
    });
    let error = api::connection_health().await.expect_err("mock rejects");
    assert_eq!(error.kind, api::ApiErrorKind::Connection);
    assert_eq!(error.message, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ");
    api::clear_mock_invoke();
}

#[wasm_bindgen_test]
async fn non_api_rejection_becomes_generic_query_error() {
    api::install_mock_invoke(|_cmd, _args| js_sys::Promise::reject(&JsValue::from_str("boom")));
    let error = api::connection_health().await.expect_err("mock rejects");
    assert_eq!(error.kind, api::ApiErrorKind::Query);
    api::clear_mock_invoke();
}
