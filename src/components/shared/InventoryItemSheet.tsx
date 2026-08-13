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
import {
  formatCurrencyInput,
  formatCurrencyInputValue,
  normalizeCurrencyInput,
  normalizeIntegerInput,
  sanitizeIntegerInput,
} from "@/lib/numeric-input";
import { isUnchangedDuplicate } from "@/lib/inventory-duplicate";

type InventoryItemFormData = Pick<
  InventoryItem,
  "name" | "description" | "type"
> & {
  supplierName: string;
  minQuantity: string;
  initialQuantity: string;
  costPrice: string;
  salePrice: string;
};

type InventoryItemPayload = Omit<InventoryItemFormData, "minQuantity" | "initialQuantity" | "costPrice" | "salePrice"> & {
  minQuantity: number;
  initialQuantity: number;
  costPrice: number;
  salePrice: number;
};

interface InventoryItemSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialType?: InventoryItem["type"];
  initialPartQuantity?: number;
  item?: InventoryItem | null;
  duplicateItem?: InventoryItem | null;
  onCreated?: (item: InventoryItem) => void;
}

const createInitialFormData = (
  type: InventoryItem["type"],
  initialPartQuantity: number,
): InventoryItemFormData => ({
  name: "",
  description: "",
  type,
  minQuantity: String(type === "part" ? 5 : 0),
  costPrice: formatCurrencyInputValue(0),
  salePrice: formatCurrencyInputValue(0),
  supplierName: "",
  initialQuantity: String(type === "part" ? initialPartQuantity : 0),
});

