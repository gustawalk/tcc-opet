use crate::commands::{
    attachment_commands, checklist_commands, customer_commands, dashboard_commands,
    inventory_commands, report_commands, service_order_commands, settings_commands, user_commands,
};
use crate::models::checklist::ChecklistItem;
use crate::test_helpers::setup_global_backend;
use rusqlite::Connection;
use std::fs;
use std::sync::mpsc;
use std::time::Duration;

fn create_part(name: &str, stock: i32, cost: i64, price: i64) -> String {
    let item = inventory_commands::create_inventory_item(
        name.to_string(),
        format!("{name} para teste E2E"),
        "part".to_string(),
        2,
        0,
        cost,
        price,
        Some("Fornecedor E2E".to_string()),
    )
    .unwrap();
    inventory_commands::restock_inventory_item(
        item.id.clone(),
        stock,
        Some(cost),
        Some("Carga inicial E2E".to_string()),
    )
    .unwrap();
    item.id
}

fn create_service(name: &str, price: i64) -> String {
    inventory_commands::create_inventory_item(
        name.to_string(),
        format!("{name} para teste E2E"),
        "service".to_string(),
        0,
        0,
        0,
        price,
        None,
    )
    .unwrap()
    .id
}

fn new_customer_action(name: &str) -> service_order_commands::CustomerAction {
    service_order_commands::CustomerAction::New {
        name: name.to_string(),
        phone: "41999999999".to_string(),
        email: format!("{}@example.com", name.to_lowercase().replace(' ', ".")),
        address: "Rua dos Testes, 100".to_string(),
    }
}

fn count_stored_attachments(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }
    fs::read_dir(path)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .count()
}

