export type ImportEntity = "customers" | "employees" | "inventory";
export type NumberFormat = "brazilian" | "international";
export interface SourceRow { sourceRow: number; cells: string[]; numericColumns: number[] }
export interface ParsedImport { headers: string[]; rows: SourceRow[]; warnings: string[] }
export interface ImportInspection { worksheets: string[]; delimiter: string }
export interface ImportRow { sourceRow: number; values: Record<string, string>; numericFields: string[] }
export interface DraftRow extends ImportRow { excluded: boolean }
export interface ImportRequest { entity: ImportEntity; numberFormat: NumberFormat; rows: ImportRow[] }
export interface ValidatedRow { sourceRow: number; values: Record<string, string>; errors: Record<string, string> }
export interface ImportValidation { rows: ValidatedRow[]; validCount: number; invalidCount: number }
export interface ImportResult { createdCount: number; ids: string[] }
export interface CommitOutcome { result: ImportResult | null; validation: ImportValidation | null }
export interface ImportField { key: string; label: string; required?: boolean; aliases: string[]; defaultValue?: string }
export type FieldMapping = { kind: "column"; index: number } | { kind: "fixed"; value: string } | { kind: "ignore" };
export type ImportMapping = Record<string, FieldMapping>;

const name: ImportField = { key: "name", label: "Nome completo", required: true, aliases: ["name", "full name", "nome", "nome completo", "cliente", "funcionario"] };
const phone: ImportField = { key: "phone", label: "Telefone", aliases: ["phone", "phone number", "telefone", "celular", "whatsapp"] };
const email: ImportField = { key: "email", label: "E-mail", aliases: ["email", "e mail"] };
export const IMPORT_FIELDS: Record<ImportEntity, ImportField[]> = {
  customers: [name, { ...phone, required: true }, email, { key: "address", label: "Endereço", aliases: ["address", "endereco"] }],
  employees: [name, { ...email, required: true }, phone, { key: "cpf", label: "CPF", aliases: ["cpf", "document", "document number", "documento"] }, { key: "joinDate", label: "Data de admissão", aliases: ["join date", "joining date", "hire date", "data de admissao", "admissao"] }],
  inventory: [
    { key: "name", label: "Nome", required: true, aliases: ["name", "nome", "produto", "product", "item name"] },
    { key: "description", label: "Descrição", required: true, aliases: ["description", "descricao"] },
    { key: "type", label: "Tipo", required: true, aliases: ["type", "tipo"], defaultValue: "part" },
    { key: "tracksStock", label: "Controla estoque", aliases: ["tracks stock", "controla estoque", "controle de estoque"], defaultValue: "false" },
    { key: "minQuantity", label: "Quantidade mínima", aliases: ["min quantity", "minimum quantity", "quantidade minima", "estoque minimo"], defaultValue: "0" },
    { key: "initialQuantity", label: "Quantidade inicial", aliases: ["quantity", "current quantity", "initial quantity", "quantidade", "quantidade inicial", "estoque", "saldo"], defaultValue: "0" },
    { key: "costPrice", label: "Preço de custo", aliases: ["cost", "cost price", "preco de custo", "custo"], defaultValue: "0" },
    { key: "salePrice", label: "Preço de venda", aliases: ["sale price", "price", "preco de venda", "preco", "venda"], defaultValue: "0" },
    { key: "supplierName", label: "Fornecedor", aliases: ["supplier", "supplier name", "fornecedor"] },
  ],
};

const normalizeHeader = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9]/g, "");

export function suggestMapping(entity: ImportEntity, headers: string[]): ImportMapping {
  return Object.fromEntries(IMPORT_FIELDS[entity].map((field) => {
    const candidates = headers.flatMap((header, index) => field.aliases.some((alias) => normalizeHeader(alias) === normalizeHeader(header)) ? [index] : []);
    const mapping: FieldMapping = candidates.length === 1 ? { kind: "column", index: candidates[0] } : field.defaultValue !== undefined ? { kind: "fixed", value: field.defaultValue } : { kind: "ignore" };
    return [field.key, mapping];
  }));
}

export function mapImportRows(entity: ImportEntity, source: ParsedImport, mapping: ImportMapping): DraftRow[] {
  return source.rows.map((row) => {
    const values: Record<string, string> = {};
    const numericFields: string[] = [];
    for (const field of IMPORT_FIELDS[entity]) {
      const choice = mapping[field.key];
      if (choice?.kind === "column") {
        values[field.key] = row.cells[choice.index] ?? "";
        if (row.numericColumns.includes(choice.index)) numericFields.push(field.key);
      } else { values[field.key] = choice?.kind === "fixed" ? choice.value : ""; }
    }
    return { sourceRow: row.sourceRow, values, numericFields, excluded: false };
  });
}

export function importRequest(entity: ImportEntity, numberFormat: NumberFormat, rows: DraftRow[]): ImportRequest {
  return { entity, numberFormat, rows: rows.filter((row) => !row.excluded).map(({ sourceRow, values, numericFields }) => ({ sourceRow, values, numericFields })) };
}

export const IMPORT_QUERY_PREFIXES: Record<ImportEntity, string[]> = {
  customers: ["customersPage", "customersLookup", "dashboard-data"],
  employees: ["usersPage", "usersLookup", "users"],
  inventory: ["inventoryItemsPage", "inventorySummary", "inventory-insights", "inventory-movements", "inventory-lookup", "dashboard-data", "financial-report"],
};
