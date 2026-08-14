import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ServiceOrderCreate } from "@/views/ServiceOrderCreate";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@/lib/errors", () => ({
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const existingCustomer = {
  id: "customer-1",
  name: "Cliente Existente",
  phone: "41999999999",
  email: "cliente@example.com",
  address: "Rua de Teste, 123",
};

function renderCreate(initialEntries = ["/os/new"]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={initialEntries} initialIndex={initialEntries.length - 1}>
        <Routes>
          <Route path="/os/new" element={<ServiceOrderCreate />} />
          <Route path="/os" element={<p>Lista de ordens</p>} />
          <Route path="/anterior" element={<p>Tela anterior</p>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ServiceOrderCreate", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_customers") return Promise.resolve([existingCustomer]);
      if (
        command === "get_users" ||
        command === "get_checklist_templates" ||
        command === "get_inventory_items"
      ) {
        return Promise.resolve([]);
      }
      if (command === "create_full_service_order") {
        return Promise.resolve("order-created");
      }
      return Promise.resolve(null);
    });
  });

  it("shows customer matches only while the combobox has focus", async () => {
    const user = userEvent.setup();
    renderCreate();
    const customerInput = await screen.findByLabelText("Nome do Cliente");

    await user.type(customerInput, "Cliente");
    expect(
      screen.getByRole("button", { name: /Cliente Existente/ }),
    ).toBeInTheDocument();

    await user.click(screen.getByLabelText("Telefone"));
    expect(
      screen.queryByRole("button", { name: /Cliente Existente/ }),
    ).not.toBeInTheDocument();
  });

  it("does not navigate back when Enter is pressed in a regular input", async () => {
    const user = userEvent.setup();
    renderCreate(["/anterior", "/os/new"]);
    const customerInput = await screen.findByLabelText("Nome do Cliente");

    await user.type(customerInput, "Cliente{Enter}");

    expect(screen.getByText("Nova Ordem")).toBeInTheDocument();
    expect(screen.queryByText("Tela anterior")).not.toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      "create_full_service_order",
      expect.anything(),
    );
  });

  it("returns to the service order list after saving", async () => {
    const user = userEvent.setup();
    renderCreate();

    await user.type(await screen.findByLabelText("Nome do Cliente"), "Novo Cliente");
    await user.type(screen.getByLabelText("Telefone"), "41999999999");
    await user.type(screen.getByLabelText("E-mail"), "novo@example.com");
    await user.type(screen.getByLabelText("Endereço Completo"), "Rua de Teste, 123");
    await user.type(screen.getByLabelText("Equipamento"), "Notebook");
    await user.type(
      screen.getByLabelText("Descrição do Problema"),
      "Equipamento não inicializa corretamente",
    );
    await user.click(screen.getByRole("button", { name: "Criar ordem" }));

    expect(await screen.findByText("Lista de ordens")).toBeInTheDocument();
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_full_service_order",
        expect.anything(),
      );
    });
  });
});
