use crate::database::run_migrations;
use crate::error::AppError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
#[cfg(test)]
use std::path::PathBuf;
use std::path::{Component, Path};
use uuid::Uuid;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DATABASE_ENTRY: &str = "database.db";
const ATTACHMENTS_PREFIX: &str = "attachments/";
const MANIFEST_ENTRY: &str = "opets-backup.json";
const BACKUP_FORMAT_VERSION: u8 = 1;
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub path: String,
    pub attachment_count: usize,
}

fn backup_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        format!("Backup operation failed: {error}"),
        format!("A operação de backup falhou: {error}"),
    )
}

fn create_snapshot(database_path: &Path, snapshot_path: &Path) -> Result<(), AppError> {
    let connection = Connection::open(database_path)?;
    connection.execute(
        "VACUUM INTO ?1",
        params![snapshot_path.to_string_lossy().to_string()],
    )?;
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
    }
}

fn validate_manifest(manifest: BackupManifest) -> Result<(), AppError> {
    if manifest.application != "com.walk.tcc-opet"
        || manifest.format_version != BACKUP_FORMAT_VERSION
    {
        return Err(AppError::new(
            "Backup manifest is not compatible with this application.",
            "O manifesto do backup não é compatível com este aplicativo.",
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
        // rename atomically overwrites destination on the same filesystem (POSIX).
        // On Windows, rename may fail if destination exists; fall back to copy+remove temp.
        fs::rename(&temporary_destination, destination)
            .or_else(|_| {
                fs::copy(&temporary_destination, destination)
                    .and_then(|_| fs::remove_file(&temporary_destination))
            })
            .map_err(backup_error)?;
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
            crate::attachment_service::validate_attachment_file(&output_path)?;
            crate::database::secure_private_file(&output_path).map_err(backup_error)?;
            attachment_count += 1;
        }
        drop(archive);

        let validation_connection = Connection::open(&staging_database)?;
        if legacy_backup {
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
        assert_eq!(
            fs::read(restore_attachments.join("photo.jpg")).unwrap(),
            b"\x89PNG\r\n\x1a\n"
        );

        let restored_conn = Connection::open(&restore_database).unwrap();
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
