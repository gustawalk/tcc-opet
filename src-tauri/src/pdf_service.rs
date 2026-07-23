use crate::error::{not_found, AppError};
use crate::models::checklist::ChecklistItem;
use crate::repositories::checklist_repo::ChecklistRepository;
use crate::repositories::financial_report_repo::FinancialReportRepository;
use crate::repositories::service_order_repo::{ServiceOrderPart, ServiceOrderRepository};
use crate::repositories::settings_repo::SettingsRepository;
use base64::Engine;
use chrono::{DateTime, Local};
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tera::{Context, Tera};
use uuid::Uuid;

const SERVICE_ORDER_TEMPLATE: &str = include_str!("../templates/service_order.html");
const FINANCIAL_REPORT_TEMPLATE: &str = include_str!("../templates/financial_report.html");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfPreview {
    pub token: String,
    pub data_url: String,
    pub file_name: String,
}

#[derive(Debug, Clone)]
struct PendingPdfPreview {
    bytes: Vec<u8>,
    file_name: String,
}

static PENDING_PDF_PREVIEWS: Lazy<Mutex<HashMap<String, PendingPdfPreview>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize)]
struct PdfCustomer {
    name: String,
    phone: String,
    email: String,
    address: String,
}

#[derive(Debug, Serialize)]
struct PdfPart {
    name: String,
    quantity: i32,
    unit_price: String,
    total_price: String,
}

#[derive(Debug, Serialize)]
struct PdfChecklistItem {
    label: String,
    checked: bool,
}

fn pdf_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        format!("Failed to generate service order PDF: {error}"),
        format!("Erro ao gerar o PDF da ordem de serviço: {error}"),
    )
}

fn format_currency(value: f64) -> String {
    let formatted = format!("{value:.2}");
    let (integer, decimal) = formatted.split_once('.').unwrap_or((&formatted, "00"));
    let mut grouped = String::new();
    for (index, character) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(character);
    }
    format!(
        "R$ {},{}",
        grouped.chars().rev().collect::<String>(),
        decimal
    )
}

fn format_date(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|date| {
            date.with_timezone(&Local)
                .format("%d/%m/%Y %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_string())
}

fn load_customer(conn: &Connection, customer_id: &str) -> Result<PdfCustomer, AppError> {
    conn.query_row(
        "SELECT name, phone, email, address FROM customers WHERE id = ?1",
        params![customer_id],
        |row| {
            Ok(PdfCustomer {
                name: row.get(0)?,
                phone: row.get(1)?,
                email: row.get(2)?,
                address: row.get(3)?,
            })
        },
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => not_found("Customer", "Cliente"),
        other => other.into(),
    })
}

fn build_html_with_conn(conn: &Connection, service_order_id: &str) -> Result<String, AppError> {
    let order = ServiceOrderRepository::get_by_id_with_conn(conn, service_order_id)?
        .ok_or_else(|| not_found("Service order", "Ordem de serviço"))?;
    let customer = load_customer(conn, &order.customer_id)?;
    let settings = SettingsRepository::get_settings_with_conn(conn)?;
    let parts = ServiceOrderRepository::get_service_order_parts_with_conn(conn, service_order_id)?;
    let checklist = ChecklistRepository::get_os_checklist_with_conn(conn, service_order_id)?;
    let gross_total = parts
        .iter()
        .map(|part| part.quantity as f64 * part.unit_price)
        .sum::<f64>();
    let total = gross_total * (1.0 - order.discount_percent / 100.0);

    let mut context = Context::new();
    context.insert("settings", &settings);
    context.insert("customer", &customer);
    context.insert("display_id", &order.display_id);
    context.insert("status", &order.status);
    context.insert("equipment", &order.equipment);
    context.insert("imei", &order.imei.unwrap_or_default());
    context.insert("description", &order.description);
    context.insert(
        "technician",
        &order
            .user_name
            .unwrap_or_else(|| "Não atribuído".to_string()),
    );
    context.insert("created_at", &format_date(&order.created_at));
    context.insert(
        "closed_at",
        &order
            .closed_at
            .as_deref()
            .map(format_date)
            .unwrap_or_default(),
    );
    context.insert("discount_percent", &order.discount_percent);
    context.insert("gross_total", &format_currency(gross_total));
    context.insert("total", &format_currency(total));
    context.insert("parts", &parts.iter().map(pdf_part).collect::<Vec<_>>());
    context.insert(
        "checklist",
        &checklist.iter().map(pdf_checklist_item).collect::<Vec<_>>(),
    );
    context.insert(
        "generated_at",
        &Local::now().format("%d/%m/%Y %H:%M").to_string(),
    );

    Tera::one_off(SERVICE_ORDER_TEMPLATE, &context, true).map_err(pdf_error)
}

fn pdf_part(part: &ServiceOrderPart) -> PdfPart {
    PdfPart {
        name: part.inventory_item_name.clone(),
        quantity: part.quantity,
        unit_price: format_currency(part.unit_price),
        total_price: format_currency(part.unit_price * part.quantity as f64),
    }
}

fn pdf_checklist_item(item: &ChecklistItem) -> PdfChecklistItem {
    PdfChecklistItem {
        label: item.label.clone(),
        checked: item.checked,
    }
}

fn chrome_path() -> Result<PathBuf, AppError> {
    if let Ok(path) = std::env::var("CHROME_BIN") {
        return Ok(PathBuf::from(path));
    }

    browser_candidates()
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            AppError::new(
                "Chrome, Chromium, or Microsoft Edge was not found. Set CHROME_BIN to generate PDFs.",
                "O Chrome, Chromium ou Microsoft Edge não foi encontrado. Configure CHROME_BIN para gerar PDFs.",
            )
        })
}

