import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StrictMode } from "react";
import { PdfPreviewDialog } from "@/components/shared/PdfPreviewDialog";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const preview = {
  token: "preview-token",
  fileName: "OS-000001.pdf",
  html: "<!doctype html><html><body><h1>OS-000001</h1></body></html>",
};

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("PdfPreviewDialog", () => {
  it("renders the HTML preview and saves the real PDF by token", async () => {
    mockedInvoke.mockResolvedValue(true);
    const user = userEvent.setup();

    render(
      <StrictMode>
        <PdfPreviewDialog preview={preview} onClose={() => undefined} />
      </StrictMode>,
    );

    const iframe = screen.getByTitle("Pré-visualização de OS-000001.pdf");
    expect(iframe).toHaveAttribute("srcdoc", preview.html);

    await user.click(screen.getByRole("button", { name: "Salvar PDF" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("save_pdf_preview", {
        token: "preview-token",
      });
    });
    expect(mockedInvoke).not.toHaveBeenCalledWith("discard_pdf_preview", {
      token: "preview-token",
    });
  });

  it("prints the isolated preview document", async () => {
    const user = userEvent.setup();
    render(<PdfPreviewDialog preview={preview} onClose={() => undefined} />);

    const iframe = screen.getByTitle("Pré-visualização de OS-000001.pdf") as HTMLIFrameElement;
    const print = vi.fn();
    Object.defineProperty(iframe, "contentWindow", {
      configurable: true,
      value: { print },
    });
    iframe.dispatchEvent(new Event("load"));

    await user.click(screen.getByRole("button", { name: "Imprimir" }));
    expect(print).toHaveBeenCalledOnce();
  });

  it("discards the token after the preview really closes", async () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <StrictMode>
        <PdfPreviewDialog preview={preview} onClose={() => undefined} />
      </StrictMode>,
    );

    expect(mockedInvoke).not.toHaveBeenCalledWith("discard_pdf_preview", {
      token: "preview-token",
    });

    rerender(
      <StrictMode>
        <PdfPreviewDialog preview={null} onClose={() => undefined} />
      </StrictMode>,
    );
    await vi.runAllTimersAsync();

    expect(mockedInvoke).toHaveBeenCalledWith("discard_pdf_preview", {
      token: "preview-token",
    });
  });
});
