//! Joins the configured catalog with HOSxP usage into the printable
//! requisition (AGENTS.md section 8), and applies the operator's manual
//! additions / quantity changes with their required reasons.

use std::collections::HashMap;

use er_refill_models::{Catalog, DrugCategory, ErDrug, ReportRow, ReportSection, UsageRow};

use crate::RequisitionError;

/// Builds the requisition sections from catalog + usage.
///
/// - Only drugs with `usage > 0` appear; a quiet morning should not print the
///   whole catalog. Operators add configured drugs that had no usage through
///   [`add_configured_drug`], with a reason.
/// - Sections follow category `sort_order`; categories without printed rows
///   are omitted.
/// - Rows follow drug `sort_order` - the order mirrors the physical form and
///   the pharmacy's pick route, never re-sorted by quantity.
/// - `requested` starts at the whole-pack count implied by `stock_text`
///   (`units/stock`): `ceil(usage / units_per_pack)`, capped at the stock
///   pack count. Without a pack label, whole usage is requested. The operator
///   may adjust it (with a reason) before printing.
/// - Usage for icodes not in the catalog is ignored.
///
/// # Examples
///
/// ```
/// use er_refill_core::build_report;
/// use er_refill_models::{Catalog, DrugItem, UsageRow};
///
/// let mut catalog = Catalog::seed_default();
/// er_refill_core::add_drug(
///     &mut catalog,
///     "cat-1",
///     &DrugItem {
///         icode: "1001".into(),
///         name: "Paracetamol".into(),
///         strength: "500".into(),
///         units: "mg".into(),
///     },
///     "10/20",
/// )?;
/// let sections = build_report(
///     &catalog,
///     &[UsageRow { icode: "1001".into(), qty: 40.0 }],
/// );
/// assert_eq!(sections.len(), 1);
/// assert_eq!(sections[0].rows[0].usage, 40.0);
/// assert_eq!(sections[0].rows[0].requested, 4.0);
/// assert_eq!(sections[0].rows[0].max_requested, Some(20.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn build_report(catalog: &Catalog, usage: &[UsageRow]) -> Vec<ReportSection> {
    let usage_by_icode: HashMap<&str, f64> = usage
        .iter()
        .map(|row| (row.icode.as_str(), row.qty))
        .collect();

    let mut categories: Vec<&DrugCategory> = catalog.categories.iter().collect();
    categories.sort_by_key(|category| category.sort_order);

    categories
        .into_iter()
        .filter_map(|category| {
            let drugs: Vec<&ErDrug> = catalog.drugs_in_category(&category.id);
            if drugs.is_empty() {
                return None;
            }
            let rows: Vec<ReportRow> = drugs
                .into_iter()
                .filter_map(|drug| {
                    let used = usage_by_icode
                        .get(drug.icode.as_str())
                        .copied()
                        .unwrap_or(0.0);
                    if used <= 0.0 {
                        return None;
                    }
                    let (requested, max_requested) = requested_for(drug, used);
                    Some(ReportRow {
                        icode: drug.icode.clone(),
                        name: drug.name.clone(),
                        strength_text: drug.strength_text.clone(),
                        stock_text: drug.stock_text.clone(),
                        usage: used,
                        requested,
                        max_requested,
                        added: false,
                        reason: String::new(),
                    })
                })
                .collect();
            if rows.is_empty() {
                return None;
            }
            Some(ReportSection {
                category_id: category.id.clone(),
                category_name: category.name.clone(),
                rows,
            })
        })
        .collect()
}

