import { describe, expect, it } from "vitest";
import { importRequest, mapImportRows, suggestMapping } from "./data-import";
import type { ParsedImport } from "./data-import";

describe("import mapping", () => {
  it("suggests unambiguous bilingual headers and leaves competing name columns for review", () => {
    expect(
      suggestMapping("customers", [
        "Name",
        "Full name",
        "Phone number",
        "ENDEREÇO",
        "date of birth",
      ]),
    ).toEqual({
      name: { kind: "ignore" },
      phone: { kind: "column", index: 2 },
      email: { kind: "ignore" },
      address: { kind: "column", index: 3 },
    });
    expect(
      suggestMapping("customers", ["Nome completo", "Phone", "E-mail"]).name,
    ).toEqual({ kind: "column", index: 0 });
  });
  it("uses indexed columns, fixed values and only existing fields while preserving spreadsheet numeric types", () => {
    const source: ParsedImport = {
      headers: ["Name", "Name", "Unknown"],
      warnings: [],
      rows: [
        {
          sourceRow: 9,
          cells: ["First", "Tela", "12.34"],
          numericColumns: [2],
        },
      ],
    };
    const mapping = suggestMapping("inventory", source.headers);
    mapping.name = { kind: "column", index: 1 };
    mapping.description = { kind: "column", index: 1 };
    mapping.costPrice = { kind: "column", index: 2 };
    mapping.supplierName = { kind: "fixed", value: "Fornecedor" };
    const rows = mapImportRows("inventory", source, mapping);
    expect(rows[0].values).toEqual({
      name: "Tela",
      description: "Tela",
      type: "part",
      tracksStock: "false",
      minQuantity: "0",
      initialQuantity: "0",
      costPrice: "12.34",
      salePrice: "0",
      supplierName: "Fornecedor",
    });
    expect(rows[0].numericFields).toEqual(["costPrice"]);
    expect(rows[0].sourceRow).toBe(9);
    rows.push({ ...rows[0], sourceRow: 10, excluded: true });
    expect(importRequest("inventory", "brazilian", rows)).toEqual({
      entity: "inventory",
      numberFormat: "brazilian",
      rows: [
        { sourceRow: 9, values: rows[0].values, numericFields: ["costPrice"] },
      ],
    });
  });
});
