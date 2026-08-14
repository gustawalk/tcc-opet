import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SearchableSelect } from "@/components/shared/SearchableSelect";

const options = [{ id: "customer-1", name: "Cliente Teste" }];

describe("SearchableSelect", () => {
  afterEach(cleanup);

  it("delegates remote search without filtering server results locally", async () => {
    const user = userEvent.setup();
    const onSearchChange = vi.fn();
    render(
      <SearchableSelect<{ id: string; name: string }>
        options={options}
        value={null}
        onSelect={vi.fn()}
        onSearchChange={onSearchChange}
        placeholder="Clientes"
        searchPlaceholder="Buscar cliente..."
        getKey={(option) => option.id}
        getLabel={(option) => option.name}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Clientes" }));
    await user.type(screen.getByPlaceholderText("Buscar cliente..."), "9999");

    expect(onSearchChange).toHaveBeenLastCalledWith("9999");
  });

  it("does not expose stale remote options while loading", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <SearchableSelect
        options={options}
        value={null}
        onSelect={onSelect}
        onSearchChange={vi.fn()}
        isLoading
        placeholder="Clientes"
        searchPlaceholder="Buscar cliente..."
        getKey={(option) => option.id}
        getLabel={(option) => option.name}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Clientes" }));
    expect(screen.getByText("Carregando...")).toBeInTheDocument();
    expect(screen.queryByText("Cliente Teste")).not.toBeInTheDocument();
    await user.keyboard("{Enter}");
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("preserves a selected label outside the current remote page", () => {
    render(
      <SearchableSelect<{ id: string; name: string }>
        options={[]}
        value="customer-1"
        selectedLabel="Cliente Selecionado"
        onSelect={vi.fn()}
        onSearchChange={vi.fn()}
        placeholder="Clientes"
        getKey={(option) => option.id}
        getLabel={(option) => option.name}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Cliente Selecionado" }),
    ).toBeInTheDocument();
  });
});
