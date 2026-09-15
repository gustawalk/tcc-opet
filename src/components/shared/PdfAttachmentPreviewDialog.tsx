import { FileText, LoaderCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PdfAttachmentPreview } from "@/components/shared/PdfAttachmentPreview";

export function PdfAttachmentPreviewDialog({
  open,
  dataUrl,
  error,
  fileName,
  onClose,
}: {
  open: boolean;
  dataUrl?: string;
  error?: boolean;
  fileName: string;
  onClose: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="flex h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-6xl">
        <DialogHeader className="border-b px-6 py-4 pr-12">
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5 text-primary" />
            Visualização do PDF
          </DialogTitle>
          <DialogDescription className="truncate" title={fileName}>
            {fileName}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-auto bg-muted/50 p-4 sm:p-6">
          {dataUrl ? (
            <PdfAttachmentPreview dataUrl={dataUrl} fileName={fileName} />
          ) : error ? (
            <p className="flex min-h-64 items-center justify-center text-sm text-destructive">
              Não foi possível carregar o PDF.
            </p>
          ) : (
            <div className="flex min-h-64 items-center justify-center gap-2 text-sm text-muted-foreground">
              <LoaderCircle className="h-5 w-5 animate-spin" />
              Carregando PDF...
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
