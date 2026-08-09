use crate::database::{attachments_dir, get_db};
use crate::error::{business_error, not_found, AppError};
use crate::models::service_order_attachment::ServiceOrderAttachment;
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::service_order_attachment_repo::ServiceOrderAttachmentRepository;
use crate::repositories::service_order_event_repo::ServiceOrderEventRepository;
use crate::repositories::service_order_repo::ServiceOrderRepository;
use base64::Engine;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_ATTACHMENT_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const ENVELOPE_MAGIC: &[u8] = b"OPETA\x01";

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

fn attachment_aad(attachment: &ServiceOrderAttachment) -> Vec<u8> {
    format!(
        "{}:{}:{}:{}:{}",
        attachment.id,
        attachment.service_order_id,
        attachment.storage_name,
        attachment.mime_type,
        attachment.size_bytes,
    )
    .into_bytes()
}

fn encrypt_attachment_bytes(
    attachment: &ServiceOrderAttachment,
    bytes: &[u8],
) -> Result<Vec<u8>, AppError> {
    let cipher = XChaCha20Poly1305::new((&crate::encryption::attachment_key()).into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: bytes,
                aad: &attachment_aad(attachment),
            },
        )
        .map_err(|_| {
            business_error(
                "Could not encrypt attachment.",
                "Não foi possível criptografar o anexo.",
            )
        })?;
    let mut envelope = Vec::with_capacity(ENVELOPE_MAGIC.len() + nonce.len() + ciphertext.len());
    envelope.extend_from_slice(ENVELOPE_MAGIC);
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

fn decrypt_attachment_bytes(
    attachment: &ServiceOrderAttachment,
    envelope: &[u8],
) -> Result<Vec<u8>, AppError> {
    if !envelope.starts_with(ENVELOPE_MAGIC) {
        return Err(business_error(
            "Stored attachment is not encrypted.",
            "O anexo armazenado não está criptografado.",
        ));
    }
    let nonce_end = ENVELOPE_MAGIC.len() + 24;
    if envelope.len() <= nonce_end {
        return Err(business_error(
            "Stored attachment envelope is invalid.",
            "O anexo armazenado é inválido.",
        ));
    }
    let cipher = XChaCha20Poly1305::new((&crate::encryption::attachment_key()).into());
    cipher
        .decrypt(
            XNonce::from_slice(&envelope[ENVELOPE_MAGIC.len()..nonce_end]),
            Payload {
                msg: &envelope[nonce_end..],
                aad: &attachment_aad(attachment),
            },
        )
        .map_err(|_| {
            business_error(
                "Stored attachment failed authentication.",
                "Não foi possível autenticar o anexo armazenado.",
            )
        })
}

fn read_stored_attachment(
    storage_dir: &Path,
    attachment: &ServiceOrderAttachment,
) -> Result<Vec<u8>, AppError> {
    let envelope = fs::read(storage_dir.join(&attachment.storage_name)).map_err(|error| {
        AppError::new(
            format!("Failed to read attachment: {error}"),
            format!("Erro ao ler o anexo: {error}"),
        )
    })?;
    let bytes = decrypt_attachment_bytes(attachment, &envelope)?;
    let mime_type = validate_attachment_bytes(&bytes)?;
    if mime_type != attachment.mime_type || bytes.len() as i64 != attachment.size_bytes {
        return Err(business_error(
            "Stored attachment content does not match its metadata.",
            "O conteúdo do anexo armazenado não corresponde aos metadados.",
        ));
    }
    Ok(bytes)
}

