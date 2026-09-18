import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GlobalSearch } from "@/components/shared/GlobalSearch";

vi.mock("@/components/ui/sidebar", () => ({ useSidebar: () => ({ toggleSidebar: vi.fn() }) }));
vi.mock("@/components/shared/ServiceOrderDrawerProvider", () => ({ useServiceOrderDrawer: () => ({ openServiceOrder: vi.fn() }) }));
vi.mock("@/components/shared/CustomerDrawerProvider", () => ({ useCustomerDrawer: () => ({ openCustomerHistory: vi.fn() }) }));
vi.mock("@/components/shared/InventoryDrawerProvider", () => ({ useInventoryDrawer: () => ({ openInventoryItem: vi.fn() }) }));
vi.mock("@/lib/data-client", () => ({
  dataCommand: vi.fn((command: string) => {
    if (command === "get_checklist_templates_page") {
      return Promise.resolve({ items: [{ id: "template-1", title: "Checklist notebook", items: ["Liga"], createdAt: "2026-01-01" }], total: 1 });
    }
    return Promise.resolve({ items: [], total: 0 });
  }),
}));

function LocationState() {
  const location = useLocation();
  const state = location.state as { detailTemplate?: { title: string } } | null;
  return <output>{`${location.pathname}${location.search}|${state?.detailTemplate?.title ?? ""}`}</output>;
}

describe("GlobalSearch", () => {
  afterEach(cleanup);

  it("opens a checklist template selected with Enter", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><GlobalSearch /><LocationState /></MemoryRouter>);

    await user.click(screen.getByRole("button", { name: /Buscar/ }));
    await user.type(screen.getByPlaceholderText(/Buscar clientes/), "checklist");
    await waitFor(() => expect(screen.getByText("Checklist notebook")).toBeInTheDocument());
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("/templates|Checklist notebook"));
  });

  it("offers creation actions and routes a new item action to its creation mode", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><GlobalSearch /><LocationState /></MemoryRouter>);

    await user.click(screen.getByRole("button", { name: /Buscar/ }));

    expect(screen.getByText("Novo item")).toBeInTheDocument();
    expect(screen.getByText("Novo serviço")).toBeInTheDocument();
    expect(screen.getByText("Nova peça")).toBeInTheDocument();
    expect(screen.getByText("Novo cliente")).toBeInTheDocument();
    expect(screen.getByText("Novo checklist")).toBeInTheDocument();

    await user.click(screen.getByText("Novo item"));
    expect(screen.getByRole("status")).toHaveTextContent("/inventory?new=item|");
  });
});