/// Adds a configured drug with no usage to the sheet (usage 0) and records
/// the operator's reason. The row is inserted in catalog order, and its
/// category section is created if this is the first row from that category.
///
/// # Errors
///
/// Returns [`RequisitionError::DrugNotConfigured`] when `icode` is not in the
/// catalog, [`RequisitionError::DrugAlreadyInReport`] when it is already on
/// the sheet, [`RequisitionError::MissingReason`] when `reason` is blank, and
/// [`RequisitionError::InvalidRequested`] for a negative/non-finite quantity.
pub fn add_configured_drug(
    catalog: &Catalog,
    sections: &mut Vec<ReportSection>,
    icode: &str,
    requested: f64,
    reason: &str,
) -> Result<(), RequisitionError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(RequisitionError::MissingReason);
    }
    if sections
        .iter()
        .any(|section| section.rows.iter().any(|row| row.icode == icode))
    {
        return Err(RequisitionError::DrugAlreadyInReport(icode.to_string()));
    }
    let Some(drug) = catalog.drugs.iter().find(|drug| drug.icode == icode) else {
        return Err(RequisitionError::DrugNotConfigured(icode.to_string()));
    };
    let Some(category) = catalog
        .categories
        .iter()
        .find(|category| category.id == drug.category_id)
    else {
        return Err(RequisitionError::CategoryNotFound(drug.category_id.clone()));
    };

    let requested = normalized_requested(requested)?;
    let (_, max_requested) = requested_for(drug, 0.0);
    let requested = max_requested.map_or(requested, |max| requested.min(max));

    let row = ReportRow {
        icode: drug.icode.clone(),
        name: drug.name.clone(),
        strength_text: drug.strength_text.clone(),
        stock_text: drug.stock_text.clone(),
        usage: 0.0,
        requested,
        max_requested,
        added: true,
        reason: reason.to_string(),
    };
    insert_in_catalog_order(catalog, sections, category, row);
    Ok(())
}

/// Changes a row's requested quantity and records the required reason. The
/// value is rounded to a whole pack (or unit) and capped at the configured
/// stock ceiling.
///
/// # Errors
///
/// Returns [`RequisitionError::MissingReason`] when `reason` is blank,
/// [`RequisitionError::RowNotFound`] when `icode` is not on the sheet, and
/// [`RequisitionError::InvalidRequested`] for a negative/non-finite quantity.
pub fn adjust_requested(
    sections: &mut [ReportSection],
    icode: &str,
    requested: f64,
    reason: &str,
) -> Result<(), RequisitionError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(RequisitionError::MissingReason);
    }
    let Some(row) = sections
        .iter_mut()
        .flat_map(|section| section.rows.iter_mut())
        .find(|row| row.icode == icode)
    else {
        return Err(RequisitionError::RowNotFound(icode.to_string()));
    };
    let requested = normalized_requested(requested)?;
    row.requested = row
        .max_requested
        .map_or(requested, |max| requested.min(max));
    row.reason = reason.to_string();
    Ok(())
}

/// Converts base-unit usage into whole packs and reports the stock ceiling.
/// `stock_text` must be `units-per-pack/stock-packs`; otherwise fall back to
/// whole usage with no stock cap.
fn requested_for(drug: &ErDrug, used: f64) -> (f64, Option<f64>) {
    let Some(pack) = drug.pack_stock() else {
        let requested = if used > 0.0 { used.ceil() } else { 0.0 };
        return (requested, None);
    };
    let max_requested = pack.max_requested();
    if used <= 0.0 {
        return (0.0, Some(max_requested));
    }
    let packs = (used / pack.units_per_pack).ceil();
    (packs.min(max_requested).max(0.0), Some(max_requested))
}

/// Rounds a requested quantity to a whole number, rejecting values that are
/// negative or not finite.
fn normalized_requested(requested: f64) -> Result<f64, RequisitionError> {
    if !requested.is_finite() || requested < 0.0 {
        return Err(RequisitionError::InvalidRequested);
    }
    Ok(requested.round())
}

