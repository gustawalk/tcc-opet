import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import {
  ServiceOrderItemLine,
  ServiceOrderItemsEditor,
} from "@/components/shared/ServiceOrderItemsEditor";

beforeAll(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
});

describe("ServiceOrderItemsEditor", () => {
  it("limits a manually entered part quantity to available stock", async () => {
    const user = userEvent.setup();
    const onQuantityChange = vi.fn();
    const initialLine: ServiceOrderItemLine = {
      id: "line-1",
      inventoryItemId: "part-1",
      inventoryItemName: "Tela OLED",
      itemType: "part",
      quantity: 1,
      unitPrice: 15000,
      maxQuantity: 5,
    };

    function Harness() {
      const [line, setLine] = useState(initialLine);
      return (
        <ServiceOrderItemsEditor
          inventory={[]}
          lines={[line]}
          onSelectItem={vi.fn()}
          onQuantityChange={(_, quantity) => {
            onQuantityChange(quantity);
            setLine((current) => ({ ...current, quantity }));
          }}
          onRemove={vi.fn()}
        />
      );
    }

    render(<Harness />);
    const quantityInput = screen.getByLabelText("Quantidade de Tela OLED");

    await user.clear(quantityInput);
    await user.type(quantityInput, "99");
    expect(quantityInput).toHaveValue("5");

    await user.tab();
    await waitFor(() => expect(onQuantityChange).toHaveBeenCalledWith(5));
    expect(quantityInput).toHaveValue("5");
  });
});
