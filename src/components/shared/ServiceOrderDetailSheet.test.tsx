import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ServiceOrderDetailSheet } from "@/components/shared/ServiceOrderDetailSheet";
import { configureDataClient } from "@/lib/data-client";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("@/components/shared/ServiceOrderDrawerProvider", () => ({
  useServiceOrderDrawer: () => ({ openCustomerHistory: vi.fn() }),
}));
vi.mock("@/components/shared/PdfAttachmentPreviewDialog", () => ({
  PdfAttachmentPreviewDialog: ({
    open,
    dataUrl,
    fileName,
    error,
    onClose,
  }: {
    open: boolean;
    dataUrl: string;
    fileName: string;
    error: boolean;
    onClose: () => void;
  }) => open ? (
    <div role="dialog" aria-label="Visualização do PDF">
      {error ? <p>Não foi possível carregar o PDF.</p> : dataUrl ? <div title={`Visualização de ${fileName}`} data-url={dataUrl} /> : <p>Carregando PDF...</p>}
      <button type="button" onClick={onClose}>Fechar PDF</button>
    </div>
  ) : null,
}));

const mockedInvoke = vi.mocked(invoke);
const pdfAttachment = {
  id: "attachment-pdf",
  serviceOrderId: "order-1",
  fileName: "laudo.pdf",
  storageName: "attachment-pdf",
  mimeType: "application/pdf",
  sizeBytes: 512,
  createdAt: "2026-09-14T12:00:00Z",
};
const imageAttachment = {
  ...pdfAttachment,
  id: "attachment-image",
  fileName: "entrada.png",
  mimeType: "image/png",
};

function renderDetail() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ServiceOrderDetailSheet orderId="order-1" open onClose={() => undefined} />
    </QueryClientProvider>,
  );
}

describe("ServiceOrderDetailSheet attachments", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedInvoke.mockImplementation((command, args) => {
      if (command === "get_service_order") {
        return Promise.resolve({
          id: "order-1",
          customerId: "customer-1",
          customerName: "Cliente Teste",
          equipment: "Notebook",
          description: "Não liga",
          status: "Orçamento",
          totalPrice: 0,
          createdAt: "2026-09-14T12:00:00Z",
          displayId: "OS-000001",
          discountBasisPoints: 0,
        });
      }
      if (
        command === "get_service_order_parts" ||
        command === "get_service_order_checklist" ||
        command === "get_service_order_events"
      ) {
        return Promise.resolve([]);
      }
      if (command === "get_service_order_attachments") return Promise.resolve([pdfAttachment]);
      if (command === "read_service_order_attachment") {
        expect(args).toEqual({ id: pdfAttachment.id });
        return Promise.resolve("data:application/pdf;base64,JVBERi0=");
      }
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    cleanup();
    configureDataClient("local");
  });

  it("loads, displays, and closes an attached PDF in its dialog", async () => {
    const user = userEvent.setup();
    let resolvePdf: ((value: string) => void) | undefined;
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_service_order") {
        return Promise.resolve({
          id: "order-1", customerId: "customer-1", equipment: "Notebook", description: "Não liga", status: "Orçamento", totalPrice: 0, createdAt: "2026-09-14T12:00:00Z", displayId: "OS-000001", discountBasisPoints: 0,
        });
      }
      if (command === "get_service_order_attachments") return Promise.resolve([pdfAttachment]);
      if (command === "read_service_order_attachment") {
        return new Promise((resolve) => {
          resolvePdf = resolve;
        });
      }
      return Promise.resolve([]);
    });
    renderDetail();

    const previewButton = await screen.findByRole("button", {
      name: "Visualizar PDF",
    });
    await user.click(previewButton);

    expect(screen.getByText("Carregando PDF...")).toBeInTheDocument();
    resolvePdf?.("data:application/pdf;base64,JVBERi0=");
    const viewer = await screen.findByTitle("Visualização de laudo.pdf");
    expect(viewer).toHaveAttribute("data-url", "data:application/pdf;base64,JVBERi0=");
    expect(mockedInvoke).toHaveBeenCalledWith("read_service_order_attachment", {
      id: pdfAttachment.id,
    });

    await user.click(screen.getByRole("button", { name: "Fechar PDF" }));
    expect(screen.queryByTitle("Visualização de laudo.pdf")).not.toBeInTheDocument();
  });

  it("shows the specified error and retains download when PDF loading fails", async () => {
    const user = userEvent.setup();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_service_order") {
        return Promise.resolve({
          id: "order-1", customerId: "customer-1", equipment: "Notebook", description: "Não liga", status: "Orçamento", totalPrice: 0, createdAt: "2026-09-14T12:00:00Z", displayId: "OS-000001", discountBasisPoints: 0,
        });
      }
      if (command === "get_service_order_attachments") return Promise.resolve([pdfAttachment]);
      if (command === "read_service_order_attachment") return Promise.reject(new Error("Falha"));
      return Promise.resolve([]);
    });
    renderDetail();

    await user.click(await screen.findByRole("button", { name: "Visualizar PDF" }));
    expect(await screen.findByText("Não foi possível carregar o PDF.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Baixar" })).toBeEnabled();
  });

  it("does not offer an image preview label for PDF attachments", async () => {
    renderDetail();
    await screen.findByRole("button", { name: "Visualizar PDF" });
    expect(screen.queryByRole("button", { name: "Visualizar imagem" })).not.toBeInTheDocument();
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("get_service_order_attachments", { serviceOrderId: "order-1" }));
  });

  it("keeps the existing image preview data URL unchanged", async () => {
    const user = userEvent.setup();
    mockedInvoke.mockImplementation((command, args) => {
      if (command === "get_service_order") {
        return Promise.resolve({
          id: "order-1", customerId: "customer-1", equipment: "Notebook", description: "Não liga", status: "Orçamento", totalPrice: 0, createdAt: "2026-09-14T12:00:00Z", displayId: "OS-000001", discountBasisPoints: 0,
        });
      }
      if (command === "get_service_order_attachments") return Promise.resolve([imageAttachment]);
      if (command === "read_service_order_attachment") {
        expect(args).toEqual({ id: imageAttachment.id });
        return Promise.resolve("data:image/png;base64,iVBORw0KGgo=");
      }
      return Promise.resolve([]);
    });
    renderDetail();

    await user.click(await screen.findByRole("button", { name: "Visualizar imagem" }));

    expect(await screen.findByAltText("entrada.png")).toHaveAttribute(
      "src",
      "data:image/png;base64,iVBORw0KGgo=",
    );
  });

  it("uses the LAN data command when a client previews an attached PDF", async () => {
    const user = userEvent.setup();
    renderDetail();
    await screen.findByRole("button", { name: "Visualizar PDF" });
    configureDataClient("client");
    mockedInvoke.mockImplementation((command, args) => {
      if (
        command === "lan_remote_command" &&
        (args as { operation?: string }).operation === "read_service_order_attachment"
      ) {
        return Promise.resolve("data:application/pdf;base64,JVBERi0=");
      }
      return Promise.resolve([]);
    });

    await user.click(screen.getByRole("button", { name: "Visualizar PDF" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("lan_remote_command", {
        operation: "read_service_order_attachment",
        payload: { id: pdfAttachment.id },
        idempotencyKey: null,
      });
    });
    expect(await screen.findByTitle("Visualização de laudo.pdf")).toHaveAttribute(
      "data-url",
      "data:application/pdf;base64,JVBERi0=",
    );
  });
});
