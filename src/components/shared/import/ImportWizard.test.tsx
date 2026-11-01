import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { configureDataClient } from "@/lib/data-client";
import type {
  ImportRequest,
  ImportValidation,
  ParsedImport,
} from "@/lib/data-import";
import { ImportDataButton } from "./ImportDataButton";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);
const mockedOpen = vi.mocked(open);
let parsed: ParsedImport;
let commitCalls: Array<{ batchId: string; request: ImportRequest }>;
let validationHandler: (request: ImportRequest) => ImportValidation;
let commitHandler: (args: {
  batchId: string;
  request: ImportRequest;
}) => unknown;

function validation(request: ImportRequest): ImportValidation {
  const rows = request.rows.map((row) => {
    const errors: Record<string, string> = {};
    if (row.values.phone.length < 10)
      errors.phone = "Telefone deve ter ao menos 10 dígitos.";
    return { ...row, errors };
  });
  return {
    rows,
    validCount: rows.filter((row) => !row.errors.phone).length,
    invalidCount: rows.filter((row) => row.errors.phone).length,
  };
}

function mount(entity: "customers" | "employees" | "inventory" = "customers") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidate = vi.spyOn(client, "invalidateQueries");
  render(
    <QueryClientProvider client={client}>
      <ImportDataButton entity={entity} />
    </QueryClientProvider>,
  );
  return { client, invalidate };
}
async function openAndMap() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Importar dados" }));
  await user.click(screen.getByRole("button", { name: "Selecionar arquivo" }));
  await user.click(
    await screen.findByRole("button", { name: "Mapear campos" }),
  );
  return user;
}
async function startReview() {
  const user = await openAndMap();
  await user.click(screen.getByRole("button", { name: "Revisar dados" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled(),
  );
  return user;
}

beforeEach(() => {
  vi.clearAllMocks();
  configureDataClient("local");
  commitCalls = [];
  parsed = {
    headers: ["Full name", "Phone number", "Emergency contact"],
    warnings: [],
    rows: [
      {
        sourceRow: 2,
        cells: ["Maria", "41999999999", "Unknown"],
        numericColumns: [],
      },
    ],
  };
  validationHandler = validation;
  commitHandler = () => ({
    result: { createdCount: 1, ids: ["new-id"] },
    validation: null,
  });
  mockedOpen.mockResolvedValue("/imports/clients.csv");
  mockedInvoke.mockImplementation(async (operation, payload) => {
    if (operation === "inspect_import_file")
      return { worksheets: [], delimiter: ";" };
    if (operation === "parse_import_file") return parsed;
    const args = payload as {
      batchId: string;
      request: ImportRequest;
      operation?: string;
      payload?: { batchId: string; request: ImportRequest };
    };
    const command =
      operation === "lan_remote_command" ? args.operation : operation;
    const input = operation === "lan_remote_command" ? args.payload! : args;
    if (command === "validate_import_rows")
      return validationHandler(input.request);
    if (command === "commit_import") {
      commitCalls.push(structuredClone(input));
      return commitHandler(input);
    }
    throw new Error(`Unexpected command: ${operation}`);
  });
});
afterEach(() => {
  cleanup();
  configureDataClient("local");
});

describe("ImportWizard", () => {
  it("closes on an outside click before a file is selected", async () => {
    mount();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Importar dados" }));
    await user.click(document.querySelector(".bg-black\\/80")!);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("introduces file selection with one action and shows file limits only on the file step", async () => {
    mount();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Importar dados" }));
    expect(
      screen.getByRole("heading", { name: "Comece pelo seu arquivo" }),
    ).toBeVisible();
    expect(
      screen
        .getByRole("navigation", { name: "Etapas da importação" })
        .querySelector('[aria-current="step"]'),
    ).toHaveTextContent("Arquivo");
    expect(screen.getByText("Até 5.000 linhas")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Mapear campos" }),
    ).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Selecionar arquivo" }),
    );
    expect(
      await screen.findByRole("region", { name: "Arquivo selecionado" }),
    ).toHaveTextContent("clients.csv");
    expect(screen.getByText("Arquivo lido com sucesso")).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Prévia do arquivo" }),
    ).toBeVisible();
    expect(screen.getByText("1 linha")).toBeVisible();
    expect(screen.getByText("3 colunas")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Mapear campos" }));
    expect(screen.queryByText("Até 5.000 linhas")).not.toBeInTheDocument();
    expect(
      screen
        .getByRole("navigation", { name: "Etapas da importação" })
        .querySelector('[aria-current="step"]'),
    ).toHaveTextContent("Mapeamento");
    expect(
      screen
        .getByRole("dialog")
        .querySelectorAll('select:not([aria-hidden="true"])'),
    ).toHaveLength(0);
  });

  it("uses the app dropdown for column mapping and keeps the wizard open when Escape closes it", async () => {
    mount();
    const user = await openAndMap();
    const trigger = screen.getByRole("combobox", { name: "Nome completo *" });
    expect(trigger.tagName).toBe("BUTTON");
    trigger.focus();
    await user.keyboard("{ArrowDown}");
    expect(await screen.findByRole("listbox")).toBeVisible();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeVisible();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
    await user.click(trigger);
    await user.click(screen.getByRole("option", { name: "2. Phone number" }));
    expect(trigger).toHaveTextContent("2. Phone number");
  });

  it("maps existing fields, allows fixed values, confirms before writing and refreshes clients", async () => {
    const { invalidate } = mount();
    const user = await openAndMap();
    expect(
      screen.getByRole("combobox", { name: "Nome completo *" }),
    ).toHaveTextContent("1. Full name");
    await user.click(screen.getByRole("combobox", { name: "Endereço" }));
    await user.click(screen.getByRole("option", { name: "Valor fixo" }));
    await user.type(
      screen.getByRole("textbox", { name: "Valor fixo: Endereço" }),
      "Rua A, 10",
    );
    await user.click(screen.getByRole("button", { name: "Revisar dados" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled(),
    );
    expect(commitCalls).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    expect(commitCalls).toHaveLength(0);
    await user.click(
      screen.getByRole("button", { name: "Confirmar importação" }),
    );
    expect(await screen.findByText("Importação concluída.")).toBeVisible();
    expect(commitCalls[0].request.rows).toEqual([
      {
        sourceRow: 2,
        values: {
          name: "Maria",
          phone: "41999999999",
          email: "",
          address: "Rua A, 10",
        },
        numericFields: [],
      },
    ]);
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["customersPage"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["customersLookup"] });
  });

  it("corrects errors in place with the errors-only filter and permits exclusion/restoration", async () => {
    parsed.rows.push({
      sourceRow: 3,
      cells: ["Ana", "123", ""],
      numericColumns: [],
    });
    mount();
    const user = await openAndMap();
    await user.click(screen.getByRole("button", { name: "Revisar dados" }));
    expect(
      await screen.findByText("Telefone deve ter ao menos 10 dígitos."),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
    await user.click(
      screen.getByRole("checkbox", { name: "Somente linhas com erros" }),
    );
    expect(
      screen.queryByRole("textbox", { name: "Nome completo, linha 2" }),
    ).not.toBeInTheDocument();
    const phone = screen.getByRole("textbox", { name: "Telefone, linha 3" });
    await user.clear(phone);
    await user.type(phone, "41988888888");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled(),
    );
    await user.click(
      screen.getByRole("checkbox", { name: "Somente linhas com erros" }),
    );
    expect(
      screen.getByRole("textbox", { name: "Telefone, linha 3" }),
    ).toHaveValue("41988888888");
    await user.click(screen.getByRole("checkbox", { name: "Incluir linha 3" }));
    expect(
      screen.getByRole("textbox", { name: "Telefone, linha 3" }),
    ).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: "Incluir linha 3" }));
    expect(
      screen.getByRole("textbox", { name: "Telefone, linha 3" }),
    ).toBeEnabled();
    await user.click(screen.getByRole("checkbox", { name: "Incluir linha 3" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled(),
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    await user.click(
      screen.getByRole("button", { name: "Confirmar importação" }),
    );
    expect(
      await screen.findByText("1 registros criados · 1 linhas excluídas."),
    ).toBeVisible();
    expect(commitCalls[0].request.rows.map((row) => row.sourceRow)).toEqual([
      2,
    ]);
  });

  it("keeps edits and exclusions across paginated review of a 5,000-row source", async () => {
    parsed.rows = Array.from({ length: 5000 }, (_, i) => ({
      sourceRow: i + 2,
      cells: [`Cliente ${i}`, "41999999999", ""],
      numericColumns: [],
    }));
    mount();
    const user = await startReview();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Nome completo, linha 2" }),
      { target: { value: "Nome corrigido" } },
    );
    await user.click(screen.getByRole("checkbox", { name: "Incluir linha 3" }));
    await user.click(screen.getByRole("button", { name: "Próxima página" }));
    expect(
      screen.getByRole("textbox", { name: "Nome completo, linha 22" }),
    ).toHaveValue("Cliente 20");
    expect(
      screen.queryByRole("textbox", { name: "Nome completo, linha 2" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Página anterior" }));
    expect(
      screen.getByRole("textbox", { name: "Nome completo, linha 2" }),
    ).toHaveValue("Nome corrigido");
    expect(
      screen.getByRole("checkbox", { name: "Incluir linha 3" }),
    ).not.toBeChecked();
    expect(screen.getAllByRole("textbox")).toHaveLength(80);
  });

  it("confirms discard and retains the draft when the user cancels that confirmation", async () => {
    mount();
    const user = await startReview();
    await user.click(
      screen.getByRole("button", { name: "Alterar mapeamento" }),
    );
    expect(screen.getByRole("alertdialog")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Manter revisão" }));
    expect(
      screen.getByRole("textbox", { name: "Nome completo, linha 2" }),
    ).toHaveValue("Maria");
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    await user.click(
      screen.getByRole("button", { name: "Descartar e continuar" }),
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("asks before discarding an imported draft on an outside click", async () => {
    mount();
    const user = await startReview();
    await user.click(document.querySelector(".bg-black\\/80")!);
    expect(screen.getByRole("alertdialog")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Manter revisão" }));
    expect(screen.getByRole("textbox", { name: "Nome completo, linha 2" })).toHaveValue("Maria");
    expect(commitCalls).toHaveLength(0);
  });

  it("freezes payload and batch ID on an uncertain save and retries without duplicate submission", async () => {
    let attempts = 0;
    commitHandler = () => {
      if (attempts++ === 0)
        throw { en: "Unreachable", pt: "Host indisponível." };
      return {
        result: { createdCount: 1, ids: ["same-id"] },
        validation: null,
      };
    };
    mount();
    const user = await startReview();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    await user.click(
      screen.getByRole("button", { name: "Confirmar importação" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "O resultado ainda não foi confirmado",
    );
    expect(
      screen.getByRole("button", { name: "Voltar à revisão" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancelar" })).toBeDisabled();
    await user.click(document.querySelector(".bg-black\\/80")!);
    expect(screen.getByRole("dialog")).toBeVisible();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(await screen.findByText("Importação concluída.")).toBeVisible();
    expect(commitCalls).toHaveLength(2);
    expect(commitCalls[1]).toEqual(commitCalls[0]);
  });

  it("returns authoritative commit conflicts to editable review", async () => {
    commitHandler = (args) => ({
      result: null,
      validation: {
        validCount: 0,
        invalidCount: 1,
        rows: args.request.rows.map((row) => ({
          ...row,
          errors: { phone: "Conflito durante a gravação" },
        })),
      },
    });
    mount();
    const user = await startReview();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    await user.click(
      screen.getByRole("button", { name: "Confirmar importação" }),
    );
    expect(
      await screen.findByText("Conflito durante a gravação"),
    ).toBeVisible();
    expect(
      screen.getByRole("textbox", { name: "Telefone, linha 2" }),
    ).toBeEnabled();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  });

  it("reads the file locally in Client mode and sends only mapped rows to the Host", async () => {
    configureDataClient("client");
    mount();
    const user = await startReview();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    await user.click(
      screen.getByRole("button", { name: "Confirmar importação" }),
    );
    expect(await screen.findByText("Importação concluída.")).toBeVisible();
    expect(mockedInvoke).toHaveBeenCalledWith("inspect_import_file", {
      path: "/imports/clients.csv",
    });
    expect(mockedInvoke).toHaveBeenCalledWith(
      "lan_remote_command",
      expect.objectContaining({
        operation: "commit_import",
        idempotencyKey: commitCalls[0].batchId,
      }),
    );
    const remoteCalls = mockedInvoke.mock.calls.filter(
      ([command]) => command === "lan_remote_command",
    );
    expect(JSON.stringify(remoteCalls)).not.toContain("/imports/");
  });

  it("allows selecting an XLSX worksheet and reports file read failures without saving", async () => {
    mockedInvoke.mockImplementation(async (operation) => {
      if (operation === "inspect_import_file")
        return { worksheets: ["Clientes", "Funcionários"], delimiter: ";" };
      if (operation === "parse_import_file") return parsed;
      throw new Error("unexpected");
    });
    mount();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Importar dados" }));
    await user.click(
      screen.getByRole("button", { name: "Selecionar arquivo" }),
    );
    await user.click(await screen.findByRole("combobox", { name: "Planilha" }));
    await user.click(screen.getByRole("option", { name: "Funcionários" }));
    await user.click(
      within(screen.getByRole("alertdialog")).getByRole("button", {
        name: "Descartar e continuar",
      }),
    );
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "parse_import_file",
        expect.objectContaining({ worksheet: "Funcionários" }),
      ),
    );
    mockedInvoke.mockRejectedValueOnce({
      pt: "Arquivo inválido",
      en: "Invalid file",
    });
    await user.click(screen.getByRole("button", { name: "Trocar arquivo" }));
    await user.click(
      screen.getByRole("button", { name: "Descartar e continuar" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Arquivo inválido",
    );
    expect(
      screen.queryByRole("button", { name: "Mapear campos" }),
    ).not.toBeInTheDocument();
    expect(commitCalls).toHaveLength(0);
  });
  it.each(["employees", "inventory"] as const)(
    "imports %s through the same wizard with entity fields and cache invalidation",
    async (entity) => {
      parsed =
        entity === "inventory"
          ? {
              headers: ["Nome", "Descrição", "Preço de custo"],
              warnings: [],
              rows: [
                {
                  sourceRow: 2,
                  cells: ["Tela", "Tela OLED", "12.34"],
                  numericColumns: [2],
                },
              ],
            }
          : {
              headers: ["Nome", "E-mail", "CPF", "Data de admissão"],
              warnings: [],
              rows: [
                {
                  sourceRow: 2,
                  cells: [
                    "Maria",
                    "maria@example.com",
                    "00123456789",
                    "20/10/2026",
                  ],
                  numericColumns: [],
                },
              ],
            };
      validationHandler = (request) => ({
        rows: request.rows.map((row) => ({ ...row, errors: {} })),
        validCount: 1,
        invalidCount: 0,
      });
      const { invalidate } = mount(entity);
      const user = await startReview();
      if (entity === "inventory") {
        expect(
          screen.getByRole("textbox", { name: "Controla estoque, linha 2" }),
        ).toHaveValue("true");
        expect(
          screen.getByRole("textbox", { name: "Controla estoque, linha 2" }),
        ).toHaveAttribute("readonly");
      }
      await user.click(screen.getByRole("button", { name: "Continuar" }));
      await user.click(
        screen.getByRole("button", { name: "Confirmar importação" }),
      );
      expect(await screen.findByText("Importação concluída.")).toBeVisible();
      expect(commitCalls[0].request.entity).toBe(entity);
      if (entity === "inventory") {
        expect(commitCalls[0].request.rows[0].values).toMatchObject({
          type: "part",
          description: "Tela OLED",
          costPrice: "12.34",
          initialQuantity: "0",
        });
        expect(commitCalls[0].request.rows[0].numericFields).toEqual([
          "costPrice",
        ]);
        expect(invalidate).toHaveBeenCalledWith({
          queryKey: ["inventoryItemsPage"],
        });
        expect(invalidate).toHaveBeenCalledWith({
          queryKey: ["inventorySummary"],
        });
      } else {
        expect(commitCalls[0].request.rows[0].values).toMatchObject({
          email: "maria@example.com",
          cpf: "00123456789",
          joinDate: "20/10/2026",
        });
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ["usersPage"] });
      }
    },
  );

  it("blocks confirmation when Host validation is unavailable and never falls back locally", async () => {
    configureDataClient("client");
    validationHandler = () => {
      throw { pt: "Host indisponível.", en: "Host unavailable" };
    };
    mount();
    const user = await openAndMap();
    await user.click(screen.getByRole("button", { name: "Revisar dados" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Host indisponível.",
    );
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
    expect(
      mockedInvoke.mock.calls.some(
        ([command]) => command === "validate_import_rows",
      ),
    ).toBe(false);
    expect(commitCalls).toHaveLength(0);
    validationHandler = validation;
    await user.click(
      screen.getByRole("button", { name: "Tentar validar novamente" }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled(),
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(commitCalls).toHaveLength(0);
  });

  it("prevents duplicate submissions while the commit is in flight", async () => {
    let finish: ((outcome: unknown) => void) | undefined;
    commitHandler = () =>
      new Promise((resolve) => {
        finish = resolve;
      });
    mount();
    const user = await startReview();
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    const confirm = screen.getByRole("button", {
      name: "Confirmar importação",
    });
    fireEvent.click(confirm);
    fireEvent.click(confirm);
    expect(commitCalls).toHaveLength(1);
    expect(
      screen.getByRole("button", { name: "Tentar novamente" }),
    ).toBeDisabled();
    finish?.({ result: { createdCount: 1, ids: ["one"] }, validation: null });
    expect(await screen.findByText("Importação concluída.")).toBeVisible();
    expect(commitCalls).toHaveLength(1);
  });
});
