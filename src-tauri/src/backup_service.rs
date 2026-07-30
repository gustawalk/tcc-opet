use crate::database::{
    is_plaintext_database, migrate_plaintext_database, open_encrypted_database, run_migrations,
};
use crate::error::AppError;
use argon2::Argon2;
use base64::Engine;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rusqlite::backup::Backup;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(test)]
use std::path::PathBuf;
use std::path::{Component, Path};
use std::time::Duration;
use uuid::Uuid;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DATABASE_ENTRY: &str = "database.db";
const ATTACHMENTS_PREFIX: &str = "attachments/";
const MANIFEST_ENTRY: &str = "opets-backup.json";
const BACKUP_FORMAT_VERSION: u8 = 2;
const ENCRYPTED_BACKUP_MAGIC: &[u8] = b"OPETBKP2";
const MAX_BACKUP_FILE_SIZE_BYTES: u64 = 250 * 1024 * 1024;
const MAX_DATABASE_SIZE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_ATTACHMENT_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_ATTACHMENT_COUNT: usize = 200;
const MAX_ARCHIVE_ENTRIES: usize = MAX_ATTACHMENT_COUNT + 2;

const REQUIRED_TABLES: [&str; 14] = [
    "settings",
    "customers",
    "users",
    "inventory_items",
    "service_orders",
    "checklist_templates",
    "template_items",
    "service_order_checklists",
    "service_order_parts",
    "inventory_movements",
    "financial_snapshots",
    "service_order_sequences",
    "service_order_events",
    "service_order_attachments",
];

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    application: String,
    format_version: u8,
    #[serde(default)]
    key_version: Option<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupEnvelopeHeader {
    format_version: u8,
    key_version: u8,
    requires_passphrase: bool,
    salt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub path: String,
    pub attachment_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInspection {
    pub requires_passphrase: bool,
}

fn backup_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        format!("Backup operation failed: {error}"),
        format!("A operação de backup falhou: {error}"),
    )
}

fn activate_backup_file(temporary: &Path, destination: &Path) -> Result<(), AppError> {
    let parent = destination
        .parent()
        .ok_or_else(|| backup_error("Backup destination has no parent directory."))?;
    let previous = parent.join(format!(".opets-backup-previous-{}", Uuid::new_v4()));
    let had_destination = destination.exists();
    if had_destination {
        fs::rename(destination, &previous).map_err(backup_error)?;
    }
    if let Err(error) = fs::rename(temporary, destination) {
        if had_destination {
            let _ = fs::rename(&previous, destination);
        }
        return Err(backup_error(error));
    }
    if had_destination {
        let _ = fs::remove_file(previous);
    }
    Ok(())
}

fn backup_envelope_key(
    passphrase: Option<&str>,
    salt: Option<&[u8]>,
) -> Result<([u8; 32], Option<Vec<u8>>, bool), AppError> {
    let passphrase = passphrase.unwrap_or("");
    if passphrase.is_empty() {
        return Ok((
            crate::encryption::derive_key("com.walk.tcc-opet/backup/v1"),
            None,
            false,
        ));
    }
    let salt = salt.map(Vec::from).unwrap_or_else(|| {
        let mut value = vec![0_u8; 16];
        use rand::RngCore;
        OsRng.fill_bytes(&mut value);
        value
    });
    let mut key = [0_u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(backup_error)?;
    Ok((key, Some(salt), true))
}

fn encrypt_backup_archive(archive: &[u8], passphrase: Option<&str>) -> Result<Vec<u8>, AppError> {
    let (key, salt, requires_passphrase) = backup_envelope_key(passphrase, None)?;
    let header = BackupEnvelopeHeader {
        format_version: BACKUP_FORMAT_VERSION,
        key_version: crate::encryption::ACTIVE_KEY_VERSION,
        requires_passphrase,
        salt: salt.map(|value| base64::engine::general_purpose::STANDARD.encode(value)),
    };
    let header_bytes = serde_json::to_vec(&header).map_err(backup_error)?;
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let cipher = XChaCha20Poly1305::new((&key).into());
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: archive,
                aad: &header_bytes,
            },
        )
        .map_err(backup_error)?;
    let mut output = Vec::with_capacity(
        ENCRYPTED_BACKUP_MAGIC.len() + 4 + header_bytes.len() + nonce.len() + ciphertext.len(),
    );
    output.extend_from_slice(ENCRYPTED_BACKUP_MAGIC);
    output.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    output.extend_from_slice(&header_bytes);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn validate_backup_envelope_header(header: &BackupEnvelopeHeader) -> Result<(), AppError> {
    if header.format_version != BACKUP_FORMAT_VERSION
        || header.key_version != crate::encryption::ACTIVE_KEY_VERSION
    {
        return Err(backup_error(
            "Encrypted backup uses an unsupported format or key version.",
        ));
    }
    if header.requires_passphrase != header.salt.is_some() {
        return Err(backup_error("Encrypted backup header is invalid."));
    }
    Ok(())
}

