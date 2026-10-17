//! The configured ER drug catalog: categories and their drugs (AGENTS.md section 4).
//!
//! The catalog is the pharmacy's approved ER drug list - the requisition
//! table is built from it, so nothing appears on the printed form that is
//! not in this catalog. Mutations (add/remove/reorder) live in
//! `er-fill-core::catalog_ops`.

use serde::{Deserialize, Serialize};

/// A requisition category ("ยาเม็ด", "ยาฉีด (HAD)", ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugCategory {
    /// Stable id ("cat-1", ...) - assigned once, never reused.
    pub id: String,
    /// Display name (Thai).
    pub name: String,
    /// Section order on screen and paper; lower first.
    pub sort_order: i32,
}

/// One drug in the ER list, mapped to a HOSxP `drugitems.icode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErDrug {
    /// HOSxP `drugitems.icode` - the join key for usage.
    pub icode: String,
    /// Display name; seeded from HOSxP, editable (Thai transliteration).
    pub name: String,
    /// Strength label seeded from `drugitems` (`strength` + `units`, e.g.
    /// "500 mg"); editable, may be empty. Distinguishes same-name drugs
    /// (Paracetamol 325 vs 500) on screen and on the printed sheet.
    #[serde(default)]
    pub strength_text: String,
    /// Free-text stock label ("20 เม็ด/10"), printed as-is.
    pub stock_text: String,
    /// Owning [`DrugCategory::id`].
    pub category_id: String,
    /// Row order inside the category; lower first.
    pub sort_order: i32,
}

/// The full catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Catalog {
    /// Categories in no particular order (sort via `sort_order`).
    pub categories: Vec<DrugCategory>,
    /// Drugs in no particular order (sort via `sort_order` per category).
    pub drugs: Vec<ErDrug>,
}

impl Catalog {
    /// The six categories of the approved ER drug list
    /// (`docs/reference/er-drug-list.md`), with no drugs yet.
    ///
    /// Drugs are added in the app by searching HOSxP `drugitems`.
    pub fn seed_default() -> Self {
        const CATEGORY_NAMES: [&str; 6] =
            ["ยาเม็ด", "ยาน้ำ", "ยาฉีด", "ยาฉีด (HAD)", "สารน้ำ", "ยาใช้ภายนอก"];
        Self {
            categories: CATEGORY_NAMES
                .iter()
                .enumerate()
                .map(|(index, name)| DrugCategory {
                    id: format!("cat-{}", index + 1),
                    name: (*name).to_string(),
                    sort_order: index as i32,
                })
                .collect(),
            drugs: Vec::new(),
        }
    }

    /// True when no drug is configured (categories alone are not enough to
    /// produce a requisition).
    pub fn is_empty(&self) -> bool {
        self.drugs.is_empty()
    }

    /// Drugs of `category_id`, ordered by `sort_order`.
    pub fn drugs_in_category(&self, category_id: &str) -> Vec<&ErDrug> {
        let mut drugs: Vec<&ErDrug> = self
            .drugs
            .iter()
            .filter(|drug| drug.category_id == category_id)
            .collect();
        drugs.sort_by_key(|drug| drug.sort_order);
        drugs
    }

    /// All configured icodes (order irrelevant - used to build the usage
    /// query's `IN` list).
    pub fn icodes(&self) -> Vec<String> {
        self.drugs.iter().map(|drug| drug.icode.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_default_creates_six_ordered_categories_and_no_drugs() {
        let catalog = Catalog::seed_default();
        assert_eq!(catalog.categories.len(), 6);
        assert!(catalog.is_empty());
        let names: Vec<&str> = catalog
            .categories
            .iter()
            .map(|category| category.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["ยาเม็ด", "ยาน้ำ", "ยาฉีด", "ยาฉีด (HAD)", "สารน้ำ", "ยาใช้ภายนอก"]
        );
        let orders: Vec<i32> = catalog
            .categories
            .iter()
            .map(|category| category.sort_order)
            .collect();
        assert_eq!(orders, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn drugs_in_category_filters_and_sorts() {
        let mut catalog = Catalog::seed_default();
        catalog.drugs.push(ErDrug {
            icode: "1002".into(),
            name: "b".into(),
            strength_text: String::new(),
            stock_text: String::new(),
            category_id: "cat-1".into(),
            sort_order: 1,
        });
        catalog.drugs.push(ErDrug {
            icode: "1001".into(),
            name: "a".into(),
            strength_text: String::new(),
            stock_text: String::new(),
            category_id: "cat-1".into(),
            sort_order: 0,
        });
        catalog.drugs.push(ErDrug {
            icode: "2001".into(),
            name: "c".into(),
            strength_text: String::new(),
            stock_text: String::new(),
            category_id: "cat-2".into(),
            sort_order: 0,
        });
        let first = catalog.drugs_in_category("cat-1");
        let codes: Vec<&str> = first.iter().map(|drug| drug.icode.as_str()).collect();
        assert_eq!(codes, vec!["1001", "1002"]);
        assert_eq!(catalog.drugs_in_category("cat-2").len(), 1);
        assert!(catalog.drugs_in_category("cat-9").is_empty());
    }

    #[test]
    fn icodes_collects_every_drug() {
        let mut catalog = Catalog::seed_default();
        catalog.drugs.push(ErDrug {
            icode: "1001".into(),
            name: "a".into(),
            strength_text: String::new(),
            stock_text: String::new(),
            category_id: "cat-1".into(),
            sort_order: 0,
        });
        assert_eq!(catalog.icodes(), vec!["1001".to_string()]);
    }

    #[test]
    fn serde_defaults_missing_fields() -> Result<(), Box<dyn std::error::Error>> {
        let catalog: Catalog = serde_json::from_str("{}")?;
        assert!(catalog.categories.is_empty());
        assert!(catalog.drugs.is_empty());
        Ok(())
    }

    #[test]
    fn serde_defaults_missing_drug_strength() -> Result<(), Box<dyn std::error::Error>> {
        // Settings files written before `strengthText` existed must load.
        let catalog: Catalog = serde_json::from_str(
            r#"{"categories":[],"drugs":[{"icode":"1001","name":"a","stockText":"","categoryId":"cat-1","sortOrder":0}]}"#,
        )?;
        assert_eq!(catalog.drugs[0].strength_text, "");
        Ok(())
    }
}
