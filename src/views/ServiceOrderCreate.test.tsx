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
const existingEmployee = {
  id: "employee-1",
  name: "Funcionário Existente",
  email: "funcionario@example.com",
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
      if (command === "get_customers_page") {
        return Promise.resolve({ items: [existingCustomer], total: 1 });
      }
      if (command === "get_users_page") {
        return Promise.resolve({ items: [existingEmployee], total: 1 });
      }
      if (
        command === "get_checklist_templates" ||
        command === "get_inventory_items"
      ) {
        return Promise.resolve([]);
      }
      if (command === "create_user") return Promise.resolve("employee-created");
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

  it("loads customers on demand and debounces remote search", async () => {
    const user = userEvent.setup();
    renderCreate();
    const customerInput = await screen.findByLabelText("Nome do Cliente");

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_customers");
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_customers_page", expect.anything());

    await user.click(customerInput);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_customers_page", {
        limit: 20,
        offset: 0,
        search: "",
      });
    });

    await user.type(customerInput, "Outro");
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_customers_page", {
        limit: 20,
        offset: 0,
        search: "Outro",
      });
    });
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_customers");
  });

  it("clears reused customer identity when the selected name changes", async () => {
    const user = userEvent.setup();
    renderCreate();
    const customerInput = await screen.findByLabelText("Nome do Cliente");

    await user.type(customerInput, "Cliente");
    await user.click(await screen.findByRole("button", { name: /Cliente Existente/ }));
    await user.clear(customerInput);
    await user.type(customerInput, "Novo Cliente");
    await user.type(screen.getByLabelText("Equipamento"), "Notebook");
    await user.type(
      screen.getByLabelText("Descrição do Problema"),
      "Equipamento não inicializa corretamente",
    );
    await user.click(screen.getByRole("button", { name: "Criar ordem" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_full_service_order",
        expect.objectContaining({
          request: expect.objectContaining({
            customerAction: expect.objectContaining({
              type: "new",
              name: "Novo Cliente",
            }),
          }),
        }),
      );
    });
  });

  it("keeps valid new-customer submission available after lookup failure", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_customers_page") return Promise.reject(new Error("offline"));
      if (
        command === "get_checklist_templates" ||
        command === "get_inventory_items"
      ) {
        return Promise.resolve([]);
      }
      if (command === "create_full_service_order") return Promise.resolve("order-created");
      return Promise.resolve(null);
    });
    const user = userEvent.setup();
    renderCreate();
    const customerInput = await screen.findByLabelText("Nome do Cliente");

    await user.type(customerInput, "Novo Cliente");
    expect(
      await screen.findByText("Não foi possível buscar clientes. Você ainda pode cadastrar um novo cliente."),
    ).toBeInTheDocument();
    await user.type(screen.getByLabelText("Telefone"), "41999999999");
    await user.type(screen.getByLabelText("E-mail"), "novo@example.com");
    await user.type(screen.getByLabelText("Endereço Completo"), "Rua de Teste, 123");
    await user.type(screen.getByLabelText("Equipamento"), "Notebook");
    await user.type(
      screen.getByLabelText("Descrição do Problema"),
      "Equipamento não inicializa corretamente",
    );
    await user.click(screen.getByRole("button", { name: "Criar ordem" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_full_service_order",
        expect.anything(),
      );
    });
  });

  it("loads technicians on demand and retains a selection outside the current page", async () => {
    mockedInvoke.mockImplementation((command, args) => {
      if (command === "get_customers_page") {
        return Promise.resolve({ items: [existingCustomer], total: 1 });
      }
      if (command === "get_users_page") {
        const search = (args as { search?: string } | undefined)?.search;
        return Promise.resolve({
          items: search ? [] : [existingEmployee],
          total: search ? 0 : 1,
        });
      }
      if (
        command === "get_checklist_templates" ||
        command === "get_inventory_items"
      ) {
        return Promise.resolve([]);
      }
      return Promise.resolve(null);
    });
    const user = userEvent.setup();
    renderCreate();
    const employeeSelector = await screen.findByRole("button", {
      name: "Selecione um responsável...",
    });

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_users");
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_users_page", expect.anything());

    await user.click(employeeSelector);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_users_page", {
        limit: 20,
        offset: 0,
        search: "",
      });
    });
    await user.click(await screen.findByRole("option", { name: /Funcionário Existente/ }));

    await user.click(
      screen.getByRole("button", { name: "Funcionário Existente" }),
    );
    await user.type(screen.getByPlaceholderText("Buscar por nome..."), "Ausente");
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_users_page", {
        limit: 20,
        offset: 0,
        search: "Ausente",
      });
    });
    expect(
      screen.getByRole("button", { name: "Funcionário Existente" }),
    ).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_users");
  });

  it("shows a local technician lookup error", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_users_page") return Promise.reject(new Error("offline"));
      if (command === "get_customers_page") {
        return Promise.resolve({ items: [existingCustomer], total: 1 });
      }
      if (
        command === "get_checklist_templates" ||
        command === "get_inventory_items"
      ) {
        return Promise.resolve([]);
      }
      return Promise.resolve(null);
    });
    const user = userEvent.setup();
    renderCreate();

    await user.click(
      await screen.findByRole("button", {
        name: "Selecione um responsável...",
      }),
    );
    expect(
      await screen.findByText("Não foi possível buscar funcionários."),
    ).toBeInTheDocument();
  });

  it("selects an employee created inline without loading the full employee list", async () => {
    const user = userEvent.setup();
    renderCreate();

    await user.click(
      await screen.findByRole("button", {
        name: "Selecione um responsável...",
      }),
    );
    await user.click(screen.getByRole("button", { name: "Criar funcionário" }));
    await user.type(screen.getByLabelText("Nome completo"), "Ana Técnica");
    await user.type(
      screen.getByLabelText("E-mail", {
        selector: "#employee-create-email",
      }),
      "ana@example.com",
    );
    await user.click(screen.getByRole("button", { name: "Criar funcionário" }));

    expect(
      await screen.findByRole("button", { name: "Ana Técnica" }),
    ).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_users");
  });

  it("keeps the selected technician when inline employee creation is cancelled", async () => {
    const user = userEvent.setup();
    renderCreate();

    await user.click(
      await screen.findByRole("button", {
        name: "Selecione um responsável...",
      }),
    );
    await user.click(await screen.findByRole("option", { name: /Funcionário Existente/ }));
    await user.click(
      screen.getByRole("button", { name: "Funcionário Existente" }),
    );
    await user.click(screen.getByRole("button", { name: "Criar funcionário" }));
    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(
      await screen.findByRole("button", { name: "Funcionário Existente" }),
    ).toBeInTheDocument();
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
