use crate::database::LanIdempotencyLookup;
use crate::error::AppError;
use rusqlite::{params, Connection, TransactionBehavior};
use serde_json::Value;

#[derive(Default)]
pub(crate) struct LanIdempotencyService;

impl LanIdempotencyService {
    pub(crate) fn execute<F>(
        &self,
        conn: &mut Connection,
        device_id: &str,
        idempotency_key: &str,
        route: &str,
        body: &[u8],
        operation: F,
    ) -> Result<Value, AppError>
    where
        F: FnOnce() -> Result<Value, AppError>,
    {
        if idempotency_key.trim().is_empty() {
            return Err(idempotency_error(
                "Idempotency key is required for mutating requests.",
                "A chave de idempotência é obrigatória para alterações.",
            ));
        }
        let body_hash = blake3::hash(body).to_hex().to_string();
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO lan_idempotency_records
             (device_id, idempotency_key, route, body_hash, status)
             VALUES (?1, ?2, ?3, ?4, 'in_progress')",
            params![device_id, idempotency_key, route, body_hash],
        )?;
        if inserted == 0 {
            let lookup = crate::database::lookup_lan_idempotency(
                &transaction,
                device_id,
                idempotency_key,
                route,
                &body_hash,
            )?;
            transaction.commit()?;
            return match lookup {
                LanIdempotencyLookup::Replay(response) => {
                    serde_json::from_str(&response).map_err(|error| {
                        idempotency_error(
                            format!("Stored idempotency response is invalid: {error}"),
                            "A resposta armazenada para esta operação é inválida.",
                        )
                    })
                }
                LanIdempotencyLookup::BodyConflict => Err(idempotency_error(
                    "Idempotency key was already used for a different request.",
                    "A chave de idempotência já foi usada em outra requisição.",
                )),
                LanIdempotencyLookup::InProgress => Err(idempotency_error(
                    "A request with this idempotency key is still in progress.",
                    "Uma requisição com esta chave de idempotência ainda está em andamento.",
                )),
                LanIdempotencyLookup::Missing => Err(idempotency_error(
                    "Idempotency reservation disappeared during request processing.",
                    "A reserva de idempotência desapareceu durante o processamento.",
                )),
            };
        }
        transaction.commit()?;

        match operation() {
            Ok(response) => {
                let response_json = serde_json::to_string(&response).map_err(|error| {
                    idempotency_error(
                        format!("Failed to serialize idempotent response: {error}"),
                        "Não foi possível armazenar a resposta da operação.",
                    )
                })?;
                conn.execute(
                    "UPDATE lan_idempotency_records
                     SET status = 'completed', response_json = ?1, updated_at = CURRENT_TIMESTAMP
                     WHERE device_id = ?2 AND idempotency_key = ?3",
                    params![response_json, device_id, idempotency_key],
                )?;
                Ok(response)
            }
            Err(error) => {
                conn.execute(
                    "DELETE FROM lan_idempotency_records
                     WHERE device_id = ?1 AND idempotency_key = ?2 AND status = 'in_progress'",
                    params![device_id, idempotency_key],
                )?;
                Err(error)
            }
        }
    }
}

fn idempotency_error(en: impl Into<String>, pt: impl Into<String>) -> AppError {
    AppError::new(en, pt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::mpsc;
    use std::thread;

    fn insert_device(conn: &Connection) {
        conn.execute(
            "INSERT INTO lan_devices (id, name, token_fingerprint, app_version)
             VALUES ('device-1', 'Balcao', 'fingerprint', '0.3.2')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn lan_idempotency_replays_completed_response_without_repeating_operation() {
        let mut conn = crate::test_helpers::setup_db();
        insert_device(&conn);
        let service = LanIdempotencyService;
        let first = service
            .execute(
                &mut conn,
                "device-1",
                "key-1",
                "/orders",
                br#"{"name":"OS"}"#,
                || Ok(json!({ "id": "order-1" })),
            )
            .unwrap();
        let replay = service
            .execute(
                &mut conn,
                "device-1",
                "key-1",
                "/orders",
                br#"{"name":"OS"}"#,
                || panic!("duplicate operation must not execute"),
            )
            .unwrap();

        assert_eq!(first, json!({ "id": "order-1" }));
        assert_eq!(replay, first);
    }

    #[test]
    fn lan_idempotency_rejects_same_key_with_different_body() {
        let mut conn = crate::test_helpers::setup_db();
        insert_device(&conn);
        let service = LanIdempotencyService;
        service
            .execute(&mut conn, "device-1", "key-1", "/orders", b"body-a", || {
                Ok(json!({ "id": "order-1" }))
            })
            .unwrap();

        let error = service
            .execute(&mut conn, "device-1", "key-1", "/orders", b"body-b", || {
                Ok(json!({ "id": "order-2" }))
            })
            .unwrap_err();

        assert_eq!(
            error.pt,
            "A chave de idempotência já foi usada em outra requisição."
        );
    }

    #[test]
    fn lan_idempotency_failure_removes_reservation_for_retry() {
        let mut conn = crate::test_helpers::setup_db();
        insert_device(&conn);
        let service = LanIdempotencyService;
        let failure = service.execute(&mut conn, "device-1", "key-1", "/orders", b"body", || {
            Err(AppError::new("Operation failed.", "A operação falhou."))
        });
        assert!(failure.is_err());

        let retry = service
            .execute(&mut conn, "device-1", "key-1", "/orders", b"body", || {
                Ok(json!({ "id": "order-1" }))
            })
            .unwrap();

        assert_eq!(retry, json!({ "id": "order-1" }));
    }

    #[test]
    fn lan_idempotency_concurrent_duplicate_does_not_execute_twice() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("lan-idempotency.db");
        crate::database::initialize_storage_at(&database_path, false).unwrap();
        let setup = crate::database::open_encrypted_database(&database_path).unwrap();
        insert_device(&setup);
        drop(setup);
        let (reserved_tx, reserved_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        let first_path = database_path.clone();
        let first = thread::spawn(move || {
            let mut conn = crate::database::open_encrypted_database(&first_path).unwrap();
            LanIdempotencyService
                .execute(&mut conn, "device-1", "key-1", "/orders", b"body", || {
                    reserved_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                    Ok(json!({ "id": "order-1" }))
                })
                .unwrap()
        });
        reserved_rx.recv().unwrap();
        let mut duplicate_conn = crate::database::open_encrypted_database(&database_path).unwrap();

        let duplicate = LanIdempotencyService.execute(
            &mut duplicate_conn,
            "device-1",
            "key-1",
            "/orders",
            b"body",
            || panic!("concurrent duplicate operation must not execute"),
        );
        finish_tx.send(()).unwrap();

        assert_eq!(
            duplicate.unwrap_err().pt,
            "Uma requisição com esta chave de idempotência ainda está em andamento."
        );
        assert_eq!(first.join().unwrap(), json!({ "id": "order-1" }));
    }
}