/// Inserts `row` into its category's section, creating the section when the
/// category had no usage, while keeping catalog order on both levels.
fn insert_in_catalog_order(
    catalog: &Catalog,
    sections: &mut Vec<ReportSection>,
    category: &DrugCategory,
    row: ReportRow,
) {
    let ordered = catalog.drugs_in_category(&category.id);
    let target_position = ordered
        .iter()
        .position(|drug| drug.icode == row.icode)
        .unwrap_or(usize::MAX);

    if let Some(section) = sections
        .iter_mut()
        .find(|section| section.category_id == category.id)
    {
        let insert_at = section
            .rows
            .iter()
            .position(|existing| {
                ordered
                    .iter()
                    .position(|drug| drug.icode == existing.icode)
                    .unwrap_or(usize::MAX)
                    > target_position
            })
            .unwrap_or(section.rows.len());
        section.rows.insert(insert_at, row);
        return;
    }

    sections.push(ReportSection {
        category_id: category.id.clone(),
        category_name: category.name.clone(),
        rows: vec![row],
    });
    let category_order: HashMap<&str, i32> = catalog
        .categories
        .iter()
        .map(|category| (category.id.as_str(), category.sort_order))
        .collect();
    sections.sort_by_key(|section| {
        category_order
            .get(section.category_id.as_str())
            .copied()
            .unwrap_or(i32::MAX)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use er_refill_models::DrugItem;

    fn setup_catalog() -> Result<Catalog, Box<dyn std::error::Error>> {
        let mut catalog = Catalog::seed_default();
        crate::catalog_ops::add_drug(
            &mut catalog,
            "cat-1",
            &DrugItem {
                icode: "1001".into(),
                name: "Paracetamol 500".into(),
                strength: "500".into(),
                units: "mg".into(),
            },
            "10/20",
        )?;
        crate::catalog_ops::add_drug(
            &mut catalog,
            "cat-1",
            &DrugItem {
                icode: "1002".into(),
                name: "CPM 4".into(),
                strength: "4".into(),
                units: "mg".into(),
            },
            "",
        )?;
        crate::catalog_ops::add_drug(
            &mut catalog,
            "cat-3",
            &DrugItem {
                icode: "3001".into(),
                name: "Ceftriaxone".into(),
                strength: "1".into(),
                units: "g".into(),
            },
            "1 g",
        )?;
        Ok(catalog)
    }

    fn usage(icode: &str, qty: f64) -> UsageRow {
        UsageRow {
            icode: icode.into(),
            qty,
        }
    }

    #[test]
    fn build_report_orders_sections_and_rows_by_catalog_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(
            &catalog,
            &[usage("1001", 5.0), usage("1002", 2.0), usage("3001", 1.0)],
        );
        let names: Vec<&str> = sections
            .iter()
            .map(|section| section.category_name.as_str())
            .collect();
        assert_eq!(names, vec!["ยาเม็ด", "ยาฉีด"]);
        assert_eq!(sections[0].rows[0].icode, "1001");
        assert_eq!(sections[0].rows[1].icode, "1002");
        assert_eq!(sections[1].rows[0].icode, "3001");
        Ok(())
    }

    #[test]
    fn build_report_shows_only_drugs_with_usage() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[usage("1001", 6.0)]);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].rows.len(), 1);
        assert_eq!(sections[0].rows[0].icode, "1001");
        assert!(!sections[0].rows[0].added);
        assert_eq!(sections[0].rows[0].reason, "");
        Ok(())
    }

    #[test]
    fn build_report_omits_sections_without_usage() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[usage("3001", 1.5)]);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].category_name, "ยาฉีด");
        assert_eq!(sections[0].rows.len(), 1);
        Ok(())
    }

    #[test]
    fn build_report_fills_usage_and_derives_whole_pack_requests()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[usage("1001", 6.0), usage("3001", 1.5)]);
        assert_eq!(sections[0].rows[0].usage, 6.0);
        assert_eq!(sections[0].rows[0].requested, 1.0);
        assert_eq!(sections[0].rows[0].max_requested, Some(20.0));
        assert_eq!(sections[1].rows[0].usage, 1.5);
        assert_eq!(sections[1].rows[0].requested, 2.0);
        assert_eq!(sections[1].rows[0].max_requested, None);
        Ok(())
    }

    #[test]
    fn build_report_converts_usage_to_packs_and_caps_at_stock()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[usage("1001", 45.0)]);
        assert_eq!(sections[0].rows[0].requested, 5.0);

        let capped = build_report(&catalog, &[usage("1001", 205.0)]);
        assert_eq!(capped[0].rows[0].requested, 20.0);
        Ok(())
    }

    #[test]
    fn build_report_ignores_usage_for_unknown_icodes() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[usage("9999", 42.0)]);
        assert!(sections.is_empty());
        Ok(())
    }

    #[test]
    fn build_report_returns_empty_when_no_drugs_configured() {
        let catalog = Catalog::seed_default();
        assert!(build_report(&catalog, &[]).is_empty());
    }

    #[test]
    fn build_report_carries_pack_and_strength_through() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(
            &catalog,
            &[usage("1001", 5.0), usage("1002", 1.0), usage("3001", 1.0)],
        );
        assert_eq!(sections[0].rows[0].stock_text, "10/20");
        assert_eq!(sections[0].rows[0].strength_text, "500 mg");
        assert_eq!(sections[0].rows[1].stock_text, "");
        assert_eq!(sections[1].rows[0].strength_text, "1 g");
        Ok(())
    }

    #[test]
    fn add_configured_drug_inserts_zero_usage_row_in_catalog_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let mut sections = build_report(&catalog, &[usage("1001", 5.0)]);
        add_configured_drug(&catalog, &mut sections, "1002", 2.0, "สำรองไว้ห้องฉุกเฉิน")
            .expect("configured drug added");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].rows.len(), 2);
        assert_eq!(sections[0].rows[0].icode, "1001");
        assert_eq!(sections[0].rows[1].icode, "1002");
        let added = &sections[0].rows[1];
        assert_eq!(added.usage, 0.0);
        assert_eq!(added.requested, 2.0);
        assert!(added.added);
        assert_eq!(added.reason, "สำรองไว้ห้องฉุกเฉิน");
        Ok(())
    }

    #[test]
    fn add_configured_drug_creates_a_missing_category_section()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let mut sections = build_report(&catalog, &[usage("1001", 5.0)]);
        add_configured_drug(&catalog, &mut sections, "3001", 1.0, "ผู้ป่วยส่งต่อ")
            .expect("configured drug added");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].category_name, "ยาฉีด");
        assert_eq!(sections[1].rows[0].icode, "3001");
        assert_eq!(sections[1].rows[0].requested, 1.0);
        Ok(())
    }

    #[test]
    fn add_configured_drug_rejects_unknown_duplicate_and_blank_reason()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let mut sections = build_report(&catalog, &[usage("1001", 5.0)]);

        assert_eq!(
            add_configured_drug(&catalog, &mut sections, "9999", 1.0, "เหตุผล"),
            Err(RequisitionError::DrugNotConfigured("9999".into()))
        );
        assert_eq!(
            add_configured_drug(&catalog, &mut sections, "1001", 1.0, "เหตุผล"),
            Err(RequisitionError::DrugAlreadyInReport("1001".into()))
        );
        assert_eq!(
            add_configured_drug(&catalog, &mut sections, "1002", 1.0, "   "),
            Err(RequisitionError::MissingReason)
        );
        Ok(())
    }

    #[test]
    fn add_configured_drug_caps_at_stock() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let mut sections = build_report(&catalog, &[usage("1001", 5.0)]);
        add_configured_drug(&catalog, &mut sections, "1002", 99.0, "สำรอง").expect("added");
        let added = sections[0]
            .rows
            .iter()
            .find(|row| row.icode == "1002")
            .expect("row");
        assert_eq!(added.requested, 99.0);
        assert_eq!(added.max_requested, None);

        let mut sections = build_report(&catalog, &[]);
        add_configured_drug(&catalog, &mut sections, "1001", 99.0, "สำรอง").expect("added");
        assert_eq!(sections[0].rows[0].requested, 20.0);
        Ok(())
    }

    #[test]
    fn adjust_requested_records_reason_and_caps_at_stock() -> Result<(), Box<dyn std::error::Error>>
    {
        let catalog = setup_catalog()?;
        let mut sections = build_report(&catalog, &[usage("1001", 6.0)]);

        adjust_requested(&mut sections, "1001", 3.0, "ผู้ป่วยใช้จริงมากกว่าที่บันทึก").expect("adjusted");
        assert_eq!(sections[0].rows[0].requested, 3.0);
        assert_eq!(sections[0].rows[0].reason, "ผู้ป่วยใช้จริงมากกว่าที่บันทึก");
        assert!(sections[0].rows[0].has_adjustment());

        adjust_requested(&mut sections, "1001", 999.0, "ปรับตามสต็อก").expect("adjusted");
        assert_eq!(sections[0].rows[0].requested, 20.0);

        assert_eq!(
            adjust_requested(&mut sections, "9999", 1.0, "เหตุผล"),
            Err(RequisitionError::RowNotFound("9999".into()))
        );
        assert_eq!(
            adjust_requested(&mut sections, "1001", 1.0, "   "),
            Err(RequisitionError::MissingReason)
        );
        Ok(())
    }
}
