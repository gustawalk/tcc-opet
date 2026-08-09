export const formatCurrency = (value: number) => {
  if (!Number.isSafeInteger(value)) {
    throw new TypeError("Money values must be safe integers");
  }
  return new Intl.NumberFormat("pt-BR", {
    style: "currency",
    currency: "BRL",
  }).format(value / 100);
};

export const applyDiscount = (value: number, discountBasisPoints: number) => {
  if (
    !Number.isSafeInteger(value) ||
    !Number.isSafeInteger(discountBasisPoints)
  ) {
    throw new TypeError("Money and discount values must be safe integers");
  }
  if (discountBasisPoints < 0 || discountBasisPoints > 10_000) {
    throw new RangeError("Discount must be between 0 and 10000 basis points");
  }

  const numerator = BigInt(value) * BigInt(10_000 - discountBasisPoints);
  const rounded =
    (numerator >= 0 ? numerator + 5_000n : numerator - 5_000n) / 10_000n;
  const result = Number(rounded);
  if (!Number.isSafeInteger(result)) {
    throw new RangeError("Discounted value is too large");
  }
  return result;
};

export const formatDate = (date: string) => {
  return new Date(date).toLocaleDateString("pt-BR");
};

export const formatBRPhone = (value: string) => {
  const digits = value.replace(/\D/g, "").slice(0, 11);
  if (digits.length <= 2) return `(${digits}`;
  if (digits.length <= 7) return `(${digits.slice(0, 2)}) ${digits.slice(2)}`;
  return `(${digits.slice(0, 2)}) ${digits.slice(2, 7)}-${digits.slice(7)}`;
};

export const formatCPF = (value: string) => {
  const digits = value.replace(/\D/g, "").slice(0, 11);
  if (digits.length <= 3) return digits;
  if (digits.length <= 6) return `${digits.slice(0, 3)}.${digits.slice(3)}`;
  if (digits.length <= 9) return `${digits.slice(0, 3)}.${digits.slice(3, 6)}.${digits.slice(6)}`;
  return `${digits.slice(0, 3)}.${digits.slice(3, 6)}.${digits.slice(6, 9)}-${digits.slice(9)}`;
};

export const formatCNPJ = (value: string) => {
  const digits = value.replace(/\D/g, "").slice(0, 14);
  if (digits.length <= 2) return digits;
  if (digits.length <= 5) return `${digits.slice(0, 2)}.${digits.slice(2)}`;
  if (digits.length <= 8) return `${digits.slice(0, 2)}.${digits.slice(2, 5)}.${digits.slice(5)}`;
  if (digits.length <= 12) return `${digits.slice(0, 2)}.${digits.slice(2, 5)}.${digits.slice(5, 8)}/${digits.slice(8)}`;
  return `${digits.slice(0, 2)}.${digits.slice(2, 5)}.${digits.slice(5, 8)}/${digits.slice(8, 12)}-${digits.slice(12)}`;
};

export const formatName = (value: string) => {
  const trimmed = value.replace(/\s+/g, " ").trim();
  return trimmed
    .split(" ")
    .map((word, index) =>
      index === 0 || word.length > 3
        ? word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
        : word.toLowerCase()
    )
    .join(" ");
};
