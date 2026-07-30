import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Box, DollarSign, Save, TrendingUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  clearFieldError,
  inventoryItemSchema,
  parseErrors,
  ValidationErrors,
} from "@/lib/validation";
import { InventoryItem } from "@/lib/types";

type InventoryItemFormData = Pick<
  InventoryItem,
  | "name"
  | "description"
  | "type"
  | "minQuantity"
  | "costPrice"
  | "salePrice"
> & {
  supplierName: string;
};

interface InventoryItemSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialType?: InventoryItem["type"];
  initialPartQuantity?: number;
  item?: InventoryItem | null;
  onCreated?: (item: InventoryItem) => void;
}

const createInitialFormData = (
  type: InventoryItem["type"],
): InventoryItemFormData => ({
  name: "",
  description: "",
  type,
  minQuantity: type === "part" ? 5 : 0,
  costPrice: 0,
  salePrice: 0,
  supplierName: "",
});

export function InventoryItemSheet({
  open,
  onOpenChange,
  initialType = "part",
  initialPartQuantity = 0,
  item = null,
  onCreated,
}: InventoryItemSheetProps) {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState<InventoryItemFormData>(() =>
    createInitialFormData(initialType),
  );
  const [errors, setErrors] = useState<ValidationErrors>({});

  useEffect(() => {
    if (!open) return;

    setErrors({});
    setFormData(
      item
        ? {
            name: item.name,
            description: item.description,
            type: item.type,
            minQuantity: item.minQuantity,
            costPrice: item.costPrice,
            salePrice: item.salePrice,
            supplierName: item.supplierName ?? "",
          }
        : createInitialFormData(initialType),
    );
  }, [initialType, item, open]);

  const createMutation = useMutation({
    mutationFn: async (data: InventoryItemFormData) =>
      invoke<InventoryItem>("create_inventory_item", {
        name: data.name,
        description: data.description,
        type: data.type,
        minQuantity: data.minQuantity,
        currentQuantity: data.type === "part" ? initialPartQuantity : 999,
        costPrice: data.costPrice,
        salePrice: data.salePrice,
        supplierName: data.supplierName,
      }),
  });
  const updateMutation = useMutation({
    mutationFn: async (data: InventoryItemFormData) => {
      if (!item) return;
      await invoke("update_inventory_item", {
        id: item.id,
        ...data,
        currentQuantity: item.currentQuantity,
      });
    },
  });
  const isSaving = createMutation.isPending || updateMutation.isPending;

  const updateField = <K extends keyof InventoryItemFormData>(
    field: K,
    value: InventoryItemFormData[K],
  ) => {
    setFormData((current) => ({ ...current, [field]: value }));
    setErrors((current) => clearFieldError(current, field));
  };

  const handleSave = async () => {
    if (isSaving) return;
    const fieldErrors = parseErrors(inventoryItemSchema.safeParse(formData));
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }

    setErrors({});
    try {
      if (item) {
        await updateMutation.mutateAsync(formData);
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["inventory"] }),
          queryClient.invalidateQueries({ queryKey: ["inventory-lookup"] }),
          queryClient.invalidateQueries({ queryKey: ["inventory-insights"] }),
        ]);
        toastSuccess("Item atualizado com sucesso.");
      } else {
        const created = await createMutation.mutateAsync(formData);
        queryClient.setQueryData<InventoryItem[]>(["inventory"], (items = []) => [
          created,
          ...items.filter((entry) => entry.id !== created.id),
        ]);
        queryClient.setQueryData<InventoryItem[]>(
          ["inventory-lookup"],
          (items = []) => [created, ...items.filter((entry) => entry.id !== created.id)],
        );
        await queryClient.invalidateQueries({ queryKey: ["inventory-insights"] });
        onCreated?.(created);
        toastSuccess("Item criado com sucesso.");
      }
      onOpenChange(false);
    } catch (error) {
      toastError(error, item ? "Erro ao atualizar item." : "Erro ao criar item.");
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>
            {item ? "Editar" : "Novo"}{" "}
            {formData.type === "part" ? "Item no Estoque" : "Servico"}
          </SheetTitle>
          <SheetDescription>
            {formData.type === "part"
              ? "Cadastre pecas e insumos para gerenciar seu estoque."
              : "Cadastre servicos e mao de obra para suas ordens de servico."}
          </SheetDescription>
        </SheetHeader>

        <div className="grid gap-4 py-6">
          <div className="grid gap-2">
            <Label htmlFor="inventory-item-name">
              Nome do {formData.type === "part" ? "Produto" : "Servico"}
            </Label>
            <Input
              id="inventory-item-name"
              value={formData.name}
              placeholder={
                formData.type === "part"
                  ? "Ex: Tela iPhone 11"
                  : "Ex: Mao de obra Drone"
              }
              onChange={(event) => updateField("name", event.target.value)}
            />
            {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="inventory-item-description">Descricao</Label>
            <Textarea
              id="inventory-item-description"
              value={formData.description}
              placeholder="Ex: Detalhes adicionais..."
              onChange={(event) => updateField("description", event.target.value)}
            />
            {errors.description && (
              <p className="text-xs text-destructive">{errors.description}</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="inventory-item-supplier">Fornecedor (opcional)</Label>
            <Input
              id="inventory-item-supplier"
              value={formData.supplierName}
              placeholder="Ex.: Distribuidora ABC"
              onChange={(event) => updateField("supplierName", event.target.value)}
            />
          </div>

          <Separator />

          {formData.type === "part" && (
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-min">Qtd. Minima (Alerta)</Label>
              <div className="relative">
                <Box className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="inventory-item-min"
                  type="number"
                  className="pl-9"
                  value={formData.minQuantity}
                  onChange={(event) =>
                    updateField("minQuantity", parseInt(event.target.value) || 0)
                  }
                />
              </div>
              {errors.minQuantity && (
                <p className="text-xs text-destructive">{errors.minQuantity}</p>
              )}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-cost">
                {formData.type === "part" ? "Preco de Custo" : "Custo Estimado"}
              </Label>
              <div className="relative">
                <DollarSign className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="inventory-item-cost"
                  type="number"
                  step="0.01"
                  className="pl-9"
                  value={formData.costPrice}
                  onChange={(event) =>
                    updateField("costPrice", parseFloat(event.target.value) || 0)
                  }
                />
              </div>
              {errors.costPrice && (
                <p className="text-xs text-destructive">{errors.costPrice}</p>
              )}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-sale">Preco de Venda</Label>
              <div className="relative">
                <TrendingUp className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-primary" />
                <Input
                  id="inventory-item-sale"
                  type="number"
                  step="0.01"
                  className="pl-9"
                  value={formData.salePrice}
                  onChange={(event) =>
                    updateField("salePrice", parseFloat(event.target.value) || 0)
                  }
                />
              </div>
            </div>
          </div>
        </div>

        <SheetFooter>
          <Button
            type="button"
            variant="outline"
            className="w-full"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button
            type="button"
            className="w-full gap-2"
            onClick={handleSave}
            disabled={isSaving}
          >
            <Save className="h-4 w-4" />
            {isSaving ? "Salvando..." : item ? "Salvar Alteracoes" : "Cadastrar"}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
