#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod attachment_service;
mod backup_service;
mod commands;
mod database;
mod error;
mod models;
mod pdf_service;
mod repositories;
mod seeds;
#[cfg(test)]
mod test_helpers;

use dotenv::dotenv;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            commands::inventory_commands::create_inventory_item,
            commands::inventory_commands::get_inventory_item,
            commands::inventory_commands::get_inventory_items,
            commands::inventory_commands::update_inventory_item,
            commands::inventory_commands::delete_inventory_item,
            commands::inventory_commands::restock_inventory_item,
            commands::inventory_commands::remove_stock_inventory_item,
            commands::inventory_commands::get_inventory_movements,
            commands::inventory_commands::get_inventory_insights,
            commands::service_order_commands::create_service_order,
            commands::service_order_commands::create_full_service_order,
            commands::service_order_commands::get_service_order,
            commands::service_order_commands::get_service_orders,
            commands::service_order_commands::get_service_orders_by_customer_id,
            commands::service_order_commands::get_service_order_events,
            commands::service_order_commands::update_service_order,
            commands::service_order_commands::transition_service_order_status,
            commands::service_order_commands::save_service_order_edit,
            commands::service_order_commands::delete_service_order,
            commands::service_order_commands::add_part_to_service_order,
            commands::service_order_commands::remove_part_from_service_order,
            commands::service_order_commands::update_service_order_part_quantity,
            commands::service_order_commands::get_service_order_parts,
            commands::dashboard_commands::get_dashboard_data,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            commands::settings_commands::reset_database,
            commands::settings_commands::select_company_logo,
            commands::settings_commands::get_system_info,
            commands::settings_commands::check_for_updates,
            commands::settings_commands::export_backup,
            commands::settings_commands::restore_backup,
            commands::checklist_commands::create_checklist_template,
            commands::checklist_commands::get_checklist_templates,
            commands::checklist_commands::get_checklist_template_items,
            commands::checklist_commands::update_checklist_template,
            commands::checklist_commands::delete_checklist_template,
            commands::checklist_commands::save_service_order_checklist,
            commands::checklist_commands::get_service_order_checklist,
            commands::attachment_commands::select_service_order_attachments,
            commands::attachment_commands::select_pending_service_order_attachments,
            commands::attachment_commands::attach_pending_service_order_attachments,
            commands::attachment_commands::discard_pending_service_order_attachments,
            commands::attachment_commands::get_service_order_attachments,
            commands::attachment_commands::delete_service_order_attachment,
            commands::attachment_commands::read_service_order_attachment,
            commands::attachment_commands::export_service_order_attachment,
            commands::pdf_commands::preview_service_order_pdf,
            commands::pdf_commands::save_pdf_preview,
            commands::pdf_commands::discard_pdf_preview,
            commands::report_commands::get_financial_report,
            commands::report_commands::export_financial_report_csv,
            commands::report_commands::preview_financial_report_pdf,
        ])
        .setup(|app| {
            let _ = dotenv();
            // Startup must fail visibly when the local data store cannot initialize.
            database::init_db(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
