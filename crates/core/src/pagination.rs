//! Sheet pagination (DESIGN.md "sheet"): the requisition is laid out as
//! fixed A4 pages of at most [`ROWS_PER_PAGE`] drug rows, so a print run is
//! deterministic - the 69-item pilot list always prints on two sheets.
//!
//! Category header rows do **not** count toward the row budget (they are
//! thin and repeat on continuation). A category that is split across a page
//! boundary continues on the next page with `continued = true`, rendered as
//! "(ต่อ)". Every page repeats the document header, the window sentence,
//! and the table's column headers; the signature blocks and the print
//! stamp render on the last page only.

use er_refill_models::{ReportRow, ReportSection};

/// Drug rows per A4 page. Chosen so 69 items print on two sheets with room
/// for the document header, the repeated column headers, category rows,
/// and (on the last page) the signature blocks.
pub const ROWS_PER_PAGE: usize = 40;

/// One drug row placed on a page, with its original coordinates so the UI
/// can edit `requested` back in `AppSettings.sections`.
#[derive(Debug, Clone, PartialEq)]
pub struct PageRow {
    /// Index of the [`ReportSection`] this row came from.
    pub section_index: usize,
    /// Index of the row inside that section.
    pub row_index: usize,
    /// The row itself.
    pub row: ReportRow,
}

/// One category (or category continuation) inside a page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSection {
    /// Owning category id.
    pub category_id: String,
    /// Category display name.
    pub category_name: String,
    /// True when this is a continuation of a category split on a previous
    /// page ("(ต่อ)").
    pub continued: bool,
    /// Rows of this category on this page, in catalog order.
    pub rows: Vec<PageRow>,
}

/// One A4 page of the requisition.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPage {
    /// 1-based page number.
    pub number: usize,
    /// Total page count.
    pub count: usize,
    /// True for the final page (signatures and print stamp render here).
    pub is_last: bool,
    /// Category groups on this page, in catalog order.
    pub sections: Vec<PageSection>,
}

