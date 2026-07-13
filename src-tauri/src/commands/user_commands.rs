use crate::models::user::User;
use crate::repositories::user_repo::UserRepository;
use tauri::command;

#[command]
pub fn create_user(name: String, email: String, phone: Option<String>, cpf: Option<String>, join_date: Option<String>) -> Result<String, String> {
    let mut user = User::new(name, email);
    user.phone = phone;
    user.cpf = cpf;
    user.join_date = join_date;
    UserRepository::create(&user).map_err(|e| e.to_string())?;
    Ok(user.id)
}

#[command]
pub fn get_user(id: String) -> Result<Option<User>, String> {
    UserRepository::get_by_id(&id).map_err(|e| e.to_string())
}

#[command]
pub fn get_user_by_email(email: String) -> Result<Option<User>, String> {
    UserRepository::get_by_email(&email).map_err(|e| e.to_string())
}

#[command]
pub fn get_users() -> Result<Vec<User>, String> {
    UserRepository::get_all().map_err(|e| e.to_string())
}

#[command]
pub fn update_user(id: String, name: String, email: String, phone: Option<String>, cpf: Option<String>, join_date: Option<String>) -> Result<(), String> {
    let mut user = UserRepository::get_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "User not found".to_string())?;

    user.name = name;
    user.email = email;
    user.phone = phone;
    user.cpf = cpf;
    user.join_date = join_date;

    UserRepository::update(&user).map_err(|e| e.to_string())
}

#[command]
pub fn delete_user(id: String) -> Result<(), String> {
    UserRepository::delete(&id).map_err(|e| e.to_string())
}
