//! Catalog mutations used by the settings UI (AGENTS.md section 4).
//!
//! Every operation works on an owned [`Catalog`] so the Tauri layer can
//! validate the whole result before persisting it.

use er_fill_models::{Catalog, DrugCategory, DrugItem, ErDrug};

use crate::error::CatalogOpsError;

/// Adds a category with the next free `cat-<n>` id, appended last.
///
/// Returns the new category's id.
///
/// # Errors
///
/// Returns [`CatalogOpsError::EmptyName`] when `name` is blank after
/// trimming.
pub fn add_category(catalog: &mut Catalog, name: &str) -> Result<String, CatalogOpsError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CatalogOpsError::EmptyName);
    }
    let id = next_category_id(catalog);
    let sort_order = catalog
        .categories
        .iter()
        .map(|category| category.sort_order)
        .max()
        .map_or(0, |max| max + 1);
    catalog.categories.push(DrugCategory {
        id: id.clone(),
        name: name.to_string(),
        sort_order,
    });
    Ok(id)
}

/// Renames a category.
///
/// # Errors
///
/// [`CatalogOpsError::EmptyName`] for a blank name,
/// [`CatalogOpsError::CategoryNotFound`] for an unknown id.
pub fn rename_category(
    catalog: &mut Catalog,
    category_id: &str,
    name: &str,
) -> Result<(), CatalogOpsError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CatalogOpsError::EmptyName);
    }
    let category = catalog
        .categories
        .iter_mut()
        .find(|category| category.id == category_id)
        .ok_or_else(|| CatalogOpsError::CategoryNotFound(category_id.to_string()))?;
    category.name = name.to_string();
    Ok(())
}

/// Removes an empty category.
///
/// # Errors
///
/// [`CatalogOpsError::CategoryNotFound`] for an unknown id;
/// [`CatalogOpsError::CategoryNotEmpty`] when drugs still reference it -
/// move or remove them first (deliberately explicit, not a cascade).
pub fn remove_category(catalog: &mut Catalog, category_id: &str) -> Result<(), CatalogOpsError> {
    if !catalog
        .categories
        .iter()
        .any(|category| category.id == category_id)
    {
        return Err(CatalogOpsError::CategoryNotFound(category_id.to_string()));
    }
    if catalog
        .drugs
        .iter()
        .any(|drug| drug.category_id == category_id)
    {
        return Err(CatalogOpsError::CategoryNotEmpty(category_id.to_string()));
    }
    catalog
        .categories
        .retain(|category| category.id != category_id);
    Ok(())
}

/// Adds a `drugitems` search hit to a category, appended last in it.
///
/// # Errors
///
/// [`CatalogOpsError::EmptyIcode`] for a blank icode,
/// [`CatalogOpsError::CategoryNotFound`] for an unknown category,
/// [`CatalogOpsError::DuplicateDrug`] when the icode is already configured
/// (a drug belongs to exactly one category).
pub fn add_drug(
    catalog: &mut Catalog,
    category_id: &str,
    item: &DrugItem,
    stock_text: &str,
) -> Result<(), CatalogOpsError> {
    let icode = item.icode.trim();
    if icode.is_empty() {
        return Err(CatalogOpsError::EmptyIcode);
    }
    if !catalog
        .categories
        .iter()
        .any(|category| category.id == category_id)
    {
        return Err(CatalogOpsError::CategoryNotFound(category_id.to_string()));
    }
    if catalog.drugs.iter().any(|drug| drug.icode == icode) {
        return Err(CatalogOpsError::DuplicateDrug(icode.to_string()));
    }
    let sort_order = catalog
        .drugs
        .iter()
        .filter(|drug| drug.category_id == category_id)
        .map(|drug| drug.sort_order)
        .max()
        .map_or(0, |max| max + 1);
    catalog.drugs.push(ErDrug {
        icode: icode.to_string(),
        name: item.name.trim().to_string(),
        strength_text: item.strength_label(),
        stock_text: stock_text.trim().to_string(),
        category_id: category_id.to_string(),
        sort_order,
    });
    Ok(())
}

/// Updates a drug's display name, strength label, and stock label.
///
/// # Errors
///
/// [`CatalogOpsError::EmptyName`] for a blank name,
/// [`CatalogOpsError::DrugNotFound`] for an unknown icode.
pub fn update_drug(
    catalog: &mut Catalog,
    icode: &str,
    name: &str,
    strength_text: &str,
    stock_text: &str,
) -> Result<(), CatalogOpsError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CatalogOpsError::EmptyName);
    }
    let drug = catalog
        .drugs
        .iter_mut()
        .find(|drug| drug.icode == icode)
        .ok_or_else(|| CatalogOpsError::DrugNotFound(icode.to_string()))?;
    drug.name = name.to_string();
    drug.strength_text = strength_text.trim().to_string();
    drug.stock_text = stock_text.trim().to_string();
    Ok(())
}

