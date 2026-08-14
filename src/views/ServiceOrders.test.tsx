import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ServiceOrders } from "@/views/ServiceOrders";

const openServiceOrder = vi.fn();
const openCustomerHistory = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@/components/shared/ServiceOrderDrawerProvider", () => ({
  useServiceOrderDrawer: () => ({ openServiceOrder }),
}));
vi.mock("@/components/shared/CustomerDrawerProvider", () => ({
  useCustomerDrawer: () => ({ openCustomerHistory }),
}));

const mockedInvoke = vi.mocked(invoke);
const order = {
  id: "order-1",
  customerId: "customer-1",
  customerName: "Cliente Teste",
  equipment: "iPhone 15",
  status: "Orçamento",
  totalPrice: 0,
  createdAt: "2026-01-01T00:00:00Z",
  displayId: "OS-000001",
  discountBasisPoints: 0,
};
const customer = {
  id: "customer-1",
  name: "Cliente Teste",
  phone: "41999999999",
  email: "cliente@example.com",
  address: "Rua Teste, 123",
};
const user = {
  id: "user-1",
  name: "Funcionário Teste",
  email: "funcionario@example.com",
};

function renderOrders() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <ServiceOrders />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ServiceOrders", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_service_orders_page") {
        return Promise.resolve({ items: [order], total: 1 });
      }
      if (command === "get_customers_page") {
        return Promise.resolve({ items: [customer], total: 1 });
      }
      if (command === "get_users_page") {
        return Promise.resolve({ items: [user], total: 1 });
      }
      return Promise.resolve(null);
    });
  });

  it("loads only 20 customers and employees by default", async () => {
    renderOrders();

    expect(
      await screen.findByRole("heading", { name: "Ordens de Serviço" }),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_customers_page", {
        limit: 20,
        offset: 0,
        search: "",
      });
      expect(mockedInvoke).toHaveBeenCalledWith("get_users_page", {
        limit: 20,
        offset: 0,
        search: "",
      });
    });
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_customers");
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_users");
  });

  it("searches customers remotely from the filter component", async () => {
    const userEventApi = userEvent.setup();
    renderOrders();

    await userEventApi.click(
      await screen.findByRole("button", { name: "Filtrar por cliente" }),
    );
    await userEventApi.type(
      screen.getByPlaceholderText("Buscar cliente..."),
      "9999",
    );

    await waitFor(
      () => {
        expect(mockedInvoke).toHaveBeenCalledWith("get_customers_page", {
          limit: 20,
          offset: 0,
          search: "9999",
        });
      },
      { timeout: 1_000 },
    );
  });

  it("keeps existing filters visible and sends advanced creation dates", async () => {
    const userEventApi = userEvent.setup();
    renderOrders();

    expect(await screen.findByPlaceholderText(/Buscar por ID/)).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Filtrar por cliente" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Filtrar por funcionário" }),
    ).toBeVisible();
    const details = screen.getByText("Mais filtros").closest("details");
    expect(details).not.toHaveAttribute("open");

    await userEventApi.click(screen.getByText("Mais filtros"));
    const today = new Date();
    const todayIso = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    const todayLabel = today.toLocaleDateString("pt-BR", {
      weekday: "long",
      day: "numeric",
      month: "long",
      year: "numeric",
    });
    await userEventApi.click(screen.getByLabelText("Data inicial da criação"));
    await userEventApi.click(
      screen.getByRole("gridcell", { name: todayLabel }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "get_service_orders_page",
        expect.objectContaining({ createdDateFrom: todayIso }),
      );
    });
  });

  it("opens customer history only from the customer name", async () => {
    const userEventApi = userEvent.setup();
    renderOrders();

    const customerName = await screen.findByRole("button", {
      name: "Cliente Teste",
    });
    await userEventApi.click(customerName);
    expect(openCustomerHistory).toHaveBeenCalledWith("customer-1");
    expect(openServiceOrder).not.toHaveBeenCalled();

    await userEventApi.click(screen.getByText("iPhone 15"));
    await waitFor(() => expect(openServiceOrder).toHaveBeenCalledWith("order-1"));
  });
});
