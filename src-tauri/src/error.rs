use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub en: String,
    pub pt: String,
}

impl AppError {
    pub fn new(en: impl Into<String>, pt: impl Into<String>) -> Self {
        Self {
            en: en.into(),
            pt: pt.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pt)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        match &e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::new("Record not found.", "Registro não encontrado.")
            }
            rusqlite::Error::ToSqlConversionFailure(_) => {
                AppError::new("Invalid data format.", "Formato de dados inválido.")
            }
            rusqlite::Error::InvalidParameterName(_) => {
                AppError::new("Invalid parameter name.", "Parâmetro inválido.")
            }
            rusqlite::Error::InvalidColumnType(..) => {
                AppError::new("Invalid column type.", "Tipo de coluna inválido.")
            }
            _ => AppError::new(
                format!("Database error: {}", e),
                format!("Erro no banco de dados: {}", e),
            ),
        }
    }
}

pub fn not_found(entity: &str, entity_pt: &str) -> AppError {
    AppError::new(
        format!("{} not found.", entity),
        format!("{} não encontrado(a).", entity_pt),
    )
}

pub fn business_error(en: impl Into<String>, pt: impl Into<String>) -> AppError {
    AppError::new(en, pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_app_error_with_translations() {
        let err = AppError::new("User not found.", "Usuário não encontrado.");

        assert_eq!(err.en, "User not found.");
        assert_eq!(err.pt, "Usuário não encontrado.");
    }

    #[test]
    fn display_uses_portuguese_message() {
        let err = AppError::new("File not found.", "Arquivo não encontrado.");

        assert_eq!(err.to_string(), "Arquivo não encontrado.");
    }

    #[test]
    fn maps_query_returned_no_rows_error() {
        let err = AppError::from(rusqlite::Error::QueryReturnedNoRows);

        assert_eq!(err.en, "Record not found.");
        assert_eq!(err.pt, "Registro não encontrado.");
    }

    #[test]
    fn builds_not_found_error_with_bilingual_messages() {
        let err = not_found("User", "Usuário");

        assert_eq!(err.en, "User not found.");
        assert_eq!(err.pt, "Usuário não encontrado(a).");
    }
}
