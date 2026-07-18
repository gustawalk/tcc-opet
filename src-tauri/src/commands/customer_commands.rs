use crate::error::AppError;
use crate::models::customer::Customer;
use crate::repositories::customer_repo::CustomerRepository;
use tauri::command;

fn require_existing_customer(customer: Option<Customer>) -> Result<Customer, AppError> {
    customer.ok_or_else(|| crate::error::not_found("Customer", "Cliente"))
}

#[command]
pub fn create_customer(
    name: String,
    phone: String,
    email: String,
    address: String,
) -> Result<String, AppError> {
    let customer = Customer::new(name, phone, email, address);
    CustomerRepository::create(&customer)?;
    Ok(customer.id)
}

#[command]
pub fn get_customer(id: String) -> Result<Option<Customer>, AppError> {
    Ok(CustomerRepository::get_by_id(&id)?)
}

#[command]
pub fn get_customers() -> Result<Vec<Customer>, AppError> {
    Ok(CustomerRepository::get_all()?)
}

#[command]
pub fn update_customer(
    id: String,
    name: String,
    phone: String,
    email: String,
    address: String,
) -> Result<(), AppError> {
    let mut customer = require_existing_customer(CustomerRepository::get_by_id(&id)?)?;

    customer.name = name;
    customer.phone = phone;
    customer.email = email;
    customer.address = address;

    Ok(CustomerRepository::update(&customer)?)
}

#[command]
pub fn delete_customer(id: String) -> Result<(), AppError> {
    Ok(CustomerRepository::delete(&id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_existing_customer_returns_not_found_error() {
        let err = require_existing_customer(None).unwrap_err();

        assert_eq!(err.en, "Customer not found.");
        assert_eq!(err.pt, "Cliente não encontrado(a).");
    }
}
