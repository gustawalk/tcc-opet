import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Reports } from "@/views/Reports";
import type { FinancialReport } from "@/lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("recharts", () => {
  const Container = ({ children }: { children?: React.ReactNode }) => (
    <div>{children}</div>
  );
  const Empty = () => null;
  return {
    ResponsiveContainer: Container,
    AreaChart: Empty,
    BarChart: Empty,
    PieChart: Empty,
    Area: Empty,
    Bar: Empty,
    CartesianGrid: Empty,
    Cell: Empty,
    Legend: Empty,
    Pie: Container,
    Tooltip: Empty,
    XAxis: Empty,
    YAxis: Empty,
  };
});

const report: FinancialReport = {
  startDate: "2026-01-01",
  endDate: "2026-01-31",
  totalRevenue: 50_000,
  totalCost: 5_000,
  estimatedGrossProfit: 45_000,
  averageTicket: 50_000,
  finalizedOrders: 1,
  newCustomers: 1,
  newOrders: 1,
  completionRate: 100,
  cancelledOrders: 0,
  cancellationRate: 0,
  averageTurnaroundHours: 12,
  returningCustomers: 0,
  totalDiscounts: 0,
  rankingMetric: "revenue",
  rankingLimit: 5,
  byTechnician: [],
  byItemType: [],
  topItems: [
    {
      key: "item-1|part|Tela",
      inventoryItemId: "item-1",
      label: "Tela",
      itemType: "part",
      displayLabel: "Tela (Peça · item-1)",
      revenue: 10_000,
      cost: 1_000,
      profit: 9_000,
      count: 1,
    },
    {
      key: "item-2|part|Tela",
      inventoryItemId: "item-2",
      label: "Tela",
      itemType: "part",
      displayLabel: "Tela (Peça · item-2)",
      revenue: 40_000,
      cost: 4_000,
      profit: 36_000,
      count: 2,
    },
  ],
  byMonth: [],
};

describe("Reports", () => {
  afterEach(cleanup);

  it("renders equal-name inventory items as separate financial rows", async () => {
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === "get_financial_report") return Promise.resolve(report);
      if (command === "get_users") return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <Reports />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("Tela (Peça · item-1)")).toBeInTheDocument();
    expect(screen.getByText("Tela (Peça · item-2)")).toBeInTheDocument();
  });
});