/// Splits the built report into A4 pages of at most `rows_per_page` drug
/// rows each.
///
/// Returns an empty vector when there is nothing to print or when
/// `rows_per_page` is zero (callers must pass at least 1; the UI uses
/// [`ROWS_PER_PAGE`]).
///
/// # Examples
///
/// ```
/// use er_refill_core::{paginate_sections, ROWS_PER_PAGE};
/// use er_refill_models::{ReportRow, ReportSection};
///
/// let row = |i: usize| ReportRow {
///     icode: format!("100{i}"),
///     name: format!("drug {i}"),
///     strength_text: String::new(),
///     stock_text: String::new(),
///     usage: i as f64,
///     requested: i as f64,
///     max_requested: None,
///     added: false,
///     reason: String::new(),
/// };
/// let section = ReportSection {
///     category_id: "cat-1".into(),
///     category_name: "ยาเม็ด".into(),
///     rows: (0..41).map(row).collect(),
/// };
/// let pages = paginate_sections(&[section], ROWS_PER_PAGE);
/// assert_eq!(pages.len(), 2);
/// assert_eq!(pages[0].sections[0].rows.len(), 40);
/// assert!(pages[1].sections[0].continued);
/// assert!(pages[1].is_last);
/// ```
pub fn paginate_sections(sections: &[ReportSection], rows_per_page: usize) -> Vec<SheetPage> {
    if rows_per_page == 0 {
        return Vec::new();
    }

    let mut pages: Vec<Vec<PageSection>> = Vec::new();
    let mut current: Vec<PageSection> = Vec::new();
    let mut used = 0usize;

    for (section_index, section) in sections.iter().enumerate() {
        if section.rows.is_empty() {
            continue;
        }
        let mut row_index = 0usize;
        while row_index < section.rows.len() {
            let take = (rows_per_page - used).min(section.rows.len() - row_index);
            let rows: Vec<PageRow> = (row_index..row_index + take)
                .map(|index| PageRow {
                    section_index,
                    row_index: index,
                    row: section.rows[index].clone(),
                })
                .collect();
            current.push(PageSection {
                category_id: section.category_id.clone(),
                category_name: section.category_name.clone(),
                continued: row_index > 0,
                rows,
            });
            used += take;
            row_index += take;
            if used == rows_per_page {
                pages.push(std::mem::take(&mut current));
                used = 0;
            }
        }
    }
    if !current.is_empty() {
        pages.push(current);
    }

    let count = pages.len();
    pages
        .into_iter()
        .enumerate()
        .map(|(index, sections)| SheetPage {
            number: index + 1,
            count,
            is_last: index + 1 == count,
            sections,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(icode: &str) -> ReportRow {
        ReportRow {
            icode: icode.to_string(),
            name: format!("drug {icode}"),
            strength_text: String::new(),
            stock_text: String::new(),
            usage: 1.0,
            requested: 1.0,
            max_requested: None,
            added: false,
            reason: String::new(),
        }
    }

    fn section(id: &str, rows: usize) -> ReportSection {
        ReportSection {
            category_id: id.to_string(),
            category_name: format!("หมวด {id}"),
            rows: (0..rows).map(|i| row(&format!("{id}-{i}"))).collect(),
        }
    }

    /// The approved ER list shape: 69 items across six categories.
    fn pilot_sections() -> Vec<ReportSection> {
        [18, 11, 17, 8, 13, 2]
            .into_iter()
            .enumerate()
            .map(|(index, rows)| section(&format!("cat-{}", index + 1), rows))
            .collect()
    }

    #[test]
    fn pilot_list_paginates_into_two_pages_of_40_and_29() {
        let pages = paginate_sections(&pilot_sections(), ROWS_PER_PAGE);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].number, 1);
        assert_eq!(pages[1].number, 2);
        assert_eq!(pages[0].count, 2);
        assert!(!pages[0].is_last);
        assert!(pages[1].is_last);
        let page1_rows: usize = pages[0].sections.iter().map(|s| s.rows.len()).sum();
        let page2_rows: usize = pages[1].sections.iter().map(|s| s.rows.len()).sum();
        assert_eq!(page1_rows, ROWS_PER_PAGE);
        assert_eq!(page2_rows, 29);
    }

    #[test]
    fn a_split_category_continues_with_continuation_flag() {
        let pages = paginate_sections(&pilot_sections(), ROWS_PER_PAGE);
        // Category 3 (17 rows) starts at row 29 of page 1: 11 rows there,
        // 6 rows continue on page 2.
        let cat3_page1 = pages[0]
            .sections
            .iter()
            .find(|section| section.category_id == "cat-3")
            .expect("cat-3 on page 1");
        assert!(!cat3_page1.continued);
        assert_eq!(cat3_page1.rows.len(), 11);

        let cat3_page2 = pages[1]
            .sections
            .iter()
            .find(|section| section.category_id == "cat-3")
            .expect("cat-3 on page 2");
        assert!(cat3_page2.continued);
        assert_eq!(cat3_page2.rows.len(), 6);
        // The first continuation row is row index 11 of the original section.
        assert_eq!(cat3_page2.rows[0].section_index, 2);
        assert_eq!(cat3_page2.rows[0].row_index, 11);
    }

    #[test]
    fn an_exact_fill_starts_the_next_category_on_a_new_page() {
        let sections = vec![section("cat-1", ROWS_PER_PAGE), section("cat-2", 3)];
        let pages = paginate_sections(&sections, ROWS_PER_PAGE);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].sections.len(), 1);
        assert_eq!(pages[1].sections.len(), 1);
        assert_eq!(pages[1].sections[0].category_id, "cat-2");
        assert!(!pages[1].sections[0].continued);
    }

    #[test]
    fn continuation_rows_keep_original_coordinates() {
        let sections = vec![section("cat-1", 45)];
        let pages = paginate_sections(&sections, ROWS_PER_PAGE);
        let first = &pages[1].sections[0].rows[0];
        assert_eq!(first.section_index, 0);
        assert_eq!(first.row_index, ROWS_PER_PAGE);
        assert_eq!(first.row.icode, format!("cat-1-{ROWS_PER_PAGE}"));
    }

    #[test]
    fn one_row_per_page_keeps_the_document_order() {
        let sections = vec![section("cat-1", 2), section("cat-2", 1)];
        let pages = paginate_sections(&sections, 1);
        assert_eq!(pages.len(), 3);
        let codes: Vec<&str> = pages
            .iter()
            .map(|page| page.sections[0].rows[0].row.icode.as_str())
            .collect();
        assert_eq!(codes, vec!["cat-1-0", "cat-1-1", "cat-2-0"]);
    }

    #[test]
    fn empty_input_and_empty_sections_produce_no_pages() {
        assert!(paginate_sections(&[], ROWS_PER_PAGE).is_empty());
        assert!(paginate_sections(&[section("cat-1", 0)], ROWS_PER_PAGE).is_empty());
        assert!(paginate_sections(&[section("cat-1", 5)], 0).is_empty());
    }
}
