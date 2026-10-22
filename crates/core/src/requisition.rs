//! Joins the configured catalog with HOSxP usage into the printable
//! requisition (AGENTS.md section 8).

use std::collections::HashMap;

use er_refill_models::{Catalog, ErDrug, ReportRow, ReportSection, UsageRow};

/// Builds the requisition sections from catalog + usage.
///
/// - Sections follow category `sort_order`; categories without drugs are
///   omitted (nothing to requisition).
/// - Rows follow drug `sort_order` - the order mirrors the physical form and
///   the pharmacy's pick route, never re-sorted by quantity.
/// - `requested` starts at the whole-pack count implied by `stock_text`
///   (`units/stock`): `ceil(usage / units_per_pack)`, capped at the stock
///   pack count. Without a pack label, whole usage is requested. The operator
///   may adjust it before printing.
/// - Usage for icodes not in the catalog is ignored; configured drugs with
///   no usage show `0` (the pharmacy sees they were not used).
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

    let mut categories: Vec<&er_refill_models::DrugCategory> = catalog.categories.iter().collect();
    categories.sort_by_key(|category| category.sort_order);

    categories
        .into_iter()
        .filter_map(|category| {
            let drugs: Vec<&ErDrug> = catalog.drugs_in_category(&category.id);
            if drugs.is_empty() {
                return None;
            }
            let rows = drugs
                .into_iter()
                .map(|drug| {
                    let used = usage_by_icode
                        .get(drug.icode.as_str())
                        .copied()
                        .unwrap_or(0.0);
                    let (requested, max_requested) = requested_for(drug, used);
                    ReportRow {
                        icode: drug.icode.clone(),
                        name: drug.name.clone(),
                        strength_text: drug.strength_text.clone(),
                        stock_text: drug.stock_text.clone(),
                        usage: used,
                        requested,
                        max_requested,
                    }
                })
                .collect();
            Some(ReportSection {
                category_id: category.id.clone(),
                category_name: category.name.clone(),
                rows,
            })
        })
        .collect()
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

    #[test]
    fn build_report_orders_sections_and_rows_by_catalog_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[]);
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
    fn build_report_fills_usage_and_derives_whole_pack_requests()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(
            &catalog,
            &[
                UsageRow {
                    icode: "1001".into(),
                    qty: 6.0,
                },
                UsageRow {
                    icode: "3001".into(),
                    qty: 1.5,
                },
            ],
        );
        assert_eq!(sections[0].rows[0].usage, 6.0);
        assert_eq!(sections[0].rows[0].requested, 1.0);
        assert_eq!(sections[0].rows[0].max_requested, Some(20.0));
        assert_eq!(sections[0].rows[1].usage, 0.0);
        assert_eq!(sections[0].rows[1].requested, 0.0);
        assert_eq!(sections[0].rows[1].max_requested, None);
        assert_eq!(sections[1].rows[0].usage, 1.5);
        assert_eq!(sections[1].rows[0].requested, 2.0);
        assert_eq!(sections[1].rows[0].max_requested, None);
        Ok(())
    }

    #[test]
    fn build_report_converts_usage_to_packs_and_caps_at_stock()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(
            &catalog,
            &[UsageRow {
                icode: "1001".into(),
                qty: 45.0,
            }],
        );
        assert_eq!(sections[0].rows[0].requested, 5.0);

        let capped = build_report(
            &catalog,
            &[UsageRow {
                icode: "1001".into(),
                qty: 205.0,
            }],
        );
        assert_eq!(capped[0].rows[0].requested, 20.0);
        Ok(())
    }

    #[test]
    fn build_report_ignores_usage_for_unknown_icodes() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(
            &catalog,
            &[UsageRow {
                icode: "9999".into(),
                qty: 42.0,
            }],
        );
        let total: f64 = sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .map(|row| row.usage)
            .sum();
        assert_eq!(total, 0.0);
        Ok(())
    }

    #[test]
    fn build_report_returns_empty_when_no_drugs_configured() {
        let catalog = Catalog::seed_default();
        assert!(build_report(&catalog, &[]).is_empty());
    }

    #[test]
    fn build_report_carries_pack_text_through() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[]);
        assert_eq!(sections[0].rows[0].stock_text, "10/20");
        assert_eq!(sections[0].rows[1].stock_text, "");
        Ok(())
    }

    #[test]
    fn build_report_carries_strength_through() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = setup_catalog()?;
        let sections = build_report(&catalog, &[]);
        assert_eq!(sections[0].rows[0].strength_text, "500 mg");
        assert_eq!(sections[1].rows[0].strength_text, "1 g");
        Ok(())
    }
}
