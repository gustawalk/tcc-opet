use crate::error::AppError;
use crate::models::inventory_item::InventoryItem;
use crate::models::inventory_movement::InventoryMovement;
use crate::page::Page;
use crate::repositories::inventory_repo::{
    InventoryInsights, InventoryRepository, InventorySummary,
};
use base64::Engine;
use tauri::command;

const MAX_INVENTORY_PHOTO_SIZE_BYTES: usize = 1024 * 1024;

fn validate_photo_data_url(photo_data_url: Option<String>) -> Result<Option<String>, AppError> {
    let Some(photo_data_url) = photo_data_url.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let encoded = photo_data_url
        .strip_prefix("data:")
        .and_then(|value| value.split_once(";base64,"))
        .filter(|(mime, _)| matches!(*mime, "image/png" | "image/jpeg" | "image/webp"))
        .map(|(_, encoded)| encoded)
        .ok_or_else(|| {
            crate::error::business_error(
                "Inventory photo must be a PNG, JPEG, or WEBP data URL.",
                "A foto do item deve ser uma imagem PNG, JPEG ou WEBP válida.",
            )
        })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            crate::error::business_error(
                "Inventory photo has invalid base64 data.",
                "A foto do item possui dados inválidos.",
            )
        })?;
    if bytes.len() > MAX_INVENTORY_PHOTO_SIZE_BYTES {
        return Err(crate::error::business_error(
            "Inventory photo exceeds the 1 MB limit.",
            "A foto do item excede o limite de 1 MB.",
        ));
    }
    let mime = infer::get(&bytes)
        .map(|kind| kind.mime_type())
        .filter(|mime| matches!(*mime, "image/png" | "image/jpeg" | "image/webp"))
        .ok_or_else(|| {
            crate::error::business_error(
                "Inventory photo contents are not a supported image.",
                "O conteúdo da foto não é uma imagem aceita.",
            )
        })?;
    Ok(Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}

fn require_existing_inventory_item(item: Option<InventoryItem>) -> Result<InventoryItem, AppError> {
    item.ok_or_else(|| crate::error::not_found("Inventory item", "Item de inventário"))
}

fn validate_inventory_values(
    item_type: &str,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: i64,
    sale_price: i64,
) -> Result<(), AppError> {
    if !matches!(item_type, "part" | "service" | "item") {
        return Err(crate::error::business_error(
            "Inventory item type must be part, service, or item.",
            "O tipo do item deve ser peça, serviço ou item.",
        ));
    }
    if min_quantity < 0 || current_quantity < 0 || cost_price < 0 || sale_price < 0 {
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
    if !item.tracks_stock {
        return Err(crate::error::business_error(
            "Only stock-controlled items can have stock movements.",
            "Apenas itens com estoque controlado podem ter movimentações.",
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
#[allow(clippy::too_many_arguments)]
pub fn create_inventory_item(
    name: String,
    description: String,
    r#type: String,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: i64,
    sale_price: i64,
    supplier_name: Option<String>,
    tracks_stock: Option<bool>,
    photo_data_url: Option<String>,
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
    item.tracks_stock = match item.r#type.as_str() {
        "part" => true,
        "service" => false,
        "item" => tracks_stock.unwrap_or(false),
        _ => unreachable!("validated inventory type"),
    };
    item.photo_data_url = validate_photo_data_url(photo_data_url)?;
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

const INVENTORY_PAGE_DEFAULT_LIMIT: u32 = 200;

#[command]
pub fn get_inventory_items_page(
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    item_type: Option<String>,
) -> Result<Page<InventoryItem>, AppError> {
    let conn = crate::database::get_db()?;
    let limit = limit.unwrap_or(INVENTORY_PAGE_DEFAULT_LIMIT).clamp(1, 1000);
    let offset = offset.unwrap_or(0);
    let search = search.unwrap_or_default();
    let item_type = item_type
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_lowercase());
    if let Some(value) = &item_type {
        if !matches!(value.as_str(), "part" | "service" | "item") {
            return Err(crate::error::business_error(
                "Inventory item type must be part, service, or item.",
                "O tipo do item deve ser peça, serviço ou item.",
            ));
        }
    }
    let items = InventoryRepository::get_page_with_conn(
        &conn,
        limit,
        offset,
        &search,
        item_type.as_deref(),
    )?;
    let total = InventoryRepository::count_all_with_conn(&conn, &search, item_type.as_deref())?;
    Ok(Page { items, total })
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
    cost_price: i64,
    sale_price: i64,
    supplier_name: Option<String>,
    tracks_stock: Option<bool>,
    photo_data_url: Option<String>,
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
    item.tracks_stock = match item.r#type.as_str() {
        "part" => true,
        "service" => false,
        "item" => tracks_stock.unwrap_or(false),
        _ => unreachable!("validated inventory type"),
    };
    item.photo_data_url = validate_photo_data_url(photo_data_url)?;

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
    unit_cost: Option<i64>,
    reason: Option<String>,
) -> Result<(), AppError> {
    validate_stock_change(&id, quantity, false)?;
    if unit_cost.is_some_and(|cost| cost <= 0) {
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

#[command]
pub fn get_inventory_summary() -> Result<InventorySummary, AppError> {
    Ok(InventoryRepository::get_summary()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_normalizes_supported_inventory_photo() {
        let photo = validate_photo_data_url(Some("data:image/png;base64,iVBORw0KGgo=".to_string()))
            .unwrap()
            .unwrap();
        assert_eq!(photo, "data:image/png;base64,iVBORw0KGgo=");
    }

    #[test]
    fn normalizes_inventory_photo_mime_from_its_contents() {
        let photo =
            validate_photo_data_url(Some("data:image/jpeg;base64,iVBORw0KGgo=".to_string()))
                .unwrap()
                .unwrap();
        assert_eq!(photo, "data:image/png;base64,iVBORw0KGgo=");
    }

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
        let err = validate_inventory_values("part", 0, 0, -1, 1_000).unwrap_err();

        assert_eq!(
            err.pt,
            "Quantidades e preços do inventário não podem ser negativos."
        );
    }

    #[test]
    fn rejects_invalid_inventory_types() {
        assert!(validate_inventory_values("other", 0, 0, 100, 100).is_err());
    }
}
