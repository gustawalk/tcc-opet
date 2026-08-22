use crate::error::AppError;
use crate::pdf_service::{self, PdfPreview};
use base64::Engine;
use serde::Serialize;
use std::fs;
use tauri::{command, AppHandle};
use tauri_plugin_dialog::DialogExt;

#[command]
pub fn preview_service_order_pdf(service_order_id: String) -> Result<PdfPreview, AppError> {
    pdf_service::preview_service_order_pdf(&service_order_id)
}

#[command]
pub async fn save_pdf_preview(app: AppHandle, token: String) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let (file_name, bytes) = pdf_service::get_pdf_preview(&token)?;
        let destination = app
            .dialog()
            .file()
            .add_filter("PDF", &["pdf"])
            .set_file_name(file_name)
            .blocking_save_file();
        let Some(destination) = destination else {
            return Ok(false);
        };
        let destination = destination.into_path().map_err(|error| {
            AppError::new(
                format!("Failed to access PDF destination: {error}"),
                format!("Erro ao acessar o destino do PDF: {error}"),
            )
        })?;
        fs::write(destination, bytes).map_err(|error| {
            AppError::new(
                format!("Failed to save PDF preview: {error}"),
                format!("Erro ao salvar a pré-visualização do PDF: {error}"),
            )
        })?;
        Ok(true)
    })
    .await
    .map_err(|error| {
        AppError::new(
            format!("Failed to save PDF preview: {error}"),
            format!("Erro ao salvar a pré-visualização do PDF: {error}"),
        )
    })?
}

#[command]
pub fn discard_pdf_preview(token: String) {
    pdf_service::discard_pdf_preview(&token);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PdfDownload {
    file_name: String,
    data_base64: String,
}

pub(crate) fn download_pdf_preview(token: String) -> Result<PdfDownload, AppError> {
    let (file_name, bytes) = pdf_service::get_pdf_preview(&token)?;
    Ok(PdfDownload {
        file_name,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}
