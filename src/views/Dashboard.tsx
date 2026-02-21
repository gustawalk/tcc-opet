import { useQuery } from "@tanstack/react-query";
import { invoke } from '@tauri-apps/api/core';
import { useNavigate } from "react-router-dom";
import {
  TrendingUp,
  Wallet,
  ShoppingBag,
  Wrench,
  Clock,
  CheckCircle2,
  AlertTriangle,
  ArrowRight,
  Plus
} from "lucide-react";
import { FinancialCard } from "@/components/shared/FinancialCard";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow
} from "@/components/ui/table";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { DashboardData } from "@/lib/types";
import { formatCurrency } from "@/lib/formatters";

// Mock function representing the Tauri invoke("get_dashboard_data")
const fetchDashboardData = async (): Promise<DashboardData> => {
  // Simulate delay
  await new Promise(resolve => setTimeout(resolve, 1000));

  return {
    summary: {
      totalRevenue: 12450.00,
      netProfit: 8120.00,
      partsInUseCost: 4330.00,
      activeOrdersCount: 24,
      revenueTrend: { value: "12%", isPositive: true },
      profitTrend: { value: "8%", isPositive: true },
    },
    recentOrders: [
      { id: "OS-1234", customerName: "Maria Silva", equipment: "Notebook Dell G15", status: "Em Manutenção", createdAt: "2023-10-15", totalPrice: 450.00 },
      { id: "OS-1235", customerName: "João Pereira", equipment: "iPhone 13 Pro", status: "Aguardando Peça", createdAt: "2023-10-16", totalPrice: 1200.00 },
      { id: "OS-1236", customerName: "Empresa ABC", equipment: "Impressora HP LaserJet", status: "Finalizada", createdAt: "2023-10-17", totalPrice: 320.00 },
      { id: "OS-1237", customerName: "Carlos Oliveira", equipment: "PlayStation 5", status: "Orçamento", createdAt: "2023-10-18", totalPrice: 0.00 },
    ],
    inventoryAlerts: [
      { id: "p1", name: "Pasta Térmica Arctic Silver", currentStock: 2, minStock: 5 },
      { id: "p2", name: "Conector de Carga iPhone 11", currentStock: 1, minStock: 3 },
      { id: "p3", name: "Bateria Dell XPS 13", currentStock: 0, minStock: 2 },
    ],
    statusCounts: [
      { status: "Orçamento", count: 8 },
      { status: "Em Manutenção", count: 12 },
      { status: "Finalizada", count: 45 },
    ]
  };
};

