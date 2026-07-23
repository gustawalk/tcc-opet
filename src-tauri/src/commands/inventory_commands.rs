use crate::error::AppError;
use crate::models::inventory_item::InventoryItem;
use crate::models::inventory_movement::InventoryMovement;
use crate::repositories::inventory_repo::InventoryInsights;
use crate::repositories::inventory_repo::InventoryRepository;
use tauri::command;

fn require_existing_inventory_item(item: Option<InventoryItem>) -> Result<InventoryItem, AppError> {
    item.ok_or_else(|| crate::error::not_found("Inventory item", "Item de inventário"))
}

fn validate_inventory_values(
    item_type: &str,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: f64,
    sale_price: f64,
) -> Result<(), AppError> {
    if !matches!(item_type, "part" | "service") {
        return Err(crate::error::business_error(
            "Inventory item type must be part or service.",
            "O tipo do item deve ser peça ou serviço.",
        ));
    }
    if min_quantity < 0 || current_quantity < 0 || cost_price < 0.0 || sale_price < 0.0 {
        return Err(crate::error::business_error(
            "Inventory quantities and prices cannot be negative.",
            "Quantidades e preços do inventário não podem ser negativos.",
        ));
    }
    Ok(())
}

fn validate_stock_change(id: &str, quantity: i32, removing: bool) -> Result<(), AppError> {
    if quantity <= 0 {
        return Err(crate::error::business_error(
            "Stock quantity must be greater than zero.",
            "A quantidade deve ser maior que zero.",
        ));
    }

    let item = require_existing_inventory_item(InventoryRepository::get_by_id(id)?)?;
    if item.r#type != "part" {
        return Err(crate::error::business_error(
            "Only parts can have stock movements.",
            "Apenas peças podem ter movimentações de estoque.",
        ));
    }
    if removing && item.current_quantity < quantity {
        return Err(crate::error::business_error(
            "Insufficient stock for this removal.",
            "Estoque insuficiente para esta saída.",
        ));
    }
    Ok(())
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
    supplier_name: Option<String>,
) -> Result<InventoryItem, AppError> {
    validate_inventory_values(
        &r#type,
        min_quantity,
        current_quantity,
        cost_price,
        sale_price,
    )?;
    let mut item = InventoryItem::new(
        name,
        description,
        r#type,
        min_quantity,
        current_quantity,
        cost_price,
        sale_price,
    );
    item.supplier_name = supplier_name.filter(|name| !name.trim().is_empty());
    InventoryRepository::create(&item)?;
    Ok(item)
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
    supplier_name: Option<String>,
) -> Result<(), AppError> {
    validate_inventory_values(
        &r#type,
        min_quantity,
        current_quantity,
        cost_price,
        sale_price,
    )?;
    let mut item = require_existing_inventory_item(InventoryRepository::get_by_id(&id)?)?;

    item.name = name;
    item.description = description;
    item.r#type = r#type;
    item.min_quantity = min_quantity;
    item.cost_price = cost_price;
    item.sale_price = sale_price;
    item.supplier_name = supplier_name.filter(|name| !name.trim().is_empty());

    Ok(InventoryRepository::update(&item)?)
}

#[command]
pub fn delete_inventory_item(id: String) -> Result<(), AppError> {
    Ok(InventoryRepository::delete(&id)?)
}

#[command]
pub fn restock_inventory_item(
    id: String,
    quantity: i32,
    unit_cost: Option<f64>,
    reason: Option<String>,
) -> Result<(), AppError> {
    validate_stock_change(&id, quantity, false)?;
    if unit_cost.is_some_and(|cost| cost <= 0.0) {
        return Err(crate::error::business_error(
            "Restock unit cost must be greater than zero.",
            "O custo unitário da reposição deve ser maior que zero.",
        ));
    }
    Ok(InventoryRepository::add_stock_with_details(
        &id,
        quantity,
        unit_cost,
        reason.filter(|value| !value.trim().is_empty()),
    )?)
}

#[command]
pub fn remove_stock_inventory_item(id: String, quantity: i32) -> Result<(), AppError> {
    validate_stock_change(&id, quantity, true)?;
    Ok(InventoryRepository::remove_stock(&id, quantity)?)
}

#[command]
pub fn get_inventory_movements(id: String) -> Result<Vec<InventoryMovement>, AppError> {
    require_existing_inventory_item(InventoryRepository::get_by_id(&id)?)?;
    Ok(InventoryRepository::get_movements(&id)?)
}

#[command]
pub fn get_inventory_insights(inactive_days: Option<i32>) -> Result<InventoryInsights, AppError> {
    let inactive_days = inactive_days.unwrap_or(90);
    if inactive_days < 0 {
        return Err(crate::error::business_error(
            "Inactive days cannot be negative.",
            "Os dias de inatividade não podem ser negativos.",
        ));
    }
    Ok(InventoryRepository::get_insights(inactive_days)?)
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

    #[test]
    fn rejects_zero_stock_change() {
        let err = validate_stock_change("item-1", 0, false).unwrap_err();

        assert_eq!(err.pt, "A quantidade deve ser maior que zero.");
    }

    #[test]
    fn rejects_negative_inventory_values() {
        let err = validate_inventory_values("part", 0, 0, -1.0, 10.0).unwrap_err();

        assert_eq!(
            err.pt,
            "Quantidades e preços do inventário não podem ser negativos."
        );
    }
}