pub fn inspect_backup(source: &Path) -> Result<BackupInspection, AppError> {
    let metadata = fs::symlink_metadata(source).map_err(backup_error)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_BACKUP_FILE_SIZE_BYTES {
        return Err(backup_error(
            "Backup source is invalid or exceeds the allowed size limit.",
        ));
    }

    let mut file = File::open(source).map_err(backup_error)?;
    let mut prefix = [0_u8; ENCRYPTED_BACKUP_MAGIC.len() + 4];
    file.read_exact(&mut prefix).map_err(backup_error)?;
    if !prefix.starts_with(ENCRYPTED_BACKUP_MAGIC) {
        return Ok(BackupInspection {
            requires_passphrase: false,
        });
    }

    let header_length = u32::from_be_bytes(
        prefix[ENCRYPTED_BACKUP_MAGIC.len()..]
            .try_into()
            .map_err(backup_error)?,
    ) as usize;
    if header_length == 0 || header_length > 64 * 1024 {
        return Err(backup_error("Encrypted backup header is invalid."));
    }
    let mut header_bytes = vec![0_u8; header_length];
    file.read_exact(&mut header_bytes).map_err(backup_error)?;
    let header: BackupEnvelopeHeader =
        serde_json::from_slice(&header_bytes).map_err(backup_error)?;
    validate_backup_envelope_header(&header)?;
    Ok(BackupInspection {
        requires_passphrase: header.requires_passphrase,
    })
}

fn decrypt_backup_archive(
    bytes: &[u8],
    passphrase: Option<&str>,
) -> Result<Option<Vec<u8>>, AppError> {
    if !bytes.starts_with(ENCRYPTED_BACKUP_MAGIC) {
        return Ok(None);
    }
    let header_length_start = ENCRYPTED_BACKUP_MAGIC.len();
    let header_length_end = header_length_start + 4;
    if bytes.len() <= header_length_end {
        return Err(backup_error("Encrypted backup header is incomplete."));
    }
    let header_length = u32::from_be_bytes(
        bytes[header_length_start..header_length_end]
            .try_into()
            .map_err(backup_error)?,
    ) as usize;
    let header_end = header_length_end
        .checked_add(header_length)
        .ok_or_else(|| backup_error("Encrypted backup header is invalid."))?;
    let nonce_end = header_end + 24;
    if bytes.len() <= nonce_end {
        return Err(backup_error("Encrypted backup payload is incomplete."));
    }
    let header_bytes = &bytes[header_length_end..header_end];
    let header: BackupEnvelopeHeader =
        serde_json::from_slice(header_bytes).map_err(backup_error)?;
    validate_backup_envelope_header(&header)?;
    let salt = header
        .salt
        .as_deref()
        .map(|value| {
            base64::engine::general_purpose::STANDARD
                .decode(value)
                .map_err(backup_error)
        })
        .transpose()?;
    if header.requires_passphrase && passphrase.unwrap_or("").is_empty() {
        return Err(AppError::new(
            "Backup passphrase is required.",
            "Este backup requer uma senha.",
        ));
    }
    let (key, _, _) = backup_envelope_key(passphrase, salt.as_deref())?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let archive = cipher
        .decrypt(
            XNonce::from_slice(&bytes[header_end..nonce_end]),
            Payload {
                msg: &bytes[nonce_end..],
                aad: header_bytes,
            },
        )
        .map_err(|_| {
            AppError::new(
                "Backup authentication failed.",
                "Não foi possível autenticar o backup.",
            )
        })?;
    Ok(Some(archive))
}

