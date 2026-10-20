use crate::error::AppError;
use crate::import_file::{ImportInspection, ParsedImport};
use std::path::Path;

#[tauri::command]
pub(crate) async fn inspect_import_file(path: String) -> Result<ImportInspection, AppError> {
    tauri::async_runtime::spawn_blocking(move || crate::import_file::inspect(Path::new(&path)))
        .await
        .map_err(|_| crate::import_file::file_error("Não foi possível analisar o arquivo."))?
}

#[tauri::command]
pub(crate) async fn parse_import_file(
    path: String,
    worksheet: Option<String>,
    delimiter: Option<String>,
) -> Result<ParsedImport, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::import_file::parse(Path::new(&path), worksheet.as_deref(), delimiter.as_deref())
    })
    .await
    .map_err(|_| crate::import_file::file_error("Não foi possível ler o arquivo."))?
}
