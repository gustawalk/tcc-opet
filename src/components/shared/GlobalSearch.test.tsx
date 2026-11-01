import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GlobalSearch } from "@/components/shared/GlobalSearch";
import { dataCommand } from "@/lib/data-client";

const { openChecklistTemplate, openInventoryItemCreate } = vi.hoisted(() => ({ openChecklistTemplate: vi.fn(), openInventoryItemCreate: vi.fn() }));

vi.mock("@/components/ui/sidebar", () => ({ useSidebar: () => ({ toggleSidebar: vi.fn() }) }));
vi.mock("@/components/shared/ServiceOrderDrawerProvider", () => ({ useServiceOrderDrawer: () => ({ openServiceOrder: vi.fn() }) }));
vi.mock("@/components/shared/CustomerDrawerProvider", () => ({ useCustomerDrawer: () => ({ openCustomerHistory: vi.fn() }) }));
vi.mock("@/components/shared/InventoryDrawerProvider", () => ({ useInventoryDrawer: () => ({ openInventoryItem: vi.fn() }) }));
vi.mock("@/components/shared/ChecklistTemplateDrawerProvider", () => ({ useChecklistTemplateDrawer: () => ({ openChecklistTemplate }) }));
vi.mock("@/components/shared/InventoryItemCreateProvider", () => ({ useInventoryItemCreate: () => ({ openInventoryItemCreate }) }));
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

    await user.keyboard("{Control>}k{/Control}");
    await user.type(screen.getByPlaceholderText(/Buscar clientes/), "checklist");
    await waitFor(() => expect(screen.getByText("Checklist notebook")).toBeInTheDocument());
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");

    expect(openChecklistTemplate).toHaveBeenCalledWith(expect.objectContaining({ id: "template-1", title: "Checklist notebook" }));
    expect(screen.getByRole("status")).toHaveTextContent("/|");
  });

  it("opens a global item creation sheet without changing the current route", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><GlobalSearch /><LocationState /></MemoryRouter>);

    await user.keyboard("{Control>}k{/Control}");

    expect(screen.getByText("Novo item")).toBeInTheDocument();
    expect(screen.getByText("Novo serviço")).toBeInTheDocument();
    expect(screen.getByText("Nova peça")).toBeInTheDocument();
    expect(screen.getByText("Novo cliente")).toBeInTheDocument();
    expect(screen.getByText("Novo checklist")).toBeInTheDocument();

    await user.click(screen.getByText("Novo item"));
    expect(openInventoryItemCreate).toHaveBeenCalledWith("item");
    expect(screen.getByRole("status")).toHaveTextContent("/|");
  });

  it("opens header results only on interaction and keeps Ctrl K in a modal", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    const header = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    expect(screen.queryByRole("region", { name: "Resultados da busca" })).not.toBeInTheDocument();
    await user.click(header);
    await user.type(header, "checklist");
    await waitFor(() => expect(screen.getByText("Checklist notebook")).toBeInTheDocument());
    await user.click(screen.getByRole("tab", { name: "Clientes 0" }));
    expect(screen.queryByText("Checklist notebook")).not.toBeInTheDocument();
    expect(screen.getByText("Nenhum resultado encontrado.")).toBeInTheDocument();
    await user.keyboard("{Control>}k{/Control}");
    expect(screen.queryByRole("region", { name: "Resultados da busca" })).not.toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(header).not.toHaveFocus();
  });

  it("dismisses header results with Escape or an outside click", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><GlobalSearch /><button>Outside</button></MemoryRouter>);
    const header = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    await user.click(header);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("region")).not.toBeInTheDocument();
    await user.click(header);
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(screen.queryByRole("region")).not.toBeInTheDocument();
  });

  it("reports lookup failures and recovers on the next search", async () => {
    const user = userEvent.setup();
    vi.mocked(dataCommand).mockRejectedValueOnce(new Error("Host unavailable"));
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    const header = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    await user.click(header);
    await user.type(header, "missing");
    expect(await screen.findByRole("alert")).toHaveTextContent("Não foi possível buscar");
    expect(screen.getByRole("tab", { name: "Todos" })).toHaveTextContent(/^Todos$/);
    const countSlot = screen.getByRole("tab", { name: "Todos" }).querySelector("span");
    expect(countSlot).toHaveClass("w-7", "invisible");
    expect(screen.getByRole("tablist")).toHaveClass("grid", "w-full");
    await user.clear(header);
    await user.type(header, "checklist");
    expect(await screen.findByText("Checklist notebook")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("ignores an older response after the query changes", async () => {
    const user = userEvent.setup();
    let resolveOld!: (value: unknown) => void;
    vi.mocked(dataCommand).mockReturnValueOnce(new Promise((resolve) => { resolveOld = resolve; }));
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    const header = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    await user.click(header);
    await user.type(header, "older");
    await waitFor(() => expect(dataCommand).toHaveBeenCalledWith("get_customers_page", expect.objectContaining({ search: "older" })));
    await user.clear(header);
    await user.type(header, "newer");
    expect(await screen.findByText("Checklist notebook")).toBeInTheDocument();
    await act(async () => resolveOld({ items: [{ id: "old", name: "Outdated customer", phone: "123", email: "" }], total: 1 }));
    expect(screen.queryByText("Outdated customer")).not.toBeInTheDocument();
    expect(screen.getByText("Checklist notebook")).toBeInTheDocument();
  });

  it.each(["header", "modal"])("shows backend totals rather than preview lengths in the %s search", async (surface) => {
    const user = userEvent.setup();
    vi.mocked(dataCommand)
      .mockResolvedValueOnce({ items: Array.from({ length: 5 }, (_, index) => ({ id: `customer-${index}`, name: `Customer ${index}`, phone: "123", email: "" })), total: 12 })
      .mockResolvedValueOnce({ items: [{ id: "order-1", customerName: "Customer", displayId: "OS-1", status: "open" }], total: 9 })
      .mockResolvedValueOnce({ items: [], total: 0 })
      .mockResolvedValueOnce({ items: [{ id: "template-1", title: "Template", items: [] }], total: 7 });
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    if (surface === "modal") await user.keyboard("{Control>}k{/Control}");
    else await user.click(screen.getByRole("textbox", { name: "Busca global no cabeçalho" }));
    expect(screen.getByRole("tab", { name: "Todos" })).toHaveTextContent(/^Todos$/);
    const countSlot = screen.getByRole("tab", { name: "Todos" }).querySelector("span");
    expect(countSlot).toHaveClass("w-7", "invisible");
    expect(screen.getByRole("tablist")).toHaveClass("grid", "w-full");
    const input = screen.getByRole("textbox", { name: surface === "modal" ? "Busca global no modal" : "Busca global no cabeçalho" });
    await user.type(input, "novo");
    expect(screen.getByRole("tablist").querySelector(".animate-spin")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Buscando...");
    expect(document.querySelectorAll(".animate-spin")).toHaveLength(1);
    expect(await screen.findByRole("tab", { name: "Todos 28" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Todos 28" }).querySelector("span")).toBe(countSlot);
    expect(countSlot).toHaveClass("w-7");
    expect(countSlot).not.toHaveClass("invisible");
    expect(screen.getByRole("tab", { name: "Clientes 12" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Ordens de serviço 9" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Estoque 0" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Modelos 7" })).toBeInTheDocument();
    expect(screen.getByText("Exibindo 5 de 12 resultados")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Clientes 12" }));
    expect(screen.getByRole("tab", { name: "Todos 28" })).toBeInTheDocument();
    await user.clear(input);
    expect(screen.getByRole("tab", { name: "Clientes" })).toHaveTextContent(/^Clientes$/);
    expect(screen.queryByText("Exibindo 5 de 12 resultados")).not.toBeInTheDocument();
  });

  it("debounces typing into one batch of four limited requests and reuses it for tabs", async () => {
    const user = userEvent.setup();
    vi.mocked(dataCommand).mockClear();
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    const input = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    await user.click(input);
    await user.type(input, "checklist");
    expect(dataCommand).not.toHaveBeenCalled();
    expect(await screen.findByRole("tab", { name: "Modelos 1" })).toBeInTheDocument();
    expect(dataCommand).toHaveBeenCalledTimes(4);
    for (const [, payload] of vi.mocked(dataCommand).mock.calls) {
      expect(payload).toEqual({ limit: 5, offset: 0, search: "checklist" });
    }
    await user.click(screen.getByRole("tab", { name: "Modelos 1" }));
    expect(screen.getByText("Checklist notebook")).toBeInTheDocument();
    expect(dataCommand).toHaveBeenCalledTimes(4);
  });

  it("renders an order status as a badge without a leading separator when its number is empty", async () => {
    const user = userEvent.setup();
    vi.mocked(dataCommand)
      .mockResolvedValueOnce({ items: [], total: 0 })
      .mockResolvedValueOnce({ items: [{ id: "order-status", customerName: "Roberto Mendes", displayId: "", status: "Em Manutenção" }], total: 1 });
    render(<MemoryRouter><GlobalSearch /></MemoryRouter>);
    const input = screen.getByRole("textbox", { name: "Busca global no cabeçalho" });
    await user.click(input);
    await user.type(input, "roberto");
    const status = await screen.findByText("Em Manutenção");
    expect(status).toHaveClass("rounded-md", "border");
    const order = screen.getByRole("button", { name: "Roberto Mendes Em Manutenção" });
    expect(order).not.toHaveTextContent("·");
    expect(screen.getByRole("tab", { name: "Todos 2" })).toHaveClass("justify-center", "text-center");
  });
});
