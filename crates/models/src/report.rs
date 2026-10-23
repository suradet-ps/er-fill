//! Report-side domain types: HOSxP search/usage results and the printable
//! requisition rows (AGENTS.md sections 4 and 8).

use serde::{Deserialize, Serialize};

/// A `drugitems` search hit from HOSxP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugItem {
    /// HOSxP `drugitems.icode`.
    pub icode: String,
    /// HOSxP `drugitems.name`.
    pub name: String,
    /// HOSxP `drugitems.strength` (e.g. "500"); may be empty.
    pub strength: String,
    /// HOSxP `drugitems.units` (e.g. "mg"); may be empty.
    pub units: String,
}

impl DrugItem {
    /// "500 mg" when both strength and unit are present, otherwise whichever
    /// one exists ("500" or "mg"), or an empty string.
    pub fn strength_label(&self) -> String {
        match (self.strength.trim(), self.units.trim()) {
            ("", "") => String::new(),
            (strength, "") => strength.to_string(),
            ("", units) => units.to_string(),
            (strength, units) => format!("{strength} {units}"),
        }
    }
}

/// Summed usage of one drug inside a requisition window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRow {
    /// HOSxP `opitemrece.icode`.
    pub icode: String,
    /// `SUM(qty)` over the window.
    pub qty: f64,
}

/// One printable requisition row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    /// HOSxP icode (shown on paper).
    pub icode: String,
    /// Display name.
    pub name: String,
    /// Strength label from the catalog ("500 mg"); may be empty. Shown
    /// beside the name on screen and on the printed sheet so same-name
    /// drugs stay distinguishable.
    pub strength_text: String,
    /// Stock label (จำนวน/Stock column), printed as-is.
    pub stock_text: String,
    /// Usage from HOSxP in the window, in base units (e.g. tablets).
    pub usage: f64,
    /// Whole packs to requisition, derived from the pack/stock label and
    /// editable in the UI before printing.
    pub requested: f64,
    /// Highest whole pack count allowed by the configured stock; `None` when
    /// the stock label is not in `units/stock` form.
    pub max_requested: Option<f64>,
    /// True when the operator added this row manually because it had no
    /// usage in the window. Added rows always carry a [`Self::reason`].
    #[serde(default)]
    pub added: bool,
    /// Why the operator added this row or changed its requested quantity.
    /// Empty for rows the report derived automatically.
    #[serde(default)]
    pub reason: String,
}

impl ReportRow {
    /// True when the row must appear in the printed adjustment table: it was
    /// added manually or its requested quantity was edited with a reason.
    pub fn has_adjustment(&self) -> bool {
        self.added || !self.reason.trim().is_empty()
    }
}

/// A category section of the requisition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSection {
    /// Owning category id.
    pub category_id: String,
    /// Category display name (printed as the section heading).
    pub category_name: String,
    /// Rows in catalog order - the order mirrors the physical form and the
    /// pharmacy's pick route and is never re-sorted by quantity.
    pub rows: Vec<ReportRow>,
}

/// The printed header block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReportMeta {
    /// Hospital name printed at the top of the form.
    pub hospital_name: String,
    /// Document title.
    pub document_title: String,
    /// Issuing department line under the title.
    pub department_name: String,
}

impl Default for ReportMeta {
    fn default() -> Self {
        Self {
            hospital_name: "โรงพยาบาลสระโบสถ์".to_string(),
            document_title: "ใบเบิกยาประจำห้องฉุกเฉินรายวัน".to_string(),
            department_name: "กลุ่มงานเภสัชกรรม".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_meta_default_matches_the_hospital_form() {
        let meta = ReportMeta::default();
        assert_eq!(meta.hospital_name, "โรงพยาบาลสระโบสถ์");
        assert_eq!(meta.document_title, "ใบเบิกยาประจำห้องฉุกเฉินรายวัน");
        assert_eq!(meta.department_name, "กลุ่มงานเภสัชกรรม");
    }

    #[test]
    fn report_meta_serde_fills_missing_fields_from_defaults()
    -> Result<(), Box<dyn std::error::Error>> {
        let meta: ReportMeta = serde_json::from_str(r#"{ "hospitalName": "รพ.ทดสอบ" }"#)?;
        assert_eq!(meta.hospital_name, "รพ.ทดสอบ");
        assert_eq!(meta.document_title, "ใบเบิกยาประจำห้องฉุกเฉินรายวัน");
        Ok(())
    }

    #[test]
    fn usage_row_serde_uses_camel_case() -> Result<(), Box<dyn std::error::Error>> {
        let row = UsageRow {
            icode: "1001".into(),
            qty: 2.5,
        };
        let json = serde_json::to_string(&row)?;
        assert_eq!(json, r#"{"icode":"1001","qty":2.5}"#);
        Ok(())
    }

    #[test]
    fn report_row_adjustment_detection_covers_added_and_reasoned_rows() {
        let mut row = ReportRow {
            icode: "1001".into(),
            name: "Paracetamol".into(),
            strength_text: "500 mg".into(),
            stock_text: "10/20".into(),
            usage: 2.0,
            requested: 1.0,
            max_requested: Some(20.0),
            added: false,
            reason: String::new(),
        };
        assert!(!row.has_adjustment());

        row.reason = "ผู้ป่วยใช้มากกว่าปกติ".into();
        assert!(row.has_adjustment());

        row.reason = "   ".into();
        assert!(!row.has_adjustment());

        row.added = true;
        assert!(row.has_adjustment());
    }

    #[test]
    fn drug_item_strength_label_combines_and_falls_back() {
        let full = DrugItem {
            icode: "1001".into(),
            name: "Paracetamol".into(),
            strength: "500".into(),
            units: "mg".into(),
        };
        assert_eq!(full.strength_label(), "500 mg");

        let strength_only = DrugItem {
            strength: "500".into(),
            units: "".into(),
            ..full.clone()
        };
        assert_eq!(strength_only.strength_label(), "500");

        let unit_only = DrugItem {
            strength: "".into(),
            units: "mg".into(),
            ..full.clone()
        };
        assert_eq!(unit_only.strength_label(), "mg");

        let empty = DrugItem {
            strength: "".into(),
            units: "".into(),
            ..full
        };
        assert_eq!(empty.strength_label(), "");
    }
}
