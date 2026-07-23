import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Eye, Paperclip, Save, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import {
  ServiceOrderItemLine,
  ServiceOrderItemsEditor,
} from "@/components/shared/ServiceOrderItemsEditor";
import {
  clearFieldError,
  editServiceOrderSchema,
  parseErrors,
  ValidationErrors,
} from "@/lib/validation";
import { formatCurrency } from "@/lib/formatters";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  ChecklistItem,
  InventoryItem,
  OSStatus,
  ServiceOrder,
  ServiceOrderAttachment,
  ServiceOrderPart,
} from "@/lib/types";

interface ServiceOrderEditorSheetProps {
  orderId: string | null;
  open: boolean;
  onClose: () => void;
  onView: () => void;
}

const fetchOrder = (id: string) =>
  invoke<ServiceOrder | null>("get_service_order", { id });
const fetchInventory = () => invoke<InventoryItem[]>("get_inventory_items");
const fetchItems = (id: string) =>
  invoke<ServiceOrderPart[]>("get_service_order_parts", { serviceOrderId: id });
const fetchChecklist = (id: string) =>
  invoke<ChecklistItem[]>("get_service_order_checklist", { osId: id });
const fetchAttachments = (id: string) =>
  invoke<ServiceOrderAttachment[]>("get_service_order_attachments", {
    serviceOrderId: id,
  });

const formatFileSize = (sizeBytes: number) => {
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
};

