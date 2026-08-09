use crate::models::customer::Customer;
use crate::models::service_order::ServiceOrder;
use crate::models::service_order_attachment::ServiceOrderAttachment;
use crate::repositories::customer_repo::CustomerRepository;
use crate::repositories::service_order_attachment_repo::ServiceOrderAttachmentRepository;
use crate::repositories::service_order_repo::ServiceOrderRepository;
use crate::test_helpers::TestStorage;
use rusqlite::Connection;
use std::fs;

fn table_count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

#[test]
fn clean_storage_initialization_is_encrypted_and_idempotent() {
    let storage = TestStorage::new(false);
    assert!(storage.database_path.exists());
    assert!(!crate::database::is_plaintext_database(&storage.database_path).unwrap());
    let mut metadata_path = storage.database_path.clone();
    metadata_path.set_extension("encryption.json");
    assert!(metadata_path.exists());

    let conn = storage.open();
    assert_eq!(table_count(&conn, "settings"), 1);
    assert_eq!(table_count(&conn, "customers"), 0);
    assert_eq!(table_count(&conn, "financial_snapshots"), 1);
    drop(conn);

    crate::database::initialize_storage_at(&storage.database_path, false).unwrap();
    let conn = storage.open();
    assert_eq!(table_count(&conn, "settings"), 1);
    assert_eq!(table_count(&conn, "financial_snapshots"), 1);
}

#[test]
fn seeded_storage_does_not_duplicate_demo_data_on_restart() {
    let storage = TestStorage::new(true);
    let conn = storage.open();
    let initial = (
        table_count(&conn, "users"),
        table_count(&conn, "customers"),
        table_count(&conn, "inventory_items"),
        table_count(&conn, "service_orders"),
        table_count(&conn, "checklist_templates"),
    );
    drop(conn);
    assert!(initial.0 > 0);
    assert!(initial.1 > 0);
    assert!(initial.2 > 0);
    assert!(initial.3 > 0);
    assert!(initial.4 > 0);

    crate::database::initialize_storage_at(&storage.database_path, true).unwrap();
    let conn = storage.open();
    assert_eq!(
        initial,
        (
            table_count(&conn, "users"),
            table_count(&conn, "customers"),
            table_count(&conn, "inventory_items"),
            table_count(&conn, "service_orders"),
            table_count(&conn, "checklist_templates"),
        )
    );
}

