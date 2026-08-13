use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
}

pub(crate) fn like_pattern(search: &str) -> String {
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

pub(crate) fn like_search_clause(search: &str, columns: &[&str]) -> (String, Vec<String>) {
    let search = search.trim();
    if search.is_empty() {
        return (String::new(), Vec::new());
    }
    let pattern = like_pattern(search);
    let joined = columns
        .iter()
        .map(|column| format!("{column} LIKE ? ESCAPE '\\'"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let patterns = vec![pattern; columns.len()];
    (format!(" AND ({joined})"), patterns)
}
