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
    technician_id: Option<String>,
    ranking_metric: Option<String>,
    ranking_limit: Option<i32>,
) -> Result<FinancialReport, AppError> {
    Ok(FinancialReportRepository::get_report_filtered(
        start_date.as_deref(),
        end_date.as_deref(),
        technician_id.as_deref(),
        ranking_metric.as_deref(),
        ranking_limit,
    )?)
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn financial_report_csv(report: &FinancialReport) -> String {
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
    for item in &report.by_technician {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    csv.push_str("\nCategoria,Faturamento,Custo,Lucro,OS finalizadas\n");
    for item in &report.by_item_type {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    let ranking_label = if report.ranking_metric == "quantity" {
        "Itens e serviços mais vendidos por quantidade"
    } else {
        "Itens e serviços mais vendidos por faturamento"
    };
    csv.push_str(&format!(
        "\n{ranking_label},Faturamento,Custo,Lucro,Quantidade\n"
    ));
    for item in &report.top_items {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{}\n",
            csv_escape(&item.label),
            item.revenue,
            item.cost,
            item.profit,
            item.count,
        ));
    }
    csv
}

#[command]
pub fn export_financial_report_csv(
    start_date: Option<String>,
    end_date: Option<String>,
    technician_id: Option<String>,
    ranking_metric: Option<String>,
    ranking_limit: Option<i32>,
    destination: String,
) -> Result<(), AppError> {
    let report = FinancialReportRepository::get_report_filtered(
        start_date.as_deref(),
        end_date.as_deref(),
        technician_id.as_deref(),
        ranking_metric.as_deref(),
        ranking_limit,
    )?;
    let csv = financial_report_csv(&report);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::financial_report_repo::{FinancialBreakdown, FinancialMonth};

    #[test]
    fn renders_escaped_csv_with_quantity_ranking() {
        let breakdown = FinancialBreakdown {
            label: "Técnica \"Ana\"".to_string(),
            revenue: 120.0,
            cost: 50.0,
            profit: 70.0,
            count: 2,
        };
        let report = FinancialReport {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-31".to_string(),
            total_revenue: 120.0,
            total_cost: 50.0,
            net_profit: 70.0,
            average_ticket: 60.0,
            finalized_orders: 2,
            new_customers: 1,
            new_orders: 3,
            completion_rate: 66.67,
            cancelled_orders: 1,
            cancellation_rate: 33.33,
            average_turnaround_hours: 24.0,
            returning_customers: 1,
            total_discounts: 10.0,
            ranking_metric: "quantity".to_string(),
            ranking_limit: 5,
            by_technician: vec![breakdown.clone()],
            by_item_type: vec![breakdown.clone()],
            top_items: vec![breakdown],
            by_month: vec![FinancialMonth {
                month: "2026-01".to_string(),
                revenue: 120.0,
                profit: 70.0,
                order_count: 2,
            }],
        };

        let csv = financial_report_csv(&report);

        assert!(csv.contains("\"Técnica \"\"Ana\"\"\",120.00,50.00,70.00,2"));
        assert!(csv.contains("Itens e serviços mais vendidos por quantidade"));
    }
}

#[command]
pub fn preview_financial_report_pdf(
    start_date: Option<String>,
    end_date: Option<String>,
    technician_id: Option<String>,
    ranking_metric: Option<String>,
    ranking_limit: Option<i32>,
) -> Result<PdfPreview, AppError> {
    preview_report_pdf(
        start_date.as_deref(),
        end_date.as_deref(),
        technician_id.as_deref(),
        ranking_metric.as_deref(),
        ranking_limit,
    )
}
