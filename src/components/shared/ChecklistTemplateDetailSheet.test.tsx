import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ChecklistTemplateDetailSheet } from "@/components/shared/ChecklistTemplateDetailSheet";

describe("ChecklistTemplateDetailSheet", () => {
  it("shows the selected model and its items in read-only mode", () => {
    render(
      <ChecklistTemplateDetailSheet
        open
        onClose={vi.fn()}
        template={{
          id: "template-1",
          title: "Entrada de smartphone",
          items: ["Verificar tela", "Testar carregamento"],
          createdAt: "2026-01-15T12:00:00Z",
        }}
      />,
    );

    expect(screen.getByRole("heading", { name: "Entrada de smartphone" })).toBeInTheDocument();
    expect(screen.getByText("2 itens")).toBeInTheDocument();
    expect(screen.getByText("Verificar tela")).toBeInTheDocument();
    expect(screen.getByText("Testar carregamento")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /editar/i })).not.toBeInTheDocument();
  });
});
