import { z } from "zod";

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
  costPrice: z.number().min(0, "Preço de custo deve ser maior ou igual a 0"),
  salePrice: z.number().min(0, "Preço de venda deve ser maior ou igual a 0"),
  minQuantity: z.number().int("Deve ser um número inteiro").min(0, "Quantidade mínima deve ser maior ou igual a 0"),
});

export const quantitySchema = z.object({
  quantity: z.number().int("Deve ser um número inteiro").min(1, "A quantidade deve ser pelo menos 1"),
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
  discount: z.number().min(0, "Desconto não pode ser negativo").max(100, "Desconto não pode exceder 100%").optional(),
});

export const settingsSchema = z.object({
  companyName: z.string().min(3, "Nome da empresa deve ter ao menos 3 caracteres"),
  cnpj: z.string().refine((val) => val.replace(/\D/g, "").length === 14, "CNPJ deve ter 14 dígitos"),
  address: z.string().min(5, "Endereço deve ter ao menos 5 caracteres"),
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
