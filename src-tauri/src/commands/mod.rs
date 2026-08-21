pub mod attachment_commands;
pub mod checklist_commands;
pub mod customer_commands;
pub mod dashboard_commands;
pub mod inventory_commands;
pub mod pdf_commands;
pub mod report_commands;
pub mod service_order_commands;
pub mod settings_commands;
pub mod user_commands;

// Shared product-operation surface for both Tauri IPC and the LAN API. Command
// functions remain ordinary typed Rust functions; transport layers only adapt
// request and response serialization.
pub(crate) mod facade {
    pub use super::attachment_commands::*;
    pub use super::checklist_commands::*;
    pub use super::customer_commands::*;
    pub use super::dashboard_commands::*;
    pub use super::inventory_commands::*;
    pub use super::pdf_commands::*;
    pub use super::report_commands::*;
    pub use super::service_order_commands::*;
    pub use super::settings_commands::*;
    pub use super::user_commands::*;
}
