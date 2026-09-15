import { useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, LoaderCircle } from "lucide-react";
import {
  getDocument,
  GlobalWorkerOptions,
} from "pdfjs-dist/legacy/build/pdf.mjs";
import pdfWorkerUrl from "pdfjs-dist/legacy/build/pdf.worker.mjs?url";
import { Button } from "@/components/ui/button";

GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

function dataUrlToBytes(dataUrl: string): Uint8Array {
  const base64 = dataUrl.split(",", 2)[1];
  if (!base64) throw new Error("PDF data URL inválida.");
  return Uint8Array.from(atob(base64), (character) => character.charCodeAt(0));
}

export function PdfAttachmentPreview({
  dataUrl,
  fileName,
}: {
  dataUrl: string;
  fileName: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [pageNumber, setPageNumber] = useState(1);
  const [pageCount, setPageCount] = useState<number | null>(null);
  const [error, setError] = useState(false);
  const [isRendering, setIsRendering] = useState(true);

  useEffect(() => {
    let cancelled = false;
    let loadingTask: ReturnType<typeof getDocument> | undefined;
    let renderTask:
      | { cancel: () => void; promise: Promise<void> }
      | undefined;

    const renderPage = async () => {
      setIsRendering(true);
      setError(false);
      try {
        loadingTask = getDocument({ data: dataUrlToBytes(dataUrl) });
        const document = await loadingTask.promise;
        if (cancelled) return;

        setPageCount(document.numPages);
        const page = await document.getPage(pageNumber);
        if (cancelled) return;

        const viewport = page.getViewport({ scale: 1.25 });
        const canvas = canvasRef.current;
        const context = canvas?.getContext("2d");
        if (!canvas || !context) throw new Error("Canvas indisponível.");

        const outputScale = window.devicePixelRatio || 1;
        canvas.width = Math.floor(viewport.width * outputScale);
        canvas.height = Math.floor(viewport.height * outputScale);
        canvas.style.width = `${Math.floor(viewport.width)}px`;
        canvas.style.height = `${Math.floor(viewport.height)}px`;
        renderTask = page.render({
          canvas,
          canvasContext: context,
          viewport,
          transform: outputScale === 1 ? undefined : [outputScale, 0, 0, outputScale, 0, 0],
        });
        await renderTask.promise;
      } catch {
        if (!cancelled) setError(true);
      } finally {
        if (!cancelled) setIsRendering(false);
      }
    };

    void renderPage();
    return () => {
      cancelled = true;
      renderTask?.cancel();
      loadingTask?.destroy();
    };
  }, [dataUrl, pageNumber]);

  return (
    <div className="overflow-auto rounded-md border bg-muted/30 p-3">
      <div className="mb-3 flex items-center justify-between gap-2">
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label="Página anterior"
          disabled={isRendering || pageNumber <= 1}
          onClick={() => setPageNumber((page) => page - 1)}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <span className="text-xs text-muted-foreground">
          Página {pageNumber} de {pageCount ?? "..."}
        </span>
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label="Próxima página"
          disabled={isRendering || pageCount === null || pageNumber >= pageCount}
          onClick={() => setPageNumber((page) => page + 1)}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
      {isRendering && (
        <div className="flex min-h-64 items-center justify-center gap-2 text-xs text-muted-foreground">
          <LoaderCircle className="h-4 w-4 animate-spin" /> Carregando página...
        </div>
      )}
      {error ? (
        <p className="flex min-h-64 items-center justify-center text-xs text-destructive">
          Não foi possível renderizar o PDF.
        </p>
      ) : (
        <canvas
          ref={canvasRef}
          aria-label={`Página ${pageNumber} de ${fileName}`}
          className={isRendering ? "hidden" : "mx-auto bg-white shadow-sm"}
        />
      )}
    </div>
  );
}