export function Dashboard() {
  const navigate = useNavigate();
  const { data, isLoading, error } = useQuery({
    queryKey: ["dashboard-data"],
    queryFn: fetchDashboardData,
  });

  const test = async () => {
    const res = await invoke<string>("greet", { name: "Gustavo" })
    console.log(res)
  }

  if (isLoading) {
    return (
      <div className="flex flex-col gap-8 animate-in fade-in duration-500">
        <div className="flex items-center justify-between">
          <Skeleton className="h-10 w-48" />
          <Skeleton className="h-10 w-32" />
        </div>
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
          {[1, 2, 3, 4].map(i => <Skeleton key={i} className="h-32 w-full" />)}
        </div>
        <div className="grid gap-6 md:grid-cols-7">
          <Skeleton className="col-span-4 h-96 w-full" />
          <Skeleton className="col-span-3 h-96 w-full" />
        </div>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="flex flex-col items-center justify-center h-[50vh] gap-4">
        <AlertTriangle className="h-12 w-12 text-destructive" />
        <h3 className="text-xl font-bold">Erro ao carregar dashboard</h3>
        <p className="text-muted-foreground text-center max-w-sm">
          Não foi possível conectar ao banco de dados local. Tente reiniciar o sistema.
        </p>
        <Button onClick={() => window.location.reload()}>Tentar Novamente</Button>
      </div>
    );
  }

  const { summary, recentOrders, inventoryAlerts, statusCounts } = data;

  return (
    <div className="flex flex-col gap-8 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Visão Geral</h2>
          <p className="text-muted-foreground mt-1">
            Olá, administrador. Veja o desempenho da sua assistência técnica hoje.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">Exportar Relatório</Button>
          <Button size="sm" className="gap-2" onClick={() => navigate("/os/new")} >
            <Plus className="h-4 w-4" /> Nova Ordem
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        <FinancialCard
          title="Faturamento Bruto"
          value={formatCurrency(summary.totalRevenue)}
          icon={TrendingUp}
          description="Total acumulado do mês"
          trend={summary.revenueTrend}
        />
        <FinancialCard
          title="Lucro Líquido"
          value={formatCurrency(summary.netProfit)}
          icon={Wallet}
          description="Faturamento - Custo de Peças"
          trend={summary.profitTrend}
        />
        <FinancialCard
          title="Peças em Uso"
          value={formatCurrency(summary.partsInUseCost)}
          icon={ShoppingBag}
          description="Custo total de peças em OS abertas"
        />
        <FinancialCard
          title="Ordens Ativas"
          value={summary.activeOrdersCount.toString()}
          icon={Wrench}
          description="Total de serviços não finalizados"
        />
      </div>

      <div className="grid gap-6 md:grid-cols-7">
        <Card className="col-span-4">
          <CardHeader className="flex flex-row items-center justify-between">
            <div>
              <CardTitle>Ordens Recentes</CardTitle>
              <CardDescription>As últimas atividades registradas no sistema.</CardDescription>
            </div>
            <Button variant="ghost" size="sm" className="gap-2">
              Ver Todas <ArrowRight className="h-4 w-4" />
            </Button>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[100px]">ID</TableHead>
                  <TableHead>Cliente</TableHead>
                  <TableHead className="hidden md:table-cell">Equipamento</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Valor</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {recentOrders.map((os) => (
                  <TableRow key={os.id} className="cursor-pointer hover:bg-muted/50 transition-colors">
                    <TableCell className="font-medium">{os.id}</TableCell>
                    <TableCell>{os.customerName}</TableCell>
                    <TableCell className="hidden md:table-cell text-muted-foreground">{os.equipment}</TableCell>
                    <TableCell>
                      <Badge variant={
                        os.status === "Finalizada" ? "secondary" :
                          os.status === "Em Manutenção" ? "default" :
                            os.status === "Aguardando Peça" ? "destructive" : "outline"
                      }>
                        {os.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right font-medium">{formatCurrency(os.totalPrice)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <div className="col-span-3 flex flex-col gap-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Resumo de Status</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {statusCounts.map((s, i) => (
                <div key={s.status}>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div className={`p-1.5 rounded-md ${s.status === 'Orçamento' ? 'bg-blue-100 text-blue-600' :
                        s.status === 'Em Manutenção' ? 'bg-amber-100 text-amber-600' :
                          'bg-green-100 text-green-600'
                        }`}>
                        {s.status === 'Orçamento' && <Clock className="h-4 w-4" />}
                        {s.status === 'Em Manutenção' && <Wrench className="h-4 w-4" />}
                        {s.status === 'Finalizada' && <CheckCircle2 className="h-4 w-4" />}
                      </div>
                      <span className="text-sm font-medium">{s.status === 'Finalizada' ? 'Finalizadas (Mês)' : s.status}</span>
                    </div>
                    <span className="font-bold">{s.count.toString().padStart(2, '0')}</span>
                  </div>
                  {i < statusCounts.length - 1 && <Separator className="mt-4" />}
                </div>
              ))}
            </CardContent>
            <CardFooter>
              <Button variant="outline" className="w-full text-xs h-8">Ver Dashboard Financeiro Completo</Button>
            </CardFooter>
          </Card>

          <Card className="border-destructive/20 bg-destructive/5">
            <CardHeader className="pb-3">
              <CardTitle className="text-lg flex items-center gap-2 text-destructive">
                <AlertTriangle className="h-5 w-5" /> Alertas de Estoque
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {inventoryAlerts.map((alert) => (
                <div key={alert.id} className="flex flex-col gap-1">
                  <div className="flex justify-between text-sm">
                    <span className="font-medium">{alert.name}</span>
                    <span className={alert.currentStock === 0 ? "text-destructive font-bold" : "text-muted-foreground"}>
                      {alert.currentStock} un.
                    </span>
                  </div>
                  <div className="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                    <div
                      className={`h-full ${alert.currentStock === 0 ? "bg-destructive" : "bg-amber-500"}`}
                      style={{ width: `${Math.max((alert.currentStock / alert.minStock) * 100, 5)}%` }}
                    />
                  </div>
                </div>
              ))}
            </CardContent>
            <CardFooter>
              <Button variant="destructive" size="sm" className="w-full h-8">Fazer Reposição</Button>
            </CardFooter>
          </Card>
        </div>
      </div>
    </div>
  );
}
