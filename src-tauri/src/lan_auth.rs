#![cfg_attr(not(test), allow(dead_code))]

use crate::error::AppError;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingCode {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairedDevice {
    pub device_id: String,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthenticatedDevice {
    pub id: String,
    pub name: String,
}

#[derive(Default)]
pub(crate) struct LanAuthService {
    pairing_codes: Mutex<HashMap<String, DateTime<Utc>>>,
}

impl LanAuthService {
    pub(crate) fn create_pairing_code(
        &self,
        ttl_seconds: i64,
        now: DateTime<Utc>,
    ) -> Result<PairingCode, AppError> {
        if ttl_seconds <= 0 {
            return Err(auth_error(
                "Pairing code lifetime must be positive.",
                "A validade do código de pareamento deve ser positiva.",
            ));
        }
        let code = format!("{:06}", OsRng.next_u32() % 1_000_000);
        let expires_at = now + Duration::seconds(ttl_seconds);
        self.pairing_codes
            .lock()
            .map_err(|_| {
                auth_error(
                    "Pairing service is unavailable.",
                    "O serviço de pareamento está indisponível.",
                )
            })?
            .insert(code.clone(), expires_at);
        Ok(PairingCode { code, expires_at })
    }

    pub(crate) fn pair_device(
        &self,
        conn: &Connection,
        code: &str,
        device_name: &str,
        client_version: &str,
        host_version: &str,
        now: DateTime<Utc>,
    ) -> Result<PairedDevice, AppError> {
        if client_version != host_version {
            return Err(auth_error(
                format!("Client build {client_version} does not match host build {host_version}."),
                format!("A versão do cliente {client_version} não corresponde à versão do host {host_version}."),
            ));
        }
        if device_name.trim().is_empty() {
            return Err(auth_error(
                "Device name is required.",
                "O nome do dispositivo é obrigatório.",
            ));
        }

        let expires_at = self
            .pairing_codes
            .lock()
            .map_err(|_| {
                auth_error(
                    "Pairing service is unavailable.",
                    "O serviço de pareamento está indisponível.",
                )
            })?
            .remove(code)
            .ok_or_else(|| {
                auth_error(
                    "Pairing code is invalid.",
                    "O código de pareamento é inválido.",
                )
            })?;
        if expires_at < now {
            return Err(auth_error(
                "Pairing code has expired.",
                "O código de pareamento expirou.",
            ));
        }

        let mut token_bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut token_bytes);
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);
        let fingerprint = token_fingerprint(&token);
        let device_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO lan_devices
             (id, name, token_fingerprint, app_version, created_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                device_id,
                device_name.trim(),
                fingerprint,
                client_version,
                now.to_rfc3339()
            ],
        )?;
        Ok(PairedDevice { device_id, token })
    }

    pub(crate) fn authenticate(
        &self,
        conn: &Connection,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<AuthenticatedDevice, AppError> {
        let fingerprint = token_fingerprint(token);
        let device = conn.query_row(
            "SELECT id, name FROM lan_devices WHERE token_fingerprint = ?1",
            [fingerprint],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        let (id, name) = match device {
            Ok(device) => device,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(auth_error(
                    "Device token is invalid.",
                    "O token do dispositivo é inválido.",
                ))
            }
            Err(error) => return Err(error.into()),
        };
        if crate::database::lan_device_is_revoked(conn, &id)? {
            return Err(auth_error(
                "Device access has been revoked.",
                "O acesso deste dispositivo foi revogado.",
            ));
        }
        conn.execute(
            "UPDATE lan_devices SET last_seen_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), id],
        )?;
        Ok(AuthenticatedDevice { id, name })
    }
}

fn token_fingerprint(token: &str) -> String {
    blake3::hash(token.as_bytes()).to_hex().to_string()
}

fn auth_error(en: impl Into<String>, pt: impl Into<String>) -> AppError {
    AppError::new(en, pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service_and_database() -> (LanAuthService, Connection) {
        (LanAuthService::default(), crate::test_helpers::setup_db())
    }

    fn pair(service: &LanAuthService, conn: &Connection, now: DateTime<Utc>) -> PairedDevice {
        let code = service.create_pairing_code(60, now).unwrap();
        service
            .pair_device(conn, &code.code, "Balcao 2", "0.3.2", "0.3.2", now)
            .unwrap()
    }

    #[test]
    fn lan_auth_pairs_and_authenticates_a_device_without_storing_raw_token() {
        let (service, conn) = service_and_database();
        let now = Utc::now();
        let paired = pair(&service, &conn, now);

        let authenticated = service
            .authenticate(&conn, &paired.token, now + Duration::seconds(5))
            .unwrap();

        assert_eq!(authenticated.id, paired.device_id);
        assert_eq!(authenticated.name, "Balcao 2");
        let stored: (String, String) = conn
            .query_row(
                "SELECT token_fingerprint, last_seen_at FROM lan_devices WHERE id = ?1",
                [&paired.device_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored.0, token_fingerprint(&paired.token));
        assert_ne!(stored.0, paired.token);
        assert_eq!(stored.1, (now + Duration::seconds(5)).to_rfc3339());
    }

    #[test]
    fn lan_auth_rejects_invalid_pairing_code() {
        let (service, conn) = service_and_database();

        let error = service
            .pair_device(&conn, "000000", "Balcao", "0.3.2", "0.3.2", Utc::now())
            .unwrap_err();

        assert_eq!(error.pt, "O código de pareamento é inválido.");
    }

    #[test]
    fn lan_auth_rejects_expired_pairing_code() {
        let (service, conn) = service_and_database();
        let now = Utc::now();
        let code = service.create_pairing_code(10, now).unwrap();

        let error = service
            .pair_device(
                &conn,
                &code.code,
                "Balcao",
                "0.3.2",
                "0.3.2",
                now + Duration::seconds(11),
            )
            .unwrap_err();

        assert_eq!(error.pt, "O código de pareamento expirou.");
    }

    #[test]
    fn lan_auth_rejects_different_client_build() {
        let (service, conn) = service_and_database();
        let now = Utc::now();
        let code = service.create_pairing_code(60, now).unwrap();

        let error = service
            .pair_device(&conn, &code.code, "Balcao", "0.3.1", "0.3.2", now)
            .unwrap_err();

        assert!(error.pt.contains("0.3.1"));
        assert!(error.pt.contains("0.3.2"));
    }

    #[test]
    fn lan_auth_rejects_invalid_token() {
        let (service, conn) = service_and_database();

        let error = service
            .authenticate(&conn, "invalid-token", Utc::now())
            .unwrap_err();

        assert_eq!(error.pt, "O token do dispositivo é inválido.");
    }

    #[test]
    fn lan_auth_rejects_revoked_token() {
        let (service, conn) = service_and_database();
        let now = Utc::now();
        let paired = pair(&service, &conn, now);
        conn.execute(
            "UPDATE lan_devices SET revoked_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), paired.device_id],
        )
        .unwrap();

        let error = service.authenticate(&conn, &paired.token, now).unwrap_err();

        assert_eq!(error.pt, "O acesso deste dispositivo foi revogado.");
    }
}
