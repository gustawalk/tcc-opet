use crate::error::AppError;
use crate::pdf_service::{preview_financial_report_pdf as preview_report_pdf, PdfPreview};
use crate::repositories::financial_report_repo::{FinancialReport, FinancialReportRepository};
use std::fs;
use std::path::Path;
use tauri::command;

#[command]
pub fn get_financial_report(
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<FinancialReport, AppError> {
    Ok(FinancialReportRepository::get_report(
        start_date.as_deref(),
        end_date.as_deref(),
    )?)
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[command]
pub fn export_financial_report_csv(
    start_date: Option<String>,
    end_date: Option<String>,
    destination: String,
) -> Result<(), AppError> {
    let report = FinancialReportRepository::get_report(start_date.as_deref(), end_date.as_deref())?;
    let mut csv = String::from(
        "Período inicial,Período final,Faturamento,Custo,Lucro,Ticket médio,OS finalizadas,Novos clientes,Novas OS,Taxa de conclusão,OS canceladas,Taxa de cancelamento,Tempo médio de conclusão (horas),Clientes recorrentes,Descontos concedidos\n",
    );
    csv.push_str(&format!(
        "{},{},{:.2},{:.2},{:.2},{:.2},{},{},{},{:.2},{},{:.2},{:.2},{},{:.2}\n\n",
        csv_escape(&report.start_date),
        csv_escape(&report.end_date),
        report.total_revenue,
        report.total_cost,
        report.net_profit,
        report.average_ticket,
        report.finalized_orders,
        report.new_customers,
        report.new_orders,
        report.completion_rate,
        report.cancelled_orders,
        report.cancellation_rate,
        report.average_turnaround_hours,
        report.returning_customers,
        report.total_discounts,
    ));
    csv.push_str("Técnico,Faturamento,Custo,Lucro,OS finalizadas\n");
    for item in report.by_technician {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    csv.push_str("\nCategoria,Faturamento,Custo,Lucro,Quantidade\n");
    for item in report.by_item_type {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    csv.push_str("\nItens e serviços mais vendidos,Faturamento,Custo,Lucro,Quantidade\n");
    for item in report.top_items {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    if let Some(parent) = Path::new(&destination).parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(
                format!("Failed to create export directory: {error}"),
                format!("Erro ao criar o diretório de exportação: {error}"),
            )
        })?;
    }
    fs::write(destination, csv).map_err(|error| {
        AppError::new(
            format!("Failed to write financial report: {error}"),
            format!("Erro ao gravar o relatório financeiro: {error}"),
        )
    })?;
    Ok(())
}

#[command]
pub fn preview_financial_report_pdf(
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<PdfPreview, AppError> {
    preview_report_pdf(start_date.as_deref(), end_date.as_deref())
}
