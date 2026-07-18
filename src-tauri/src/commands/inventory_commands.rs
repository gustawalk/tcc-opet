use crate::error::AppError;
use crate::models::inventory_item::InventoryItem;
use crate::models::inventory_movement::InventoryMovement;
use crate::repositories::inventory_repo::InventoryRepository;
use tauri::command;

fn require_existing_inventory_item(item: Option<InventoryItem>) -> Result<InventoryItem, AppError> {
    item.ok_or_else(|| crate::error::not_found("Inventory item", "Item de inventário"))
}

#[command]
pub fn create_inventory_item(
    name: String,
    description: String,
    r#type: String,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: f64,
    sale_price: f64,
) -> Result<String, AppError> {
    let item = InventoryItem::new(
        name,
        description,
        r#type,
        min_quantity,
        current_quantity,
        cost_price,
        sale_price,
    );
    InventoryRepository::create(&item)?;
    Ok(item.id)
}

#[command]
pub fn get_inventory_item(id: String) -> Result<Option<InventoryItem>, AppError> {
    Ok(InventoryRepository::get_by_id(&id)?)
}

#[command]
pub fn get_inventory_items() -> Result<Vec<InventoryItem>, AppError> {
    Ok(InventoryRepository::get_all()?)
}

#[command]
#[allow(clippy::too_many_arguments)]
pub fn update_inventory_item(
    id: String,
    name: String,
    description: String,
    r#type: String,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: f64,
    sale_price: f64,
) -> Result<(), AppError> {
    let mut item = require_existing_inventory_item(InventoryRepository::get_by_id(&id)?)?;

    item.name = name;
    item.description = description;
    item.r#type = r#type;
    item.min_quantity = min_quantity;
    item.current_quantity = current_quantity;
    item.cost_price = cost_price;
    item.sale_price = sale_price;

    Ok(InventoryRepository::update(&item)?)
}

#[command]
pub fn delete_inventory_item(id: String) -> Result<(), AppError> {
    Ok(InventoryRepository::delete(&id)?)
}

#[command]
pub fn restock_inventory_item(id: String, quantity: i32) -> Result<(), AppError> {
    Ok(InventoryRepository::add_stock(&id, quantity)?)
}

#[command]
pub fn remove_stock_inventory_item(id: String, quantity: i32) -> Result<(), AppError> {
    Ok(InventoryRepository::remove_stock(&id, quantity)?)
}

#[command]
pub fn get_inventory_movements(id: String) -> Result<Vec<InventoryMovement>, AppError> {
    Ok(InventoryRepository::get_movements(&id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_existing_inventory_item_returns_not_found_error() {
        let err = require_existing_inventory_item(None).unwrap_err();

        assert_eq!(err.en, "Inventory item not found.");
        assert_eq!(err.pt, "Item de inventário não encontrado(a).");
    }
}