fn create_snapshot(database_path: &Path, snapshot_path: &Path) -> Result<(), AppError> {
    if is_plaintext_database(database_path).map_err(backup_error)? {
        let source = Connection::open(database_path)?;
        let mut destination = Connection::open(snapshot_path)?;
        let backup = Backup::new(&source, &mut destination)?;
        backup.run_to_completion(64, Duration::from_millis(5), None)?;
    } else {
        let source = open_encrypted_database(database_path)?;
        let mut destination = open_encrypted_database(snapshot_path)?;
        let backup = Backup::new(&source, &mut destination)?;
        backup.run_to_completion(64, Duration::from_millis(5), None)?;
    }
    Ok(())
}

fn zip_options() -> FileOptions {
    FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600)
}

fn backup_manifest() -> BackupManifest {
    BackupManifest {
        application: "com.walk.tcc-opet".to_string(),
        format_version: BACKUP_FORMAT_VERSION,
        key_version: Some(crate::encryption::ACTIVE_KEY_VERSION),
    }
}

fn validate_manifest(manifest: BackupManifest) -> Result<(), AppError> {
    if manifest.application != "com.walk.tcc-opet" || !matches!(manifest.format_version, 1 | 2) {
        return Err(AppError::new(
            "Backup manifest is not compatible with this application.",
            "O manifesto do backup não é compatível com este aplicativo.",
        ));
    }
    if manifest.format_version == BACKUP_FORMAT_VERSION
        && manifest.key_version != Some(crate::encryption::ACTIVE_KEY_VERSION)
    {
        return Err(AppError::new(
            "Backup uses an unsupported encryption key version.",
            "O backup usa uma versão de chave de criptografia não suportada.",
        ));
    }
    Ok(())
}

pub fn export_backup_with_paths(
    database_path: &Path,
    attachments_path: &Path,
    destination: &Path,
) -> Result<BackupSummary, AppError> {
    let parent = destination.parent().ok_or_else(|| {
        AppError::new(
            "Backup destination has no parent directory.",
            "O destino do backup não possui diretório pai.",
        )
    })?;
    fs::create_dir_all(parent).map_err(backup_error)?;

    let snapshot_path = parent.join(format!(".opet-snapshot-{}.db", Uuid::new_v4()));
    create_snapshot(database_path, &snapshot_path)?;
    crate::database::secure_private_file(&snapshot_path).map_err(backup_error)?;
    let temporary_destination = parent.join(format!(".opet-backup-{}.tmp", Uuid::new_v4()));

    let result = (|| -> Result<BackupSummary, AppError> {
        let file = File::create(&temporary_destination).map_err(backup_error)?;
        crate::database::secure_private_file(&temporary_destination).map_err(backup_error)?;
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(MANIFEST_ENTRY, zip_options())
            .map_err(backup_error)?;
        archive
            .write_all(&serde_json::to_vec(&backup_manifest()).map_err(backup_error)?)
            .map_err(backup_error)?;
        archive
            .start_file(DATABASE_ENTRY, zip_options())
            .map_err(backup_error)?;
        archive
            .write_all(&fs::read(&snapshot_path).map_err(backup_error)?)
            .map_err(backup_error)?;

        let mut attachment_count = 0;
        if attachments_path.exists() {
            for entry in fs::read_dir(attachments_path).map_err(backup_error)? {
                let entry = entry.map_err(backup_error)?;
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path).map_err(backup_error)?;
                if !metadata.file_type().is_file() {
                    continue;
                }
                if metadata.len() > MAX_ATTACHMENT_SIZE_BYTES {
                    return Err(AppError::new(
                        "Attachment exceeds the backup size limit.",
                        "O anexo excede o limite de tamanho do backup.",
                    ));
                }
                if attachment_count >= MAX_ATTACHMENT_COUNT {
                    return Err(AppError::new(
                        "Backup exceeds the attachment count limit.",
                        "O backup excede o limite de anexos.",
                    ));
                }
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| {
                        AppError::new(
                            "Attachment filename is invalid.",
                            "O nome do anexo é inválido.",
                        )
                    })?;
                archive
                    .start_file(format!("{ATTACHMENTS_PREFIX}{file_name}"), zip_options())
                    .map_err(backup_error)?;
                archive
                    .write_all(&fs::read(&path).map_err(backup_error)?)
                    .map_err(backup_error)?;
                attachment_count += 1;
            }
        }
        archive.finish().map_err(backup_error)?;
        activate_backup_file(&temporary_destination, destination)?;
        crate::database::secure_private_file(destination).map_err(backup_error)?;

        Ok(BackupSummary {
            path: destination.to_string_lossy().to_string(),
            attachment_count,
        })
    })();

    let _ = fs::remove_file(snapshot_path);
    let _ = fs::remove_file(temporary_destination);
    result
}

