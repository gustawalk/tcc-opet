use crate::models::customer::Customer;
use crate::repositories::customer_repo::CustomerRepository;
use tauri::command;

#[command]
pub fn create_customer(
    name: String,
    phone: String,
    email: String,
    address: String,
) -> Result<String, String> {
    let customer = Customer::new(name, phone, email, address);
    CustomerRepository::create(&customer).map_err(|e| e.to_string())?;
    Ok(customer.id)
}

#[command]
pub fn get_customer(id: String) -> Result<Option<Customer>, String> {
    CustomerRepository::get_by_id(&id).map_err(|e| e.to_string())
}

#[command]
pub fn get_customers() -> Result<Vec<Customer>, String> {
    CustomerRepository::get_all().map_err(|e| e.to_string())
}

#[command]
pub fn update_customer(
    id: String,
    name: String,
    phone: String,
    email: String,
    address: String,
) -> Result<(), String> {
    let mut customer = CustomerRepository::get_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Customer not found".to_string())?;

    customer.name = name;
    customer.phone = phone;
    customer.email = email;
    customer.address = address;

    CustomerRepository::update(&customer).map_err(|e| e.to_string())
}

#[command]
pub fn delete_customer(id: String) -> Result<(), String> {
    CustomerRepository::delete(&id).map_err(|e| e.to_string())
}
