use crate::error::AppError;
use crate::money::{format_csv, is_js_safe_integer};
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
    let report = FinancialReportRepository::get_report_filtered(
        start_date.as_deref(),
        end_date.as_deref(),
        technician_id.as_deref(),
        ranking_metric.as_deref(),
        ranking_limit,
    )?;
    validate_report_money_for_ipc(&report)?;
    Ok(report)
}

fn validate_report_money_for_ipc(report: &FinancialReport) -> Result<(), AppError> {
    let summary = [
        report.total_revenue,
        report.total_cost,
        report.estimated_gross_profit,
        report.average_ticket,
        report.total_discounts,
    ];
    let breakdowns = report
        .by_technician
        .iter()
        .chain(&report.by_item_type)
        .flat_map(|item| [item.revenue, item.cost, item.profit]);
    let items = report
        .top_items
        .iter()
        .flat_map(|item| [item.revenue, item.cost, item.profit]);
    let months = report
        .by_month
        .iter()
        .flat_map(|month| [month.revenue, month.profit]);
    if summary
        .into_iter()
        .chain(breakdowns)
        .chain(items)
        .chain(months)
        .all(is_js_safe_integer)
    {
        return Ok(());
    }
    Err(AppError::new(
        "Report money exceeds the JavaScript safe integer range.",
        "Um valor monetário do relatório excede o limite seguro do JavaScript.",
    ))
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn financial_report_csv(report: &FinancialReport) -> String {
    let mut csv = String::from(
        "Período inicial,Período final,Faturamento,Custo,Lucro bruto estimado,Ticket médio,OS finalizadas,Novos clientes,Novas OS,Taxa de conclusão,OS canceladas,Taxa de cancelamento,Tempo médio de conclusão (horas),Clientes recorrentes,Descontos concedidos\n",
    );
    csv.push_str(&format!(
        "{},{},{},{},{},{},{},{},{},{:.2},{},{:.2},{:.2},{},{}\n\n",
        csv_escape(&report.start_date),
        csv_escape(&report.end_date),
        format_csv(report.total_revenue),
        format_csv(report.total_cost),
        format_csv(report.estimated_gross_profit),
        format_csv(report.average_ticket),
        report.finalized_orders,
        report.new_customers,
        report.new_orders,
        report.completion_rate,
        report.cancelled_orders,
        report.cancellation_rate,
        report.average_turnaround_hours,
        report.returning_customers,
        format_csv(report.total_discounts),
    ));
    csv.push_str("Técnico,Faturamento,Custo,Lucro bruto estimado,OS finalizadas\n");
    for item in &report.by_technician {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&item.label),
            format_csv(item.revenue),
            format_csv(item.cost),
            format_csv(item.profit),
            item.count,
        ));
    }
    csv.push_str("\nCategoria,Faturamento,Custo,Lucro bruto estimado,OS finalizadas\n");
    for item in &report.by_item_type {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&item.label),
            format_csv(item.revenue),
            format_csv(item.cost),
            format_csv(item.profit),
            item.count,
        ));
    }
    let ranking_label = if report.ranking_metric == "quantity" {
        "Itens e serviços mais vendidos por quantidade"
    } else {
        "Itens e serviços mais vendidos por faturamento"
    };
    csv.push_str(&format!(
        "\n{ranking_label},Faturamento,Custo,Lucro bruto estimado,Quantidade\n"
    ));
    for item in &report.top_items {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&item.display_label),
            format_csv(item.revenue),
            format_csv(item.cost),
            format_csv(item.profit),
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
    use crate::repositories::financial_report_repo::{
        FinancialBreakdown, FinancialItemBreakdown, FinancialMonth,
    };

    #[test]
    fn renders_escaped_csv_with_quantity_ranking() {
        let breakdown = FinancialBreakdown {
            label: "Técnica \"Ana\"".to_string(),
            revenue: 12_000,
            cost: 5_000,
            profit: 7_000,
            count: 2,
        };
        let item_breakdown = FinancialItemBreakdown {
            key: "item-1|part|Tela".to_string(),
            inventory_item_id: "item-1".to_string(),
            label: "Tela".to_string(),
            item_type: "part".to_string(),
            display_label: "Tela (Peça · item-1)".to_string(),
            revenue: 12_000,
            cost: 5_000,
            profit: 7_000,
            count: 2,
        };
        let report = FinancialReport {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-31".to_string(),
            total_revenue: 12_000,
            total_cost: 5_000,
            estimated_gross_profit: 7_000,
            average_ticket: 6_000,
            finalized_orders: 2,
            new_customers: 1,
            new_orders: 3,
            completion_rate: 66.67,
            cancelled_orders: 1,
            cancellation_rate: 33.33,
            average_turnaround_hours: 24.0,
            returning_customers: 1,
            total_discounts: 1_000,
            ranking_metric: "quantity".to_string(),
            ranking_limit: 5,
            by_technician: vec![breakdown.clone()],
            by_item_type: vec![breakdown.clone()],
            top_items: vec![item_breakdown],
            by_month: vec![FinancialMonth {
                month: "2026-01".to_string(),
                revenue: 12_000,
                profit: 7_000,
                order_count: 2,
            }],
        };

        let csv = financial_report_csv(&report);

        assert!(csv.contains("\"Técnica \"\"Ana\"\"\",120.00,50.00,70.00,2"));
        assert!(csv.contains("Lucro bruto estimado"));
        assert!(csv.contains("Itens e serviços mais vendidos por quantidade"));
        assert!(csv.contains("Tela (Peça · item-1)"));
    }

    #[test]
    fn rejects_unsafe_report_money_before_ipc() {
        let report = FinancialReport {
            start_date: String::new(),
            end_date: String::new(),
            total_revenue: crate::money::JS_MAX_SAFE_INTEGER + 1,
            total_cost: 0,
            estimated_gross_profit: 0,
            average_ticket: 0,
            finalized_orders: 0,
            new_customers: 0,
            new_orders: 0,
            completion_rate: 0.0,
            cancelled_orders: 0,
            cancellation_rate: 0.0,
            average_turnaround_hours: 0.0,
            returning_customers: 0,
            total_discounts: 0,
            ranking_metric: "revenue".to_string(),
            ranking_limit: 5,
            by_technician: vec![],
            by_item_type: vec![],
            top_items: vec![],
            by_month: vec![],
        };

        assert!(validate_report_money_for_ipc(&report).is_err());
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
