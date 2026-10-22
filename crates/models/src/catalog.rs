//! The configured ER drug catalog: categories and their drugs (AGENTS.md section 4).
//!
//! The catalog is the pharmacy's approved ER drug list - the requisition
//! table is built from it, so nothing appears on the printed form that is
//! not in this catalog. Mutations (add/remove/reorder) live in
//! `er-refill-core::catalog_ops`.

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
    /// Stock label in `units-per-pack/stock-packs` form ("10/20"), printed
    /// as-is. Leading numbers on each side are used for the requisition
    /// arithmetic; a missing slash falls back to whole usage.
    pub stock_text: String,
    /// Owning [`DrugCategory::id`].
    pub category_id: String,
    /// Row order inside the category; lower first.
    pub sort_order: i32,
}

/// The arithmetic form of [`ErDrug::stock_text`]: base units in one pack and
/// the number of packs on hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackStock {
    /// Base units in one pack (e.g. tablets per pack).
    pub units_per_pack: f64,
    /// Packs on hand - the ceiling for a whole-pack requisition.
    pub stock_packs: f64,
}

impl PackStock {
    /// Largest whole number of packs that may be requested.
    pub fn max_requested(self) -> f64 {
        self.stock_packs.floor().max(0.0)
    }
}

impl ErDrug {
    /// Parses [`ErDrug::stock_text`] when it contains `units/stock` numbers.
    pub fn pack_stock(&self) -> Option<PackStock> {
        parse_pack_stock(&self.stock_text)
    }
}

/// Reads a leading number from one side of the stock label. Commas are
/// ignored ("1,000" -> 1000); units after the number are ignored.
fn leading_number(text: &str) -> Option<f64> {
    let token: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
        .filter(|c| *c != ',')
        .collect();
    token.parse::<f64>().ok()
}

/// Parses `"10/20"`, `"10 เม็ด/20 ซอง"`, etc. Both numbers must be finite,
/// the pack size positive, and the stock non-negative.
fn parse_pack_stock(text: &str) -> Option<PackStock> {
    let (units, stock) = text.split_once('/')?;
    let units_per_pack = leading_number(units)?;
    let stock_packs = leading_number(stock)?;
    (units_per_pack.is_finite()
        && stock_packs.is_finite()
        && units_per_pack > 0.0
        && stock_packs >= 0.0)
        .then_some(PackStock {
            units_per_pack,
            stock_packs,
        })
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

    #[test]
    fn pack_stock_parses_compact_and_labeled_forms() {
        assert_eq!(
            parse_pack_stock("10/20"),
            Some(PackStock {
                units_per_pack: 10.0,
                stock_packs: 20.0,
            })
        );
        assert_eq!(
            parse_pack_stock("20 เม็ด/10 ซอง"),
            Some(PackStock {
                units_per_pack: 20.0,
                stock_packs: 10.0,
            })
        );
        assert_eq!(
            parse_pack_stock("1,000 ml / 5"),
            Some(PackStock {
                units_per_pack: 1000.0,
                stock_packs: 5.0,
            })
        );
        assert_eq!(
            parse_pack_stock("10/20/30"),
            Some(PackStock {
                units_per_pack: 10.0,
                stock_packs: 20.0,
            })
        );
    }

    #[test]
    fn pack_stock_rejects_missing_or_invalid_numbers() {
        assert_eq!(parse_pack_stock("10 เม็ด"), None);
        assert_eq!(parse_pack_stock("10/"), None);
        assert_eq!(parse_pack_stock("/20"), None);
        assert_eq!(parse_pack_stock("0/20"), None);
        assert_eq!(parse_pack_stock("-10/20"), None);
        assert_eq!(parse_pack_stock("10/-20"), None);
    }

    #[test]
    fn pack_stock_max_requested_is_a_whole_pack() {
        let pack = PackStock {
            units_per_pack: 10.0,
            stock_packs: 20.9,
        };
        assert_eq!(pack.max_requested(), 20.0);
    }

    #[test]
    fn er_drug_pack_stock_reads_its_label() {
        let drug = ErDrug {
            icode: "1001".into(),
            name: "a".into(),
            strength_text: String::new(),
            stock_text: "10/20".into(),
            category_id: "cat-1".into(),
            sort_order: 0,
        };
        assert_eq!(
            drug.pack_stock(),
            Some(PackStock {
                units_per_pack: 10.0,
                stock_packs: 20.0,
            })
        );
    }
}
