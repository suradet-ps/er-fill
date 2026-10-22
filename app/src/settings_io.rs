//! Portable settings export/import: a plain JSON file carrying the
//! schedule, catalog, and report header so one machine can be configured
//! and the rest imported from it.
//!
//! Connection credentials are deliberately **not** part of the export: every
//! field is encrypted with a key held in this machine's OS keychain
//! (AGENTS.md section 10), so it is unreadable elsewhere by design. The
//! password and connection settings stay per-machine.

use er_refill_models::AppSettings;
use serde::{Deserialize, Serialize};

/// Marker written into every export file.
pub const EXPORT_FORMAT: &str = "er-refill-settings";

/// Version of the export envelope; bump if the layout changes.
///
/// Version 2 removed the obsolete `schedule.cutoff` field (the requisition
/// window now ends at opening time). Version 1 files still import: serde
/// ignores the unknown field.
pub const EXPORT_VERSION: u32 = 2;

/// The exported file: marker + version + the settings themselves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsExport {
    /// Always [`EXPORT_FORMAT`] for files this app writes.
    pub format: String,
    /// Always [`EXPORT_VERSION`] for files this build writes.
    pub version: u32,
    /// Human-readable export time (CE), for operators, not machines.
    pub exported_at: String,
    /// The schedule + catalog + report header.
    pub settings: AppSettings,
}

/// Renders the export file content (pretty JSON).
///
/// # Errors
///
/// Returns a Thai message when serialization fails (practically never).
pub fn export_json(settings: &AppSettings) -> Result<String, String> {
    let file = SettingsExport {
        format: EXPORT_FORMAT.to_string(),
        version: EXPORT_VERSION,
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        settings: settings.clone(),
    };
    serde_json::to_string_pretty(&file).map_err(|_| "สร้างไฟล์ตั้งค่าไม่สำเร็จ".to_string())
}

/// Parses an imported file, validating the marker, version, and shape.
///
/// # Errors
///
/// Returns the Thai message to display verbatim when the file is not a
/// ER-Refill settings export, was written by a newer version, or is corrupt.
pub fn parse_import(raw: &str) -> Result<AppSettings, String> {
    let file: SettingsExport = serde_json::from_str(raw)
        .map_err(|_| "ไฟล์ไม่ถูกต้อง: ไม่ใช่ไฟล์ตั้งค่าของ ER-Refill หรือไฟล์เสียหาย".to_string())?;
    if file.format != EXPORT_FORMAT {
        return Err("ไฟล์ไม่ถูกต้อง: ไม่ใช่ไฟล์ตั้งค่าของ ER-Refill".to_string());
    }
    if file.version > EXPORT_VERSION {
        return Err("ไฟล์ถูกสร้างจากเวอร์ชันใหม่กว่า กรุณาอัปเดตโปรแกรม".to_string());
    }
    if file.settings.catalog.categories.is_empty() {
        return Err("ไฟล์ไม่ถูกต้อง: ไม่พบหมวดรายการยาในไฟล์".to_string());
    }
    Ok(file.settings)
}

/// File name offered by the browser download: `er-refill-settings-YYYYMMDD.json`.
pub fn export_file_name() -> String {
    format!(
        "er-refill-settings-{}.json",
        chrono::Local::now().format("%Y%m%d")
    )
}

/// Downloads `json` as a file (blob + object URL, no filesystem scopes,
/// AGENTS.md section 15). Errors are Thai, for the status line.
///
/// # Errors
///
/// Returns a Thai message when the browser denies any DOM step.
pub fn download_json(json: &str, file_name: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| "ไม่พบหน้าต่างโปรแกรม".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "ไม่พบเอกสารของโปรแกรม".to_string())?;

    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(json));
    let blob = web_sys::Blob::new_with_str_sequence(&parts)
        .map_err(|_| "สร้างไฟล์สำหรับดาวน์โหลดไม่สำเร็จ".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "สร้างไฟล์สำหรับดาวน์โหลดไม่สำเร็จ".to_string())?;

    let anchor = document
        .create_element("a")
        .map_err(|_| "สร้างไฟล์สำหรับดาวน์โหลดไม่สำเร็จ".to_string())?
        .unchecked_into::<web_sys::HtmlAnchorElement>();
    anchor.set_href(&url);
    anchor.set_download(file_name);
    if let Some(body) = document.body() {
        let _ = body.append_child(&anchor);
        anchor.click();
        let _ = body.remove_child(&anchor);
    }
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_round_trips_through_parse_import() -> Result<(), Box<dyn std::error::Error>> {
        let json = export_json(&AppSettings::default())?;
        assert!(json.contains("\"format\": \"er-refill-settings\""));
        let parsed = parse_import(&json)?;
        assert_eq!(parsed, AppSettings::default());
        Ok(())
    }

    #[test]
    fn parse_import_accepts_a_v1_file_with_the_removed_cutoff_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "format": "er-refill-settings",
            "version": 1,
            "exportedAt": "2026-10-17 08:00",
            "settings": {
                "schedule": {
                    "weekday": { "start": "08:30", "end": "20:30" },
                    "weekend": { "start": "08:00", "end": "16:00" },
                    "cutoff": "08:00"
                },
                "catalog": {
                    "categories": [
                        { "id": "cat-1", "name": "ยาเม็ด", "sortOrder": 0 }
                    ],
                    "drugs": []
                }
            }
        }"#;
        let settings = parse_import(json)?;
        assert_eq!(settings.schedule, AppSettings::default().schedule);
        Ok(())
    }

    #[test]
    fn parse_import_rejects_a_foreign_json_file() {
        let error = parse_import("{\"hello\":\"world\"}").expect_err("must reject");
        assert!(error.contains("ไม่ใช่ไฟล์ตั้งค่าของ ER-Refill"));
    }

    #[test]
    fn parse_import_rejects_garbage() {
        let error = parse_import("not json at all").expect_err("must reject");
        assert!(error.contains("ไฟล์ไม่ถูกต้อง"));
    }

    #[test]
    fn parse_import_rejects_a_newer_version() {
        let json = r#"{
            "format": "er-refill-settings",
            "version": 99,
            "exportedAt": "2026-10-17 08:00",
            "settings": {}
        }"#;
        let error = parse_import(json).expect_err("must reject");
        assert!(error.contains("เวอร์ชันใหม่กว่า"));
    }

    #[test]
    fn parse_import_rejects_an_empty_catalog() {
        let mut settings = AppSettings::default();
        settings.catalog.categories.clear();
        let json = export_json(&settings).expect("export succeeds");
        let error = parse_import(&json).expect_err("must reject");
        assert!(error.contains("ไม่พบหมวดรายการยา"));
    }

    #[test]
    fn export_file_name_is_a_json_file() {
        let name = export_file_name();
        assert!(name.starts_with("er-refill-settings-"));
        assert!(name.ends_with(".json"));
    }
}
