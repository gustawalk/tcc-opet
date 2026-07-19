use crate::attachment_service;
use crate::error::AppError;
use crate::models::service_order_attachment::ServiceOrderAttachment;
use crate::repositories::service_order_attachment_repo::ServiceOrderAttachmentRepository;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{command, AppHandle};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

pub(crate) static PENDING_ATTACHMENT_SELECTIONS: Lazy<Mutex<HashMap<String, Vec<PathBuf>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAttachmentSelection {
    token: String,
    file_names: Vec<String>,
}

fn pick_attachment_paths(app: &AppHandle) -> Result<Vec<PathBuf>, AppError> {
    app.dialog()
        .file()
        .add_filter("Anexos", &["png", "jpg", "jpeg", "webp", "pdf"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|file| {
            file.into_path().map_err(|error| {
                AppError::new(
                    format!("Failed to access selected attachment: {error}"),
                    format!("Erro ao acessar o anexo selecionado: {error}"),
                )
            })
        })
        .collect()
}

#[command]
pub async fn select_service_order_attachments(
    app: AppHandle,
    service_order_id: String,
) -> Result<Vec<ServiceOrderAttachment>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        pick_attachment_paths(&app)?
            .into_iter()
            .map(|file| attachment_service::add_attachment(&service_order_id, &file))
            .collect()
    })
    .await
    .map_err(|error| {
        AppError::new(
            format!("Failed to select attachments: {error}"),
            format!("Erro ao selecionar anexos: {error}"),
        )
    })?
}

#[command]
pub async fn select_pending_service_order_attachments(
    app: AppHandle,
) -> Result<Option<PendingAttachmentSelection>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = pick_attachment_paths(&app)?;
        if paths.is_empty() {
            return Ok(None);
        }
        let file_names = paths
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        AppError::new(
                            "Selected attachment has an invalid filename.",
                            "O anexo selecionado possui um nome inválido.",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let token = Uuid::new_v4().to_string();
        PENDING_ATTACHMENT_SELECTIONS
            .lock()
            .map_err(|_| {
                AppError::new(
                    "Pending attachment storage is unavailable.",
                    "O armazenamento temporário de anexos está indisponível.",
                )
            })?
            .insert(token.clone(), paths);
        Ok(Some(PendingAttachmentSelection { token, file_names }))
    })
    .await
    .map_err(|error| {
        AppError::new(
            format!("Failed to select attachments: {error}"),
            format!("Erro ao selecionar anexos: {error}"),
        )
    })?
}

#[command]
pub fn attach_pending_service_order_attachments(
    service_order_id: String,
    token: String,
) -> Result<Vec<ServiceOrderAttachment>, AppError> {
    let paths = PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .map_err(|_| {
            AppError::new(
                "Pending attachment storage is unavailable.",
                "O armazenamento temporário de anexos está indisponível.",
            )
        })?
        .remove(&token)
        .ok_or_else(|| {
            AppError::new(
                "Selected attachments are no longer available.",
                "Os anexos selecionados não estão mais disponíveis.",
            )
        })?;
    paths
        .iter()
        .map(|path| attachment_service::add_attachment(&service_order_id, path))
        .collect()
}

#[command]
pub fn discard_pending_service_order_attachments(token: String) {
    if let Ok(mut selections) = PENDING_ATTACHMENT_SELECTIONS.lock() {
        selections.remove(&token);
    }
}

#[command]
pub fn get_service_order_attachments(
    service_order_id: String,
) -> Result<Vec<ServiceOrderAttachment>, AppError> {
    Ok(ServiceOrderAttachmentRepository::get_by_service_order_id(
        &service_order_id,
    )?)
}

#[command]
pub fn delete_service_order_attachment(id: String) -> Result<(), AppError> {
    attachment_service::delete_attachment(&id)
}

#[command]
pub fn read_service_order_attachment(id: String) -> Result<String, AppError> {
    attachment_service::read_attachment_as_data_url(&id)
}

#[command]
pub fn export_service_order_attachment(id: String, destination: String) -> Result<(), AppError> {
    attachment_service::export_attachment(&id, std::path::Path::new(&destination))
}