pub fn export_backup_with_passphrase(
    database_path: &Path,
    attachments_path: &Path,
    destination: &Path,
    passphrase: Option<&str>,
) -> Result<BackupSummary, AppError> {
    let parent = destination
        .parent()
        .ok_or_else(|| backup_error("Backup destination has no parent directory."))?;
    let archive_path = parent.join(format!(".opets-backup-archive-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<BackupSummary, AppError> {
        let summary = export_backup_with_paths(database_path, attachments_path, &archive_path)?;
        let encrypted =
            encrypt_backup_archive(&fs::read(&archive_path).map_err(backup_error)?, passphrase)?;
        let temporary_destination =
            parent.join(format!(".opets-backup-encrypted-{}.tmp", Uuid::new_v4()));
        fs::write(&temporary_destination, encrypted).map_err(backup_error)?;
        crate::database::secure_private_file(&temporary_destination).map_err(backup_error)?;
        activate_backup_file(&temporary_destination, destination)?;
        crate::database::secure_private_file(destination).map_err(backup_error)?;
        Ok(BackupSummary {
            path: destination.to_string_lossy().to_string(),
            attachment_count: summary.attachment_count,
        })
    })();
    let _ = fs::remove_file(archive_path);
    result
}

fn is_safe_attachment_entry(name: &str) -> bool {
    name.strip_prefix(ATTACHMENTS_PREFIX)
        .and_then(|value| Path::new(value).file_name().and_then(|file| file.to_str()))
        .map(|file_name| {
            !file_name.is_empty()
                && Path::new(file_name)
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
        })
        .unwrap_or(false)
}

fn validate_archive_entries(archive: &mut ZipArchive<File>) -> Result<Vec<usize>, AppError> {
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(AppError::new(
            "Backup contains too many entries.",
            "O backup contém arquivos demais.",
        ));
    }

    let mut names = HashSet::new();
    let mut attachment_indexes = Vec::new();
    let mut database_count = 0;
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(backup_error)?;
        let name = entry.name().to_string();
        if !names.insert(name.clone()) {
            return Err(AppError::new(
                "Backup contains duplicate entries.",
                "O backup contém arquivos duplicados.",
            ));
        }
        total_size = total_size.checked_add(entry.size()).ok_or_else(|| {
            AppError::new(
                "Backup size exceeds the allowed limit.",
                "O tamanho do backup excede o limite permitido.",
            )
        })?;
        if total_size > MAX_BACKUP_FILE_SIZE_BYTES {
            return Err(AppError::new(
                "Backup expands beyond the allowed limit.",
                "O backup descompactado excede o limite permitido.",
            ));
        }

        match name.as_str() {
            MANIFEST_ENTRY if entry.size() <= 64 * 1024 => {}
            DATABASE_ENTRY if entry.size() <= MAX_DATABASE_SIZE_BYTES => database_count += 1,
            _ if is_safe_attachment_entry(&name) && entry.size() <= MAX_ATTACHMENT_SIZE_BYTES => {
                attachment_indexes.push(index);
            }
            _ => {
                return Err(AppError::new(
                    "Backup contains an invalid or oversized entry.",
                    "O backup contém um arquivo inválido ou maior que o permitido.",
                ));
            }
        }
    }

    if database_count != 1 {
        return Err(AppError::new(
            "Backup must contain exactly one database snapshot.",
            "O backup deve conter exatamente uma cópia do banco de dados.",
        ));
    }
    if attachment_indexes.len() > MAX_ATTACHMENT_COUNT {
        return Err(AppError::new(
            "Backup exceeds the attachment count limit.",
            "O backup excede o limite de anexos.",
        ));
    }
    Ok(attachment_indexes)
}

