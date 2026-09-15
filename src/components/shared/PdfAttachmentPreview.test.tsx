import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { getDocumentMock, getPageMock, renderMock } = vi.hoisted(() => ({
  getDocumentMock: vi.fn(),
  getPageMock: vi.fn(),
  renderMock: vi.fn(),
}));

vi.mock("pdfjs-dist/legacy/build/pdf.mjs", () => ({
  getDocument: getDocumentMock,
  GlobalWorkerOptions: {},
}));

import { PdfAttachmentPreview } from "@/components/shared/PdfAttachmentPreview";

describe("PdfAttachmentPreview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getPageMock.mockImplementation(() =>
      Promise.resolve({
        getViewport: () => ({ width: 400, height: 560 }),
        render: renderMock,
      }),
    );
    renderMock.mockReturnValue({ cancel: vi.fn(), promise: Promise.resolve() });
    getDocumentMock.mockReturnValue({
      promise: Promise.resolve({ numPages: 2, getPage: getPageMock }),
      destroy: vi.fn(),
    });
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
      {} as CanvasRenderingContext2D,
    );
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders PDF pages on a canvas and navigates through the document", async () => {
    const user = userEvent.setup();
    render(
      <PdfAttachmentPreview
        dataUrl="data:application/pdf;base64,JVBERi0="
        fileName="laudo.pdf"
      />,
    );

    expect(await screen.findByLabelText("Página 1 de laudo.pdf")).toBeVisible();
    expect(getDocumentMock).toHaveBeenCalledWith({
      data: expect.any(Uint8Array),
    });
    expect(getPageMock).toHaveBeenCalledWith(1);
    expect(renderMock).toHaveBeenCalledOnce();
    expect(screen.getByText("Página 1 de 2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Próxima página" }));
    await waitFor(() => expect(getPageMock).toHaveBeenCalledWith(2));
    expect(await screen.findByLabelText("Página 2 de laudo.pdf")).toBeVisible();
    expect(screen.getByRole("button", { name: "Próxima página" })).toBeDisabled();
  });

  it("shows a clear error when PDF.js cannot render the document", async () => {
    getDocumentMock.mockReturnValue({
      promise: Promise.reject(new Error("PDF inválido")),
      destroy: vi.fn(),
    });
    render(
      <PdfAttachmentPreview
        dataUrl="data:application/pdf;base64,JVBERi0="
        fileName="laudo.pdf"
      />,
    );

    expect(await screen.findByText("Não foi possível renderizar o PDF.")).toBeVisible();
    expect(screen.queryByLabelText("Página 1 de laudo.pdf")).not.toBeInTheDocument();
  });
});
