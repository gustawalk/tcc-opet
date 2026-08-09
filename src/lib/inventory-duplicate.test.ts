import { describe, expect, it } from "vitest";
import { isUnchangedDuplicate } from "@/lib/inventory-duplicate";
import { InventoryItem } from "@/lib/types";

const item: InventoryItem = {
  id: "part-1",
  name: "Tela OLED",
  description: "Tela de reposição",
  type: "part",
  minQuantity: 2,
  currentQuantity: 7,
  costPrice: 8000,
  averageCost: 8000,
  salePrice: 15000,
  supplierName: "Distribuidora",
};

describe("inventory duplication", () => {
  it("requires confirmation only when copied data is unchanged and stock starts at zero", () => {
    expect(
      isUnchangedDuplicate(
        {
          name: "Tela OLED",
          description: "Tela de reposição",
          type: "part",
          supplierName: "Distribuidora",
          minQuantity: 2,
          initialQuantity: 0,
          costPrice: 8000,
          salePrice: 15000,
        },
        item,
      ),
    ).toBe(true);
    expect(
      isUnchangedDuplicate(
        {
          name: "Tela OLED Premium",
          description: "Tela de reposição",
          type: "part",
          supplierName: "Distribuidora",
          minQuantity: 2,
          initialQuantity: 0,
          costPrice: 8000,
          salePrice: 15000,
        },
        item,
      ),
    ).toBe(false);
  });
});
