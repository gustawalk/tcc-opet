#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod database;
mod models;
mod repositories;
mod seeds;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::customer_commands::create_customer,
            commands::customer_commands::get_customer,
            commands::customer_commands::get_customers,
            commands::customer_commands::update_customer,
            commands::customer_commands::delete_customer,
            commands::user_commands::create_user,
            commands::user_commands::get_user,
            commands::user_commands::get_user_by_email,
            commands::user_commands::get_users,
            commands::user_commands::update_user,
            commands::user_commands::delete_user,
            commands::user_commands::reset_user_password,
            commands::inventory_commands::create_inventory_item,
            commands::inventory_commands::get_inventory_item,
            commands::inventory_commands::get_inventory_items,
            commands::inventory_commands::update_inventory_item,
            commands::inventory_commands::delete_inventory_item,
            commands::service_order_commands::create_service_order,
            commands::service_order_commands::get_service_order,
            commands::service_order_commands::get_service_orders,
            commands::service_order_commands::get_service_orders_by_customer_id,
            commands::service_order_commands::update_service_order,
            commands::service_order_commands::delete_service_order,
            commands::service_order_commands::add_part_to_service_order,
            commands::service_order_commands::remove_part_from_service_order,
            commands::service_order_commands::get_service_order_parts,
            commands::dashboard_commands::get_dashboard_data,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            commands::settings_commands::reset_database,
            commands::checklist_commands::create_checklist_template,
            commands::checklist_commands::get_checklist_templates,
            commands::checklist_commands::get_checklist_template_items,
            commands::checklist_commands::update_checklist_template,
            commands::checklist_commands::delete_checklist_template,
            commands::checklist_commands::save_service_order_checklist,
            commands::checklist_commands::get_service_order_checklist,
        ])
        .setup(|_app| {
            // Initialize the database when the app starts
            let _ = database::init_db();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
