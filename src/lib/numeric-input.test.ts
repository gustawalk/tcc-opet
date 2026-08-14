import { describe, expect, it } from "vitest";
import {
  currencyInputToNumber,
  decimalInputToNumber,
  formatCurrencyInput,
  formatCurrencyInputValue,
  integerInputToNumber,
  normalizeCurrencyInput,
  normalizeIntegerInput,
  sanitizeBoundedIntegerInput,
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

  it("limits integer input to the available maximum", () => {
    expect(sanitizeBoundedIntegerInput("12 itens", 10)).toBe("10");
    expect(sanitizeBoundedIntegerInput("7", 10)).toBe("7");
    expect(sanitizeBoundedIntegerInput("", 10)).toBe("");
    expect(sanitizeBoundedIntegerInput("90071992547409999", 10)).toBe("10");
    expect(sanitizeBoundedIntegerInput("12 itens")).toBe("12");
  });

  it("formats and parses Brazilian currency values", () => {
    const formatted = formatCurrencyInput("123456");

    expect(currencyInputToNumber(formatted)).toBe(123456);
    expect(normalizeCurrencyInput("")).toContain("0,00");
    expect(currencyInputToNumber("")).toBeUndefined();
  });

  it("formats stored integer cents back into a BRL input", () => {
    expect(formatCurrencyInputValue(123456)).toContain("1.234,56");
    expect(currencyInputToNumber("R$ 0,01")).toBe(1);
  });

  it("supports comma decimals without accepting non-numeric text", () => {
    expect(sanitizeDecimalInput("12.345")).toBe("12,34");
    expect(sanitizeDecimalInput("abc,5")).toBe("0,5");
    expect(decimalInputToNumber("12,34")).toBe(12.34);
    expect(decimalInputToNumber("")).toBeUndefined();
  });
});
