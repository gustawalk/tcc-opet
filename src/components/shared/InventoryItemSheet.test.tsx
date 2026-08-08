import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InventoryItemSheet } from "@/components/shared/InventoryItemSheet";
import { InventoryItem } from "@/lib/types";

vi.mock("@/lib/errors", () => ({ toastError: vi.fn(), toastSuccess: vi.fn() }));

const item: InventoryItem = {
  id: "part-1",
  name: "Tela OLED",
  description: "Tela de reposição",
  type: "part",
  minQuantity: 2,
  currentQuantity: 7,
  costPrice: 80,
  averageCost: 80,
  salePrice: 150,
  supplierName: "Distribuidora",
};

describe("InventoryItemSheet duplication", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("releases interaction after returning to edit mode and cancelling the drawer", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const user = userEvent.setup();
    const onOutsideClick = vi.fn();

    function DuplicateHarness() {
      const [open, setOpen] = useState(true);

      return (
        <>
          <button type="button" onClick={onOutsideClick}>
            Área externa
          </button>
          <InventoryItemSheet
            open={open}
            duplicateItem={item}
            onOpenChange={setOpen}
          />
        </>
      );
    }

    render(
      <QueryClientProvider client={queryClient}>
        <DuplicateHarness />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Duplicar" })).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "Duplicar" }));
    expect(await screen.findByText("Nenhuma informação foi alterada")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Voltar e editar" }));

    const nameInput = await screen.findByLabelText("Nome do Produto");
    await user.clear(nameInput);
    await user.type(nameInput, "Tela OLED revisada");
    expect(nameInput).toHaveValue("Tela OLED revisada");

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    await waitFor(() => {
      expect(document.body).not.toHaveAttribute("data-scroll-locked");
    });

    await user.click(screen.getByRole("button", { name: "Área externa" }));
    expect(onOutsideClick).toHaveBeenCalledOnce();
  });
});
