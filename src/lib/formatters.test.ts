import { describe, expect, it } from "vitest";
import { applyDiscount, formatCurrency } from "@/lib/formatters";

describe("money formatters", () => {
  it("formats integer cents as BRL", () => {
    expect(formatCurrency(123456)).toContain("1.234,56");
    expect(formatCurrency(1)).toContain("0,01");
  });

  it("rejects money outside the safe integer boundary", () => {
    expect(() => formatCurrency(Number.MAX_SAFE_INTEGER + 1)).toThrow(
      "Money values must be safe integers",
    );
    expect(() => formatCurrency(1.5)).toThrow(
      "Money values must be safe integers",
    );
  });

  it("rounds odd-cent discounts without floating-point money arithmetic", () => {
    expect(applyDiscount(1, 5000)).toBe(1);
    expect(applyDiscount(3, 5000)).toBe(2);
    expect(applyDiscount(101, 1000)).toBe(91);
    expect(applyDiscount(-1, 5000)).toBe(-1);
  });
});