#[test]
fn tampered_storage_metadata_fails_without_damaging_database() {
    let storage = TestStorage::new(false);
    let conn = storage.open();
    conn.execute(
        "INSERT INTO customers (id, name) VALUES ('safe-customer', 'Cliente Seguro')",
        [],
    )
    .unwrap();
    conn.execute_batch("DROP TABLE inventory_movements;")
        .unwrap();
    drop(conn);
    let mut metadata_path = storage.database_path.clone();
    metadata_path.set_extension("encryption.json");
    fs::write(
        &metadata_path,
        br#"{"format_version":1,"key_version":1,"authentication":"tampered"}"#,
    )
    .unwrap();

    assert!(crate::database::initialize_storage_at(&storage.database_path, false).is_err());
    let conn = storage.open();
    assert_eq!(
        conn.query_row(
            "SELECT name FROM customers WHERE id = 'safe-customer'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap(),
        "Cliente Seguro"
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'inventory_movements'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        0
    );
}

#[test]
fn tampered_metadata_does_not_migrate_plaintext_database() {
    let root = tempfile::tempdir().unwrap();
    let database_path = root.path().join("tampered-legacy.db");
    let conn = Connection::open(&database_path).unwrap();
    crate::database::run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO customers (id, name) VALUES ('legacy-safe', 'Legado Seguro')",
        [],
    )
    .unwrap();
    drop(conn);
    let mut metadata_path = database_path.clone();
    metadata_path.set_extension("encryption.json");
    fs::write(
        metadata_path,
        br#"{"format_version":1,"key_version":1,"authentication":"tampered"}"#,
    )
    .unwrap();

    assert!(crate::database::initialize_storage_at(&database_path, false).is_err());
    assert!(crate::database::is_plaintext_database(&database_path).unwrap());
    let plaintext = Connection::open(database_path).unwrap();
    assert_eq!(
        plaintext
            .query_row(
                "SELECT name FROM customers WHERE id = 'legacy-safe'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "Legado Seguro"
    );
}

#[test]
fn plaintext_database_and_attachment_migrate_end_to_end() {
    let root = tempfile::tempdir().unwrap();
    let database_path = root.path().join("legacy.db");
    let attachments_dir = crate::database::attachments_dir_for(&database_path);
    let conn = Connection::open(&database_path).unwrap();
    crate::database::run_migrations(&conn).unwrap();
    let customer = Customer::new(
        "Cliente Legado".to_string(),
        "41933334444".to_string(),
        "legado@example.com".to_string(),
        "Rua Legada".to_string(),
    );
    CustomerRepository::create_with_conn(&conn, &customer).unwrap();
    let mut order = ServiceOrder::new(
        customer.id,
        "Equipamento legado".to_string(),
        "Migração E2E".to_string(),
    );
    ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
    let bytes = include_bytes!("../icons/32x32.png");
    let attachment = ServiceOrderAttachment::new(
        order.id.clone(),
        "legado.png".to_string(),
        "legacy-storage-file".to_string(),
        "image/png".to_string(),
        bytes.len() as i64,
    );
    ServiceOrderAttachmentRepository::create_with_conn(&conn, &attachment).unwrap();
    fs::create_dir_all(&attachments_dir).unwrap();
    fs::write(attachments_dir.join(&attachment.storage_name), bytes).unwrap();
    drop(conn);

    crate::database::initialize_storage_at(&database_path, false).unwrap();

    assert!(!crate::database::is_plaintext_database(&database_path).unwrap());
    let encrypted = crate::database::open_encrypted_database(&database_path).unwrap();
    let migrated =
        ServiceOrderAttachmentRepository::get_by_id_with_conn(&encrypted, &attachment.id)
            .unwrap()
            .unwrap();
    assert!(
        crate::attachment_service::read_attachment_as_data_url_with_paths(
            &migrated,
            &attachments_dir,
        )
        .unwrap()
        .starts_with("data:image/png;base64,")
    );
    assert!(!fs::read(attachments_dir.join(&migrated.storage_name))
        .unwrap()
        .starts_with(b"\x89PNG"));

    let plaintext = Connection::open(&database_path).unwrap();
    assert!(plaintext
        .query_row("SELECT COUNT(*) FROM customers", [], |row| row
            .get::<_, i64>(0))
        .is_err());
    assert!(fs::read_dir(root.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("opets-pre-encryption-v0.1.0-")
        }));
}

#[test]
fn startup_recovers_attachment_staged_before_database_commit() {
    let storage = TestStorage::new(false);
    let attachments_dir = crate::database::attachments_dir_for(&storage.database_path);
    let conn = storage.open();
    let customer = Customer::new(
        "Cliente Recovery".to_string(),
        "41989898989".to_string(),
        "recovery@example.com".to_string(),
        "Rua Recovery".to_string(),
    );
    CustomerRepository::create_with_conn(&conn, &customer).unwrap();
    let mut order = ServiceOrder::new(
        customer.id,
        "Equipamento recovery".to_string(),
        "Recovery E2E".to_string(),
    );
    ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
    let source = storage.database_path.with_extension("fixture.png");
    fs::write(&source, include_bytes!("../icons/32x32.png")).unwrap();
    let attachment = crate::attachment_service::add_attachment_with_paths(
        &conn,
        &order.id,
        &source,
        &attachments_dir,
    )
    .unwrap();
    let original = attachments_dir.join(&attachment.storage_name);
    let staged = attachments_dir.join(format!(".delete-{}", attachment.storage_name));
    fs::rename(&original, &staged).unwrap();
    drop(conn);

    crate::database::initialize_storage_at(&storage.database_path, false).unwrap();

    assert!(original.is_file());
    assert!(!staged.exists());
    assert!(
        crate::attachment_service::read_attachment_as_data_url_with_paths(
            &attachment,
            &attachments_dir,
        )
        .unwrap()
        .starts_with("data:image/png;base64,")
    );
}