fn validate_database_schema(connection: &Connection) -> Result<(), AppError> {
    for table in REQUIRED_TABLES {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(backup_error)?;
        if !exists {
            return Err(AppError::new(
                "Backup database does not match the OPETS schema.",
                "O banco de dados do backup não corresponde ao schema do OPETS.",
            ));
        }
    }

    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(backup_error)?;
    let mut rows = statement.query([]).map_err(backup_error)?;
    if rows.next().map_err(backup_error)?.is_some() {
        return Err(AppError::new(
            "Backup database has foreign key violations.",
            "O banco de dados do backup possui violações de chave estrangeira.",
        ));
    }
    Ok(())
}

pub fn restore_backup_with_paths(
    source: &Path,
    database_path: &Path,
    attachments_path: &Path,
) -> Result<BackupSummary, AppError> {
    let parent = database_path.parent().ok_or_else(|| {
        AppError::new(
            "Database path has no parent directory.",
            "O caminho do banco de dados não possui diretório pai.",
        )
    })?;
    let staging_path = parent.join(format!(".opet-restore-{}", Uuid::new_v4()));
    let staging_attachments = staging_path.join("attachments");
    let staging_database = staging_path.join(DATABASE_ENTRY);
    crate::database::ensure_private_dir(&staging_attachments).map_err(backup_error)?;

    let result = (|| -> Result<BackupSummary, AppError> {
        let source_metadata = fs::symlink_metadata(source).map_err(backup_error)?;
        if !source_metadata.file_type().is_file() {
            return Err(AppError::new(
                "Backup source must be a regular file.",
                "O arquivo de backup deve ser um arquivo regular.",
            ));
        }
        if source_metadata.len() > MAX_BACKUP_FILE_SIZE_BYTES {
            return Err(AppError::new(
                "Backup file exceeds the allowed size limit.",
                "O arquivo de backup excede o limite de tamanho permitido.",
            ));
        }
        let file = File::open(source).map_err(backup_error)?;
        let mut archive = ZipArchive::new(file).map_err(backup_error)?;
        let attachment_indexes = validate_archive_entries(&mut archive)?;
        let legacy_backup = match archive.by_name(MANIFEST_ENTRY) {
            Ok(manifest_entry) => {
                let manifest = serde_json::from_reader(manifest_entry).map_err(backup_error)?;
                validate_manifest(manifest)?;
                false
            }
            Err(zip::result::ZipError::FileNotFound) => true,
            Err(error) => return Err(backup_error(error)),
        };
        let mut database = archive.by_name(DATABASE_ENTRY).map_err(|_| {
            AppError::new(
                "Backup does not contain a database snapshot.",
                "O backup não contém uma cópia do banco de dados.",
            )
        })?;
        let mut database_file = File::create(&staging_database).map_err(backup_error)?;
        std::io::copy(&mut database, &mut database_file).map_err(backup_error)?;
        drop(database_file);
        drop(database);

        let mut attachment_count = 0;
        for index in attachment_indexes {
            let mut entry = archive.by_index(index).map_err(backup_error)?;
            let name = entry.name().to_string();
            let file_name = Path::new(&name)
                .file_name()
                .ok_or_else(|| backup_error("missing attachment filename"))?;
            let output_path = staging_attachments.join(file_name);
            let mut output = File::create(&output_path).map_err(backup_error)?;
            std::io::copy(&mut entry, &mut output).map_err(backup_error)?;
            drop(output);
            crate::database::secure_private_file(&output_path).map_err(backup_error)?;
            attachment_count += 1;
        }
        drop(archive);

        let plaintext_database = is_plaintext_database(&staging_database).map_err(backup_error)?;
        let validation_connection = if plaintext_database {
            Connection::open(&staging_database)?
        } else {
            open_encrypted_database(&staging_database)?
        };
        if legacy_backup || plaintext_database {
            validate_database_schema(&validation_connection)?;
        }
        run_migrations(&validation_connection)?;
        let integrity: String =
            validation_connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(AppError::new(
                "Backup database integrity check failed.",
                "A verificação de integridade do banco do backup falhou.",
            ));
        }
        validate_database_schema(&validation_connection)?;
        drop(validation_connection);

        if plaintext_database {
            let recovery_path = migrate_plaintext_database(&staging_database)?;
            let _ = fs::remove_file(recovery_path);
        }
        let encrypted_connection = open_encrypted_database(&staging_database)?;
        crate::attachment_service::migrate_legacy_attachments(
            &encrypted_connection,
            &staging_attachments,
        )?;
        drop(encrypted_connection);

        let previous_database = parent.join(format!(".opet-previous-{}.db", Uuid::new_v4()));
        let previous_attachments =
            parent.join(format!(".opet-previous-attachments-{}", Uuid::new_v4()));
        let had_database = database_path.exists();
        let had_attachments = attachments_path.exists();
        if database_path.exists() {
            fs::rename(database_path, &previous_database).map_err(backup_error)?;
        }
        if attachments_path.exists() {
            if let Err(error) = fs::rename(attachments_path, &previous_attachments) {
                if had_database {
                    let _ = fs::rename(&previous_database, database_path);
                }
                return Err(backup_error(error));
            }
        }

        let activate = (|| -> Result<(), AppError> {
            fs::rename(&staging_database, database_path).map_err(backup_error)?;
            fs::rename(&staging_attachments, attachments_path).map_err(backup_error)?;
            crate::database::secure_private_file(database_path).map_err(backup_error)?;
            crate::database::ensure_private_dir(attachments_path).map_err(backup_error)?;
            Ok(())
        })();
        if let Err(error) = activate {
            let _ = fs::remove_file(database_path);
            let _ = fs::remove_dir_all(attachments_path);
            if previous_database.exists() {
                let _ = fs::rename(&previous_database, database_path);
            }
            if previous_attachments.exists() {
                let _ = fs::rename(&previous_attachments, attachments_path);
            }
            if !had_database {
                let _ = fs::remove_file(database_path);
            }
            if !had_attachments {
                let _ = fs::remove_dir_all(attachments_path);
            }
            return Err(error);
        }

        if previous_database.exists() {
            let _ = fs::remove_file(previous_database);
        }
        if previous_attachments.exists() {
            let _ = fs::remove_dir_all(previous_attachments);
        }

        Ok(BackupSummary {
            path: source.to_string_lossy().to_string(),
            attachment_count,
        })
    })();

    let _ = fs::remove_dir_all(staging_path);
    result
}

