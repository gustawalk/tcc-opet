import { lazy, Suspense, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Clock3,
  Download,
  FileText,
  LoaderCircle,
  Repeat2,
  Tag,
  UserPlus,
  Wallet,
  XCircle,
} from "lucide-react";
import { DatePicker } from "@/components/shared/DatePicker";
import { FinancialCard } from "@/components/shared/FinancialCard";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toastError, toastSuccess } from "@/lib/errors";
import { formatCurrency } from "@/lib/formatters";
import type {
  FinancialBreakdown,
  FinancialReport,
  PdfPreview,
} from "@/lib/types";

const PdfPreviewDialog = lazy(() =>
  import("@/components/shared/PdfPreviewDialog").then(
    ({ PdfPreviewDialog }) => ({
      default: PdfPreviewDialog,
    }),
  ),
);

type ReportFilters = Record<string, string>;

function reportFilters(startDate: string, endDate: string): ReportFilters {
  return {
    ...(startDate ? { startDate } : {}),
    ...(endDate ? { endDate } : {}),
  };
}

function BreakdownTable({
  items,
  label,
}: {
  items: FinancialBreakdown[];
  label: string;
}) {
  if (!items.length)
    return (
      <p className="py-6 text-center text-sm text-muted-foreground">
        Nenhum dado disponível para este período.
      </p>
    );

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{label}</TableHead>
          <TableHead className="text-right">Faturamento</TableHead>
          <TableHead className="text-right">Custo</TableHead>
          <TableHead className="text-right">Lucro</TableHead>
          <TableHead className="text-right">OS</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((item) => (
          <TableRow key={item.label}>
            <TableCell className="font-medium">{item.label}</TableCell>
            <TableCell className="text-right">
              {formatCurrency(item.revenue)}
            </TableCell>
            <TableCell className="text-right">
              {formatCurrency(item.cost)}
            </TableCell>
            <TableCell className="text-right font-medium">
              {formatCurrency(item.profit)}
            </TableCell>
            <TableCell className="text-right">{item.count}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function formatMonth(month: string) {
  const [year, value] = month.split("-");
  return new Intl.DateTimeFormat("pt-BR", {
    month: "long",
    year: "numeric",
  }).format(new Date(Number(year), Number(value) - 1, 1));
}

function toLocalIsoDate(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function currentMonthStart() {
  const date = new Date();
  return toLocalIsoDate(new Date(date.getFullYear(), date.getMonth(), 1));
}

function formatTurnaround(hours: number) {
  if (hours < 24) return `${hours.toFixed(1)} h`;
  return `${(hours / 24).toFixed(1)} dias`;
}

export function Reports() {
  const [startDate, setStartDate] = useState(currentMonthStart);
  const [endDate, setEndDate] = useState(() => toLocalIsoDate(new Date()));
  const [exporting, setExporting] = useState<"csv" | "pdf" | null>(null);
  const [pdfPreview, setPdfPreview] = useState<PdfPreview | null>(null);
  const filters = reportFilters(startDate, endDate);
  const {
    data: report,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["financial-report", startDate, endDate],
    queryFn: () => invoke<FinancialReport>("get_financial_report", filters),
  });

  const exportCsv = async () => {
    try {
      const destination = await save({
        defaultPath: "relatorio-financeiro.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!destination) return;

      setExporting("csv");
      await invoke("export_financial_report_csv", {
        ...filters,
        destination,
      });
      toastSuccess("Relatório em CSV exportado.");
    } catch (err) {
      toastError(err, "Erro ao exportar o relatório em CSV.");
    } finally {
      setExporting(null);
    }
  };

  const previewPdf = async () => {
    try {
      setExporting("pdf");
      const preview = await invoke<PdfPreview>(
        "preview_financial_report_pdf",
        filters,
      );
      setPdfPreview(preview);
    } catch (err) {
      toastError(err, "Erro ao gerar PDF do relatório.");
    } finally {
      setExporting(null);
    }
  };

  const invalidPeriod = Boolean(startDate && endDate && startDate > endDate);
  const hasData = report ? report.finalizedOrders > 0 : false;

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">
            Relatórios Financeiros
          </h2>
          <p className="mt-1 text-muted-foreground">
            Acompanhe os resultados das ordens de serviço finalizadas.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            onClick={exportCsv}
            disabled={Boolean(exporting) || invalidPeriod}
          >
            <Download className="h-4 w-4" />{" "}
            {exporting === "csv" ? "Exportando..." : "Exportar CSV"}
          </Button>
          <Button
            size="sm"
            className="gap-2"
            onClick={previewPdf}
            disabled={Boolean(exporting) || invalidPeriod}
          >
            <FileText className="h-4 w-4" />{" "}
            {exporting === "pdf" ? "Gerando..." : "Gerar PDF"}
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader className="pb-4">
          <CardTitle className="text-lg">Período</CardTitle>
          <CardDescription>
            O relatório inicia com o mês atual até hoje. Ajuste o intervalo se
            necessário.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label htmlFor="report-start-date" className="text-sm font-medium">
              Data inicial
            </label>
            <DatePicker
              id="report-start-date"
              value={startDate}
              onChange={setStartDate}
            />
          </div>
          <div className="space-y-2">
            <label htmlFor="report-end-date" className="text-sm font-medium">
              Data final
            </label>
            <DatePicker
              id="report-end-date"
              value={endDate}
              onChange={setEndDate}
            />
          </div>
          {invalidPeriod && (
            <p className="text-sm text-destructive md:col-span-2">
              A data final deve ser igual ou posterior à data inicial.
            </p>
          )}
        </CardContent>
      </Card>

      {isLoading && (
        <div className="space-y-6">
          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-4">
            {[1, 2, 3, 4].map((item) => (
              <Skeleton key={item} className="h-32" />
            ))}
          </div>
          <Skeleton className="h-64" />
        </div>
      )}

      {!isLoading && (error || !report) && (
        <div className="flex min-h-72 flex-col items-center justify-center gap-4 rounded-lg border bg-card p-6 text-center">
          <AlertTriangle className="h-10 w-10 text-destructive" />
          <div>
            <h3 className="font-semibold">Erro ao carregar o relatório</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Não foi possível obter os dados financeiros para o período
              selecionado.
            </p>
          </div>
          <Button variant="outline" onClick={() => refetch()}>
            Tentar novamente
          </Button>
        </div>
      )}

      {!isLoading && report && (
        <>
          <p className="text-sm text-muted-foreground">
            Dados de{" "}
            {new Date(`${report.startDate}T00:00:00`).toLocaleDateString(
              "pt-BR",
            )}{" "}
            até{" "}
            {new Date(`${report.endDate}T00:00:00`).toLocaleDateString("pt-BR")}
            .
          </p>
          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-4">
            <FinancialCard
              title="Novos Clientes"
              value={report.newCustomers.toString()}
              icon={UserPlus}
              description="Cadastros realizados no período"
            />
            <FinancialCard
              title="Novas OS"
              value={report.newOrders.toString()}
              icon={FileText}
              description={`${report.completionRate.toFixed(1)}% concluídas`}
            />
            <FinancialCard
              title="Tempo Médio"
              value={formatTurnaround(report.averageTurnaroundHours)}
              icon={Clock3}
              description="Da abertura à finalização"
            />
            <FinancialCard
              title="Cancelamentos"
              value={report.cancelledOrders.toString()}
              icon={XCircle}
              description={`${report.cancellationRate.toFixed(1)}% das novas OS`}
            />
          </div>
          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-4">
            <FinancialCard
              title="Faturamento"
              value={formatCurrency(report.totalRevenue)}
              icon={Wallet}
              description="Receita das OS finalizadas"
            />
            <FinancialCard
              title="Custos"
              value={formatCurrency(report.totalCost)}
              icon={Wallet}
              description="Custo das peças utilizadas"
            />
            <FinancialCard
              title="Lucro Líquido"
              value={formatCurrency(report.netProfit)}
              icon={Wallet}
              description="Faturamento menos custos"
            />
            <FinancialCard
              title="Ticket Médio"
              value={formatCurrency(report.averageTicket)}
              icon={Wallet}
              description={`${report.finalizedOrders} OS finalizadas`}
            />
            <FinancialCard
              title="Clientes Recorrentes"
              value={report.returningCustomers.toString()}
              icon={Repeat2}
              description="Com histórico anterior ao período"
            />
            <FinancialCard
              title="Descontos Concedidos"
              value={formatCurrency(report.totalDiscounts)}
              icon={Tag}
              description="Em OS finalizadas"
            />
          </div>
          {!hasData ? (
            <div className="flex min-h-52 flex-col items-center justify-center gap-3 rounded-lg border bg-card p-6 text-center">
              <Wallet className="h-10 w-10 text-muted-foreground" />
              <div>
                <h3 className="font-semibold">
                  Nenhuma ordem finalizada no período
                </h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  Os indicadores operacionais continuam disponíveis acima.
                </p>
              </div>
            </div>
          ) : (
            <>
              <div className="grid gap-6 xl:grid-cols-2">
                <Card>
                  <CardHeader>
                    <CardTitle>Por Técnico</CardTitle>
                    <CardDescription>
                      Resultado por responsável pela OS.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.byTechnician}
                      label="Técnico"
                    />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Por Categoria</CardTitle>
                    <CardDescription>
                      Peças e serviços utilizados nas OS.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.byItemType}
                      label="Categoria"
                    />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Itens e Serviços Mais Vendidos</CardTitle>
                    <CardDescription>
                      Ranking por faturamento nas OS finalizadas.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.topItems}
                      label="Item / Serviço"
                    />
                  </CardContent>
                </Card>
              </div>
              <Card>
                <CardHeader>
                  <CardTitle>Evolução Mensal</CardTitle>
                  <CardDescription>
                    Faturamento e lucro por mês dentro do período.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {report.byMonth.length ? (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Mês</TableHead>
                          <TableHead className="text-right">
                            Faturamento
                          </TableHead>
                          <TableHead className="text-right">Lucro</TableHead>
                          <TableHead className="text-right">
                            OS finalizadas
                          </TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {report.byMonth.map((item) => (
                          <TableRow key={item.month}>
                            <TableCell className="font-medium capitalize">
                              {formatMonth(item.month)}
                            </TableCell>
                            <TableCell className="text-right">
                              {formatCurrency(item.revenue)}
                            </TableCell>
                            <TableCell className="text-right font-medium">
                              {formatCurrency(item.profit)}
                            </TableCell>
                            <TableCell className="text-right">
                              {item.orderCount}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  ) : (
                    <p className="py-6 text-center text-sm text-muted-foreground">
                      Nenhum dado mensal disponível.
                    </p>
                  )}
                </CardContent>
              </Card>
            </>
          )}
        </>
      )}
      {exporting && (
        <span className="sr-only">
          <LoaderCircle />
          Exportação em andamento
        </span>
      )}
      {pdfPreview && (
        <Suspense fallback={null}>
          <PdfPreviewDialog
            preview={pdfPreview}
            onClose={() => setPdfPreview(null)}
          />
        </Suspense>
      )}
    </div>
  );
}
