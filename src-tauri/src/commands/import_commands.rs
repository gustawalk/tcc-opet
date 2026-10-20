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

#[tauri::command]
pub(crate) fn validate_import_rows(
    request: crate::import_service::ImportRequest,
) -> Result<crate::import_service::ImportValidation, AppError> {
    let conn = crate::database::get_db()?;
    crate::import_service::validate_with_conn(&conn, &request)
}