pub fn restore_backup_with_passphrase(
    source: &Path,
    database_path: &Path,
    attachments_path: &Path,
    passphrase: Option<&str>,
) -> Result<BackupSummary, AppError> {
    let source_metadata = fs::symlink_metadata(source).map_err(backup_error)?;
    if !source_metadata.file_type().is_file() || source_metadata.len() > MAX_BACKUP_FILE_SIZE_BYTES
    {
        return Err(backup_error(
            "Backup source is invalid or exceeds the allowed size limit.",
        ));
    }
    let bytes = fs::read(source).map_err(backup_error)?;
    let Some(archive) = decrypt_backup_archive(&bytes, passphrase)? else {
        return restore_backup_with_paths(source, database_path, attachments_path);
    };
    let parent = database_path
        .parent()
        .ok_or_else(|| backup_error("Database path has no parent directory."))?;
    let temporary_archive = parent.join(format!(".opets-backup-import-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<BackupSummary, AppError> {
        fs::write(&temporary_archive, archive).map_err(backup_error)?;
        crate::database::secure_private_file(&temporary_archive).map_err(backup_error)?;
        restore_backup_with_paths(&temporary_archive, database_path, attachments_path)
    })();
    let _ = fs::remove_file(temporary_archive);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tcc-opet-{label}-{}", Uuid::new_v4()))
    }

    #[test]
    fn exports_and_restores_database_and_attachments() {
        let source_dir = temp_path("backup-source");
        let source_database = source_dir.join("database.db");
        let source_attachments = source_dir.join("database.attachments");
        fs::create_dir_all(&source_attachments).unwrap();
        let source_conn = Connection::open(&source_database).unwrap();
        run_migrations(&source_conn).unwrap();
        source_conn
            .execute(
                "UPDATE settings SET company_name = 'Assistência Backup' WHERE id = 1",
                [],
            )
            .unwrap();
        drop(source_conn);
        fs::write(source_attachments.join("photo.jpg"), b"\x89PNG\r\n\x1a\n").unwrap();

        let archive_path = source_dir.join("backup.osbkp");
        let summary =
            export_backup_with_paths(&source_database, &source_attachments, &archive_path).unwrap();
        assert_eq!(summary.attachment_count, 1);

        let restore_dir = temp_path("backup-restore");
        fs::create_dir_all(&restore_dir).unwrap();
        let restore_database = restore_dir.join("database.db");
        let restore_attachments = restore_dir.join("database.attachments");
        let restored =
            restore_backup_with_paths(&archive_path, &restore_database, &restore_attachments)
                .unwrap();
        assert_eq!(restored.attachment_count, 1);
        assert!(!restore_attachments.join("photo.jpg").exists());

        let restored_conn = open_encrypted_database(&restore_database).unwrap();
        let company_name: String = restored_conn
            .query_row(
                "SELECT company_name FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(company_name, "Assistência Backup");

        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(restore_dir);
    }

    #[test]
    fn restore_rejects_an_incompatible_manifest() {
        let temp_dir = temp_path("invalid-backup-manifest");
        fs::create_dir_all(&temp_dir).unwrap();
        let archive_path = temp_dir.join("backup.osbkp");
        let file = File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive.start_file(MANIFEST_ENTRY, zip_options()).unwrap();
        archive
            .write_all(b"{\"application\":\"invalid\",\"formatVersion\":1}")
            .unwrap();
        archive.start_file(DATABASE_ENTRY, zip_options()).unwrap();
        archive.write_all(b"not-a-database").unwrap();
        archive.finish().unwrap();

        let error = restore_backup_with_paths(
            &archive_path,
            &temp_dir.join("database.db"),
            &temp_dir.join("database.attachments"),
        )
        .unwrap_err();

        assert!(error.en.contains("manifest"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn password_protected_backup_requires_the_original_passphrase() {
        let encrypted = encrypt_backup_archive(b"backup contents", Some("senha-segura")).unwrap();

        assert!(decrypt_backup_archive(&encrypted, Some("senha-incorreta")).is_err());
        assert_eq!(
            decrypt_backup_archive(&encrypted, Some("senha-segura"))
                .unwrap()
                .unwrap(),
            b"backup contents"
        );
    }

    #[test]
    fn inspection_reports_whether_a_backup_requires_a_passphrase() {
        let encrypted_path = temp_path("encrypted-backup-inspection");
        fs::write(
            &encrypted_path,
            encrypt_backup_archive(b"backup contents", Some("senha-segura")).unwrap(),
        )
        .unwrap();
        assert!(inspect_backup(&encrypted_path).unwrap().requires_passphrase);

        let legacy_path = temp_path("legacy-backup-inspection");
        fs::write(&legacy_path, b"legacy backup contents").unwrap();
        assert!(!inspect_backup(&legacy_path).unwrap().requires_passphrase);

        let _ = fs::remove_file(encrypted_path);
        let _ = fs::remove_file(legacy_path);
    }

    #[test]
    fn restores_legacy_backup_when_its_schema_is_valid() {
        let temp_dir = temp_path("legacy-backup");
        fs::create_dir_all(&temp_dir).unwrap();
        let source_database = temp_dir.join("legacy.db");
        let source_conn = Connection::open(&source_database).unwrap();
        run_migrations(&source_conn).unwrap();
        drop(source_conn);

        let archive_path = temp_dir.join("legacy.osbkp");
        let file = File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive.start_file(DATABASE_ENTRY, zip_options()).unwrap();
        archive
            .write_all(&fs::read(&source_database).unwrap())
            .unwrap();
        archive.finish().unwrap();

        let restore_database = temp_dir.join("database.db");
        let restore_attachments = temp_dir.join("database.attachments");
        restore_backup_with_paths(&archive_path, &restore_database, &restore_attachments).unwrap();

        assert!(restore_database.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn restore_rejects_archives_with_too_many_entries() {
        let temp_dir = temp_path("oversized-backup");
        fs::create_dir_all(&temp_dir).unwrap();
        let archive_path = temp_dir.join("backup.osbkp");
        let file = File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        for index in 0..=MAX_ARCHIVE_ENTRIES {
            archive
                .start_file(format!("attachments/{index}.png"), zip_options())
                .unwrap();
            archive.write_all(b"\x89PNG\r\n\x1a\n").unwrap();
        }
        archive.finish().unwrap();

        let error = restore_backup_with_paths(
            &archive_path,
            &temp_dir.join("database.db"),
            &temp_dir.join("database.attachments"),
        )
        .unwrap_err();

        assert!(error.en.contains("too many entries"));
        let _ = fs::remove_dir_all(temp_dir);
    }
}
