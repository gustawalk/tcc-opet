import { Calendar, ClipboardList } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ChecklistTemplate } from "@/lib/types";
import { formatDate } from "@/lib/formatters";

interface ChecklistTemplateDetailSheetProps {
  template: ChecklistTemplate | null;
  open: boolean;
  onClose: () => void;
}

export function ChecklistTemplateDetailSheet({
  template,
  open,
  onClose,
}: ChecklistTemplateDetailSheetProps) {
  return (
    <Sheet
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
    >
      <SheetContent className="flex h-full flex-col sm:max-w-xl">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2 text-2xl">
            <ClipboardList className="h-6 w-6 text-primary" />
            {template?.title ?? "Modelo de checklist"}
          </SheetTitle>
          <SheetDescription>
            Itens que serão aplicados à entrada de uma ordem.
          </SheetDescription>
        </SheetHeader>

        {template && (
          <div className="flex flex-1 flex-col gap-6 overflow-hidden py-6">
            <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
              <Badge variant="secondary">{template.items.length} itens</Badge>
              {template.createdAt && (
                <span className="flex items-center gap-1.5">
                  <Calendar className="h-4 w-4" />
                  Criado em {formatDate(template.createdAt)}
                </span>
              )}
            </div>
            <Separator />
            <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
              <h3 className="text-sm font-semibold">Itens do checklist</h3>
              {template.items.length ? (
                <ol className="space-y-2">
                  {template.items.map((item, index) => (
                    <li
                      className="flex gap-3 rounded-md border bg-muted/30 p-3 text-sm"
                      key={`${item}-${index}`}
                    >
                      <span className="font-medium text-primary">{index + 1}</span>
                      <span>{item}</span>
                    </li>
                  ))}
                </ol>
              ) : (
                <p className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
                  Este modelo não possui itens.
                </p>
              )}
            </div>
          </div>
        )}

        <SheetFooter className="border-t pt-4">
          <Button type="button" className="w-full" onClick={onClose}>
            Fechar
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