fn browser_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();
        for variable in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Some(directory) = std::env::var_os(variable) {
                let directory = PathBuf::from(directory);
                candidates.push(
                    directory
                        .join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                );
                candidates.push(
                    directory
                        .join("Microsoft")
                        .join("Edge")
                        .join("Application")
                        .join("msedge.exe"),
                );
            }
        }
        candidates
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/chromium-browser"),
            PathBuf::from("/usr/bin/google-chrome"),
        ]
    }
}

pub fn preview_service_order_pdf(service_order_id: &str) -> Result<PdfPreview, AppError> {
    let conn = crate::database::get_db()?;
    let html = build_html_with_conn(&conn, service_order_id)?;
    let order = ServiceOrderRepository::get_by_id_with_conn(&conn, service_order_id)?
        .ok_or_else(|| not_found("Service order", "Ordem de serviço"))?;
    create_pdf_preview(
        format!("{}.pdf", order.display_id),
        render_html_to_pdf_bytes(&html)?,
    )
}

pub fn preview_financial_report_pdf(
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<PdfPreview, AppError> {
    let report = FinancialReportRepository::get_report(start_date, end_date)?;
    let mut context = Context::new();
    context.insert("start_date", &report.start_date);
    context.insert("end_date", &report.end_date);
    context.insert("total_revenue", &format_currency(report.total_revenue));
    context.insert("total_cost", &format_currency(report.total_cost));
    context.insert("net_profit", &format_currency(report.net_profit));
    context.insert("average_ticket", &format_currency(report.average_ticket));
    context.insert("finalized_orders", &report.finalized_orders);
    context.insert("new_customers", &report.new_customers);
    context.insert("new_orders", &report.new_orders);
    context.insert(
        "completion_rate",
        &format!("{:.1}%", report.completion_rate),
    );
    context.insert("cancelled_orders", &report.cancelled_orders);
    context.insert(
        "cancellation_rate",
        &format!("{:.1}%", report.cancellation_rate),
    );
    context.insert(
        "average_turnaround_hours",
        &format!("{:.1} h", report.average_turnaround_hours),
    );
    context.insert("returning_customers", &report.returning_customers);
    context.insert("total_discounts", &format_currency(report.total_discounts));
    context.insert(
        "by_technician",
        &report
            .by_technician
            .iter()
            .map(|item| {
                serde_json::json!({
                    "label": item.label,
                    "revenue": format_currency(item.revenue),
                    "cost": format_currency(item.cost),
                    "profit": format_currency(item.profit),
                    "count": item.count,
                })
            })
            .collect::<Vec<_>>(),
    );
    context.insert(
        "by_item_type",
        &report
            .by_item_type
            .iter()
            .map(|item| {
                serde_json::json!({
                    "label": item.label,
                    "revenue": format_currency(item.revenue),
                    "cost": format_currency(item.cost),
                    "profit": format_currency(item.profit),
                    "count": item.count,
                })
            })
            .collect::<Vec<_>>(),
    );
    context.insert(
        "by_month",
        &report
            .by_month
            .iter()
            .map(|item| {
                serde_json::json!({
                    "month": item.month,
                    "revenue": format_currency(item.revenue),
                    "profit": format_currency(item.profit),
                    "count": item.order_count,
                })
            })
            .collect::<Vec<_>>(),
    );
    context.insert(
        "top_items",
        &report
            .top_items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "label": item.label,
                    "revenue": format_currency(item.revenue),
                    "cost": format_currency(item.cost),
                    "profit": format_currency(item.profit),
                    "count": item.count,
                })
            })
            .collect::<Vec<_>>(),
    );
    context.insert(
        "generated_at",
        &Local::now().format("%d/%m/%Y %H:%M").to_string(),
    );
    let html = Tera::one_off(FINANCIAL_REPORT_TEMPLATE, &context, true).map_err(pdf_error)?;
    create_pdf_preview(
        "relatorio-financeiro.pdf".to_string(),
        render_html_to_pdf_bytes(&html)?,
    )
}

fn preview_working_dir() -> Result<PathBuf, AppError> {
    let mut path = crate::database::database_path();
    path.set_extension("pdf-previews");
    crate::database::ensure_private_dir(&path).map_err(pdf_error)?;
    Ok(path)
}

fn render_html_to_pdf_bytes(html: &str) -> Result<Vec<u8>, AppError> {
    render_html_to_pdf_bytes_in_dir(html, &preview_working_dir()?)
}

