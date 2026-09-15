import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PdfAttachmentPreviewDialog } from "@/components/shared/PdfAttachmentPreviewDialog";

vi.mock("@/components/shared/PdfAttachmentPreview", () => ({
  PdfAttachmentPreview: ({ dataUrl, fileName }: { dataUrl: string; fileName: string }) => (
    <div title={`Visualização de ${fileName}`} data-url={dataUrl} />
  ),
}));

describe("PdfAttachmentPreviewDialog", () => {
  it("uses the generated-PDF full-screen dialog treatment", () => {
    render(
      <PdfAttachmentPreviewDialog
        open
        dataUrl="data:application/pdf;base64,JVBERi0="
        fileName="laudo.pdf"
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole("dialog")).toHaveClass("h-[calc(100dvh-2rem)]");
    expect(screen.getByText("Visualização do PDF")).toBeInTheDocument();
    expect(screen.getByText("laudo.pdf")).toBeInTheDocument();
    expect(screen.getByTitle("Visualização de laudo.pdf")).toHaveAttribute(
      "data-url",
      "data:application/pdf;base64,JVBERi0=",
    );
  });

  it("shows loading and request failure states inside the dialog", () => {
    const { rerender } = render(
      <PdfAttachmentPreviewDialog open fileName="laudo.pdf" onClose={vi.fn()} />,
    );
    expect(screen.getByText("Carregando PDF...")).toBeInTheDocument();

    rerender(
      <PdfAttachmentPreviewDialog open error fileName="laudo.pdf" onClose={vi.fn()} />,
    );
    expect(screen.getByText("Não foi possível carregar o PDF.")).toBeInTheDocument();
  });
});
