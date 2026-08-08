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
  costPrice: 80,
  averageCost: 80,
  salePrice: 150,
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
          costPrice: 80,
          salePrice: 150,
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
          costPrice: 80,
          salePrice: 150,
        },
        item,
      ),
    ).toBe(false);
  });
});