fn render_html_to_pdf_bytes_in_dir(html: &str, working_dir: &Path) -> Result<Vec<u8>, AppError> {
    fs::create_dir_all(working_dir).map_err(pdf_error)?;
    let html_path = working_dir.join(format!("{}.html", Uuid::new_v4()));
    fs::write(&html_path, html).map_err(pdf_error)?;
    crate::database::secure_private_file(&html_path).map_err(pdf_error)?;

    let result = (|| -> Result<Vec<u8>, AppError> {
        let options = LaunchOptionsBuilder::default()
            .path(Some(chrome_path()?))
            .sandbox(false)
            .build()
            .map_err(pdf_error)?;
        let browser = Browser::new(options).map_err(pdf_error)?;
        let tab = browser.new_tab().map_err(pdf_error)?;
        let html_url = format!("file://{}", html_path.to_string_lossy());
        let bytes = tab
            .navigate_to(&html_url)
            .map_err(pdf_error)?
            .wait_until_navigated()
            .map_err(pdf_error)?
            .print_to_pdf(Some(PrintToPdfOptions {
                print_background: Some(true),
                paper_width: Some(8.27),
                paper_height: Some(11.69),
                // The CSS @page rule owns the document margins.
                margin_top: Some(0.0),
                margin_bottom: Some(0.0),
                margin_left: Some(0.0),
                margin_right: Some(0.0),
                prefer_css_page_size: Some(true),
                ..Default::default()
            }))
            .map_err(pdf_error)?;
        Ok(bytes)
    })();

    let _ = fs::remove_file(html_path);
    result
}

fn create_pdf_preview(file_name: String, bytes: Vec<u8>) -> Result<PdfPreview, AppError> {
    let token = Uuid::new_v4().to_string();
    let data_url = format!(
        "data:application/pdf;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    PENDING_PDF_PREVIEWS
        .lock()
        .map_err(|_| {
            AppError::new(
                "PDF preview storage is unavailable.",
                "O armazenamento temporário do PDF está indisponível.",
            )
        })?
        .insert(
            token.clone(),
            PendingPdfPreview {
                bytes,
                file_name: file_name.clone(),
            },
        );
    Ok(PdfPreview {
        token,
        data_url,
        file_name,
    })
}

pub fn get_pdf_preview(token: &str) -> Result<(String, Vec<u8>), AppError> {
    PENDING_PDF_PREVIEWS
        .lock()
        .map_err(|_| {
            AppError::new(
                "PDF preview storage is unavailable.",
                "O armazenamento temporário do PDF está indisponível.",
            )
        })?
        .get(token)
        .cloned()
        .map(|preview| (preview.file_name, preview.bytes))
        .ok_or_else(|| {
            AppError::new(
                "PDF preview is no longer available.",
                "A pré-visualização do PDF não está mais disponível.",
            )
        })
}

pub fn discard_pdf_preview(token: &str) {
    if let Ok(mut previews) = PENDING_PDF_PREVIEWS.lock() {
        previews.remove(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::inventory_item::InventoryItem;
    use crate::models::service_order::ServiceOrder;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::inventory_repo::InventoryRepository;
    use crate::test_helpers::setup_db;

    #[test]
    fn renders_customer_settings_checklist_and_item_data() {
        let mut conn = setup_db();
        let customer = Customer::new(
            "Ana & Filho".to_string(),
            "41999999999".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let mut order = ServiceOrder::new(
            customer.id.clone(),
            "iPhone 14".to_string(),
            "Tela quebrada".to_string(),
        );
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        let part = InventoryItem::new(
            "Tela OLED".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            2,
            100.0,
            200.0,
        );
        InventoryRepository::create_with_conn(&conn, &part).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 1,
        )
        .unwrap();

        let html = build_html_with_conn(&conn, &order.id).unwrap();

        assert!(html.contains("Ana &amp; Filho"));
        assert!(html.contains("Tela OLED"));
        assert!(html.contains("OS-000001"));
        assert!(html.contains("R$ 200,00"));
        assert!(html.contains("Assinatura do cliente"));
    }

    #[test]
    #[ignore = "requires a local Chromium binary"]
    fn renders_html_to_a_real_pdf() {
        let working_dir = std::env::temp_dir().join(format!("tcc-opet-pdf-{}", Uuid::new_v4()));
        let bytes = render_html_to_pdf_bytes_in_dir(
            "<html><body><h1>PDF test</h1></body></html>",
            &working_dir,
        )
        .unwrap();

        assert!(bytes.len() > 100);
        assert!(bytes.starts_with(b"%PDF"));
        let _ = fs::remove_dir_all(working_dir);
    }

    #[test]
    fn stores_and_discards_pdf_previews() {
        let preview = create_pdf_preview("ordem.pdf".to_string(), b"%PDF-test".to_vec()).unwrap();

        assert_eq!(preview.file_name, "ordem.pdf");
        assert!(preview.data_url.starts_with("data:application/pdf;base64,"));
        assert_eq!(get_pdf_preview(&preview.token).unwrap().1, b"%PDF-test");

        discard_pdf_preview(&preview.token);
        assert!(get_pdf_preview(&preview.token).is_err());
    }
}
