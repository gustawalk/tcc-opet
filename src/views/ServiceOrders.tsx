import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import {
  Calendar,
  Edit,
  Eye,
  MoreVertical,
  Paperclip,
  Plus,
  Save,
  Search,
  Smartphone,
  Trash2,
  User as UserIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ServiceOrderDetailSheet } from "@/components/shared/ServiceOrderDetailSheet";
import {
  ServiceOrderItemLine,
  ServiceOrderItemsEditor,
} from "@/components/shared/ServiceOrderItemsEditor";
import { SearchableSelect } from "@/components/shared/SearchableSelect";
import { SortableHeader } from "@/components/shared/SortableHeader";
import { useSort } from "@/hooks/useSort";
import {
  editServiceOrderSchema,
  parseErrors,
  clearFieldError,
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
  User as UserType,
} from "@/lib/types";

const fetchOrders = () => invoke<ServiceOrder[]>("get_service_orders");
const fetchInventory = () => invoke<InventoryItem[]>("get_inventory_items");
const fetchItems = (id: string) =>
  invoke<ServiceOrderPart[]>("get_service_order_parts", { serviceOrderId: id });
const fetchChecklist = (id: string) =>
  invoke<ChecklistItem[]>("get_service_order_checklist", { osId: id });
const fetchAttachments = (id: string) =>
  invoke<ServiceOrderAttachment[]>("get_service_order_attachments", {
    serviceOrderId: id,
  });
const fetchUsers = () => invoke<UserType[]>("get_users");
const EMPTY_ORDERS: ServiceOrder[] = [];
const EMPTY_INVENTORY: InventoryItem[] = [];
const EMPTY_PARTS: ServiceOrderPart[] = [];
const EMPTY_CHECKLIST: ChecklistItem[] = [];
const EMPTY_ATTACHMENTS: ServiceOrderAttachment[] = [];
const EMPTY_USERS: UserType[] = [];

const formatFileSize = (sizeBytes: number) => {
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
};

