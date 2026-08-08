import { z } from "zod";
import {
  currencyInputToNumber,
  decimalInputToNumber,
  integerInputToNumber,
} from "@/lib/numeric-input";

const nonNegativeInteger = z.preprocess(
  (value) => (typeof value === "string" ? integerInputToNumber(value) : value),
  z.number("Informe um número inteiro")
    .int("Deve ser um número inteiro")
    .min(0, "Deve ser maior ou igual a 0"),
);

const positiveInteger = z.preprocess(
  (value) => (typeof value === "string" ? integerInputToNumber(value) : value),
  z.number("Informe um número inteiro")
    .int("Deve ser um número inteiro")
    .min(1, "A quantidade deve ser pelo menos 1"),
);

const currencyValue = z.preprocess(
  (value) => (typeof value === "string" ? currencyInputToNumber(value) : value),
  z.number("Informe um valor válido")
    .finite("Informe um valor válido")
    .min(0, "O valor deve ser maior ou igual a 0"),
);

const percentageValue = z.preprocess(
  (value) => (typeof value === "string" ? decimalInputToNumber(value) : value),
  z.number("Informe um percentual válido")
    .finite("Informe um percentual válido")
    .min(0, "Desconto não pode ser negativo")
    .max(100, "Desconto não pode exceder 100%"),
);

export const userSchema = z.object({
  name: z.string().min(2, "Nome deve ter ao menos 2 caracteres"),
  email: z.string().email("E-mail inválido"),
  phone: z.string().optional(),
  cpf: z.string().optional(),
  joinDate: z.string().optional(),
});

export const customerSchema = z.object({
  name: z.string().min(2, "Nome deve ter ao menos 2 caracteres"),
  email: z.string().email("E-mail inválido"),
  phone: z.string().refine((val) => val.replace(/\D/g, "").length >= 10, "Telefone deve ter ao menos 10 dígitos"),
  address: z.string().min(5, "Endereço deve ter ao menos 5 caracteres"),
});

export const inventoryItemSchema = z.object({
  name: z.string().min(2, "Nome deve ter ao menos 2 caracteres"),
  description: z.string().min(3, "Descrição deve ter ao menos 3 caracteres"),
  costPrice: currencyValue,
  salePrice: currencyValue,
  minQuantity: nonNegativeInteger,
  initialQuantity: nonNegativeInteger,
});

export const quantitySchema = z.object({
  quantity: positiveInteger,
});

export const serviceOrderCreateSchema = z.object({
  equipment: z.string().min(2, "Equipamento é obrigatório"),
  description: z.string().min(10, "Descrição deve ter ao menos 10 caracteres"),
  imei: z.string().optional(),
  techId: z.string().optional(),
});

export const newCustomerSchema = z.object({
  name: z.string().min(2, "Nome deve ter ao menos 2 caracteres"),
  phone: z.string().refine((val) => val.replace(/\D/g, "").length >= 10, "Telefone deve ter ao menos 10 dígitos"),
  email: z.string().email("E-mail inválido"),
  address: z.string().min(5, "Endereço deve ter ao menos 5 caracteres"),
});

export const editServiceOrderSchema = z.object({
  description: z.string().min(10, "Descrição deve ter ao menos 10 caracteres"),
  discount: percentageValue.optional(),
});

export const settingsSchema = z.object({
  companyName: z.string().min(3, "Nome da empresa deve ter ao menos 3 caracteres"),
  cnpj: z.string().refine((val) => val === "" || val.replace(/\D/g, "").length === 14, "CNPJ deve ter 14 dígitos"),
  address: z.string().refine((val) => val === "" || val.length >= 5, "Endereço deve ter ao menos 5 caracteres"),
});

export const templateSchema = z.object({
  title: z.string().min(3, "Título deve ter ao menos 3 caracteres"),
  items: z.array(z.string().min(1, "Item não pode estar vazio")).min(1, "Adicione ao menos 1 item ao checklist"),
});

export type ValidationErrors = Record<string, string>;

interface SafeParseError {
  success: false;
  error: { issues: Array<{ path: unknown[]; message: string }> };
}

export function parseErrors(result: SafeParseError | { success: true }): ValidationErrors | null {
  if (result.success) return null;
  const errors: ValidationErrors = {};
  for (const issue of result.error.issues) {
    const field = String(issue.path[0]);
    if (!errors[field]) {
      errors[field] = issue.message;
    }
  }
  return errors;
}

export function clearFieldError<T extends ValidationErrors>(errors: T, field: string): T {
  const next = { ...errors };
  delete next[field];
  return next;
}
