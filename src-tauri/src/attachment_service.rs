use crate::database::{attachments_dir, get_db};
use crate::error::{business_error, not_found, AppError};
use crate::models::service_order_attachment::ServiceOrderAttachment;
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::service_order_attachment_repo::ServiceOrderAttachmentRepository;
use crate::repositories::service_order_event_repo::ServiceOrderEventRepository;
use crate::repositories::service_order_repo::ServiceOrderRepository;
use base64::Engine;
use std::fs;
use std::path::Path;
use uuid::Uuid;

const MAX_ATTACHMENT_SIZE_BYTES: u64 = 10 * 1024 * 1024;

fn attachment_extension(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "application/pdf" => "pdf",
        _ => unreachable!("validated attachment MIME types are exhaustive"),
    }
}

fn validate_attachment_bytes(bytes: &[u8]) -> Result<&'static str, AppError> {
    let mime_type = infer::get(bytes)
        .map(|kind| kind.mime_type())
        .filter(|mime| {
            matches!(
                *mime,
                "image/png" | "image/jpeg" | "image/webp" | "application/pdf"
            )
        })
        .ok_or_else(|| {
            business_error(
                "Only valid PNG, JPEG, WEBP, and PDF attachments are supported.",
                "Apenas anexos PNG, JPEG, WEBP e PDF válidos são aceitos.",
            )
        })?;
    Ok(mime_type)
}

pub(crate) fn validate_attachment_file(path: &Path) -> Result<(String, Vec<u8>), AppError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        AppError::new(
            format!("Failed to read attachment metadata: {error}"),
            format!("Erro ao ler os metadados do anexo: {error}"),
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(business_error(
            "Attachment must be a regular file.",
            "O anexo deve ser um arquivo regular.",
        ));
    }
    if metadata.len() > MAX_ATTACHMENT_SIZE_BYTES {
        return Err(business_error(
            "Attachment exceeds the 10 MB limit.",
            "O anexo excede o limite de 10 MB.",
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        AppError::new(
            format!("Failed to read attachment: {error}"),
            format!("Erro ao ler o anexo: {error}"),
        )
    })?;
    if bytes.len() as u64 != metadata.len() {
        return Err(business_error(
            "Attachment changed while it was being read.",
            "O anexo foi alterado durante a leitura.",
        ));
    }
    Ok((validate_attachment_bytes(&bytes)?.to_string(), bytes))
}

pub fn add_attachment(
    service_order_id: &str,
    source_path: &Path,
) -> Result<ServiceOrderAttachment, AppError> {
    let conn = get_db()?;
    add_attachment_with_paths(&conn, service_order_id, source_path, &attachments_dir())
}

pub(crate) fn add_attachment_with_paths(
    conn: &rusqlite::Connection,
    service_order_id: &str,
    source_path: &Path,
    storage_dir: &Path,
) -> Result<ServiceOrderAttachment, AppError> {
    ServiceOrderRepository::get_by_id_with_conn(conn, service_order_id)?
        .ok_or_else(|| not_found("Service order", "Ordem de serviço"))?;
    let (mime_type, bytes) = validate_attachment_file(source_path)?;
    let size_bytes = bytes.len();
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            business_error(
                "Attachment file name is invalid.",
                "O nome do arquivo do anexo é inválido.",
            )
        })?
        .to_string();
    let storage_name = format!("{}.{}", Uuid::new_v4(), attachment_extension(&mime_type));
    crate::database::ensure_private_dir(storage_dir).map_err(|error| {
        AppError::new(
            format!("Failed to create attachment storage: {error}"),
            format!("Erro ao criar o armazenamento de anexos: {error}"),
        )
    })?;
    let stored_path = storage_dir.join(&storage_name);
    fs::write(&stored_path, &bytes).map_err(|error| {
        AppError::new(
            format!("Failed to store attachment: {error}"),
            format!("Erro ao armazenar o anexo: {error}"),
        )
    })?;
    crate::database::secure_private_file(&stored_path).map_err(|error| {
        AppError::new(
            format!("Failed to secure attachment storage: {error}"),
            format!("Erro ao proteger o armazenamento do anexo: {error}"),
        )
    })?;

    let attachment = ServiceOrderAttachment::new(
        service_order_id.to_string(),
        file_name,
        storage_name,
        mime_type.to_string(),
        size_bytes as i64,
    );
    if let Err(error) = ServiceOrderAttachmentRepository::create_with_conn(conn, &attachment) {
        let _ = fs::remove_file(stored_path);
        return Err(error.into());
    }
    let event = ServiceOrderEvent::new(
        service_order_id.to_string(),
        "attachment_added".to_string(),
        serde_json::json!({ "fileName": attachment.file_name }).to_string(),
    );
    ServiceOrderEventRepository::create_with_conn(conn, &event)?;
    Ok(attachment)
}