export function InventoryItemSheet({
  open,
  onOpenChange,
  initialType = "part",
  initialPartQuantity = 0,
  item = null,
  duplicateItem = null,
  onCreated,
}: InventoryItemSheetProps) {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState<InventoryItemFormData>(() =>
    createInitialFormData(initialType, initialPartQuantity),
  );
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [duplicateConfirmationOpen, setDuplicateConfirmationOpen] =
    useState(false);
  const [pendingDuplicate, setPendingDuplicate] =
    useState<InventoryItemPayload | null>(null);

  const isEditing = item !== null;
  const isDuplicating = duplicateItem !== null;

  useEffect(() => {
    if (!open) return;

    setErrors({});
    setDuplicateConfirmationOpen(false);
    setPendingDuplicate(null);
    setFormData(
      item
        ? {
            name: item.name,
            description: item.description,
            type: item.type,
            minQuantity: String(item.minQuantity),
            costPrice: formatCurrencyInputValue(item.costPrice),
            salePrice: formatCurrencyInputValue(item.salePrice),
            supplierName: item.supplierName ?? "",
            initialQuantity: String(item.currentQuantity),
          }
        : duplicateItem
          ? {
              name: duplicateItem.name,
              description: duplicateItem.description,
              type: duplicateItem.type,
              minQuantity: String(duplicateItem.minQuantity),
              costPrice: formatCurrencyInputValue(duplicateItem.costPrice),
              salePrice: formatCurrencyInputValue(duplicateItem.salePrice),
              supplierName: duplicateItem.supplierName ?? "",
              initialQuantity: "0",
            }
          : createInitialFormData(initialType, initialPartQuantity),
    );
  }, [duplicateItem, initialPartQuantity, initialType, item, open]);

  const createMutation = useMutation({
    mutationFn: async (data: InventoryItemPayload) =>
      invoke<InventoryItem>("create_inventory_item", {
        name: data.name,
        description: data.description,
        type: data.type,
        minQuantity: data.minQuantity,
        currentQuantity: data.type === "part" ? data.initialQuantity : 999,
        costPrice: data.costPrice,
        salePrice: data.salePrice,
        supplierName: data.supplierName,
      }),
  });
  const updateMutation = useMutation({
    mutationFn: async (data: InventoryItemPayload) => {
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

  const closeDuplicateConfirmation = () => {
    setDuplicateConfirmationOpen(false);
    setPendingDuplicate(null);
  };

  const createItem = async (data: InventoryItemPayload) => {
    const created = await createMutation.mutateAsync(data);
    queryClient.setQueryData<InventoryItem[]>(
      ["inventory-lookup"],
      (items = []) => [created, ...items.filter((entry) => entry.id !== created.id)],
    );
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["inventoryItemsPage"] }),
      queryClient.invalidateQueries({ queryKey: ["inventorySummary"] }),
      queryClient.invalidateQueries({ queryKey: ["inventory-insights"] }),
    ]);
    onCreated?.(created);
    toastSuccess(
      isDuplicating ? "Item duplicado com sucesso." : "Item criado com sucesso.",
    );
    onOpenChange(false);
  };

  const handleSave = async () => {
    if (isSaving) return;
    const result = inventoryItemSchema.safeParse(formData);
    if (!result.success) {
      setErrors(parseErrors(result) ?? {});
      return;
    }

    setErrors({});
    try {
      if (isEditing) {
        await updateMutation.mutateAsync({ ...formData, ...result.data });
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["inventoryItemsPage"] }),
          queryClient.invalidateQueries({ queryKey: ["inventorySummary"] }),
          queryClient.invalidateQueries({ queryKey: ["inventory-lookup"] }),
          queryClient.invalidateQueries({ queryKey: ["inventory-insights"] }),
        ]);
        toastSuccess("Item atualizado com sucesso.");
      } else {
        const data = { ...formData, ...result.data };
        if (duplicateItem && isUnchangedDuplicate(data, duplicateItem)) {
          setPendingDuplicate(data);
          setDuplicateConfirmationOpen(true);
          return;
        }
        await createItem(data);
      }
      if (isEditing) onOpenChange(false);
    } catch (error) {
      toastError(error, isEditing ? "Erro ao atualizar item." : "Erro ao criar item.");
    }
  };

  return (
    <>
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>
            {isEditing ? "Editar" : isDuplicating ? "Duplicar" : "Novo"}{" "}
            {formData.type === "part" ? "Item no Estoque" : "Serviço"}
          </SheetTitle>
          <SheetDescription>
            {formData.type === "part"
              ? "Cadastre peças e insumos para gerenciar seu estoque."
              : "Cadastre serviços e mão de obra para suas ordens de serviço."}
          </SheetDescription>
        </SheetHeader>

        <div className="grid gap-4 py-6">
          <div className="grid gap-2">
            <Label htmlFor="inventory-item-name">
              Nome do {formData.type === "part" ? "Produto" : "Serviço"}
            </Label>
            <Input
              id="inventory-item-name"
              value={formData.name}
              placeholder={
                formData.type === "part"
                  ? "Ex: Tela iPhone 11"
                  : "Ex: Mão de obra para drone"
              }
              onChange={(event) => updateField("name", event.target.value)}
            />
            {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="inventory-item-description">Descrição</Label>
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
              <Label htmlFor="inventory-item-min">Quantidade mínima (alerta)</Label>
              <div className="relative">
                <Box className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="inventory-item-min"
                  className="pl-9"
                  value={formData.minQuantity}
                  inputMode="numeric"
                  onChange={(event) => updateField("minQuantity", sanitizeIntegerInput(event.target.value))}
                  onBlur={(event) => updateField("minQuantity", normalizeIntegerInput(event.target.value))}
                />
              </div>
              {errors.minQuantity && (
                <p className="text-xs text-destructive">{errors.minQuantity}</p>
              )}
            </div>
          )}

          {formData.type === "part" && !item && (
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-initial-quantity">
                Quantidade inicial em estoque (opcional)
              </Label>
              <Input
                id="inventory-item-initial-quantity"
                value={formData.initialQuantity}
                inputMode="numeric"
                onChange={(event) => updateField("initialQuantity", sanitizeIntegerInput(event.target.value))}
                onBlur={(event) => updateField("initialQuantity", normalizeIntegerInput(event.target.value))}
              />
              <p className="text-xs text-muted-foreground">
                Deixe em 0 para cadastrar sem estoque disponível.
              </p>
              {errors.initialQuantity && (
                <p className="text-xs text-destructive">
                  {errors.initialQuantity}
                </p>
              )}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-cost">
                {formData.type === "part" ? "Preço de custo" : "Custo estimado"}
              </Label>
              <div className="relative">
                <DollarSign className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="inventory-item-cost"
                  className="pl-9"
                  value={formData.costPrice}
                  inputMode="decimal"
                  onChange={(event) => updateField("costPrice", formatCurrencyInput(event.target.value))}
                  onBlur={(event) => updateField("costPrice", normalizeCurrencyInput(event.target.value))}
                />
              </div>
              {errors.costPrice && (
                <p className="text-xs text-destructive">{errors.costPrice}</p>
              )}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="inventory-item-sale">Preço de venda</Label>
              <div className="relative">
                <TrendingUp className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-primary" />
                <Input
                  id="inventory-item-sale"
                  className="pl-9"
                  value={formData.salePrice}
                  inputMode="decimal"
                  onChange={(event) => updateField("salePrice", formatCurrencyInput(event.target.value))}
                  onBlur={(event) => updateField("salePrice", normalizeCurrencyInput(event.target.value))}
                />
              </div>
              {errors.salePrice && (
                <p className="text-xs text-destructive">{errors.salePrice}</p>
              )}
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
            {isSaving
              ? "Salvando..."
              : isEditing
                ? "Salvar alterações"
                : isDuplicating
                  ? "Duplicar"
                  : "Cadastrar"}
          </Button>
        </SheetFooter>
      {duplicateConfirmationOpen && (
        <div
          className="absolute inset-0 z-10 flex flex-col gap-6 bg-background p-6"
          role="alert"
        >
          <div className="space-y-2">
            <h2 className="text-lg font-semibold">Nenhuma informação foi alterada</h2>
            <p className="text-sm text-muted-foreground">
              {formData.type === "part"
                ? "O novo cadastro será idêntico ao item original, com estoque inicial zero."
                : "O novo cadastro será idêntico ao serviço original."}{" "}
              Deseja duplicar mesmo assim?
            </p>
          </div>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button
              type="button"
              variant="outline"
              onClick={closeDuplicateConfirmation}
              disabled={isSaving}
            >
              Voltar e editar
            </Button>
            <Button
              type="button"
              onClick={async () => {
                if (!pendingDuplicate || isSaving) return;
                try {
                  await createItem(pendingDuplicate);
                  closeDuplicateConfirmation();
                } catch (error) {
                  toastError(error, "Erro ao duplicar item.");
                }
              }}
              disabled={isSaving}
            >
              {isSaving ? "Duplicando..." : "Duplicar mesmo assim"}
            </Button>
          </div>
        </div>
      )}
      </SheetContent>
    </Sheet>
    </>
  );
}
