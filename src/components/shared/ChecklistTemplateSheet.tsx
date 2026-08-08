import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronUp, Plus, Save, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  clearFieldError,
  parseErrors,
  templateSchema,
  ValidationErrors,
} from "@/lib/validation";
import { ChecklistTemplate } from "@/lib/types";

interface ChecklistTemplateSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  template?: ChecklistTemplate | null;
  onCreated?: (template: ChecklistTemplate) => void;
}

export function ChecklistTemplateSheet({
  open,
  onOpenChange,
  template = null,
  onCreated,
}: ChecklistTemplateSheetProps) {
  const queryClient = useQueryClient();
  const [title, setTitle] = useState("");
  const [items, setItems] = useState<string[]>([]);
  const [newItem, setNewItem] = useState("");
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTitle(template?.title ?? "");
    setItems(template?.items ?? []);
    setNewItem("");
    setErrors({});
  }, [open, template]);

  const addItem = () => {
    const value = newItem.trim();
    if (!value) return;
    setItems((current) => [...current, value]);
    setNewItem("");
    setErrors((current) => clearFieldError(current, "items"));
  };

  const moveItem = (index: number, direction: -1 | 1) => {
    const nextIndex = index + direction;
    if (nextIndex < 0 || nextIndex >= items.length) return;
    setItems((current) => {
      const next = [...current];
      [next[index], next[nextIndex]] = [next[nextIndex], next[index]];
      return next;
    });
  };

  const handleSave = async () => {
    if (isSaving) return;
    const fieldErrors = parseErrors(templateSchema.safeParse({ title, items }));
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }

    setErrors({});
    setIsSaving(true);
    try {
      if (template) {
        await invoke("update_checklist_template", { id: template.id, title, items });
        await queryClient.invalidateQueries({ queryKey: ["checklist-templates"] });
        toastSuccess("Template atualizado com sucesso.");
      } else {
        const id = await invoke<string>("create_checklist_template", { title, items });
        const created = { id, title, items };
        queryClient.setQueryData<ChecklistTemplate[]>(
          ["checklist-templates"],
          (templates = []) => [
            created,
            ...templates.filter((entry) => entry.id !== created.id),
          ],
        );
        onCreated?.(created);
        toastSuccess("Modelo de checklist criado com sucesso.");
      }
      onOpenChange(false);
    } catch (error) {
      toastError(error, "Erro ao salvar modelo de checklist.");
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="flex h-full flex-col sm:max-w-md">
        <SheetHeader>
          <SheetTitle>{template ? "Editar modelo" : "Novo modelo"}</SheetTitle>
          <SheetDescription>
            Crie uma lista de verificação para a entrada de aparelhos.
          </SheetDescription>
        </SheetHeader>

        <div className="flex flex-1 flex-col gap-6 overflow-hidden py-6">
          <div className="grid gap-2">
            <Label htmlFor="checklist-template-title">Título do modelo</Label>
            <Input
              id="checklist-template-title"
              value={title}
              placeholder="Ex: Checklist iPhone"
              onChange={(event) => {
                setTitle(event.target.value);
                setErrors((current) => clearFieldError(current, "title"));
              }}
            />
            {errors.title && <p className="text-xs text-destructive">{errors.title}</p>}
          </div>

          <Separator />

          <div className="flex flex-1 flex-col gap-4 overflow-hidden">
            <Label>Itens do Checklist</Label>
            {errors.items && <p className="text-xs text-destructive">{errors.items}</p>}
            <div className="flex gap-2">
              <Input
                placeholder="Novo item..."
                value={newItem}
                onChange={(event) => setNewItem(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    addItem();
                  }
                }}
              />
              <Button type="button" size="icon" onClick={addItem}>
                <Plus className="h-4 w-4" />
              </Button>
            </div>

            <ScrollArea className="flex-1 rounded-md border p-2">
              <div className="space-y-2">
                {items.map((entry, index) => (
                  <div
                    className="group flex items-center gap-2 rounded-sm border border-transparent bg-muted/50 p-2 transition-colors hover:border-primary/20"
                    key={`${entry}-${index}`}
                  >
                    <span className="flex-1 text-sm">{entry}</span>
                    <div className="flex items-center">
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        aria-label={`Mover ${entry} para cima`}
                        onClick={() => moveItem(index, -1)}
                        disabled={index === 0}
                      >
                        <ChevronUp className="h-3 w-3" />
                      </Button>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        aria-label={`Mover ${entry} para baixo`}
                        onClick={() => moveItem(index, 1)}
                        disabled={index === items.length - 1}
                      >
                        <ChevronDown className="h-3 w-3" />
                      </Button>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-destructive opacity-0 transition-opacity group-hover:opacity-100"
                      aria-label={`Remover ${entry}`}
                      onClick={() =>
                        setItems((current) => current.filter((_, itemIndex) => itemIndex !== index))
                      }
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                ))}
                {!items.length && (
                  <div className="py-8 text-center text-xs italic text-muted-foreground">
                    Nenhum item adicionado ainda.
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>

        <SheetFooter className="border-t pt-4">
          <Button
            type="button"
            variant="outline"
            className="w-full"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button type="button" className="w-full gap-2" onClick={handleSave} disabled={isSaving}>
            <Save className="h-4 w-4" />
            {isSaving ? "Salvando..." : template ? "Salvar alterações" : "Criar modelo"}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