/// Removes a drug from the catalog.
///
/// # Errors
///
/// [`CatalogOpsError::DrugNotFound`] for an unknown icode.
pub fn remove_drug(catalog: &mut Catalog, icode: &str) -> Result<(), CatalogOpsError> {
    let before = catalog.drugs.len();
    catalog.drugs.retain(|drug| drug.icode != icode);
    if catalog.drugs.len() == before {
        return Err(CatalogOpsError::DrugNotFound(icode.to_string()));
    }
    Ok(())
}

/// Moves a drug one position up (`delta = -1`) or down (`delta = +1`)
/// within its category by swapping `sort_order` with the neighbour.
/// Moving past either end is a no-op.
///
/// # Errors
///
/// [`CatalogOpsError::DrugNotFound`] for an unknown icode.
pub fn move_drug(catalog: &mut Catalog, icode: &str, delta: i32) -> Result<(), CatalogOpsError> {
    let category_id = catalog
        .drugs
        .iter()
        .find(|drug| drug.icode == icode)
        .map(|drug| drug.category_id.clone())
        .ok_or_else(|| CatalogOpsError::DrugNotFound(icode.to_string()))?;

    let mut positions: Vec<usize> = (0..catalog.drugs.len())
        .filter(|&index| catalog.drugs[index].category_id == category_id)
        .collect();
    positions.sort_by_key(|&index| catalog.drugs[index].sort_order);

    let position = positions
        .iter()
        .position(|&index| catalog.drugs[index].icode == icode)
        .ok_or_else(|| CatalogOpsError::DrugNotFound(icode.to_string()))?;
    let target = position as i64 + i64::from(delta);
    if target < 0 || target >= positions.len() as i64 {
        return Ok(());
    }
    let (current, neighbour) = (positions[position], positions[target as usize]);
    let current_order = catalog.drugs[current].sort_order;
    catalog.drugs[current].sort_order = catalog.drugs[neighbour].sort_order;
    catalog.drugs[neighbour].sort_order = current_order;
    Ok(())
}