pub fn delete_attachment(id: &str) -> Result<(), AppError> {
    let conn = get_db()?;
    delete_attachment_with_paths(&conn, id, &attachments_dir())
}

pub(crate) fn delete_attachment_with_paths(
    conn: &rusqlite::Connection,
    id: &str,
    storage_dir: &Path,
) -> Result<(), AppError> {
    let attachment = ServiceOrderAttachmentRepository::delete_with_conn(conn, id).map_err(
        |error| match error {
            rusqlite::Error::QueryReturnedNoRows => not_found("Attachment", "Anexo"),
            other => other.into(),
        },
    )?;
    let stored_path = storage_dir.join(&attachment.storage_name);
    if stored_path.exists() {
        fs::remove_file(stored_path).map_err(|error| {
            AppError::new(
                format!("Failed to delete attachment file: {error}"),
                format!("Erro ao excluir o arquivo do anexo: {error}"),
            )
        })?;
    }
    let event = ServiceOrderEvent::new(
        attachment.service_order_id,
        "attachment_removed".to_string(),
        serde_json::json!({ "fileName": attachment.file_name }).to_string(),
    );
    ServiceOrderEventRepository::create_with_conn(conn, &event)?;
    Ok(())
}

pub fn read_attachment_as_data_url(id: &str) -> Result<String, AppError> {
    let attachment = ServiceOrderAttachmentRepository::get_by_id(id)?
        .ok_or_else(|| not_found("Attachment", "Anexo"))?;
    let bytes = fs::read(attachments_dir().join(&attachment.storage_name)).map_err(|error| {
        AppError::new(
            format!("Failed to read attachment: {error}"),
            format!("Erro ao ler o anexo: {error}"),
        )
    })?;
    let mime_type = validate_attachment_bytes(&bytes)?;
    if mime_type != attachment.mime_type {
        return Err(business_error(
            "Stored attachment content does not match its metadata.",
            "O conteúdo do anexo armazenado não corresponde aos metadados.",
        ));
    }
    Ok(format!(
        "data:{};base64,{}",
        mime_type,
        base64::engine::general_purpose::STANDARD.encode(bytes),
    ))
}

pub fn export_attachment(id: &str, destination: &Path) -> Result<(), AppError> {
    let attachment = ServiceOrderAttachmentRepository::get_by_id(id)?
        .ok_or_else(|| not_found("Attachment", "Anexo"))?;
    fs::copy(
        attachments_dir().join(&attachment.storage_name),
        destination,
    )
    .map_err(|error| {
        AppError::new(
            format!("Failed to export attachment: {error}"),
            format!("Erro ao exportar o anexo: {error}"),
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::service_order::ServiceOrder;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::test_helpers::setup_db;

    #[test]
    fn saves_and_removes_supported_attachment_files() {
        let conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41999999999".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Falha".to_string());
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        let temp_dir = std::env::temp_dir().join(format!("tcc-opet-attachment-{}", Uuid::new_v4()));
        let source = temp_dir.join("entrada.jpg");
        let storage = temp_dir.join("storage");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&source, b"\x89PNG\r\n\x1a\n").unwrap();

        let attachment = add_attachment_with_paths(&conn, &order.id, &source, &storage).unwrap();
        assert!(storage.join(&attachment.storage_name).exists());
        delete_attachment_with_paths(&conn, &attachment.id, &storage).unwrap();
        assert!(!storage.join(&attachment.storage_name).exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn rejects_attachment_with_disguised_extension() {
        let temp_dir = std::env::temp_dir().join(format!("tcc-opet-attachment-{}", Uuid::new_v4()));
        let source = temp_dir.join("malicious.jpg");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&source, b"not-an-image").unwrap();

        assert!(validate_attachment_file(&source).is_err());
        let _ = fs::remove_dir_all(temp_dir);
    }
}
