import { lazy, Suspense, useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
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
import { SearchableSelect } from "@/components/shared/SearchableSelect";
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
  User as AppUser,
} from "@/lib/types";

const PdfPreviewDialog = lazy(() =>
  import("@/components/shared/PdfPreviewDialog").then(
    ({ PdfPreviewDialog }) => ({
      default: PdfPreviewDialog,
    }),
  ),
);

type RankingMetric = "revenue" | "quantity";
type PeriodPreset = "today" | "week" | "month" | "quarter" | "year" | "custom";

type ReportFilters = {
  startDate?: string;
  endDate?: string;
  technicianId?: string;
  rankingMetric: RankingMetric;
  rankingLimit: number;
};

function reportFilters(
  startDate: string,
  endDate: string,
  technicianId: string | null,
  rankingMetric: RankingMetric,
  rankingLimit: number,
): ReportFilters {
  return {
    ...(startDate ? { startDate } : {}),
    ...(endDate ? { endDate } : {}),
    ...(technicianId ? { technicianId } : {}),
    rankingMetric,
    rankingLimit,
  };
}

const fetchUsers = () => invoke<AppUser[]>("get_users");

function BreakdownTable({
  items,
  label,
  countLabel,
}: {
  items: FinancialBreakdown[];
  label: string;
  countLabel: string;
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
          <TableHead className="text-right">Lucro bruto estimado</TableHead>
          <TableHead className="text-right">{countLabel}</TableHead>
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

function periodDates(preset: Exclude<PeriodPreset, "custom">) {
  const today = new Date();
  const end = toLocalIsoDate(today);
  if (preset === "today") return { start: end, end };
  if (preset === "week") {
    const start = new Date(today);
    start.setDate(today.getDate() - 6);
    return { start: toLocalIsoDate(start), end };
  }
  if (preset === "month") return { start: currentMonthStart(), end };
  if (preset === "quarter") {
    const start = new Date(today.getFullYear(), Math.floor(today.getMonth() / 3) * 3, 1);
    return { start: toLocalIsoDate(start), end };
  }
  return { start: toLocalIsoDate(new Date(today.getFullYear(), 0, 1)), end };
}

function formatTurnaround(hours: number) {
  if (hours < 24) return `${hours.toFixed(1)} h`;
  return `${(hours / 24).toFixed(1)} dias`;
}

const CHART_COLORS = ["#2563eb", "#0f766e", "#d97706", "#7c3aed", "#db2777"];

function formatChartCurrency(value: unknown) {
  const amount = Array.isArray(value) ? value[0] : value;
  return typeof amount === "number" || typeof amount === "string"
    ? formatCurrency(Number(amount))
    : "-";
}

function MonthlyTrendChart({ items }: { items: FinancialReport["byMonth"] }) {
  const data = items.map((item) => ({
    ...item,
    label: formatMonth(item.month),
  }));

  return (
    <ResponsiveContainer width="100%" height={240}>
      <AreaChart data={data} margin={{ top: 16, right: 32, left: 18, bottom: 20 }}>
        <defs>
          <linearGradient id="revenueFill" x1="0" x2="0" y1="0" y2="1">
            <stop offset="5%" stopColor="#2563eb" stopOpacity={0.35} />
            <stop offset="95%" stopColor="#2563eb" stopOpacity={0.02} />
          </linearGradient>
          <linearGradient id="profitFill" x1="0" x2="0" y1="0" y2="1">
            <stop offset="5%" stopColor="#059669" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#059669" stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="label" tickLine={false} axisLine={false} minTickGap={28} tick={{ fontSize: 11 }} />
        <YAxis tickLine={false} axisLine={false} tickFormatter={formatChartCurrency} width={96} tick={{ fontSize: 11 }} />
        <Tooltip formatter={formatChartCurrency} />
        <Legend />
        <Area type="monotone" dataKey="revenue" name="Faturamento" stroke="#2563eb" fill="url(#revenueFill)" strokeWidth={2.5} />
        <Area type="monotone" dataKey="profit" name="Lucro bruto estimado" stroke="#059669" fill="url(#profitFill)" strokeWidth={2.5} />
      </AreaChart>
    </ResponsiveContainer>
  );
}

function CategoryChart({ items }: { items: FinancialBreakdown[] }) {
  return (
    <ResponsiveContainer width="100%" height={260}>
      <PieChart margin={{ top: 8, right: 16, bottom: 8, left: 16 }}>
        <Pie data={items} dataKey="revenue" nameKey="label" innerRadius={55} outerRadius={88} paddingAngle={4}>
          {items.map((item, index) => <Cell key={item.label} fill={CHART_COLORS[index % CHART_COLORS.length]} />)}
        </Pie>
        <Tooltip formatter={formatChartCurrency} />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  );
}

function TechnicianChart({ items }: { items: FinancialBreakdown[] }) {
  return (
    <ResponsiveContainer width="100%" height={260}>
      <BarChart data={items} layout="vertical" margin={{ top: 16, right: 32, left: 8, bottom: 18 }}>
        <CartesianGrid strokeDasharray="3 3" horizontal={false} />
        <XAxis type="number" tickLine={false} axisLine={false} tickFormatter={formatChartCurrency} tick={{ fontSize: 11 }} />
        <YAxis type="category" dataKey="label" width={128} tickLine={false} axisLine={false} tick={{ fontSize: 11 }} />
        <Tooltip formatter={formatChartCurrency} />
        <Bar dataKey="revenue" name="Faturamento" fill="#7c3aed" radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

function formatChartQuantity(value: unknown) {
  const amount = Array.isArray(value) ? value[0] : value;
  return typeof amount === "number" || typeof amount === "string"
    ? `${Number(amount)}`
    : "-";
}

function TopItemsChart({
  items,
  metric,
}: {
  items: FinancialBreakdown[];
  metric: RankingMetric;
}) {
  const isQuantity = metric === "quantity";
  return (
    <ResponsiveContainer width="100%" height={260}>
      <BarChart data={items} layout="vertical" margin={{ top: 16, right: 32, left: 8, bottom: 18 }}>
        <CartesianGrid strokeDasharray="3 3" horizontal={false} />
        <XAxis type="number" tickLine={false} axisLine={false} tickFormatter={isQuantity ? formatChartQuantity : formatChartCurrency} tick={{ fontSize: 11 }} />
        <YAxis type="category" dataKey="label" width={128} tickLine={false} axisLine={false} tick={{ fontSize: 11 }} />
        <Tooltip formatter={isQuantity ? formatChartQuantity : formatChartCurrency} />
        <Bar dataKey={isQuantity ? "count" : "revenue"} name={isQuantity ? "Quantidade" : "Faturamento"} fill="#d97706" radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

export function Reports() {
  const [startDate, setStartDate] = useState(currentMonthStart);
  const [endDate, setEndDate] = useState(() => toLocalIsoDate(new Date()));
  const [periodPreset, setPeriodPreset] = useState<PeriodPreset>("month");
  const [technicianId, setTechnicianId] = useState<string | null>(null);
  const [rankingMetric, setRankingMetric] = useState<RankingMetric>("revenue");
  const [rankingLimit, setRankingLimit] = useState(5);
  const [exporting, setExporting] = useState<"csv" | "pdf" | null>(null);
  const [pdfPreview, setPdfPreview] = useState<PdfPreview | null>(null);
  const filters = reportFilters(
    startDate,
    endDate,
    technicianId,
    rankingMetric,
    rankingLimit,
  );
  const usersQuery = useQuery({
    queryKey: ["users"],
    queryFn: fetchUsers,
  });
  const {
    data: report,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["financial-report", startDate, endDate, technicianId, rankingMetric, rankingLimit],
    queryFn: () => invoke<FinancialReport>("get_financial_report", filters),
    placeholderData: keepPreviousData,
  });

  const applyPeriodPreset = (preset: Exclude<PeriodPreset, "custom">) => {
    const dates = periodDates(preset);
    setStartDate(dates.start);
    setEndDate(dates.end);
    setPeriodPreset(preset);
  };

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
          <CardTitle className="text-lg">Filtros gerais</CardTitle>
          <CardDescription>
            Aplique os filtros aos indicadores, gráficos, tabela, CSV e PDF.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <div className="space-y-2 md:col-span-2 lg:col-span-3">
            <span className="text-sm font-medium">Períodos rápidos</span>
            <div className="flex flex-wrap gap-2">
              {(
                [
                  ["today", "Hoje"],
                  ["week", "Últimos 7 dias"],
                  ["month", "Mês atual"],
                  ["quarter", "Trimestre"],
                  ["year", "Ano atual"],
                ] as const
              ).map(([preset, label]) => (
                <Button
                  key={preset}
                  type="button"
                  variant={periodPreset === preset ? "default" : "outline"}
                  size="sm"
                  onClick={() => applyPeriodPreset(preset)}
                >
                  {label}
                </Button>
              ))}
            </div>
          </div>
          <div className="space-y-2">
            <label htmlFor="report-start-date" className="text-sm font-medium">
              Data inicial
            </label>
            <DatePicker
              id="report-start-date"
              value={startDate}
              onChange={(value) => {
                setStartDate(value);
                setPeriodPreset("custom");
              }}
            />
          </div>
          <div className="space-y-2">
            <label htmlFor="report-end-date" className="text-sm font-medium">
              Data final
            </label>
            <DatePicker
              id="report-end-date"
              value={endDate}
              onChange={(value) => {
                setEndDate(value);
                setPeriodPreset("custom");
              }}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Responsável técnico</label>
            <SearchableSelect
              options={usersQuery.data ?? []}
              value={technicianId}
              onSelect={(user) => setTechnicianId(technicianId === user.id ? null : user.id)}
              placeholder="Todos os técnicos"
              searchPlaceholder="Buscar técnico..."
              getKey={(user) => user.id}
              getLabel={(user) => user.name}
              className="w-full"
            />
          </div>
          {invalidPeriod && (
            <p className="text-sm text-destructive md:col-span-2 lg:col-span-3">
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
              description="Custos dos itens e serviços utilizados"
            />
            <FinancialCard
              title="Lucro bruto estimado"
              value={formatCurrency(report.estimatedGrossProfit)}
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
              <Card className="border-primary/20 bg-primary/[0.03]">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base">Filtros dos gráficos</CardTitle>
                  <CardDescription>
                    Personalize o ranking de itens e serviços mais vendidos.
                  </CardDescription>
                </CardHeader>
                <CardContent className="grid gap-5 md:grid-cols-2">
                  <div className="space-y-2">
                    <span className="text-sm font-medium">Ordenar ranking por</span>
                    <div className="flex flex-wrap gap-2">
                      <Button
                        type="button"
                        variant={rankingMetric === "revenue" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setRankingMetric("revenue")}
                      >
                        Faturamento
                      </Button>
                      <Button
                        type="button"
                        variant={rankingMetric === "quantity" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setRankingMetric("quantity")}
                      >
                        Quantidade
                      </Button>
                    </div>
                  </div>
                  <label className="space-y-2">
                    <span className="flex items-center justify-between gap-3 text-sm font-medium">
                      Posições exibidas <output>{rankingLimit}</output>
                    </span>
                    <input
                      type="range"
                      min="5"
                      max="20"
                      step="1"
                      value={rankingLimit}
                      className="h-2 w-full cursor-pointer accent-primary"
                      onChange={(event) => setRankingLimit(Number(event.target.value))}
                    />
                    <span className="flex justify-between text-xs text-muted-foreground">
                      <span>Top 5</span>
                      <span>Top 20</span>
                    </span>
                  </label>
                </CardContent>
              </Card>
              <div className="grid gap-6 md:grid-cols-2">
                <Card>
                  <CardHeader>
                    <CardTitle>Evolução Financeira</CardTitle>
                    <CardDescription>
                      Faturamento e lucro bruto estimado das OS finalizadas por mês.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="bg-white p-5 text-slate-900">
                      <MonthlyTrendChart items={report.byMonth} />
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-4">
                    <CardTitle className="text-base">Faturamento por Categoria</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="bg-white p-5 text-slate-900">
                      <CategoryChart items={report.byItemType} />
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-4">
                    <CardTitle className="text-base">Faturamento por Técnico</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="bg-white p-5 text-slate-900">
                      <TechnicianChart items={report.byTechnician} />
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-4">
                    <CardTitle className="text-base">Itens e Serviços Mais Vendidos</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="bg-white p-5 text-slate-900">
                      <p className="mb-2 text-sm font-semibold">
                        Top {report.rankingLimit} por {report.rankingMetric === "quantity" ? "quantidade" : "faturamento"}
                      </p>
                      <TopItemsChart items={report.topItems} metric={report.rankingMetric} />
                    </div>
                  </CardContent>
                </Card>
              </div>
              <div className="grid gap-6 xl:grid-cols-2">
                <Card>
                  <CardHeader>
                    <CardTitle>Por Técnico</CardTitle>
                    <CardDescription>
                      Resultado por responsável pela ordem.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.byTechnician}
                      label="Técnico"
                      countLabel="Ordens"
                    />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Por Categoria</CardTitle>
                    <CardDescription>
                      Peças e serviços utilizados nas ordens.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.byItemType}
                      label="Categoria"
                      countLabel="Ordens"
                    />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Itens e Serviços Mais Vendidos</CardTitle>
                    <CardDescription>
                      Top {report.rankingLimit} por {report.rankingMetric === "quantity" ? "quantidade vendida" : "faturamento"} nas ordens finalizadas.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <BreakdownTable
                      items={report.topItems}
                      label="Item / Serviço"
                      countLabel="Quantidade"
                    />
                  </CardContent>
                </Card>
              </div>
              <Card>
                <CardHeader>
                  <CardTitle>Evolução Mensal</CardTitle>
                  <CardDescription>
                    Faturamento e lucro bruto estimado por mês dentro do período.
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
                          <TableHead className="text-right">Lucro bruto estimado</TableHead>
                          <TableHead className="text-right">
                            Ordens finalizadas
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
        <Suspense fallback={<span className="sr-only">Carregando pré-visualização do PDF...</span>}>
          <PdfPreviewDialog
            preview={pdfPreview}
            onClose={() => setPdfPreview(null)}
          />
        </Suspense>
      )}
    </div>
  );
}
