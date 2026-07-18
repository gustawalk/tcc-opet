use crate::error::AppError;
use crate::models::user::User;
use crate::repositories::user_repo::UserRepository;
use tauri::command;

fn require_existing_user(user: Option<User>) -> Result<User, AppError> {
    user.ok_or_else(|| crate::error::not_found("User", "Usuário"))
}

#[command]
pub fn create_user(name: String, email: String, phone: Option<String>, cpf: Option<String>, join_date: Option<String>) -> Result<String, AppError> {
    let mut user = User::new(name, email);
    user.phone = phone;
    user.cpf = cpf;
    user.join_date = join_date;
    UserRepository::create(&user)?;
    Ok(user.id)
}

#[command]
pub fn get_user(id: String) -> Result<Option<User>, AppError> {
    Ok(UserRepository::get_by_id(&id)?)
}

#[command]
pub fn get_user_by_email(email: String) -> Result<Option<User>, AppError> {
    Ok(UserRepository::get_by_email(&email)?)
}

#[command]
pub fn get_users() -> Result<Vec<User>, AppError> {
    Ok(UserRepository::get_all()?)
}

#[command]
pub fn update_user(id: String, name: String, email: String, phone: Option<String>, cpf: Option<String>, join_date: Option<String>) -> Result<(), AppError> {
    let mut user = require_existing_user(UserRepository::get_by_id(&id)?)?;

    user.name = name;
    user.email = email;
    user.phone = phone;
    user.cpf = cpf;
    user.join_date = join_date;

    Ok(UserRepository::update(&user)?)
}

#[command]
pub fn delete_user(id: String) -> Result<(), AppError> {
    Ok(UserRepository::delete(&id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_existing_user_returns_not_found_error() {
        let err = require_existing_user(None).unwrap_err();

        assert_eq!(err.en, "User not found.");
        assert_eq!(err.pt, "Usuário não encontrado(a).");
    }
}
