import { describe, expect, it } from "vitest";
import {
  editServiceOrderSchema,
  inventoryItemSchema,
  quantitySchema,
} from "@/lib/validation";

describe("numeric validation", () => {
  it("converts inventory text inputs to finite numeric payloads", () => {
    const result = inventoryItemSchema.safeParse({
      name: "Tela OLED",
      description: "Tela de reposição para aparelho",
      costPrice: "R$ 80,50",
      salePrice: "R$ 150,00",
      minQuantity: "2",
      initialQuantity: "5",
    });

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.costPrice).toBe(8050);
      expect(result.data.salePrice).toBe(15000);
      expect(result.data.initialQuantity).toBe(5);
    }
  });

  it("rejects empty or zero operational quantities", () => {
    expect(quantitySchema.safeParse({ quantity: "" }).success).toBe(false);
    expect(quantitySchema.safeParse({ quantity: "0" }).success).toBe(false);
    expect(quantitySchema.safeParse({ quantity: "2" }).success).toBe(true);
  });

  it("validates Brazilian decimal discounts before saving", () => {
    expect(
      editServiceOrderSchema.safeParse({
        description: "Descrição válida da manutenção",
        discountBasisPoints: "12,5",
      }).success,
    ).toBe(true);
    const valid = editServiceOrderSchema.safeParse({
      description: "Descrição válida da manutenção",
      discountBasisPoints: "12,5",
    });
    if (valid.success) expect(valid.data.discountBasisPoints).toBe(1250);
    expect(
      editServiceOrderSchema.safeParse({
        description: "Descrição válida da manutenção",
        discountBasisPoints: "100,1",
      }).success,
    ).toBe(false);
  });
});