export function ServiceOrders() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { sortConfig, cycleSort } = useSort();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selectedOS, setSelectedOS] = useState<ServiceOrder | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [cancelConfirmationOpen, setCancelConfirmationOpen] = useState(false);
  const [editStatus, setEditStatus] = useState<OSStatus>("Orçamento");
  const [editDescription, setEditDescription] = useState("");
  const [discountInput, setDiscountInput] = useState("0");
  const [editErrors, setEditErrors] = useState<ValidationErrors>({});
  const [isSaving, setIsSaving] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isUploadingAttachments, setIsUploadingAttachments] = useState(false);
  const [userFilter, setUserFilter] = useState<string | null>(null);
  const [itemActionId, setItemActionId] = useState<string | null>(null);
  const ordersQuery = useQuery({
    queryKey: ["service-orders"],
    queryFn: fetchOrders,
  });
  const inventoryQuery = useQuery({
    queryKey: ["inventory-lookup"],
    queryFn: fetchInventory,
  });
  const itemsQuery = useQuery({
    queryKey: ["service-order-parts", selectedOS?.id],
    queryFn: () => fetchItems(selectedOS!.id),
    enabled: editOpen && !!selectedOS,
  });
  const checklistQuery = useQuery({
    queryKey: ["service-order-checklist", selectedOS?.id],
    queryFn: () => fetchChecklist(selectedOS!.id),
    enabled: editOpen && !!selectedOS,
  });
  const usersQuery = useQuery({
    queryKey: ["users"],
    queryFn: fetchUsers,
  });
  const users = usersQuery.data ?? EMPTY_USERS;
  const attachmentsQuery = useQuery({
    queryKey: ["service-order-attachments", selectedOS?.id],
    queryFn: () => fetchAttachments(selectedOS!.id),
    enabled: editOpen && !!selectedOS,
  });
  const orders = ordersQuery.data ?? EMPTY_ORDERS;
  const inventory = inventoryQuery.data ?? EMPTY_INVENTORY;
  const items = itemsQuery.data ?? EMPTY_PARTS;
  const checklist = checklistQuery.data ?? EMPTY_CHECKLIST;
  const attachments = attachmentsQuery.data ?? EMPTY_ATTACHMENTS;
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

  const filteredOrders = useMemo(() => {
    const result = orders.filter(
      (order) =>
        (order.displayId.toLowerCase().includes(searchTerm.toLowerCase()) ||
          order.customerName
            ?.toLowerCase()
            .includes(searchTerm.toLowerCase()) ||
          order.equipment.toLowerCase().includes(searchTerm.toLowerCase())) &&
        (statusFilter === "all"
          ? order.status !== "Cancelada"
          : statusFilter === "Cancelada"
            ? order.status === "Cancelada"
            : order.status === statusFilter) &&
        (!userFilter || order.userId === userFilter),
    );
    if (!sortConfig.column || !sortConfig.direction) return result;
    return [...result].sort((a, b) => {
      const av = a[sortConfig.column as keyof ServiceOrder] ?? "";
      const bv = b[sortConfig.column as keyof ServiceOrder] ?? "";
      return sortConfig.direction === "asc"
        ? String(av).localeCompare(String(bv), "pt-BR")
        : String(bv).localeCompare(String(av), "pt-BR");
    });
  }, [orders, searchTerm, statusFilter, sortConfig, userFilter]);
  const total = items.reduce(
    (sum, item) => sum + item.unitPrice * item.quantity,
    0,
  );
  const discount = Math.max(0, Math.min(100, Number(discountInput) || 0));
  const statusBadge = (status: OSStatus) => (
    <Badge
      variant={
        status === "Finalizada"
          ? "secondary"
          : status === "Aguardando Peça"
            ? "destructive"
            : "outline"
      }
      className={
        status === "Em Manutenção"
          ? "bg-blue-600 text-white"
          : status === "Finalizada"
            ? "bg-green-600 text-white"
            : ""
      }
    >
      {status}
    </Badge>
  );
  const invalidateOrder = async () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ["service-orders"] }),
      queryClient.invalidateQueries({ queryKey: ["service-order"] }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-parts", selectedOS?.id],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-checklist", selectedOS?.id],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-events", selectedOS?.id],
      }),
      queryClient.invalidateQueries({
        queryKey: ["service-order-attachments"],
      }),
      queryClient.invalidateQueries({ queryKey: ["inventory-lookup"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard-data"] }),
    ]);
  const editOrder = (order: ServiceOrder) => {
    setSelectedOS(order);
    setEditStatus(order.status);
    setEditDescription(order.description);
    setDiscountInput(String(order.discountPercent));
    setEditOpen(true);
  };
  const viewOrder = (order: ServiceOrder) => {
    setSelectedOS(order);
    setDetailOpen(true);
  };
  const addItem = async (item: InventoryItem) => {
    if (
      !selectedOS ||
      itemActionId ||
      (item.type === "part" && item.currentQuantity <= 0)
    )
      return;
    const existingItem = items.find(
      (part) => part.inventoryItemId === item.id,
    );
    if (existingItem) {
      await removeItem(existingItem.id);
      return;
    }
    setItemActionId(item.id);
    try {
      await invoke("add_part_to_service_order", {
        serviceOrderId: selectedOS.id,
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
    if (itemActionId) return;
    const current = items.find((item) => item.id === partId);
    if (!current || current.quantity === quantity) return;

    setItemActionId(partId);
    queryClient.setQueryData<ServiceOrderPart[]>(
      ["service-order-parts", selectedOS?.id],
      (currentItems) =>
        currentItems?.map((item) =>
          item.id === partId ? { ...item, quantity } : item,
        ),
    );
    try {
      await invoke("update_service_order_part_quantity", { partId, quantity });
      await invalidateOrder();
    } catch (error) {
      toastError(error, "Erro ao atualizar quantidade do item.");
      await invalidateOrder();
    } finally {
      setItemActionId(null);
    }
  };
  const toggleChecklistItem = (id: string) => {
    if (!selectedOS) return;
    queryClient.setQueryData<ChecklistItem[]>(
      ["service-order-checklist", selectedOS.id],
      (current) =>
        current?.map((item) =>
          item.id === id ? { ...item, checked: !item.checked } : item,
        ),
    );
  };
  const addAttachments = async () => {
    if (!selectedOS || isUploadingAttachments) return;

    try {
      setIsUploadingAttachments(true);
      const attachments = await invoke<ServiceOrderAttachment[]>(
        "select_service_order_attachments",
        { serviceOrderId: selectedOS.id },
      );

      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["service-order-attachments", selectedOS.id],
        }),
        queryClient.invalidateQueries({
          queryKey: ["service-order-events", selectedOS.id],
        }),
      ]);

      if (attachments.length)
        toastSuccess(`${attachments.length} anexo(s) adicionado(s) à OS.`);
    } catch (error) {
      toastError(error, "Erro ao selecionar anexos.");
    } finally {
      setIsUploadingAttachments(false);
    }
  };
  const saveEdit = async (confirmedCancellation = false) => {
    if (!selectedOS || isSaving) return;
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
      selectedOS.status !== "Cancelada" &&
      !confirmedCancellation
    ) {
      setCancelConfirmationOpen(true);
      return;
    }
    setEditErrors({});
    setIsSaving(true);
    try {
      await invoke("save_service_order_edit", {
        request: {
          id: selectedOS.id,
          description: editDescription,
          discountPercent: discount,
          status: editStatus,
          restoreStock: editStatus === "Cancelada",
          checklist,
        },
      });
      await invalidateOrder();
      toastSuccess("Alterações salvas com sucesso.");
      setEditOpen(false);
      setCancelConfirmationOpen(false);
    } catch (error) {
      toastError(
        error,
        "Não foi possível salvar as alterações. Verifique a transição de status e o checklist.",
      );
    } finally {
      setIsSaving(false);
    }
  };
  const deleteOrder = async () => {
    if (!deleteId || isDeleting) return;
    setIsDeleting(true);
    try {
      await invoke("delete_service_order", { id: deleteId });
      await invalidateOrder();
      toastSuccess("Ordem de serviço excluída.");
      setDeleteId(null);
    } catch (error) {
      toastError(error, "Erro ao excluir ordem de serviço.");
      setDeleteId(null);
    } finally {
      setIsDeleting(false);
    }
  };
  const handleStatusClick = async (newStatus: OSStatus) => {
    if (!selectedOS || newStatus === editStatus || isSaving) return;
    if (newStatus === "Cancelada" && selectedOS.status !== "Cancelada") {
      setCancelConfirmationOpen(true);
      return;
    }
    try {
      await invoke("transition_service_order_status", {
        id: selectedOS.id,
        status: newStatus,
        restoreStock: false,
      });
      setEditStatus(newStatus);
      setSelectedOS((prev) => (prev ? { ...prev, status: newStatus } : null));
      await invalidateOrder();
      toastSuccess(`Status alterado para "${newStatus}".`);
    } catch (error) {
      toastError(error, "Erro ao alterar status.");
    }
  };
  const handleCancelConfirm = async () => {
    if (!selectedOS || isSaving) return;
    setIsSaving(true);
    try {
      await invoke("transition_service_order_status", {
        id: selectedOS.id,
        status: "Cancelada",
        restoreStock: true,
      });
      setEditStatus("Cancelada");
      setSelectedOS((prev) => (prev ? { ...prev, status: "Cancelada" } : null));
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
    <div className="flex flex-col gap-6 animate-in fade-in duration-200">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">
            Ordens de Serviço
          </h2>
          <p className="text-muted-foreground mt-1">
            Gerencie atendimentos, ordens de serviço e orçamentos.
          </p>
        </div>
        <Button className="gap-2" onClick={() => navigate("/os/new")}>
          <Plus className="h-4 w-4" />
          Nova OS
        </Button>
      </div>
      <Tabs defaultValue="all" onValueChange={setStatusFilter}>
        <div className="flex flex-col md:flex-row justify-between gap-4">
          <TabsList>
            <TabsTrigger value="all">Todas</TabsTrigger>
            <TabsTrigger value="Orçamento">Orçamentos</TabsTrigger>
            <TabsTrigger value="Em Manutenção">Em Manutenção</TabsTrigger>
            <TabsTrigger value="Aguardando Peça">Pendentes</TabsTrigger>
            <TabsTrigger value="Finalizada">Finalizadas</TabsTrigger>
            <TabsTrigger value="Cancelada">Canceladas</TabsTrigger>
          </TabsList>
          <div className="flex gap-2 items-center w-full md:w-auto">
            <SearchableSelect
              options={users}
              value={userFilter}
              onSelect={(user) =>
                setUserFilter(userFilter === user.id ? null : user.id)
              }
              placeholder="Todos os funcionários"
              searchPlaceholder="Buscar funcionário..."
              getKey={(u) => u.id}
              getLabel={(u) => u.name}
              className="w-full md:w-48"
            />
            <div className="relative w-full md:w-72">
            <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
            <Input
              className="pl-9"
              placeholder="Buscar por ID, Cliente ou Equipamento..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
          </div>
        </div>
      </div>
      </Tabs>
      {ordersQuery.isError && (
        <Card className="border-destructive">
          <CardContent className="p-4 text-sm text-destructive">
            Não foi possível carregar as ordens de serviço.
          </CardContent>
        </Card>
      )}
      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <SortableHeader
                  column="displayId"
                  label="ID"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                />
                <SortableHeader
                  column="customerName"
                  label="Cliente & Equipamento"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                />
                <SortableHeader
                  column="status"
                  label="Status"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                  className="hidden md:table-cell"
                />
                <TableHead className="hidden lg:table-cell">Abertura</TableHead>
                <TableHead className="text-right">Valor</TableHead>
                <TableHead className="text-right">Ações</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {ordersQuery.isLoading ? (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="h-24 text-center text-muted-foreground"
                  >
                    Carregando ordens de serviço...
                  </TableCell>
                </TableRow>
              ) : filteredOrders.length ? (
                filteredOrders.map((order) => (
                  <TableRow key={order.id}>
                    <TableCell className="font-mono text-xs font-bold">
                      {order.displayId}
                    </TableCell>
                    <TableCell>
                      <p className="flex gap-1 text-sm font-medium">
                        <UserIcon className="h-3 w-3" />
                        {order.customerName}
                      </p>
                      <p className="flex gap-1 text-xs text-muted-foreground">
                        <Smartphone className="h-3 w-3" />
                        {order.equipment}
                      </p>
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                      {statusBadge(order.status)}
                    </TableCell>
                    <TableCell className="hidden lg:table-cell text-xs">
                      <Calendar className="inline h-3 w-3 mr-1" />
                      {new Date(order.createdAt).toLocaleDateString("pt-BR")}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex flex-wrap items-center justify-end gap-2">
                        {order.discountPercent > 0 && (
                          <span className="text-xs text-muted-foreground line-through">
                            {formatCurrency(order.totalPrice || 0)}
                          </span>
                        )}
                        <span>
                          {formatCurrency(
                            (order.totalPrice || 0) *
                              (1 - order.discountPercent / 100),
                          )}
                        </span>
                        {order.discountPercent > 0 && (
                          <Badge variant="outline" className="text-[10px]">
                            -{order.discountPercent}%
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuLabel>Ações</DropdownMenuLabel>
                          <DropdownMenuItem onClick={() => viewOrder(order)}>
                            <Eye className="mr-2 h-4 w-4" />
                            Visualizar
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => editOrder(order)}>
                            <Edit className="mr-2 h-4 w-4" />
                            Editar
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-destructive"
                            onClick={() => setDeleteId(order.id)}
                          >
                            <Trash2 className="mr-2 h-4 w-4" />
                            Excluir
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="h-24 text-center text-muted-foreground"
                  >
                    Nenhuma ordem de serviço encontrada.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
      <ServiceOrderDetailSheet
        orderId={selectedOS?.id ?? null}
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
      />
      <Sheet open={editOpen} onOpenChange={(open) => {
        if (!open && cancelConfirmationOpen) return;
        setEditOpen(open);
      }}>
        <SheetContent className="sm:max-w-xl overflow-y-auto">
          <SheetHeader>
            <SheetTitle>Editar - {selectedOS?.displayId}</SheetTitle>
            <SheetDescription>
              Atualize os dados, checklist, status e itens desta OS.
            </SheetDescription>
          </SheetHeader>
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
                onChange={(e) => {
                  setEditDescription(e.target.value);
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
              {checklistQuery.isLoading && (
                <p className="text-sm text-muted-foreground">
                  Carregando checklist...
                </p>
              )}
              {checklistQuery.isError && (
                <p className="text-sm text-destructive">
                  Não foi possível carregar o checklist.
                </p>
              )}
              {!checklistQuery.isLoading &&
                !checklistQuery.isError &&
                checklist.map((item) => (
                  <div className="flex items-center gap-2" key={item.id}>
                    <Checkbox
                      checked={item.checked}
                      onChange={() => toggleChecklistItem(item.id)}
                    />
                    <Label>{item.label}</Label>
                  </div>
                ))}
              {!checklistQuery.isLoading &&
                !checklistQuery.isError &&
                !checklist.length && (
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
              <Label className="text-xs font-semibold uppercase">Anexos</Label>
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
                  {isUploadingAttachments ? "Enviando..." : "Adicionar anexos"}
                </Button>
                <p className="text-xs text-muted-foreground">
                  PNG, JPEG, WEBP ou PDF.
                </p>
              </div>
              {attachmentsQuery.isLoading && (
                <p className="text-sm text-muted-foreground">
                  Carregando anexos...
                </p>
              )}
              {attachmentsQuery.isError && (
                <p className="text-sm text-destructive">
                  Não foi possível carregar os anexos.
                </p>
              )}
              {!attachmentsQuery.isLoading &&
                !attachmentsQuery.isError &&
                !attachments.length && (
                  <p className="text-sm text-muted-foreground">
                    Nenhum anexo adicionado.
                  </p>
                )}
              {attachments.map((attachment) => (
                <div
                  className="flex items-center gap-2 rounded-md border p-2"
                  key={attachment.id}
                >
                  <Paperclip className="h-4 w-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">
                      {attachment.fileName}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {attachment.mimeType} · {formatFileSize(attachment.sizeBytes)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
            <Separator />
            <div className="flex items-center justify-between gap-4">
              <div className="flex gap-2 items-center">
                <Label>Desconto</Label>
                <Input
                  type="number"
                  min="0"
                  max="100"
                  className="w-20"
                  value={discountInput}
                  onChange={(e) => setDiscountInput(e.target.value)}
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
            <Button
              variant="outline"
              onClick={() => setEditOpen(false)}
              disabled={isSaving}
            >
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
        </SheetContent>
      </Sheet>
      {deleteId && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !isDeleting && setDeleteId(null)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Excluir ordem de serviço</h3>
            <p className="text-sm text-muted-foreground">
              Esta ação não pode ser desfeita. Deseja realmente excluir esta
              ordem de serviço?
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setDeleteId(null)}
                disabled={isDeleting}
              >
                Cancelar
              </Button>
              <Button
                variant="destructive"
                onClick={deleteOrder}
                disabled={isDeleting}
              >
                {isDeleting ? "Excluindo..." : "Excluir"}
              </Button>
            </div>
          </div>
        </div>
      )}
      {cancelConfirmationOpen && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !isSaving && setCancelConfirmationOpen(false)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
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
    </div>
  );
}