#[test]
fn complete_service_order_lifecycle_keeps_stock_dashboard_and_reports_consistent() {
    let _backend = setup_global_backend();
    let technician_id = user_commands::create_user(
        "Técnica E2E".to_string(),
        "tecnica.e2e@example.com".to_string(),
        Some("41988887777".to_string()),
        Some("12345678901".to_string()),
        Some("2026-01-10".to_string()),
    )
    .unwrap();
    let part_id = create_part("Tela E2E", 5, 4_000, 10_000);
    let service_id = create_service("Mão de obra E2E", 15_000);

    let order_id = service_order_commands::create_full_service_order(
        service_order_commands::CreateFullServiceOrderRequest {
            customer_action: new_customer_action("Cliente Fluxo"),
            user_id: Some(technician_id.clone()),
            equipment: "Notebook E2E".to_string(),
            imei: Some("E2E-IMEI-001".to_string()),
            description: "Fluxo completo".to_string(),
            discount_basis_points: None,
            parts: vec![
                service_order_commands::CreateServiceOrderPartRequest {
                    inventory_item_id: part_id.clone(),
                    quantity: 2,
                },
                service_order_commands::CreateServiceOrderPartRequest {
                    inventory_item_id: service_id,
                    quantity: 1,
                },
            ],
            checklist_items: vec![
                service_order_commands::ChecklistItemInput {
                    label: "Inspeção visual".to_string(),
                    checked: false,
                },
                service_order_commands::ChecklistItemInput {
                    label: "Teste funcional".to_string(),
                    checked: false,
                },
            ],
            attachment_token: None,
        },
    )
    .unwrap();

    let order = service_order_commands::get_service_order(order_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(order.status, "Orçamento");
    assert_eq!(order.total_price, Some(35_000));
    assert_eq!(
        inventory_commands::get_inventory_item(part_id.clone())
            .unwrap()
            .unwrap()
            .current_quantity,
        3
    );

    let checklist = checklist_commands::get_service_order_checklist(order_id.clone()).unwrap();
    checklist_commands::save_service_order_checklist(
        order_id.clone(),
        checklist
            .into_iter()
            .map(|item| ChecklistItem {
                checked: true,
                ..item
            })
            .collect(),
    )
    .unwrap();
    service_order_commands::transition_service_order_status(
        order_id.clone(),
        "Em Manutenção".to_string(),
        false,
    )
    .unwrap();
    service_order_commands::transition_service_order_status(
        order_id.clone(),
        "Finalizada".to_string(),
        false,
    )
    .unwrap();

    let dashboard = dashboard_commands::get_dashboard_data().unwrap();
    assert_eq!(dashboard.summary.total_revenue, 35_000);
    assert_eq!(dashboard.summary.estimated_gross_profit, 27_000);
    assert_eq!(dashboard.summary.active_orders_count, 0);

    let report = report_commands::get_financial_report(
        Some("2000-01-01".to_string()),
        Some("2100-12-31".to_string()),
        Some(technician_id),
        Some("quantity".to_string()),
        Some(10),
    )
    .unwrap();
    assert_eq!(report.total_revenue, 35_000);
    assert_eq!(report.total_cost, 8_000);
    assert_eq!(report.finalized_orders, 1);

    service_order_commands::transition_service_order_status(
        order_id.clone(),
        "Cancelada".to_string(),
        true,
    )
    .unwrap();
    assert_eq!(
        inventory_commands::get_inventory_item(part_id.clone())
            .unwrap()
            .unwrap()
            .current_quantity,
        5
    );

    service_order_commands::transition_service_order_status(
        order_id.clone(),
        "Em Manutenção".to_string(),
        false,
    )
    .unwrap();
    assert_eq!(
        inventory_commands::get_inventory_item(part_id.clone())
            .unwrap()
            .unwrap()
            .current_quantity,
        3
    );
    service_order_commands::transition_service_order_status(
        order_id.clone(),
        "Cancelada".to_string(),
        true,
    )
    .unwrap();
    service_order_commands::delete_service_order(order_id.clone()).unwrap();

    assert!(service_order_commands::get_service_order(order_id.clone())
        .unwrap()
        .is_none());
    assert!(service_order_commands::get_service_orders()
        .unwrap()
        .iter()
        .all(|order| order.id != order_id));
    assert_eq!(
        dashboard_commands::get_dashboard_data()
            .unwrap()
            .summary
            .active_orders_count,
        0
    );
    assert!(service_order_commands::get_service_order_events(order_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == "status_changed"));
}

#[test]
fn failed_full_creation_rolls_back_customer_update_stock_and_order() {
    let _backend = setup_global_backend();
    let customer_id = customer_commands::create_customer(
        "Cliente Existente".to_string(),
        "41911112222".to_string(),
        "original@example.com".to_string(),
        "Endereço original".to_string(),
    )
    .unwrap();
    let part_id = create_part("Bateria E2E", 1, 3_000, 9_000);

    let result = service_order_commands::create_full_service_order(
        service_order_commands::CreateFullServiceOrderRequest {
            customer_action: service_order_commands::CustomerAction::Existing {
                id: customer_id.clone(),
                update: Some(service_order_commands::CustomerUpdate {
                    phone: None,
                    email: Some("alterado@example.com".to_string()),
                    address: None,
                }),
            },
            user_id: None,
            equipment: "Celular E2E".to_string(),
            imei: None,
            description: "Deve falhar".to_string(),
            discount_basis_points: None,
            parts: vec![service_order_commands::CreateServiceOrderPartRequest {
                inventory_item_id: part_id.clone(),
                quantity: 2,
            }],
            checklist_items: vec![service_order_commands::ChecklistItemInput {
                label: "Não deve persistir".to_string(),
                checked: false,
            }],
            attachment_token: None,
        },
    );

    assert!(result.is_err());
    assert_eq!(
        customer_commands::get_customer(customer_id)
            .unwrap()
            .unwrap()
            .email,
        "original@example.com"
    );
    assert_eq!(
        inventory_commands::get_inventory_item(part_id)
            .unwrap()
            .unwrap()
            .current_quantity,
        1
    );
    assert!(service_order_commands::get_service_orders()
        .unwrap()
        .is_empty());
}

#[test]
fn failed_attachment_batch_rolls_back_database_and_created_files() {
    let backend = setup_global_backend();
    let part_id = create_part("Conector E2E", 3, 1_000, 3_000);
    let valid = backend.temp_file(
        "valid-attachment",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    let invalid = backend.temp_file("invalid-attachment", "png", b"not a png");
    let token = "attachment-rollback-token".to_string();
    attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .insert(token.clone(), vec![valid, invalid]);

    let result = service_order_commands::create_full_service_order(
        service_order_commands::CreateFullServiceOrderRequest {
            customer_action: new_customer_action("Cliente Attachment Rollback"),
            user_id: None,
            equipment: "Equipamento E2E".to_string(),
            imei: None,
            description: "Attachment rollback".to_string(),
            discount_basis_points: None,
            parts: vec![service_order_commands::CreateServiceOrderPartRequest {
                inventory_item_id: part_id.clone(),
                quantity: 1,
            }],
            checklist_items: vec![],
            attachment_token: Some(token.clone()),
        },
    );

    assert!(result.is_err());
    assert!(service_order_commands::get_service_orders()
        .unwrap()
        .is_empty());
    assert_eq!(
        inventory_commands::get_inventory_item(part_id)
            .unwrap()
            .unwrap()
            .current_quantity,
        3
    );
    assert_eq!(count_stored_attachments(&backend.attachments_dir()), 0);
    assert!(attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .contains_key(&token));
}

#[test]
fn successful_attachment_batch_is_readable_and_consumes_its_token() {
    let backend = setup_global_backend();
    let attachment_path = backend.temp_file(
        "successful-attachment",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    let token = "attachment-success-token".to_string();
    attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .insert(token.clone(), vec![attachment_path]);

    let order_id = service_order_commands::create_full_service_order(
        service_order_commands::CreateFullServiceOrderRequest {
            customer_action: new_customer_action("Cliente Attachment Success"),
            user_id: None,
            equipment: "Equipamento com anexo".to_string(),
            imei: None,
            description: "Attachment success".to_string(),
            discount_basis_points: None,
            parts: vec![],
            checklist_items: vec![],
            attachment_token: Some(token.clone()),
        },
    )
    .unwrap();

    let attachments = attachment_commands::get_service_order_attachments(order_id).unwrap();
    assert_eq!(attachments.len(), 1);
    assert!(
        attachment_commands::read_service_order_attachment(attachments[0].id.clone())
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    assert_eq!(count_stored_attachments(&backend.attachments_dir()), 1);
    assert!(!attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .contains_key(&token));
}

#[test]
fn attaching_pending_batch_is_atomic_and_retains_token_after_failure() {
    let backend = setup_global_backend();
    let customer_id = customer_commands::create_customer(
        "Cliente Anexo Pendente".to_string(),
        "41922223333".to_string(),
        "pendente@example.com".to_string(),
        "Rua Pendente".to_string(),
    )
    .unwrap();
    let order_id = service_order_commands::create_service_order(
        customer_id,
        Some("Cliente Anexo Pendente".to_string()),
        None,
        "Equipamento pendente".to_string(),
        None,
        "Teste de lote pendente".to_string(),
        None,
    )
    .unwrap();
    let valid = backend.temp_file("pending-valid", "png", include_bytes!("../icons/32x32.png"));
    let invalid = backend.temp_file("pending-invalid", "png", b"invalid image");
    let token = "pending-atomic-token".to_string();
    attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .insert(token.clone(), vec![valid, invalid]);

    assert!(
        attachment_commands::attach_pending_service_order_attachments(
            order_id.clone(),
            token.clone(),
        )
        .is_err()
    );
    assert!(attachment_commands::get_service_order_attachments(order_id)
        .unwrap()
        .is_empty());
    assert_eq!(count_stored_attachments(&backend.attachments_dir()), 0);
    assert!(attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .contains_key(&token));
}

#[test]
fn pending_attachment_reservation_prevents_replay_and_restores_failed_selection() {
    let _backend = setup_global_backend();
    let token = "reservation-token".to_string();
    attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .insert(token.clone(), vec!["selected.png".into()]);

    let reservation = attachment_commands::reserve_pending_attachment_selection(&token).unwrap();
    assert!(attachment_commands::reserve_pending_attachment_selection(&token).is_err());
    drop(reservation);
    assert!(attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .contains_key(&token));

    let mut reservation =
        attachment_commands::reserve_pending_attachment_selection(&token).unwrap();
    reservation.commit();
    drop(reservation);
    assert!(!attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .contains_key(&token));
}

#[test]
fn exclusive_storage_operation_waits_for_active_database_connections() {
    let _backend = setup_global_backend();
    let connection = crate::database::get_db().unwrap();
    let (sender, receiver) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let _guard = crate::database::exclusive_storage_guard().unwrap();
        sender.send(()).unwrap();
    });

    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
    drop(connection);
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    waiter.join().unwrap();
}

#[test]
fn exclusive_storage_operation_can_close_and_reopen_the_shared_connection() {
    let backend = setup_global_backend();
    assert!(crate::database::shared_connection_is_open());

    let guard = crate::database::exclusive_storage_guard().unwrap();
    crate::database::close_shared_connection(backend.database_path(), &guard).unwrap();
    assert!(!crate::database::shared_connection_is_open());
    drop(guard);

    let connection = crate::database::get_db().unwrap();
    drop(connection);
    assert!(crate::database::shared_connection_is_open());
}

#[test]
fn attachment_delete_failure_preserves_metadata_and_storage_entry() {
    let backend = setup_global_backend();
    let attachment_path = backend.temp_file(
        "delete-failure",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    let customer_id = customer_commands::create_customer(
        "Cliente Exclusão".to_string(),
        "41944445555".to_string(),
        "exclusao@example.com".to_string(),
        "Rua Exclusão".to_string(),
    )
    .unwrap();
    let order_id = service_order_commands::create_service_order(
        customer_id,
        Some("Cliente Exclusão".to_string()),
        None,
        "Equipamento exclusão".to_string(),
        None,
        "Falha de exclusão".to_string(),
        None,
    )
    .unwrap();
    let attachment =
        crate::attachment_service::add_attachment(&order_id, &attachment_path).unwrap();
    let stored_path = backend.attachments_dir().join(&attachment.storage_name);
    fs::remove_file(&stored_path).unwrap();
    fs::create_dir(&stored_path).unwrap();

    assert!(attachment_commands::delete_service_order_attachment(attachment.id).is_err());
    assert_eq!(
        attachment_commands::get_service_order_attachments(order_id)
            .unwrap()
            .len(),
        1
    );
    assert!(stored_path.is_dir());
}

#[test]
fn attachment_delete_database_failure_restores_staged_file_and_metadata() {
    let backend = setup_global_backend();
    let attachment_path = backend.temp_file(
        "delete-compensation",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    let customer_id = customer_commands::create_customer(
        "Cliente Compensação".to_string(),
        "41912121212".to_string(),
        "compensacao@example.com".to_string(),
        "Rua Compensação".to_string(),
    )
    .unwrap();
    let order_id = service_order_commands::create_service_order(
        customer_id,
        Some("Cliente Compensação".to_string()),
        None,
        "Equipamento compensação".to_string(),
        None,
        "Compensação de anexo".to_string(),
        None,
    )
    .unwrap();
    let attachment =
        crate::attachment_service::add_attachment(&order_id, &attachment_path).unwrap();
    let stored_path = backend.attachments_dir().join(&attachment.storage_name);
    let conn = crate::database::get_db().unwrap();
    conn.execute_batch(
        "CREATE TRIGGER fail_attachment_removed
         BEFORE INSERT ON service_order_events
         WHEN NEW.event_type = 'attachment_removed'
         BEGIN SELECT RAISE(ABORT, 'forced attachment event failure'); END;",
    )
    .unwrap();
    drop(conn);

    assert!(attachment_commands::delete_service_order_attachment(attachment.id.clone()).is_err());
    assert!(stored_path.is_file());
    assert_eq!(
        attachment_commands::get_service_order_attachments(order_id)
            .unwrap()
            .len(),
        1
    );
    assert!(
        attachment_commands::read_service_order_attachment(attachment.id)
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
}

#[test]
fn encrypted_backup_round_trip_restores_commands_and_attachments() {
    let backend = setup_global_backend();
    let attachment_path = backend.temp_file(
        "backup-attachment",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    let token = "backup-attachment-token".to_string();
    attachment_commands::PENDING_ATTACHMENT_SELECTIONS
        .lock()
        .unwrap()
        .insert(token.clone(), vec![attachment_path]);
    let order_id = service_order_commands::create_full_service_order(
        service_order_commands::CreateFullServiceOrderRequest {
            customer_action: new_customer_action("Cliente Backup E2E"),
            user_id: None,
            equipment: "Equipamento Backup".to_string(),
            imei: None,
            description: "Backup E2E".to_string(),
            discount_basis_points: None,
            parts: vec![],
            checklist_items: vec![],
            attachment_token: Some(token),
        },
    )
    .unwrap();
    let attachment_id = attachment_commands::get_service_order_attachments(order_id.clone())
        .unwrap()[0]
        .id
        .clone();
    let backup_path = backend.temp_path("round-trip.osbkp");
    settings_commands::export_backup(
        backup_path.to_string_lossy().into_owned(),
        Some("senha-e2e".to_string()),
    )
    .unwrap();

    let conn = crate::database::get_db().unwrap();
    settings_commands::reset_database_with_conn(&conn).unwrap();
    drop(conn);
    fs::remove_dir_all(backend.attachments_dir()).unwrap();
    assert!(service_order_commands::get_service_order(order_id.clone())
        .unwrap()
        .is_none());

    settings_commands::restore_backup(
        backup_path.to_string_lossy().into_owned(),
        Some("senha-e2e".to_string()),
    )
    .unwrap();
    assert!(!crate::database::shared_connection_is_open());
    assert!(service_order_commands::get_service_order(order_id)
        .unwrap()
        .is_some());
    assert!(
        attachment_commands::read_service_order_attachment(attachment_id)
            .unwrap()
            .starts_with("data:image/png;base64,")
    );

    let plaintext = Connection::open(backend.database_path()).unwrap();
    assert!(plaintext
        .query_row("SELECT COUNT(*) FROM customers", [], |row| row
            .get::<_, i64>(0))
        .is_err());
}

#[test]
fn invalid_backup_passphrase_does_not_replace_current_data() {
    let backend = setup_global_backend();
    customer_commands::create_customer(
        "Cliente Antes do Backup".to_string(),
        "41900000000".to_string(),
        "antes@example.com".to_string(),
        "Rua Antes".to_string(),
    )
    .unwrap();
    let backup_path = backend.temp_path("wrong-passphrase.osbkp");
    settings_commands::export_backup(
        backup_path.to_string_lossy().into_owned(),
        Some("senha-correta".to_string()),
    )
    .unwrap();
    let current_customer_id = customer_commands::create_customer(
        "Cliente Atual".to_string(),
        "41911111111".to_string(),
        "atual@example.com".to_string(),
        "Rua Atual".to_string(),
    )
    .unwrap();
    assert!(crate::database::shared_connection_is_open());

    assert!(settings_commands::restore_backup(
        backup_path.to_string_lossy().into_owned(),
        Some("senha-incorreta".to_string()),
    )
    .is_err());
    assert!(crate::database::shared_connection_is_open());
    assert!(customer_commands::get_customer(current_customer_id)
        .unwrap()
        .is_some());
}

#[test]
fn corrupted_backup_does_not_replace_current_data() {
    let backend = setup_global_backend();
    let customer_id = customer_commands::create_customer(
        "Cliente Protegido".to_string(),
        "41966667777".to_string(),
        "protegido@example.com".to_string(),
        "Rua Protegida".to_string(),
    )
    .unwrap();
    let corrupted = backend.temp_file("corrupted-backup", "osbkp", b"not a backup");
    assert!(crate::database::shared_connection_is_open());

    assert!(
        settings_commands::restore_backup(corrupted.to_string_lossy().into_owned(), None,).is_err()
    );
    assert!(crate::database::shared_connection_is_open());
    assert!(customer_commands::get_customer(customer_id)
        .unwrap()
        .is_some());
}

#[test]
fn reset_removes_business_data_and_attachment_storage() {
    let backend = setup_global_backend();
    let customer_id = customer_commands::create_customer(
        "Cliente Reset".to_string(),
        "41977778888".to_string(),
        "reset@example.com".to_string(),
        "Rua Reset".to_string(),
    )
    .unwrap();
    let order_id = service_order_commands::create_service_order(
        customer_id,
        Some("Cliente Reset".to_string()),
        None,
        "Equipamento Reset".to_string(),
        None,
        "Reset E2E".to_string(),
        None,
    )
    .unwrap();
    let attachment_path = backend.temp_file(
        "reset-attachment",
        "png",
        include_bytes!("../icons/32x32.png"),
    );
    crate::attachment_service::add_attachment(&order_id, &attachment_path).unwrap();
    assert!(backend.attachments_dir().exists());

    settings_commands::reset_database_data().unwrap();

    assert!(customer_commands::get_customers().unwrap().is_empty());
    assert!(service_order_commands::get_service_orders()
        .unwrap()
        .is_empty());
    assert!(!backend.attachments_dir().exists());
    assert_eq!(settings_commands::get_settings().unwrap().id, 1);
}
