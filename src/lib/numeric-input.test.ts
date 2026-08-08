import { describe, expect, it } from "vitest";
import {
  currencyInputToNumber,
  decimalInputToNumber,
  formatCurrencyInput,
  integerInputToNumber,
  normalizeCurrencyInput,
  normalizeIntegerInput,
  sanitizeDecimalInput,
  sanitizeIntegerInput,
} from "@/lib/numeric-input";

describe("numeric input helpers", () => {
  it("keeps only integer digits and fills an empty blur with zero", () => {
    expect(sanitizeIntegerInput("12abc.3")).toBe("123");
    expect(integerInputToNumber("123")).toBe(123);
    expect(integerInputToNumber("")).toBeUndefined();
    expect(normalizeIntegerInput("")).toBe("0");
  });

  it("formats and parses Brazilian currency values", () => {
    const formatted = formatCurrencyInput("123456");

    expect(currencyInputToNumber(formatted)).toBe(1234.56);
    expect(normalizeCurrencyInput("")).toContain("0,00");
    expect(currencyInputToNumber("")).toBeUndefined();
  });

  it("supports comma decimals without accepting non-numeric text", () => {
    expect(sanitizeDecimalInput("12.345")).toBe("12,34");
    expect(sanitizeDecimalInput("abc,5")).toBe("0,5");
    expect(decimalInputToNumber("12,34")).toBe(12.34);
    expect(decimalInputToNumber("")).toBeUndefined();
  });
});