fn write_encrypted_attachment(
    storage_dir: &Path,
    attachment: &ServiceOrderAttachment,
    bytes: &[u8],
) -> Result<(), AppError> {
    crate::database::ensure_private_dir(storage_dir).map_err(|error| {
        AppError::new(
            format!("Failed to create attachment storage: {error}"),
            format!("Erro ao criar o armazenamento de anexos: {error}"),
        )
    })?;
    let destination = storage_dir.join(&attachment.storage_name);
    let temporary = storage_dir.join(format!(
        ".{}-{}.tmp",
        attachment.storage_name,
        Uuid::new_v4()
    ));
    let result = (|| -> Result<(), AppError> {
        let envelope = encrypt_attachment_bytes(attachment, bytes)?;
        let mut file = File::create(&temporary).map_err(|error| {
            AppError::new(
                format!("Failed to store attachment: {error}"),
                format!("Erro ao armazenar o anexo: {error}"),
            )
        })?;
        file.write_all(&envelope).map_err(|error| {
            AppError::new(
                format!("Failed to store attachment: {error}"),
                format!("Erro ao armazenar o anexo: {error}"),
            )
        })?;
        file.sync_all().map_err(|error| {
            AppError::new(
                format!("Failed to store attachment: {error}"),
                format!("Erro ao armazenar o anexo: {error}"),
            )
        })?;
        crate::database::secure_private_file(&temporary).map_err(|error| {
            AppError::new(
                format!("Failed to secure attachment storage: {error}"),
                format!("Erro ao proteger o armazenamento de anexos: {error}"),
            )
        })?;
        fs::rename(&temporary, &destination).map_err(|error| {
            AppError::new(
                format!("Failed to store attachment: {error}"),
                format!("Erro ao armazenar o anexo: {error}"),
            )
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
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

#[cfg(test)]
pub fn add_attachment(
    service_order_id: &str,
    source_path: &Path,
) -> Result<ServiceOrderAttachment, AppError> {
    let conn = get_db()?;
    let mut attachments = add_attachments_atomically_with_paths(
        &conn,
        service_order_id,
        &[source_path.to_path_buf()],
        &attachments_dir(),
    )?;
    Ok(attachments.remove(0))
}

pub(crate) fn add_attachments_atomically_with_paths(
    conn: &rusqlite::Connection,
    service_order_id: &str,
    source_paths: &[PathBuf],
    storage_dir: &Path,
) -> Result<Vec<ServiceOrderAttachment>, AppError> {
    let transaction = conn.unchecked_transaction()?;
    let mut attachments = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        match add_attachment_with_paths(&transaction, service_order_id, source_path, storage_dir) {
            Ok(attachment) => attachments.push(attachment),
            Err(error) => {
                for attachment in &attachments {
                    let _ = fs::remove_file(storage_dir.join(&attachment.storage_name));
                }
                return Err(error);
            }
        }
    }
    if let Err(error) = transaction.commit() {
        for attachment in &attachments {
            let _ = fs::remove_file(storage_dir.join(&attachment.storage_name));
        }
        return Err(error.into());
    }
    Ok(attachments)
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
    let attachment = ServiceOrderAttachment::new(
        service_order_id.to_string(),
        file_name,
        Uuid::new_v4().to_string(),
        mime_type.to_string(),
        size_bytes as i64,
    );
    write_encrypted_attachment(storage_dir, &attachment, &bytes)?;
    if let Err(error) = ServiceOrderAttachmentRepository::create_with_conn(conn, &attachment) {
        let _ = fs::remove_file(storage_dir.join(&attachment.storage_name));
        return Err(error.into());
    }
    let event = ServiceOrderEvent::new(
        service_order_id.to_string(),
        "attachment_added".to_string(),
        serde_json::json!({ "fileName": attachment.file_name }).to_string(),
    );
    if let Err(error) = ServiceOrderEventRepository::create_with_conn(conn, &event) {
        let _ = fs::remove_file(storage_dir.join(&attachment.storage_name));
        return Err(error.into());
    }
    Ok(attachment)
}

pub fn delete_attachment(id: &str) -> Result<(), AppError> {
    let _guard = crate::database::exclusive_storage_guard()?;
    let conn = crate::database::open_encrypted_database(&crate::database::database_path())?;
    delete_attachment_with_paths(&conn, id, &attachments_dir())
}

pub(crate) fn delete_attachment_with_paths(
    conn: &rusqlite::Connection,
    id: &str,
    storage_dir: &Path,
) -> Result<(), AppError> {
    let attachment = ServiceOrderAttachmentRepository::get_by_id_with_conn(conn, id)?
        .ok_or_else(|| not_found("Attachment", "Anexo"))?;
    let transaction = conn.unchecked_transaction()?;
    let stored_path = storage_dir.join(&attachment.storage_name);
    let staged_path = if stored_path.exists() {
        let metadata = fs::symlink_metadata(&stored_path).map_err(|error| {
            AppError::new(
                format!("Failed to inspect attachment file: {error}"),
                format!("Erro ao inspecionar o arquivo do anexo: {error}"),
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(business_error(
                "Stored attachment is not a regular file.",
                "O anexo armazenado não é um arquivo regular.",
            ));
        }
        let staged_path = storage_dir.join(format!(".delete-{}", attachment.storage_name));
        fs::rename(&stored_path, &staged_path).map_err(|error| {
            AppError::new(
                format!("Failed to stage attachment deletion: {error}"),
                format!("Erro ao preparar a exclusão do anexo: {error}"),
            )
        })?;
        Some(staged_path)
    } else {
        None
    };
    if let Err(error) = ServiceOrderAttachmentRepository::delete_with_conn(&transaction, id) {
        restore_staged_attachment_file(staged_path.as_deref(), &stored_path)?;
        return Err(match error {
            rusqlite::Error::QueryReturnedNoRows => not_found("Attachment", "Anexo"),
            other => other.into(),
        });
    }
    let event = ServiceOrderEvent::new(
        attachment.service_order_id,
        "attachment_removed".to_string(),
        serde_json::json!({ "fileName": attachment.file_name }).to_string(),
    );
    if let Err(error) = ServiceOrderEventRepository::create_with_conn(&transaction, &event) {
        restore_staged_attachment_file(staged_path.as_deref(), &stored_path)?;
        return Err(error.into());
    }
    if let Err(error) = transaction.commit() {
        restore_staged_attachment_file(staged_path.as_deref(), &stored_path)?;
        return Err(error.into());
    }
    if let Some(staged_path) = staged_path {
        if let Err(error) = fs::remove_file(staged_path) {
            eprintln!("[ATTACHMENT] Failed to remove staged attachment: {error}");
        }
    }
    Ok(())
}

fn restore_staged_attachment_file(
    staged: Option<&Path>,
    destination: &Path,
) -> Result<(), AppError> {
    if let Some(staged) = staged {
        fs::rename(staged, destination).map_err(|error| {
            AppError::new(
                format!("Failed to restore attachment after database failure: {error}"),
                format!("Erro ao restaurar o anexo após falha no banco de dados: {error}"),
            )
        })?;
    }
    Ok(())
}

pub(crate) fn recover_staged_attachment_deletions(
    conn: &rusqlite::Connection,
    storage_dir: &Path,
) -> Result<(), AppError> {
    if !storage_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(storage_dir).map_err(|error| {
        AppError::new(
            format!("Failed to inspect attachment recovery files: {error}"),
            format!("Erro ao inspecionar arquivos de recuperação de anexos: {error}"),
        )
    })? {
        let entry = entry.map_err(|error| {
            AppError::new(
                format!("Failed to inspect attachment recovery entry: {error}"),
                format!("Erro ao inspecionar item de recuperação de anexo: {error}"),
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            AppError::new(
                format!("Failed to inspect attachment recovery type: {error}"),
                format!("Erro ao inspecionar tipo de recuperação de anexo: {error}"),
            )
        })?;
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(storage_name) = file_name
            .to_str()
            .and_then(|name| name.strip_prefix(".delete-"))
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let metadata_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM service_order_attachments WHERE storage_name = ?1)",
            [storage_name],
            |row| row.get(0),
        )?;
        let staged_path = entry.path();
        let original_path = storage_dir.join(storage_name);
        if metadata_exists && !original_path.exists() {
            fs::rename(&staged_path, &original_path).map_err(|error| {
                AppError::new(
                    format!("Failed to recover staged attachment: {error}"),
                    format!("Erro ao recuperar anexo preparado: {error}"),
                )
            })?;
        } else {
            fs::remove_file(staged_path).map_err(|error| {
                AppError::new(
                    format!("Failed to clean staged attachment: {error}"),
                    format!("Erro ao limpar anexo preparado: {error}"),
                )
            })?;
        }
    }
    Ok(())
}

pub fn read_attachment_as_data_url(id: &str) -> Result<String, AppError> {
    let conn = get_db()?;
    let attachment = ServiceOrderAttachmentRepository::get_by_id_with_conn(&conn, id)?
        .ok_or_else(|| not_found("Attachment", "Anexo"))?;
    read_attachment_as_data_url_with_paths(&attachment, &attachments_dir())
}

pub(crate) fn read_attachment_as_data_url_with_paths(
    attachment: &ServiceOrderAttachment,
    storage_dir: &Path,
) -> Result<String, AppError> {
    let bytes = read_stored_attachment(storage_dir, attachment)?;
    Ok(format!(
        "data:{};base64,{}",
        attachment.mime_type,
        base64::engine::general_purpose::STANDARD.encode(bytes),
    ))
}

pub fn export_attachment(id: &str, destination: &Path) -> Result<(), AppError> {
    let conn = get_db()?;
    let attachment = ServiceOrderAttachmentRepository::get_by_id_with_conn(&conn, id)?
        .ok_or_else(|| not_found("Attachment", "Anexo"))?;
    export_attachment_with_paths(&attachment, &attachments_dir(), destination)
}

pub(crate) fn export_attachment_with_paths(
    attachment: &ServiceOrderAttachment,
    storage_dir: &Path,
    destination: &Path,
) -> Result<(), AppError> {
    fs::write(
        destination,
        read_stored_attachment(storage_dir, attachment)?,
    )
    .map_err(|error| {
        AppError::new(
            format!("Failed to export attachment: {error}"),
            format!("Erro ao exportar o anexo: {error}"),
        )
    })?;
    Ok(())
}

pub(crate) fn migrate_legacy_attachments(
    conn: &rusqlite::Connection,
    storage_dir: &Path,
) -> Result<(), AppError> {
    if !storage_dir.exists() {
        return Ok(());
    }
    let mut statement = conn.prepare(
        "SELECT id, service_order_id, file_name, storage_name, mime_type, size_bytes, created_at
         FROM service_order_attachments",
    )?;
    let attachments = statement
        .query_map([], |row| {
            Ok(ServiceOrderAttachment {
                id: row.get(0)?,
                service_order_id: row.get(1)?,
                file_name: row.get(2)?,
                storage_name: row.get(3)?,
                mime_type: row.get(4)?,
                size_bytes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let managed_names = attachments
        .iter()
        .map(|attachment| attachment.storage_name.as_str())
        .collect::<HashSet<_>>();
    let has_only_managed_files = fs::read_dir(storage_dir)
        .map_err(|error| {
            AppError::new(
                format!("Failed to read attachment storage: {error}"),
                format!("Erro ao ler o armazenamento de anexos: {error}"),
            )
        })?
        .filter_map(Result::ok)
        .all(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| managed_names.contains(name))
                .unwrap_or(false)
        });
    if has_only_managed_files
        && attachments
            .iter()
            .all(|attachment| read_stored_attachment(storage_dir, attachment).is_ok())
    {
        return Ok(());
    }

    let parent = storage_dir.parent().ok_or_else(|| {
        business_error(
            "Attachment storage has no parent directory.",
            "O armazenamento de anexos não possui diretório pai.",
        )
    })?;
    let staging = parent.join(format!(".opets-attachments-migrating-{}", Uuid::new_v4()));
    let previous = parent.join(format!(".opets-attachments-previous-{}", Uuid::new_v4()));
    let result = (|| -> Result<(), AppError> {
        crate::database::ensure_private_dir(&staging).map_err(|error| {
            AppError::new(
                format!("Failed to prepare attachment migration: {error}"),
                format!("Erro ao preparar a migração de anexos: {error}"),
            )
        })?;
        for attachment in &attachments {
            let bytes = fs::read(storage_dir.join(&attachment.storage_name)).map_err(|error| {
                AppError::new(
                    format!("Failed to read legacy attachment: {error}"),
                    format!("Erro ao ler o anexo legado: {error}"),
                )
            })?;
            if bytes.starts_with(ENVELOPE_MAGIC) {
                fs::write(staging.join(&attachment.storage_name), bytes).map_err(|error| {
                    AppError::new(
                        format!("Failed to migrate attachment: {error}"),
                        format!("Erro ao migrar o anexo: {error}"),
                    )
                })?;
            } else {
                let mime_type = validate_attachment_bytes(&bytes)?;
                if mime_type != attachment.mime_type || bytes.len() as i64 != attachment.size_bytes
                {
                    return Err(business_error(
                        "Legacy attachment content does not match its metadata.",
                        "O conteúdo do anexo legado não corresponde aos metadados.",
                    ));
                }
                write_encrypted_attachment(&staging, attachment, &bytes)?;
            }
            read_stored_attachment(&staging, attachment)?;
        }
        fs::rename(storage_dir, &previous).map_err(|error| {
            AppError::new(
                format!("Failed to activate attachment migration: {error}"),
                format!("Erro ao ativar a migração de anexos: {error}"),
            )
        })?;
        if let Err(error) = fs::rename(&staging, storage_dir) {
            let _ = fs::rename(&previous, storage_dir);
            return Err(AppError::new(
                format!("Failed to activate attachment migration: {error}"),
                format!("Erro ao ativar a migração de anexos: {error}"),
            ));
        }
        let _ = fs::remove_dir_all(&previous);
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    result
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
        let stored = fs::read(storage.join(&attachment.storage_name)).unwrap();
        assert!(stored.starts_with(ENVELOPE_MAGIC));
        assert_ne!(stored, b"\x89PNG\r\n\x1a\n");
        assert_eq!(
            read_stored_attachment(&storage, &attachment).unwrap(),
            b"\x89PNG\r\n\x1a\n"
        );
        assert!(
            read_attachment_as_data_url_with_paths(&attachment, &storage)
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        let exported = temp_dir.join("exported.png");
        export_attachment_with_paths(&attachment, &storage, &exported).unwrap();
        assert_eq!(fs::read(exported).unwrap(), b"\x89PNG\r\n\x1a\n");
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

    #[test]
    fn encryption_rejects_invalid_or_tampered_envelopes() {
        let attachment = ServiceOrderAttachment::new(
            "order-1".to_string(),
            "entrada.png".to_string(),
            "stored-file".to_string(),
            "image/png".to_string(),
            8,
        );
        let bytes = b"\x89PNG\r\n\x1a\n";
        let envelope = encrypt_attachment_bytes(&attachment, bytes).unwrap();

        assert_eq!(
            decrypt_attachment_bytes(&attachment, &envelope).unwrap(),
            bytes
        );
        assert!(decrypt_attachment_bytes(&attachment, b"plaintext").is_err());
        assert!(decrypt_attachment_bytes(&attachment, ENVELOPE_MAGIC).is_err());

        let mut tampered = envelope;
        *tampered.last_mut().unwrap() ^= 1;
        assert!(decrypt_attachment_bytes(&attachment, &tampered).is_err());

        let mut mismatched = attachment.clone();
        mismatched.size_bytes = 9;
        assert!(decrypt_attachment_bytes(
            &mismatched,
            &encrypt_attachment_bytes(&attachment, bytes).unwrap()
        )
        .is_err());
    }

    #[test]
    fn validates_attachment_file_type_and_metadata() {
        let temp_dir = std::env::temp_dir().join(format!("tcc-opet-attachment-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let error = validate_attachment_file(&temp_dir).unwrap_err();
        assert_eq!(error.en, "Attachment must be a regular file.");

        let source = temp_dir.join("entrada.png");
        fs::write(&source, b"\x89PNG\r\n\x1a\n").unwrap();
        assert_eq!(validate_attachment_file(&source).unwrap().0, "image/png");
        assert!(validate_attachment_file(&temp_dir.join("missing.png")).is_err());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn migrates_plaintext_legacy_attachment_to_an_encrypted_envelope() {
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
        let temp_dir =
            std::env::temp_dir().join(format!("tcc-opet-legacy-attachment-{}", Uuid::new_v4()));
        let storage = temp_dir.join("storage");
        fs::create_dir_all(&storage).unwrap();
        let attachment = ServiceOrderAttachment::new(
            order.id,
            "entrada.png".to_string(),
            "legacy.png".to_string(),
            "image/png".to_string(),
            8,
        );
        ServiceOrderAttachmentRepository::create_with_conn(&conn, &attachment).unwrap();
        fs::write(storage.join(&attachment.storage_name), b"\x89PNG\r\n\x1a\n").unwrap();

        migrate_legacy_attachments(&conn, &storage).unwrap();

        let stored = fs::read(storage.join(&attachment.storage_name)).unwrap();
        assert!(stored.starts_with(ENVELOPE_MAGIC));
        assert_eq!(
            read_stored_attachment(&storage, &attachment).unwrap(),
            b"\x89PNG\r\n\x1a\n"
        );
        let _ = fs::remove_dir_all(temp_dir);
    }
}
