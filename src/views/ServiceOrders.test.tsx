import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
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

describe("ServiceOrders", () => {
  it("opens customer history only from the customer name", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_service_orders") {
        return Promise.resolve([
          {
            id: "order-1",
            customerId: "customer-1",
            customerName: "Cliente Teste",
            equipment: "iPhone 15",
            status: "Orçamento",
            totalPrice: 0,
            createdAt: "2026-01-01T00:00:00Z",
            displayId: "OS-000001",
            discountBasisPoints: 0,
          },
        ]);
      }
      return Promise.resolve([]);
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <ServiceOrders />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    const customerName = await screen.findByRole("button", {
      name: "Cliente Teste",
    });
    await user.click(customerName);
    expect(openCustomerHistory).toHaveBeenCalledWith("customer-1");
    expect(openServiceOrder).not.toHaveBeenCalled();

    await user.click(screen.getByText("iPhone 15"));
    await waitFor(() => expect(openServiceOrder).toHaveBeenCalledWith("order-1"));
  });
});
