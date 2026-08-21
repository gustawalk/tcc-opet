use crate::error::AppError;
use serde_json::Value;
use tauri::command;

#[command]
pub fn lan_remote_command(
    operation: String,
    payload: Value,
    idempotency_key: Option<String>,
) -> Result<Value, AppError> {
    crate::lan_client::remote_command(&operation, payload, idempotency_key.as_deref())
}