export function ServiceOrderEditorSheet({
  orderId,
  open,
  onClose,
  onView,
}: ServiceOrderEditorSheetProps) {
  const queryClient = useQueryClient();
  const [editStatus, setEditStatus] = useState<OSStatus>("Orçamento");
  const [editDescription, setEditDescription] = useState("");
  const [discountInput, setDiscountInput] = useState("0");
  const [editErrors, setEditErrors] = useState<ValidationErrors>({});
  const [itemActionId, setItemActionId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [isUploadingAttachments, setIsUploadingAttachments] = useState(false);
  const [cancelConfirmationOpen, setCancelConfirmationOpen] = useState(false);
  const [attachmentToDelete, setAttachmentToDelete] =
    useState<ServiceOrderAttachment | null>(null);
  const orderQuery = useQuery({
    queryKey: ["service-order", orderId],
    queryFn: () => fetchOrder(orderId!),
    enabled: open && !!orderId,
  });
  const inventoryQuery = useQuery({
    queryKey: ["inventory-lookup"],
    queryFn: fetchInventory,
    enabled: open,
  });
  const itemsQuery = useQuery({
    queryKey: ["service-order-parts", orderId],
    queryFn: () => fetchItems(orderId!),
    enabled: open && !!orderId,
  });
  const checklistQuery = useQuery({
    queryKey: ["service-order-checklist", orderId],
    queryFn: () => fetchChecklist(orderId!),
    enabled: open && !!orderId,
  });
  const attachmentsQuery = useQuery({
    queryKey: ["service-order-attachments", orderId],
    queryFn: () => fetchAttachments(orderId!),
    enabled: open && !!orderId,
  });
  const order = orderQuery.data;
  const inventory = inventoryQuery.data ?? [];
  const items = itemsQuery.data ?? [];
  const checklist = checklistQuery.data ?? [];
  const attachments = attachmentsQuery.data ?? [];
  const discount = Math.max(0, Math.min(100, Number(discountInput) || 0));
  const total = items.reduce(
    (sum, item) => sum + item.unitPrice * item.quantity,
    0,
  );
  const itemLines: ServiceOrderItemLine[] = items.map((item) => ({
    id: item.id,
    inventoryItemId: item.inventoryItemId,
    inventoryItemName: item.inventoryItemName,
    itemType: item.itemType,
    quantity: item.quantity,
    unitPrice: item.unitPrice,
    maxQuantity:
      item.itemType === "part"
        ? item.currentQuantity + item.quantity
        : undefined,
  }));

  useEffect(() => {
    if (!order) return;
    setEditStatus(order.status);
    setEditDescription(order.description);
    setDiscountInput(String(order.discountPercent));
    setEditErrors({});
  }, [order, open]);

  const invalidateOrder = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ["service-orders"] }),
      queryClient.invalidateQueries({ queryKey: ["service-order", orderId] }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-parts", orderId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-checklist", orderId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-events", orderId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-attachments", orderId],
      }),
      queryClient.invalidateQueries({ queryKey: ["inventory-lookup"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard-data"] }),
    ]);

  const addItem = async (item: InventoryItem) => {
    if (
      !orderId ||
      itemActionId ||
      (item.type === "part" && item.currentQuantity <= 0)
    )
      return;
    const existingItem = items.find((part) => part.inventoryItemId === item.id);
    if (existingItem) return removeItem(existingItem.id);
    setItemActionId(item.id);
    try {
      await invoke("add_part_to_service_order", {
        serviceOrderId: orderId,
        inventoryItemId: item.id,
        quantity: 1,
      });
      await invalidateOrder();
    } catch (error) {
      toastError(error, "Erro ao adicionar item.");
    } finally {
      setItemActionId(null);
    }
  };
  const removeItem = async (id: string) => {
    if (itemActionId) return;
    setItemActionId(id);
    try {
      await invoke("remove_part_from_service_order", { partId: id });
      await invalidateOrder();
    } catch (error) {
      toastError(error, "Erro ao remover item.");
    } finally {
      setItemActionId(null);
    }
  };
  const updateItemQuantity = async (partId: string, quantity: number) => {
    if (
      itemActionId ||
      items.find((item) => item.id === partId)?.quantity === quantity
    )
      return;
    setItemActionId(partId);
    queryClient.setQueryData<ServiceOrderPart[]>(
      ["service-order-parts", orderId],
      (current) =>
        current?.map((item) =>
          item.id === partId ? { ...item, quantity } : item,
        ),
    );
    try {
      await invoke("update_service_order_part_quantity", { partId, quantity });
    } catch (error) {
      toastError(error, "Erro ao atualizar quantidade do item.");
    } finally {
      await invalidateOrder();
      setItemActionId(null);
    }
  };
  const toggleChecklistItem = (id: string) => {
    queryClient.setQueryData<ChecklistItem[]>(
      ["service-order-checklist", orderId],
      (current) =>
        current?.map((item) =>
          item.id === id ? { ...item, checked: !item.checked } : item,
        ),
    );
  };
  const addAttachments = async () => {
    if (!orderId || isUploadingAttachments) return;
    try {
      setIsUploadingAttachments(true);
      const selected = await invoke<ServiceOrderAttachment[]>(
        "select_service_order_attachments",
        { serviceOrderId: orderId },
      );
      await invalidateOrder();
      if (selected.length)
        toastSuccess(`${selected.length} anexo(s) adicionado(s) à OS.`);
    } catch (error) {
      toastError(error, "Erro ao selecionar anexos.");
    } finally {
      setIsUploadingAttachments(false);
    }
  };
  const deleteAttachmentMutation = useMutation({
    mutationFn: (id: string) =>
      invoke("delete_service_order_attachment", { id }),
    onSuccess: async () => {
      await invalidateOrder();
      setAttachmentToDelete(null);
      toastSuccess("Anexo excluído com sucesso.");
    },
    onError: (error) => toastError(error, "Erro ao excluir anexo."),
  });
  const handleStatusClick = async (status: OSStatus) => {
    if (!orderId || !order || status === editStatus || isSaving) return;
    if (status === "Cancelada" && order.status !== "Cancelada") {
      setCancelConfirmationOpen(true);
      return;
    }
    try {
      await invoke("transition_service_order_status", {
        id: orderId,
        status,
        restoreStock: false,
      });
      setEditStatus(status);
      await invalidateOrder();
      toastSuccess(`Status alterado para "${status}".`);
    } catch (error) {
      toastError(error, "Erro ao alterar status.");
    }
  };
  const saveEdit = async (confirmedCancellation = false) => {
    if (!order || isSaving) return;
    const fieldErrors = parseErrors(
      editServiceOrderSchema.safeParse({
        description: editDescription,
        discount,
      }),
    );
    if (fieldErrors) {
      setEditErrors(fieldErrors);
      return;
    }
    if (
      editStatus === "Cancelada" &&
      order.status !== "Cancelada" &&
      !confirmedCancellation
    ) {
      setCancelConfirmationOpen(true);
      return;
    }
    setIsSaving(true);
    try {
      await invoke("save_service_order_edit", {
        request: {
          id: order.id,
          description: editDescription,
          discountPercent: discount,
          status: editStatus,
          restoreStock: editStatus === "Cancelada",
          checklist,
        },
      });
      await invalidateOrder();
      toastSuccess("Alterações salvas com sucesso.");
      setCancelConfirmationOpen(false);
      onClose();
    } catch (error) {
      toastError(
        error,
        "Não foi possível salvar as alterações. Verifique a transição de status e o checklist.",
      );
    } finally {
      setIsSaving(false);
    }
  };
  const handleCancelConfirm = async () => {
    if (!orderId || isSaving) return;
    setIsSaving(true);
    try {
      await invoke("transition_service_order_status", {
        id: orderId,
        status: "Cancelada",
        restoreStock: true,
      });
      setEditStatus("Cancelada");
      await invalidateOrder();
      toastSuccess("Ordem de serviço cancelada.");
      setCancelConfirmationOpen(false);
    } catch (error) {
      toastError(error, "Erro ao cancelar ordem de serviço.");
      setCancelConfirmationOpen(false);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <>
      <Sheet
        open={open}
        onOpenChange={(isOpen) => {
          if (!isOpen && !cancelConfirmationOpen) onClose();
        }}
      >
        <SheetContent className="sm:max-w-xl overflow-y-auto">
          <SheetHeader>
            <div className="flex items-start justify-between gap-4">
              <div>
                <SheetTitle>
                  Editar - {order?.displayId ?? "Carregando..."}
                </SheetTitle>
                <SheetDescription>
                  Atualize os dados, checklist, status e itens desta OS.
                </SheetDescription>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="mt-[1.3rem] h-8 w-8 shrink-0"
                onClick={onView}
                aria-label="Visualizar ordem de serviço"
                title="Visualizar ordem de serviço"
              >
                <Eye className="h-4 w-4" />
              </Button>
            </div>
          </SheetHeader>
          {orderQuery.isLoading ? (
            <div
              className="mt-8 space-y-6"
              aria-label="Carregando edição da ordem de serviço"
            >
              <div className="space-y-3">
                <Skeleton className="h-3 w-28" />
                <div className="grid grid-cols-2 gap-2">
                  {Array.from({ length: 4 }).map((_, index) => (
                    <Skeleton className="h-9 w-full" key={index} />
                  ))}
                </div>
              </div>
              <Separator />
              <div className="space-y-2">
                <Skeleton className="h-3 w-32" />
                <Skeleton className="h-24 w-full" />
              </div>
              <Separator />
              <Skeleton className="h-36 w-full" />
              <Separator />
              <Skeleton className="h-24 w-full" />
            </div>
          ) : orderQuery.isError || !order ? (
            <p className="py-16 text-center text-destructive">
              Não foi possível carregar a ordem de serviço.
            </p>
          ) : (
            <>
              <div className="mt-8 space-y-6">
                <div className="space-y-3">
                  <Label className="text-xs font-semibold uppercase">
                    Status da Ordem
                  </Label>
                  <div className="grid grid-cols-2 gap-2">
                    {(
                      [
                        "Orçamento",
                        "Em Manutenção",
                        "Aguardando Peça",
                        "Finalizada",
                        "Cancelada",
                      ] as OSStatus[]
                    ).map((status) => (
                      <Button
                        key={status}
                        variant={editStatus === status ? "default" : "outline"}
                        size="sm"
                        onClick={() => handleStatusClick(status)}
                        disabled={isSaving}
                      >
                        {status}
                      </Button>
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Transições inválidas serão recusadas.
                  </p>
                </div>
                <Separator />
                <div>
                  <Label>Relato / Notas Técnicas</Label>
                  <Textarea
                    value={editDescription}
                    onChange={(event) => {
                      setEditDescription(event.target.value);
                      setEditErrors(clearFieldError(editErrors, "description"));
                    }}
                  />
                  {editErrors.description && (
                    <p className="text-xs text-destructive">
                      {editErrors.description}
                    </p>
                  )}
                </div>
                <Separator />
                <div className="space-y-3">
                  <Label className="text-xs font-semibold uppercase">
                    Checklist
                  </Label>
                  {checklistQuery.isLoading ? (
                    <p className="text-sm text-muted-foreground">
                      Carregando checklist...
                    </p>
                  ) : checklistQuery.isError ? (
                    <p className="text-sm text-destructive">
                      Não foi possível carregar o checklist.
                    </p>
                  ) : checklist.length ? (
                    checklist.map((item) => (
                      <div className="flex items-center gap-2" key={item.id}>
                        <Checkbox
                          checked={item.checked}
                          onChange={() => toggleChecklistItem(item.id)}
                        />
                        <Label>{item.label}</Label>
                      </div>
                    ))
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      Esta OS não possui checklist.
                    </p>
                  )}
                </div>
                <Separator />
                {inventoryQuery.isError || itemsQuery.isError ? (
                  <p className="text-sm text-destructive">
                    Não foi possível carregar os itens ou o inventário.
                  </p>
                ) : (
                  <ServiceOrderItemsEditor
                    inventory={inventory}
                    lines={itemLines}
                    isLoading={inventoryQuery.isLoading || itemsQuery.isLoading}
                    isBusy={!!itemActionId}
                    onSelectItem={addItem}
                    onQuantityChange={(line, quantity) =>
                      updateItemQuantity(line.id, quantity)
                    }
                    onRemove={(line) => removeItem(line.id)}
                  />
                )}
                <Separator />
                <div className="space-y-2">
                  <Label className="text-xs font-semibold uppercase">
                    Anexos
                  </Label>
                  <div className="flex flex-wrap items-center gap-3 rounded-md border bg-muted/20 p-3">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="gap-2"
                      onClick={addAttachments}
                      disabled={isUploadingAttachments || isSaving}
                    >
                      <Paperclip className="h-4 w-4" />
                      {isUploadingAttachments
                        ? "Enviando..."
                        : "Adicionar anexos"}
                    </Button>
                    <p className="text-xs text-muted-foreground">
                      PNG, JPEG, WEBP ou PDF.
                    </p>
                  </div>
                  {attachmentsQuery.isLoading ? (
                    <p className="text-sm text-muted-foreground">
                      Carregando anexos...
                    </p>
                  ) : attachmentsQuery.isError ? (
                    <p className="text-sm text-destructive">
                      Não foi possível carregar os anexos.
                    </p>
                  ) : attachments.length ? (
                    attachments.map((attachment) => (
                      <div
                        className="flex items-center gap-2 rounded-md border p-2"
                        key={attachment.id}
                      >
                        <Paperclip className="h-4 w-4 shrink-0 text-muted-foreground" />
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm font-medium">
                            {attachment.fileName}
                          </p>
                          <p className="text-xs text-muted-foreground">
                            {attachment.mimeType} ·{" "}
                            {formatFileSize(attachment.sizeBytes)}
                          </p>
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 shrink-0 text-destructive hover:text-destructive"
                          onClick={() => setAttachmentToDelete(attachment)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    ))
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      Nenhum anexo adicionado.
                    </p>
                  )}
                </div>
                <Separator />
                <div className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-2">
                    <Label>Desconto</Label>
                    <Input
                      type="number"
                      min="0"
                      max="100"
                      className="w-20"
                      value={discountInput}
                      onChange={(event) => setDiscountInput(event.target.value)}
                    />
                    <span>%</span>
                  </div>
                  <div className="flex flex-wrap items-center justify-end gap-2 text-right">
                    {discount > 0 && (
                      <span className="text-sm text-muted-foreground line-through">
                        {formatCurrency(total)}
                      </span>
                    )}
                    <span className="font-bold">
                      Total: {formatCurrency(total * (1 - discount / 100))}
                    </span>
                    {discount > 0 && (
                      <Badge variant="outline" className="text-[10px]">
                        -{discount}%
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
              <SheetFooter className="mt-8">
                <Button variant="outline" onClick={onClose} disabled={isSaving}>
                  Cancelar
                </Button>
                <Button
                  className="gap-2"
                  onClick={() => saveEdit()}
                  disabled={
                    isSaving || itemsQuery.isLoading || checklistQuery.isLoading
                  }
                >
                  <Save className="h-4 w-4" />
                  {isSaving ? "Salvando..." : "Salvar Alterações"}
                </Button>
              </SheetFooter>
            </>
          )}
        </SheetContent>
      </Sheet>
      {cancelConfirmationOpen && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
          onClick={() => !isSaving && setCancelConfirmationOpen(false)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4"
            onClick={(event) => event.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Cancelar ordem de serviço</h3>
            <p className="text-sm text-muted-foreground">
              Ao cancelar esta OS, o estoque das peças físicas será restaurado e
              as linhas de peças serão removidas. Deseja continuar?
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setCancelConfirmationOpen(false)}
                disabled={isSaving}
              >
                Voltar
              </Button>
              <Button
                variant="destructive"
                onClick={handleCancelConfirm}
                disabled={isSaving}
              >
                {isSaving ? "Cancelando..." : "Confirmar cancelamento"}
              </Button>
            </div>
          </div>
        </div>
      )}
      {attachmentToDelete && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
          onClick={() =>
            !deleteAttachmentMutation.isPending && setAttachmentToDelete(null)
          }
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4"
            onClick={(event) => event.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Excluir anexo</h3>
            <p className="text-sm text-muted-foreground">
              Deseja excluir permanentemente o anexo &quot;
              {attachmentToDelete.fileName}&quot;?
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setAttachmentToDelete(null)}
                disabled={deleteAttachmentMutation.isPending}
              >
                Cancelar
              </Button>
              <Button
                variant="destructive"
                onClick={() =>
                  deleteAttachmentMutation.mutate(attachmentToDelete.id)
                }
                disabled={deleteAttachmentMutation.isPending}
              >
                {deleteAttachmentMutation.isPending
                  ? "Excluindo..."
                  : "Excluir"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
