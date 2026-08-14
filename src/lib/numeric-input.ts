const currencyFormatter = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
});

export function sanitizeIntegerInput(value: string) {
  return value.replace(/\D/g, "");
}

export function sanitizeBoundedIntegerInput(value: string, maximum?: number) {
  const digits = sanitizeIntegerInput(value);
  if (!digits || maximum === undefined) return digits;

  const parsed = integerInputToNumber(digits);
  return parsed === undefined || parsed > maximum ? String(maximum) : digits;
}

export function integerInputToNumber(value: string) {
  if (!value) return undefined;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : undefined;
}

export function normalizeIntegerInput(value: string, fallback = 0) {
  const parsed = integerInputToNumber(value);
  return String(parsed ?? fallback);
}

export function formatCurrencyInput(value: string) {
  const digits = value.replace(/\D/g, "");
  if (!digits) return "";

  const cents = Number(digits);
  return Number.isSafeInteger(cents) ? currencyFormatter.format(cents / 100) : "";
}

export function formatCurrencyInputValue(value: number) {
  return Number.isSafeInteger(value) ? currencyFormatter.format(value / 100) : "";
}

export function currencyInputToNumber(value: string) {
  const digits = value.replace(/\D/g, "");
  if (!digits) return undefined;

  const cents = Number(digits);
  return Number.isSafeInteger(cents) ? cents : undefined;
}

export function normalizeCurrencyInput(value: string) {
  return formatCurrencyInput(value) || formatCurrencyInputValue(0);
}

export function sanitizeDecimalInput(value: string) {
  const normalized = value.replace(".", ",").replace(/[^\d,]/g, "");
  const [integer = "", ...fractionParts] = normalized.split(",");
  const fraction = fractionParts.join("").slice(0, 2);
  return fractionParts.length ? `${integer || "0"},${fraction}` : integer;
}

export function decimalInputToNumber(value: string) {
  if (!value || value === ",") return undefined;
  const normalized = value.replace(",", ".");
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : undefined;
}

export function normalizeDecimalInput(value: string, fallback = "0") {
  return value ? value : fallback;
}
