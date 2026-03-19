use crate::models::inventory_item::InventoryItem;
use crate::repositories::inventory_repo::InventoryRepository;
use tauri::command;

#[command]
pub fn create_inventory_item(
    name: String,
    description: String,
    r#type: String,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: f64,
    sale_price: f64,
) -> Result<String, String> {
    let item = InventoryItem::new(
        name,
        description,
        r#type,
        min_quantity,
        current_quantity,
        cost_price,
        sale_price,
    );
    InventoryRepository::create(&item).map_err(|e: rusqlite::Error| e.to_string())?;
    Ok(item.id)
}

#[command]
pub fn get_inventory_item(id: String) -> Result<Option<InventoryItem>, String> {
    InventoryRepository::get_by_id(&id).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn get_inventory_items() -> Result<Vec<InventoryItem>, String> {
    InventoryRepository::get_all().map_err(|e: rusqlite::Error| e.to_string())
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
) -> Result<(), String> {
    let mut item = InventoryRepository::get_by_id(&id)
        .map_err(|e: rusqlite::Error| e.to_string())?
        .ok_or_else(|| "Inventory item not found".to_string())?;

    item.name = name;
    item.description = description;
    item.r#type = r#type;
    item.min_quantity = min_quantity;
    item.current_quantity = current_quantity;
    item.cost_price = cost_price;
    item.sale_price = sale_price;

    InventoryRepository::update(&item).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn delete_inventory_item(id: String) -> Result<(), String> {
    InventoryRepository::delete(&id).map_err(|e: rusqlite::Error| e.to_string())
}
