import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { PaginationPreview } from "@/views/PaginationPreview";
import type { Page, ServiceOrder } from "@/lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

function order(id: string): ServiceOrder {
  return {
    id,
    customerId: "customer-1",
    customerName: "Cliente Teste",
    equipment: "iPhone 15",
    description: "Reparo",
    status: "Orçamento",
    totalPrice: 0,
    createdAt: "2026-01-01T00:00:00Z",
    displayId: id,
    discountBasisPoints: 0,
  };
}

describe("PaginationPreview", () => {
  it("keeps the requested page selected while its data loads", async () => {
    let resolveSecondPage: (page: Page<ServiceOrder>) => void = () => undefined;
    const secondPage = new Promise<Page<ServiceOrder>>((resolve) => {
      resolveSecondPage = resolve;
    });
    const scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollIntoView = scrollIntoView;

    mockedInvoke.mockImplementation((_command, args) => {
      const offset = (args as { offset?: number } | undefined)?.offset ?? 0;
      if (offset === 20) return secondPage;
      return Promise.resolve({ items: [order("OS-000001")], total: 40 });
    });

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <PaginationPreview />
      </QueryClientProvider>,
    );

    await screen.findByText("OS-000001");
    await user.click(screen.getByRole("button", { name: "2" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "2" })).toHaveClass("bg-primary");
    });
    expect(scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "start",
    });

    resolveSecondPage({ items: [order("OS-000021")], total: 40 });
    expect(await screen.findByText("OS-000021")).toBeInTheDocument();
  });
});