/// Next free `cat-<n>` id: one past the highest well-formed numeric suffix.
fn next_category_id(catalog: &Catalog) -> String {
    let max = catalog
        .categories
        .iter()
        .filter_map(|category| {
            category
                .id
                .strip_prefix("cat-")
                .and_then(|suffix| suffix.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("cat-{}", max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(icode: &str, name: &str) -> DrugItem {
        DrugItem {
            icode: icode.to_string(),
            name: name.to_string(),
            strength: String::new(),
            units: String::new(),
        }
    }

    #[test]
    fn add_category_assigns_sequential_ids_and_order() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        let id = add_category(&mut catalog, "  ยาใหม่  ")?;
        assert_eq!(id, "cat-7");
        let added = catalog
            .categories
            .iter()
            .find(|category| category.id == "cat-7");
        assert!(added.is_some_and(|category| category.name == "ยาใหม่" && category.sort_order == 6));
        Ok(())
    }

    #[test]
    fn add_category_rejects_blank_name() {
        let mut catalog = Catalog::seed_default();
        assert_eq!(
            add_category(&mut catalog, "   "),
            Err(CatalogOpsError::EmptyName)
        );
        assert_eq!(catalog.categories.len(), 6);
    }

    #[test]
    fn add_category_survives_nonstandard_ids() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        catalog.categories.push(DrugCategory {
            id: "custom".into(),
            name: "x".into(),
            sort_order: 99,
        });
        assert_eq!(add_category(&mut catalog, "y")?, "cat-7");
        Ok(())
    }

    #[test]
    fn rename_category_updates_name() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        rename_category(&mut catalog, "cat-1", "ยาเม็ดทดสอบ")?;
        assert_eq!(catalog.categories[0].name, "ยาเม็ดทดสอบ");
        Ok(())
    }

    #[test]
    fn rename_category_rejects_unknown_id() {
        let mut catalog = Catalog::seed_default();
        assert_eq!(
            rename_category(&mut catalog, "cat-99", "x"),
            Err(CatalogOpsError::CategoryNotFound("cat-99".into()))
        );
    }

    #[test]
    fn remove_category_allows_empty_rejects_non_empty() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-2", &item("2001", "a"), "")?;
        assert_eq!(
            remove_category(&mut catalog, "cat-2"),
            Err(CatalogOpsError::CategoryNotEmpty("cat-2".into()))
        );
        remove_category(&mut catalog, "cat-1")?;
        assert!(
            !catalog
                .categories
                .iter()
                .any(|category| category.id == "cat-1")
        );
        Ok(())
    }

    #[test]
    fn add_drug_appends_with_next_order_in_category() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "a"), "10 เม็ด")?;
        add_drug(&mut catalog, "cat-1", &item("1002", "b"), "")?;
        let drugs = catalog.drugs_in_category("cat-1");
        assert_eq!(drugs.len(), 2);
        assert_eq!(drugs[0].icode, "1001");
        assert_eq!(drugs[0].stock_text, "10 เม็ด");
        assert_eq!(drugs[1].sort_order, 1);
        Ok(())
    }

    #[test]
    fn add_drug_prefills_strength_label_from_the_search_hit() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        let hit = DrugItem {
            icode: "1001".into(),
            name: "Paracetamol".into(),
            strength: "500".into(),
            units: "mg".into(),
        };
        add_drug(&mut catalog, "cat-1", &hit, "")?;
        assert_eq!(catalog.drugs[0].strength_text, "500 mg");
        Ok(())
    }

    #[test]
    fn add_drug_rejects_duplicate_icode_anywhere() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "a"), "")?;
        assert_eq!(
            add_drug(&mut catalog, "cat-2", &item("1001", "a"), ""),
            Err(CatalogOpsError::DuplicateDrug("1001".into()))
        );
        Ok(())
    }

    #[test]
    fn add_drug_rejects_unknown_category() {
        let mut catalog = Catalog::seed_default();
        assert_eq!(
            add_drug(&mut catalog, "cat-99", &item("1001", "a"), ""),
            Err(CatalogOpsError::CategoryNotFound("cat-99".into()))
        );
    }

    #[test]
    fn add_drug_rejects_blank_icode() {
        let mut catalog = Catalog::seed_default();
        assert_eq!(
            add_drug(&mut catalog, "cat-1", &item("   ", "a"), ""),
            Err(CatalogOpsError::EmptyIcode)
        );
    }

    #[test]
    fn update_drug_edits_name_strength_and_stock() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "Paracetamol"), "")?;
        update_drug(&mut catalog, "1001", "พาราเซตามอล", "500 mg", "10 เม็ด/ซอง")?;
        assert_eq!(catalog.drugs[0].name, "พาราเซตามอล");
        assert_eq!(catalog.drugs[0].strength_text, "500 mg");
        assert_eq!(catalog.drugs[0].stock_text, "10 เม็ด/ซอง");
        Ok(())
    }

    #[test]
    fn remove_drug_deletes_and_errors_when_missing() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "a"), "")?;
        remove_drug(&mut catalog, "1001")?;
        assert!(catalog.is_empty());
        assert_eq!(
            remove_drug(&mut catalog, "1001"),
            Err(CatalogOpsError::DrugNotFound("1001".into()))
        );
        Ok(())
    }

    #[test]
    fn move_drug_swaps_with_neighbour_and_stops_at_edges() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "a"), "")?;
        add_drug(&mut catalog, "cat-1", &item("1002", "b"), "")?;
        add_drug(&mut catalog, "cat-1", &item("1003", "c"), "")?;

        move_drug(&mut catalog, "1003", -1)?;
        let codes: Vec<&str> = catalog
            .drugs_in_category("cat-1")
            .iter()
            .map(|drug| drug.icode.as_str())
            .collect();
        assert_eq!(codes, vec!["1001", "1003", "1002"]);

        move_drug(&mut catalog, "1001", -1)?; // already first - no-op
        move_drug(&mut catalog, "1001", 1)?;
        let codes: Vec<&str> = catalog
            .drugs_in_category("cat-1")
            .iter()
            .map(|drug| drug.icode.as_str())
            .collect();
        assert_eq!(codes, vec!["1003", "1001", "1002"]);
        Ok(())
    }

    #[test]
    fn move_drug_is_scoped_to_its_category() -> Result<(), CatalogOpsError> {
        let mut catalog = Catalog::seed_default();
        add_drug(&mut catalog, "cat-1", &item("1001", "a"), "")?;
        add_drug(&mut catalog, "cat-1", &item("1002", "b"), "")?;
        add_drug(&mut catalog, "cat-2", &item("2001", "c"), "")?;
        add_drug(&mut catalog, "cat-2", &item("2002", "d"), "")?;

        move_drug(&mut catalog, "2002", -1)?;
        let cat1: Vec<&str> = catalog
            .drugs_in_category("cat-1")
            .iter()
            .map(|drug| drug.icode.as_str())
            .collect();
        let cat2: Vec<&str> = catalog
            .drugs_in_category("cat-2")
            .iter()
            .map(|drug| drug.icode.as_str())
            .collect();
        assert_eq!(cat1, vec!["1001", "1002"]);
        assert_eq!(cat2, vec!["2002", "2001"]);
        Ok(())
    }
}
