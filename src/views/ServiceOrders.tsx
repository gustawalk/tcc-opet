import { useState, useMemo, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import {
  Plus,
  Search,
  MoreVertical,
  Eye,
  Edit,
  Trash2,
  Calendar,
  User,
  Smartphone,
  Save,
  Trash,
  ChevronRight,
} from "lucide-react";
import {
  Card,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow
} from "@/components/ui/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ServiceOrder, OSStatus, InventoryItem, OSChecklist } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { formatCurrency } from "@/lib/formatters";
import { ServiceOrderDetailSheet } from "@/components/shared/ServiceOrderDetailSheet";
import {
  Tabs,
  TabsList,
  TabsTrigger
} from "@/components/ui/tabs";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter
} from "@/components/ui/sheet";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import { editServiceOrderSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { useSort } from "@/hooks/useSort";
import { SortableHeader } from "@/components/shared/SortableHeader";

// Estendendo o tipo ServiceOrder para incluir checklist opcional para os mocks
interface ServiceOrderWithChecklist extends ServiceOrder {
  checklist?: OSChecklist;
}

// Interface for service order parts from backend
interface ServiceOrderPart {
  id: string;
  serviceOrderId: string;
  inventoryItemId: string;
  inventoryItemName: string;
  quantity: number;
  unitCost: number;
  unitPrice: number;
}

// Fetch items (parts/services) from a service order
const fetchOSItems = async (osId: string): Promise<ServiceOrderPart[]> => {
  return await invoke("get_service_order_parts", { serviceOrderId: osId });
};

// Fetch inventory items for selection
const fetchInventory = async (): Promise<InventoryItem[]> => {
  return await invoke("get_inventory_items");
};

// Fetch all service orders from database
const fetchServiceOrders = async (): Promise<ServiceOrderWithChecklist[]> => {
  return await invoke("get_service_orders");
};

export function ServiceOrders() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const { sortConfig, cycleSort } = useSort();

  // Estados para Controle de Sheet
  const [selectedOS, setSelectedOS] = useState<ServiceOrderWithChecklist | null>(null);
  const [osItems, setOsItems] = useState<ServiceOrderPart[]>([]);
  const [isDetailOpen, setIsDetailOpen] = useState(false);
  const [isEditOpen, setIsEditOpen] = useState(false);

  // Estados de Edição Temporários
  const [editStatus, setEditStatus] = useState<OSStatus>("Orçamento");
  const [editDescription, setEditDescription] = useState("");
  const [editDiscount, setEditDiscount] = useState(0);
  const [editDiscountInput, setEditDiscountInput] = useState("0");
  const [editErrors, setEditErrors] = useState<ValidationErrors>({});

  // Estado para busca de itens no estoque
  const [inventorySearch, setInventorySearch] = useState("");
  const [showInventoryResults, setShowInventoryResults] = useState(false);

  const { data: orders = [], isLoading } = useQuery({
    queryKey: ["service-orders"],
    queryFn: fetchServiceOrders,
  });

  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: ["service-orders"] });
  }, []);

  const { data: inventory = [] } = useQuery({
    queryKey: ["inventory-lookup"],
    queryFn: fetchInventory,
  });

  const getOrderSortValue = (order: ServiceOrder, column: string): string | number => {
    switch (column) {
      case "displayId": return order.displayId;
      case "customerName": return order.customerName || "";
      case "status": return order.status;
      case "createdAt": return order.createdAt;
      case "totalPrice": return order.totalPrice || 0;
      default: return "";
    }
  };

  const filteredOrders = useMemo(() => {
    let result = orders.filter(order => {
      const matchesSearch =
        order.displayId.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (order.customerName && order.customerName.toLowerCase().includes(searchTerm.toLowerCase())) ||
        order.equipment.toLowerCase().includes(searchTerm.toLowerCase());

      const matchesStatus = statusFilter === "all" || order.status === statusFilter;

      return matchesSearch && matchesStatus;
    });

    if (sortConfig.direction && sortConfig.column) {
      const dir = sortConfig.direction;
      const col = sortConfig.column;
      result = [...result].sort((a, b) => {
        const valA = getOrderSortValue(a, col);
        const valB = getOrderSortValue(b, col);
        if (valA == null && valB == null) return 0;
        if (valA == null) return 1;
        if (valB == null) return -1;
        if (typeof valA === "string" && typeof valB === "string") {
          return dir === "asc" ? valA.localeCompare(valB, "pt-BR") : valB.localeCompare(valA, "pt-BR");
        }
        return dir === "asc" ? (valA as number) - (valB as number) : (valB as number) - (valA as number);
      });
    }

    return result;
  }, [orders, searchTerm, statusFilter, sortConfig]);

  const filteredInventory = useMemo(() => {
    if (!inventorySearch) return [];
    return inventory.filter(item =>
      item.name.toLowerCase().includes(inventorySearch.toLowerCase())
    );
  }, [inventory, inventorySearch]);

  const getStatusBadge = (status: OSStatus) => {
    switch (status) {
      case "Orçamento": return <Badge variant="outline">{status}</Badge>;
      case "Em Manutenção": return <Badge variant="default" className="bg-blue-600">{status}</Badge>;
      case "Aguardando Peça": return <Badge variant="destructive">{status}</Badge>;
      case "Finalizada": return <Badge variant="secondary" className="bg-green-600 text-white">{status}</Badge>;
      case "Cancelada": return <Badge variant="outline" className="opacity-50">{status}</Badge>;
      default: return <Badge variant="outline">{status}</Badge>;
    }
  };

  const handleNewOS = () => {
    navigate("/os/new");
  };

  const handleEditOS = async (order: ServiceOrderWithChecklist) => {
    setSelectedOS(order);
    setEditStatus(order.status);
    setEditDescription(order.description);
    setEditDiscount(order.discountPercent);
    setEditDiscountInput(String(order.discountPercent));
    const items = await fetchOSItems(order.id);
    setOsItems(items);
    setIsEditOpen(true);
  };

  const handleViewOS = async (order: ServiceOrderWithChecklist) => {
    setSelectedOS(order);
    const items = await fetchOSItems(order.id);
    setOsItems(items);
    setIsDetailOpen(true);
  };

  const handleDeleteOS = async (id: string) => {
    const confirmDelete = window.confirm(`Deseja realmente excluir a OS ${id}? Esta ação não pode ser desfeita.`);
    if (confirmDelete) {
      try {
        await invoke("delete_service_order", { id });
        await queryClient.invalidateQueries({ queryKey: ["service-orders"] });
        alert("Ordem de serviço excluída!");
      } catch (error) {
        alert(`Erro ao excluir: ${error}`);
      }
    }
  };

  const handleSaveEdit = async () => {
    if (!selectedOS) return;
    const discountVal = Math.max(0, Math.min(100, parseInt(editDiscountInput) || 0));
    const result = editServiceOrderSchema.safeParse({ description: editDescription, discount: discountVal });
    const fieldErrors = parseErrors(result);
    if (fieldErrors) {
      setEditErrors(fieldErrors);
      return;
    }
    setEditErrors({});
    try {
      const closedAt = editStatus === "Finalizada" ? new Date().toISOString() : null;
      await invoke("update_service_order", {
        id: selectedOS.id,
        customerId: selectedOS.customerId,
        customerName: selectedOS.customerName || null,
        userId: selectedOS.userId || null,
        equipment: selectedOS.equipment,
        imei: selectedOS.imei || null,
        description: editDescription,
        status: editStatus,
        totalPrice: selectedOS.totalPrice || null,
        signaturePath: selectedOS.signaturePath || null,
        closedAt,
        discountPercent: discountVal
      });
      await queryClient.invalidateQueries({ queryKey: ["service-orders"] });
      await queryClient.invalidateQueries({ queryKey: ["service-order"] });
      await queryClient.invalidateQueries({ queryKey: ["dashboard-data"] });
      alert("Alterações salvas com sucesso!");
      setIsEditOpen(false);
    } catch (error) {
      alert(`Erro ao salvar: ${error}`);
    }
  };

  const handleAddInventoryItem = async (item: InventoryItem) => {
    if (!selectedOS) return;
    try {
      await invoke("add_part_to_service_order", {
        serviceOrderId: selectedOS.id,
        inventoryItemId: item.id,
        quantity: 1
      });
      // Refresh items
      const items = await fetchOSItems(selectedOS.id);
      setOsItems(items);
      setInventorySearch("");
      setShowInventoryResults(false);
    } catch (error) {
      alert(`Erro ao adicionar item: ${error}`);
    }
  };

  const handleRemoveItem = async (itemId: string) => {
    if (!selectedOS) return;
    try {
      await invoke("remove_part_from_service_order", { partId: itemId });
      // Refresh items
      const items = await fetchOSItems(selectedOS.id);
      setOsItems(items);
    } catch (error) {
      alert(`Erro ao remover item: ${error}`);
    }
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Ordens de Serviço</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie atendimentos, ordens de serviço e orçamentos.
          </p>
        </div>
        <Button onClick={handleNewOS} className="gap-2">
          <Plus className="h-4 w-4" /> Nova OS
        </Button>
      </div>

      <div className="flex flex-col gap-4">
        <Tabs defaultValue="all" className="w-full" onValueChange={setStatusFilter}>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <TabsList>
              <TabsTrigger value="all">Todas</TabsTrigger>
              <TabsTrigger value="Orçamento">Orçamentos</TabsTrigger>
              <TabsTrigger value="Em Manutenção">Em Manutenção</TabsTrigger>
              <TabsTrigger value="Aguardando Peça">Pendentes</TabsTrigger>
              <TabsTrigger value="Finalizada">Finalizadas</TabsTrigger>
            </TabsList>

            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar por ID, Cliente ou Equipamento..."
                className="pl-9"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
              />
            </div>
          </div>
        </Tabs>

        <Card>
          <CardContent className="p-0">
            <div className="max-h-[500px] overflow-y-auto">
              <Table>
              <TableHeader>
                <TableRow>
                  <SortableHeader column="displayId" label="ID" sortConfig={sortConfig} onSort={cycleSort} className="w-[120px]" />
                  <SortableHeader column="customerName" label="Cliente & Equipamento" sortConfig={sortConfig} onSort={cycleSort} />
                  <SortableHeader column="status" label="Status" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell" />
                  <SortableHeader column="createdAt" label="Abertura" sortConfig={sortConfig} onSort={cycleSort} className="hidden lg:table-cell" />
                  <SortableHeader column="totalPrice" label="Valor" sortConfig={sortConfig} onSort={cycleSort} className="text-right" align="right" />
                  <TableHead className="text-right w-[100px]">Ações</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-10 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden lg:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-5 w-16 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : filteredOrders.length > 0 ? (
                  filteredOrders.map((order) => (
                    <TableRow key={order.id} className="group transition-colors hover:bg-muted/50">
                      <TableCell className="font-mono text-xs font-bold">
                        {order.displayId || order.id.slice(0, 8)}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          <div className="flex items-center gap-1.5 font-medium">
                            <User className="h-3 w-3 text-muted-foreground" />
                            {order.customerName}
                          </div>
                          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                            <Smartphone className="h-3 w-3" />
                            {order.equipment}
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        {getStatusBadge(order.status)}
                      </TableCell>
                      <TableCell className="hidden lg:table-cell text-xs text-muted-foreground">
                        <div className="flex items-center gap-1.5">
                          <Calendar className="h-3 w-3" />
                          {new Date(order.createdAt).toLocaleDateString('pt-BR')}
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-medium">
                        {order.discountPercent > 0
                          ? <><span className="text-xs line-through text-muted-foreground mr-1">{formatCurrency(order.totalPrice || 0)}</span> {formatCurrency((order.totalPrice || 0) * (1 - order.discountPercent / 100))}</>
                          : formatCurrency(order.totalPrice || 0)}
                      </TableCell>
                      <TableCell className="text-right">
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuLabel>Ações</DropdownMenuLabel>
                            <DropdownMenuItem onClick={() => handleViewOS(order)}>
                              <Eye className="mr-2 h-4 w-4" /> Visualizar
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleEditOS(order)}>
                              <Edit className="mr-2 h-4 w-4" /> Editar
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              className="text-destructive focus:text-destructive"
                              onClick={() => handleDeleteOS(order.id)}
                            >
                              <Trash2 className="mr-2 h-4 w-4" /> Excluir
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                      Nenhuma ordem de serviço encontrada.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
              </Table>
            </div>
          </CardContent>
          </Card>
        </div>

        <ServiceOrderDetailSheet
        orderId={selectedOS?.id ?? null}
        open={isDetailOpen}
        onClose={() => setIsDetailOpen(false)}
      />

      {/* SHEET DE EDIÇÃO (EDIT) */}
      <Sheet open={isEditOpen} onOpenChange={setIsEditOpen}>
        <SheetContent className="sm:max-w-xl overflow-y-auto">
          <SheetHeader>
            <SheetTitle>Editar - {selectedOS?.displayId || selectedOS?.id.slice(0, 8)}</SheetTitle>
            <SheetDescription>
              Atualize o status, descrição ou gerencie os itens desta OS.
            </SheetDescription>
          </SheetHeader>

          <div className="mt-8 space-y-6">
            <div className="space-y-3">
              <Label className="text-xs font-semibold uppercase">Status da Ordem</Label>
              <div className="grid grid-cols-2 gap-2">
                {(["Orçamento", "Em Manutenção", "Aguardando Peça", "Finalizada", "Cancelada"] as OSStatus[]).map((s) => (
                  <Button
                    key={s}
                    variant={editStatus === s ? "default" : "outline"}
                    size="sm"
                    className="h-8 text-xs"
                    onClick={() => setEditStatus(s)}
                  >
                    {s}
                  </Button>
                ))}
              </div>
            </div>

            <Separator />

            <div className="space-y-2">
              <Label htmlFor="edit-desc" className="text-xs font-semibold uppercase">Relato / Notas Técnicas</Label>
              <Textarea
                id="edit-desc"
                value={editDescription}
                onChange={(e) => { setEditDescription(e.target.value); setEditErrors(clearFieldError(editErrors, "description")); }}
                className="min-h-[100px] text-sm"
              />
              {editErrors.description && <p className="text-xs text-destructive">{editErrors.description}</p>}
            </div>

            <Separator />

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <Label className="text-xs font-semibold uppercase">Peças & Serviços</Label>
              </div>

              <div className="relative">
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    placeholder="Buscar peça ou serviço no estoque..."
                    className="pl-9 pr-4 text-sm"
                    value={inventorySearch}
                    onChange={(e) => {
                      setInventorySearch(e.target.value);
                      setShowInventoryResults(true);
                    }}
                    onFocus={() => setShowInventoryResults(true)}
                  />
                </div>

                {showInventoryResults && filteredInventory.length > 0 && (
                  <Card className="absolute z-20 w-full mt-1 shadow-xl border-primary/20">
                    <ScrollArea className="max-h-48">
                      <div className="p-1">
                        {filteredInventory.map((item) => (
                          <div
                            key={item.id}
                            className="flex items-center justify-between p-2 hover:bg-accent rounded-sm cursor-pointer transition-colors"
                            onClick={() => handleAddInventoryItem(item)}
                          >
                            <div className="flex flex-col">
                              <span className="text-xs font-medium">{item.name}</span>
                              <span className="text-[10px] text-muted-foreground">Estoque: {item.currentQuantity} un.</span>
                            </div>
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-bold text-primary">{formatCurrency(item.salePrice)}</span>
                              <ChevronRight className="h-3 w-3 text-muted-foreground" />
                            </div>
                          </div>
                        ))}
                      </div>
                    </ScrollArea>
                  </Card>
                )}
              </div>

              <div className="rounded-md border overflow-hidden">
                <Table>
                  <TableHeader className="bg-muted/50">
                    <TableRow>
                      <TableHead className="h-8 text-[10px]">Item</TableHead>
                      <TableHead className="h-8 text-right text-[10px]">Valor</TableHead>
                      <TableHead className="h-8 text-right text-[10px]">Ação</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {osItems.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell className="py-2 text-xs font-medium">{item.inventoryItemName}</TableCell>
                        <TableCell className="py-2 text-right text-xs font-bold">{formatCurrency(item.unitPrice)}</TableCell>
                        <TableCell className="py-2 text-right">
                          <Button variant="ghost" size="icon" className="h-6 w-6 text-destructive" onClick={() => handleRemoveItem(item.id)}>
                            <Trash className="h-3 w-3" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                    {osItems.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={3} className="h-12 text-center text-xs text-muted-foreground italic">
                          Busque uma peça acima para adicionar.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
              <Separator />
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Label className="text-xs font-semibold uppercase">Desconto</Label>
                  <Input
                    type="number"
                    value={editDiscountInput}
                    onChange={(e) => setEditDiscountInput(e.target.value)}
                    onBlur={() => {
                      const n = Math.max(0, Math.min(100, parseInt(editDiscountInput) || 0));
                      setEditDiscount(n);
                      setEditDiscountInput(String(n));
                    }}
                    className="w-20 h-8 text-sm"
                  />
                  <span className="text-sm text-muted-foreground">%</span>
                </div>
                <div className="text-right">
                  {editDiscount > 0 ? (
                    <div className="flex items-center gap-2">
                      <span className="text-xs line-through text-muted-foreground">
                        {formatCurrency(osItems.reduce((acc, i) => acc + (i.unitPrice * i.quantity), 0))}
                      </span>
                      <span className="text-sm font-bold text-primary">
                        {formatCurrency(osItems.reduce((acc, i) => acc + (i.unitPrice * i.quantity), 0) * (1 - editDiscount / 100))}
                      </span>
                    </div>
                  ) : (
                    <span className="text-sm font-bold">
                      Total: {formatCurrency(osItems.reduce((acc, i) => acc + (i.unitPrice * i.quantity), 0))}
                    </span>
                  )}
                </div>
              </div>
            </div>
          </div>

          <SheetFooter className="mt-8 flex gap-2">
            <Button variant="outline" className="flex-1" onClick={() => setIsEditOpen(false)}>Cancelar</Button>
            <Button className="flex-1 gap-2" onClick={handleSaveEdit}>
              <Save className="h-4 w-4" /> Salvar Alterações
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
