import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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

const MIN_ZOOM = 0.75;
const MAX_ZOOM = 1.5;
const ZOOM_STEP = 0.15;

export function PdfPreviewDialog({
  preview,
  onClose,
}: {
  preview: PdfPreview | null;
  onClose: () => void;
}) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const discardTimersRef = useRef(
    new Map<string, ReturnType<typeof setTimeout>>(),
  );
  const [zoom, setZoom] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    setZoom(1);
    setIsLoading(Boolean(preview));
  }, [preview]);

  useEffect(() => {
    const token = preview?.token;
    if (!token) return;
    const discardTimers = discardTimersRef.current;

    const pendingDiscard = discardTimers.get(token);
    if (pendingDiscard) {
      clearTimeout(pendingDiscard);
      discardTimers.delete(token);
    }

    return () => {
      const timer = setTimeout(() => {
        discardTimers.delete(token);
        void invoke("discard_pdf_preview", { token });
      }, 0);
      discardTimers.set(token, timer);
    };
  }, [preview?.token]);

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

  const printPreview = () => {
    iframeRef.current?.contentWindow?.print();
  };

  return (
    <Dialog
      open={Boolean(preview)}
      onOpenChange={(open) => {
        if (!open) onClose();
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

        <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-muted/30 px-4 py-2">
          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="outline"
              size="icon"
              aria-label="Diminuir zoom"
              onClick={() => setZoom((value) => Math.max(MIN_ZOOM, value - ZOOM_STEP))}
              disabled={zoom <= MIN_ZOOM || isLoading}
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
              disabled={zoom >= MAX_ZOOM || isLoading}
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
              {isSaving ? (
                <LoaderCircle className="h-4 w-4 animate-spin" />
              ) : (
                <Download className="h-4 w-4" />
              )}
              Salvar PDF
            </Button>
            <Button
              type="button"
              size="sm"
              className="gap-2"
              onClick={printPreview}
              disabled={isLoading || !preview}
            >
              <Printer className="h-4 w-4" /> Imprimir
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-auto bg-muted/50 p-4 sm:p-6">
          {isLoading && (
            <div className="flex min-h-64 items-center justify-center gap-2 text-sm text-muted-foreground">
              <LoaderCircle className="h-5 w-5 animate-spin" />
              Preparando pré-visualização...
            </div>
          )}
          {preview && (
            <div
              className="mx-auto origin-top transition-transform"
              style={{
                height: `${100 / zoom}%`,
                transform: `scale(${zoom})`,
                width: `${100 / zoom}%`,
              }}
            >
              <iframe
                ref={iframeRef}
                title={`Pré-visualização de ${preview.fileName}`}
                srcDoc={preview.html}
                sandbox="allow-modals allow-same-origin"
                className={isLoading ? "hidden" : "h-full min-h-[1123px] w-full border-0 bg-white shadow-md"}
                onLoad={() => setIsLoading(false)}
              />
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
