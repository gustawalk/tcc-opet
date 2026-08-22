#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

macro_rules! register_commands {
    ($builder:expr $(, $extra:path)* $(,)?) => {{
        use $crate::commands::facade;
        $builder.invoke_handler(tauri::generate_handler![
            facade::create_customer,
            facade::get_customer,
            facade::get_customers,
            facade::get_customers_page,
            facade::update_customer,
            facade::delete_customer,
            facade::create_user,
            facade::get_user,
            facade::get_user_by_email,
            facade::get_users,
            facade::get_users_page,
            facade::update_user,
            facade::delete_user,
            facade::create_inventory_item,
            facade::get_inventory_item,
            facade::get_inventory_items,
            facade::get_inventory_items_page,
            facade::get_inventory_summary,
            facade::update_inventory_item,
            facade::delete_inventory_item,
            facade::restock_inventory_item,
            facade::remove_stock_inventory_item,
            facade::get_inventory_movements,
            facade::get_inventory_insights,
            facade::create_service_order,
            facade::create_full_service_order,
            facade::get_service_order,
            facade::get_service_orders,
            facade::get_service_orders_page,
            facade::get_service_orders_by_customer_id,
            facade::get_service_order_events,
            facade::update_service_order,
            facade::transition_service_order_status,
            facade::save_service_order_edit,
            facade::delete_service_order,
            facade::add_part_to_service_order,
            facade::remove_part_from_service_order,
            facade::update_service_order_part_quantity,
            facade::get_service_order_parts,
            facade::get_dashboard_data,
            facade::get_settings,
            facade::update_settings,
            facade::get_lan_mode_config,
            facade::update_lan_mode_config,
            facade::lan_remote_command,
            facade::get_lan_host_status,
            facade::regenerate_lan_pairing_code,
            facade::list_lan_devices,
            facade::revoke_lan_device,
            facade::save_lan_base64_file,
            facade::save_lan_text_file,
            facade::pair_lan_client,
            facade::check_lan_client_connection,
            facade::download_lan_remote_backup,
            facade::run_scheduled_lan_remote_backup,
            facade::reset_database,
            facade::get_system_info,
            facade::check_for_updates,
            facade::export_backup,
            facade::restore_backup,
            facade::inspect_backup,
            facade::validate_backup_passphrase,
            facade::get_automatic_backup_status,
            facade::update_automatic_backup_settings,
            facade::create_checklist_template,
            facade::get_checklist_templates,
            facade::get_checklist_templates_page,
            facade::get_checklist_template_items,
            facade::update_checklist_template,
            facade::delete_checklist_template,
            facade::save_service_order_checklist,
            facade::get_service_order_checklist,
            facade::attach_pending_service_order_attachments,
            facade::discard_pending_service_order_attachments,
            facade::get_service_order_attachments,
            facade::delete_service_order_attachment,
            facade::read_service_order_attachment,
            facade::export_service_order_attachment,
            facade::preview_service_order_pdf,
            facade::discard_pdf_preview,
            facade::get_financial_report,
            facade::export_financial_report_csv,
            facade::preview_financial_report_pdf,
            $($extra,)*
        ])
    }};
}

mod attachment_service;
mod automatic_backup;
#[cfg(test)]
mod backend_e2e_tests;
mod backup_service;
mod commands;
mod database;
mod encryption;
mod error;
mod lan_api;
mod lan_auth;
mod lan_client;
mod lan_idempotency;
mod models;
mod money;
mod page;
mod pdf_service;
#[cfg(test)]
mod performance_benchmarks;
mod repositories;
mod seeds;
#[cfg(test)]
mod storage_e2e_tests;
#[cfg(test)]
mod tauri_ipc_tests;
#[cfg(test)]
mod test_helpers;

use dotenv::dotenv;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = register_commands!(
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_updater::Builder::new().build()),
        commands::settings_commands::select_company_logo,
        commands::settings_commands::select_automatic_backup_directory,
        commands::settings_commands::run_automatic_backup_now,
        commands::attachment_commands::select_service_order_attachments,
        commands::attachment_commands::select_pending_service_order_attachments,
        commands::attachment_commands::select_lan_attachment_files,
        commands::pdf_commands::save_pdf_preview,
    )
    .setup(|app| {
        let _ = dotenv();
        // Startup must fail visibly when the local data store cannot initialize.
        database::init_db(app)?;
        // Client mode deliberately has no local database path. Its backup is
        // downloaded from the host by the frontend, so starting the local
        // scheduler here would panic before the webview can render.
        if automatic_backup::should_start_scheduler(&database::storage_mode_config().mode) {
            automatic_backup::start_scheduler(app.handle().clone())?;
        }
        lan_api::start_configured_host()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application");
    app.run(|_, event| {
        if matches!(
            event,
            tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
        ) {
            automatic_backup::stop_scheduler();
            lan_api::stop_configured_host();
        }
    });
}
