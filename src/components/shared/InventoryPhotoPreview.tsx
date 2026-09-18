import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface InventoryPhotoPreviewProps {
  photoDataUrl: string;
  itemName: string;
}

export function InventoryPhotoPreview({
  photoDataUrl,
  itemName,
}: InventoryPhotoPreviewProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <button
        type="button"
        className="shrink-0 rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        aria-label={`Ampliar foto de ${itemName}`}
        onClick={() => setOpen(true)}
      >
        <img src={photoDataUrl} alt={`Foto de ${itemName}`} className="h-10 w-10 rounded object-cover" />
      </button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-3xl p-0 overflow-hidden">
          <DialogHeader className="border-b px-6 py-4 pr-12">
            <DialogTitle>Foto do item</DialogTitle>
            <DialogDescription>{itemName}</DialogDescription>
          </DialogHeader>
          <div className="flex max-h-[75dvh] min-h-64 items-center justify-center bg-muted/50 p-4">
            <img src={photoDataUrl} alt={`Foto ampliada de ${itemName}`} className="max-h-[65dvh] max-w-full rounded object-contain" />
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
