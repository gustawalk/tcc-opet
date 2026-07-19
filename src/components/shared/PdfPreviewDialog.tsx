import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  getDocument,
  GlobalWorkerOptions,
  type PDFDocumentProxy,
} from "pdfjs-dist";
import pdfWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import {
  Download,
  FileText,
  LoaderCircle,
  Printer,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { toastError, toastSuccess } from "@/lib/errors";
import type { PdfPreview } from "@/lib/types";

GlobalWorkerOptions.workerSrc = pdfWorker;

const MIN_ZOOM = 0.75;
const MAX_ZOOM = 1.5;
const ZOOM_STEP = 0.15;

type CancellableRenderTask = {
  cancel: () => void;
  promise: Promise<void>;
};

function decodePdfDataUrl(dataUrl: string) {
  const separatorIndex = dataUrl.indexOf(",");
  if (separatorIndex < 0) throw new Error("Conteúdo de PDF inválido.");

  const binary = window.atob(dataUrl.slice(separatorIndex + 1));
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

export function PdfPreviewDialog({
  preview,
  onClose,
}: {
  preview: PdfPreview | null;
  onClose: () => void;
}) {
  const canvasRefs = useRef(new Map<number, HTMLCanvasElement>());
  const renderTasks = useRef(new Set<CancellableRenderTask>());
  const [pdfDocument, setPdfDocument] = useState<PDFDocumentProxy | null>(null);
  const [pageNumbers, setPageNumbers] = useState<number[]>([]);
  const [zoom, setZoom] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [isRendering, setIsRendering] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!preview) {
      setPdfDocument(null);
      setPageNumbers([]);
      return;
    }

    let cancelled = false;
    const loadingTask = getDocument({ data: decodePdfDataUrl(preview.dataUrl) });
    canvasRefs.current.clear();
    setPdfDocument(null);
    setPageNumbers([]);
    setIsLoading(true);
    setError(null);
    setZoom(1);

    void loadingTask.promise
      .then((document) => {
        if (cancelled) {
          return;
        }
        setPdfDocument(document);
        setPageNumbers(
          Array.from({ length: document.numPages }, (_, index) => index + 1),
        );
      })
      .catch(() => {
        if (!cancelled) setError("Não foi possível carregar a pré-visualização do PDF.");
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
      void loadingTask.destroy();
    };
  }, [preview]);

  useEffect(() => {
    if (!pdfDocument || !pageNumbers.length) {
      setIsRendering(false);
      return;
    }

    let cancelled = false;
    const activeRenderTasks = renderTasks.current;
    const renderPages = async () => {
      setIsRendering(true);
      const outputScale = window.devicePixelRatio || 1;
      for (const pageNumber of pageNumbers) {
        const canvas = canvasRefs.current.get(pageNumber);
        if (!canvas) continue;

        const page = await pdfDocument.getPage(pageNumber);
        const viewport = page.getViewport({ scale: zoom * outputScale });

        canvas.width = Math.floor(viewport.width);
        canvas.height = Math.floor(viewport.height);
        canvas.style.width = `${Math.floor(viewport.width / outputScale)}px`;
        canvas.style.height = `${Math.floor(viewport.height / outputScale)}px`;
        const renderTask = page.render({ canvas, viewport });
        activeRenderTasks.add(renderTask);
        try {
          await renderTask.promise;
        } finally {
          activeRenderTasks.delete(renderTask);
        }
        if (cancelled) return;
      }
    };

    void renderPages()
      .catch(() => {
        if (!cancelled) setError("Não foi possível renderizar a pré-visualização do PDF.");
      })
      .finally(() => {
        if (!cancelled) setIsRendering(false);
      });

    return () => {
      cancelled = true;
      for (const task of activeRenderTasks) task.cancel();
      activeRenderTasks.clear();
    };
  }, [pageNumbers, pdfDocument, zoom]);

  useEffect(() => {
    const token = preview?.token;
    return () => {
      if (token) void invoke("discard_pdf_preview", { token });
    };
  }, [preview?.token]);

  const closePreview = () => {
    onClose();
  };

  const savePreview = async () => {
    if (!preview) return;

    setIsSaving(true);
    try {
      const saved = await invoke<boolean>("save_pdf_preview", {
        token: preview.token,
      });
      if (saved) toastSuccess("PDF salvo com sucesso.");
    } catch (saveError) {
      toastError(saveError, "Erro ao salvar PDF.");
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Dialog
      open={Boolean(preview)}
      onOpenChange={(open) => {
        if (!open) closePreview();
      }}
    >
      <DialogContent className="flex h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-6xl">
        <DialogHeader className="border-b px-6 py-4 pr-12">
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5 text-primary" />
            Pré-visualização do PDF
          </DialogTitle>
          <DialogDescription className="truncate" title={preview?.fileName}>
            {preview?.fileName}
          </DialogDescription>
        </DialogHeader>

        <div className="pdf-preview-toolbar flex flex-wrap items-center justify-between gap-2 border-b bg-muted/30 px-4 py-2">
          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="outline"
              size="icon"
              aria-label="Diminuir zoom"
              onClick={() => setZoom((value) => Math.max(MIN_ZOOM, value - ZOOM_STEP))}
              disabled={zoom <= MIN_ZOOM || isLoading || isRendering}
            >
              <ZoomOut className="h-4 w-4" />
            </Button>
            <span className="min-w-14 text-center text-xs text-muted-foreground">
              {Math.round(zoom * 100)}%
            </span>
            <Button
              type="button"
              variant="outline"
              size="icon"
              aria-label="Aumentar zoom"
              onClick={() => setZoom((value) => Math.min(MAX_ZOOM, value + ZOOM_STEP))}
              disabled={zoom >= MAX_ZOOM || isLoading || isRendering}
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={savePreview}
              disabled={!preview || isSaving}
            >
              {isSaving ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
              Salvar PDF
            </Button>
            <Button
              type="button"
              size="sm"
              className="gap-2"
              onClick={() => window.print()}
              disabled={isLoading || isRendering || Boolean(error)}
            >
              <Printer className="h-4 w-4" /> Imprimir
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-auto bg-muted/50 p-4 sm:p-6">
          <div className="pdf-preview-print-root mx-auto flex w-fit max-w-full flex-col gap-4">
            {(isLoading || isRendering) && (
              <div className="flex min-h-64 items-center justify-center gap-2 text-sm text-muted-foreground">
                <LoaderCircle className="h-5 w-5 animate-spin" />
                {isLoading ? "Carregando PDF..." : "Renderizando páginas..."}
              </div>
            )}
            {error && (
              <div className="rounded-md border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
                {error}
              </div>
            )}
            {pageNumbers.map((pageNumber) => (
              <canvas
                key={pageNumber}
                ref={(canvas) => {
                  if (canvas) canvasRefs.current.set(pageNumber, canvas);
                  else canvasRefs.current.delete(pageNumber);
                }}
                className="pdf-preview-page max-w-full rounded-sm bg-white shadow-md"
                aria-label={`Página ${pageNumber} do PDF`}
              />
            ))}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
