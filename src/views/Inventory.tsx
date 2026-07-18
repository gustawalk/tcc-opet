import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  Plus,
  Search,
  MoreVertical,
  Package,
  PackagePlus,
  TrendingUp,
  AlertTriangle,
  Edit,
  Trash2,
  History,
  Save,
  DollarSign,
  Box,
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle
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
import { InventoryItem, InventoryMovement } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { formatCurrency } from "@/lib/formatters";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { Separator } from "@/components/ui/separator";
import { inventoryItemSchema, quantitySchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { useSort } from "@/hooks/useSort";
import { SortableHeader } from "@/components/shared/SortableHeader";
import { toastSuccess, toastError } from "@/lib/errors";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

const fetchInventory = async (): Promise<InventoryItem[]> => {
  return await invoke<InventoryItem[]>("get_inventory_items");
};

const fetchMovements = async (itemId: string): Promise<InventoryMovement[]> => {
  return await invoke<InventoryMovement[]>("get_inventory_movements", { id: itemId });
};

const createInventoryItem = async (item: Omit<InventoryItem, "id" | "createdAt" | "deletedAt">) => {
  return await invoke<string>("create_inventory_item", {
    name: item.name,
    description: item.description,
    type: item.type,
    minQuantity: item.minQuantity,
    currentQuantity: item.type === "part" ? 0 : 999,
    costPrice: item.costPrice,
    salePrice: item.salePrice,
  });
};

const updateInventoryItem = async (item: InventoryItem) => {
  return await invoke("update_inventory_item", {
    id: item.id,
    name: item.name,
    description: item.description,
    type: item.type,
    minQuantity: item.minQuantity,
    currentQuantity: item.currentQuantity,
    costPrice: item.costPrice,
    salePrice: item.salePrice,
  });
};

const deleteInventoryItem = async (id: string) => {
  return await invoke("delete_inventory_item", { id });
};

export function Inventory() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedItem, setSelectedItem] = useState<InventoryItem | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  // Form State
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    type: "part" as "part" | "service",
    minQuantity: 0,
    costPrice: 0,
    salePrice: 0
  });
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [restockErrors, setRestockErrors] = useState<ValidationErrors>({});
  const [removeErrors, setRemoveErrors] = useState<ValidationErrors>({});

  // Restock state
  const [restockItem, setRestockItem] = useState<InventoryItem | null>(null);
  const [restockQuantity, setRestockQuantity] = useState(1);

  // Remove state
  const [removeItem, setRemoveItem] = useState<InventoryItem | null>(null);
  const [removeQuantity, setRemoveQuantity] = useState(1);

  // History state
  const [historyItem, setHistoryItem] = useState<InventoryItem | null>(null);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);

  const { data: items = [], isLoading } = useQuery({
    queryKey: ["inventory"],
    queryFn: fetchInventory,
  });

  const queryClient = useQueryClient();
  const { sortConfig, cycleSort } = useSort();

  const createMutation = useMutation({
    mutationFn: createInventoryItem,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventory"] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: updateInventoryItem,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventory"] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteInventoryItem,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventory"] });
      toastSuccess("Item excluído com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao excluir item."),
  });

  const restockMutation = useMutation({
    mutationFn: async ({ id, quantity }: { id: string; quantity: number }) => {
      return await invoke("restock_inventory_item", { id, quantity });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventory"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-movements"] });
      setRestockItem(null);
      setRestockQuantity(1);
    },
  });

  const removeStockMutation = useMutation({
    mutationFn: async ({ id, quantity }: { id: string; quantity: number }) => {
      return await invoke("remove_stock_inventory_item", { id, quantity });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventory"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-movements"] });
      setRemoveItem(null);
      setRemoveQuantity(1);
    },
  });

  const { data: movements = [] } = useQuery({
    queryKey: ["inventory-movements", historyItem?.id],
    queryFn: () => fetchMovements(historyItem!.id),
    enabled: isHistoryOpen && !!historyItem,
  });

  const parts = useMemo(() => items.filter(i => i.type === "part"), [items]);
  const services = useMemo(() => items.filter(i => i.type === "service"), [items]);

  const getPartSortValue = (item: InventoryItem, column: string): string | number => {
    switch (column) {
      case "name": return item.name;
      case "currentQuantity": return item.currentQuantity;
      case "costPrice": return item.costPrice;
      case "salePrice": return item.salePrice;
      default: return "";
    }
  };

  const getServiceSortValue = (item: InventoryItem, column: string): string | number => {
    switch (column) {
      case "name": return item.name;
      case "costPrice": return item.costPrice;
      case "salePrice": return item.salePrice;
      default: return "";
    }
  };

  const filteredParts = useMemo(() => {
    let result = parts.filter(item =>
      item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.description.toLowerCase().includes(searchTerm.toLowerCase())
    );

    if (sortConfig.direction && sortConfig.column) {
      const dir = sortConfig.direction;
      const col = sortConfig.column;
      result = [...result].sort((a, b) => {
        const valA = getPartSortValue(a, col);
        const valB = getPartSortValue(b, col);
        if (valA == null && valB == null) return 0;
        if (valA == null) return 1;
        if (valB == null) return -1;
        if (typeof valA === "string" && typeof valB === "string") {
          return dir === "asc" ? valA.localeCompare(valB, "pt-BR") : valB.localeCompare(valA, "pt-BR");
        }
        return dir === "asc" ? (valA as number) - (valB as number) : (valB as number) - (valA as number);
      });
    } else {
      result = [...result].sort((a, b) => {
        const priA = a.currentQuantity === 0 ? 0
          : a.currentQuantity <= a.minQuantity ? 1 : 2;
        const priB = b.currentQuantity === 0 ? 0
          : b.currentQuantity <= b.minQuantity ? 1 : 2;
        if (priA !== priB) return priA - priB;

        const tA = a.updatedAt
          ? new Date(a.updatedAt).getTime()
          : new Date(a.createdAt ?? 0).getTime();
        const tB = b.updatedAt
          ? new Date(b.updatedAt).getTime()
          : new Date(b.createdAt ?? 0).getTime();
        return tB - tA;
      });
    }

    return result;
  }, [parts, searchTerm, sortConfig]);

  const filteredServices = useMemo(() => {
    let result = services.filter(item =>
      item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.description.toLowerCase().includes(searchTerm.toLowerCase())
    );

    if (sortConfig.direction && sortConfig.column) {
      const dir = sortConfig.direction;
      const col = sortConfig.column;
      result = [...result].sort((a, b) => {
        const valA = getServiceSortValue(a, col);
        const valB = getServiceSortValue(b, col);
        if (valA == null && valB == null) return 0;
        if (valA == null) return 1;
        if (valB == null) return -1;
        if (typeof valA === "string" && typeof valB === "string") {
          return dir === "asc" ? valA.localeCompare(valB, "pt-BR") : valB.localeCompare(valA, "pt-BR");
        }
        return dir === "asc" ? (valA as number) - (valB as number) : (valB as number) - (valA as number);
      });
    } else {
      result = [...result].sort((a, b) => {
        const tA = a.updatedAt
          ? new Date(a.updatedAt).getTime()
          : new Date(a.createdAt ?? 0).getTime();
        const tB = b.updatedAt
          ? new Date(b.updatedAt).getTime()
          : new Date(b.createdAt ?? 0).getTime();
        return tB - tA;
      });
    }

    return result;
  }, [services, searchTerm, sortConfig]);

  const handleAddItem = (type: "part" | "service" = "part") => {
    setSelectedItem(null);
    setErrors({});
    setFormData({
      name: "",
      description: "",
      type: type,
      minQuantity: type === "part" ? 5 : 0,
      costPrice: 0,
      salePrice: 0
    });
    setIsSheetOpen(true);
  };

  const handleEditItem = (item: InventoryItem) => {
    setSelectedItem(item);
    setErrors({});
    setFormData({
      name: item.name,
      description: item.description,
      type: item.type,
      minQuantity: item.minQuantity,
      costPrice: item.costPrice,
      salePrice: item.salePrice
    });
    setIsSheetOpen(true);
  };

  const handleSave = async () => {
    const result = inventoryItemSchema.safeParse(formData);
    const fieldErrors = parseErrors(result);
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }
    setErrors({});
    if (selectedItem) {
      await updateMutation.mutateAsync({
        ...selectedItem,
        ...formData,
      });
    } else {
      await createMutation.mutateAsync({
        ...formData,
        currentQuantity: formData.type === "part" ? 0 : 999,
      } as Omit<InventoryItem, "id" | "createdAt" | "deletedAt">);
    }
    setIsSheetOpen(false);
    setSelectedItem(null);
    setFormData({
      name: "",
      description: "",
      type: "part",
      minQuantity: 0,
      costPrice: 0,
      salePrice: 0
    });
  };

  const handleDeleteItem = (id: string) => {
    setConfirmDeleteId(id);
  };

  const confirmDeleteItem = async () => {
    if (confirmDeleteId) {
      await deleteMutation.mutateAsync(confirmDeleteId);
      setConfirmDeleteId(null);
    }
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Estoque & Serviços</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie peças, insumos e serviços de mão de obra.
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={() => handleAddItem("part")} className="gap-2">
            <Plus className="h-4 w-4" /> Nova Peça
          </Button>
          <Button onClick={() => handleAddItem("service")} variant="secondary" className="gap-2">
            <Plus className="h-4 w-4" /> Novo Serviço
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Peças em Alerta</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-amber-500" />
              <div className="text-2xl font-bold">
                {parts.filter(i => i.currentQuantity <= i.minQuantity && i.currentQuantity > 0).length}
              </div>
              <span className="text-xs text-muted-foreground mt-1">abaixo do mínimo</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Peças Esgotadas</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Package className="h-5 w-5 text-destructive" />
              <div className="text-2xl font-bold">
                {parts.filter(i => i.currentQuantity === 0).length}
              </div>
              <span className="text-xs text-muted-foreground mt-1">itens sem estoque</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Valor em Peças</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-primary" />
              <div className="text-2xl font-bold">
                {formatCurrency(parts.reduce((acc, i) => acc + (i.costPrice * i.currentQuantity), 0))}
              </div>
              <span className="text-xs text-muted-foreground mt-1">preço de custo</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="space-y-6">
        <Card>
          <CardHeader>
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div>
                <CardTitle>Inventário de Peças</CardTitle>
                <CardDescription>Peças e componentes físicos cadastrados.</CardDescription>
              </div>
              <div className="relative w-full md:w-72">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Buscar peça..."
                  className="pl-9"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="max-h-[500px] overflow-y-auto rounded-md border" style={{ contentVisibility: 'auto' as const, containIntrinsicSize: '500px' }}>
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortableHeader column="name" label="Peça / Descrição" sortConfig={sortConfig} onSort={cycleSort} />
                    <SortableHeader column="currentQuantity" label="Estoque Atual" sortConfig={sortConfig} onSort={cycleSort} className="text-center" align="center" />
                    <SortableHeader column="costPrice" label="Preço de Custo" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell text-right" align="right" />
                    <SortableHeader column="salePrice" label="Preço de Venda" sortConfig={sortConfig} onSort={cycleSort} className="text-right" align="right" />
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {isLoading ? (
                    Array.from({ length: 3 }).map((_, i) => (
                      <TableRow key={i}>
                        <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                        <TableCell><div className="h-5 w-16 bg-muted animate-pulse rounded mx-auto" /></TableCell>
                        <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      </TableRow>
                    ))
                  ) : filteredParts.length > 0 ? (
                    filteredParts.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell>
                          <div className="flex flex-col gap-1">
                            <span className="font-medium">{item.name}</span>
                            <span className="text-xs text-muted-foreground line-clamp-1">{item.description}</span>
                          </div>
                        </TableCell>
                        <TableCell className="text-center">
                          <div className="flex flex-col items-center gap-1">
                            <Badge variant={
                              item.currentQuantity === 0 ? "destructive" :
                                item.currentQuantity <= item.minQuantity ? "default" : "secondary"
                            }>
                              {item.currentQuantity} un.
                            </Badge>
                            <span className="text-[10px] text-muted-foreground">Mín: {item.minQuantity}</span>
                          </div>
                        </TableCell>
                        <TableCell className="hidden md:table-cell text-right font-medium text-muted-foreground">
                          {formatCurrency(item.costPrice)}
                        </TableCell>
                        <TableCell className="text-right font-bold text-primary">
                          {formatCurrency(item.salePrice)}
                        </TableCell>
                        <TableCell className="text-right">
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuLabel>Ações</DropdownMenuLabel>
                              <DropdownMenuItem onClick={() => { setRestockItem(item); setRestockQuantity(1); }}>
                                <PackagePlus className="mr-2 h-4 w-4" /> Adicionar
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => { setRemoveItem(item); setRemoveQuantity(1); }} disabled={item.currentQuantity < 1}>
                                <Package className="mr-2 h-4 w-4" /> Remover
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => { setHistoryItem(item); setIsHistoryOpen(true); }}>
                                <History className="mr-2 h-4 w-4" /> Histórico
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => handleEditItem(item)}>
                                <Edit className="mr-2 h-4 w-4" /> Editar
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                className="text-destructive focus:text-destructive"
                                onClick={() => handleDeleteItem(item.id)}
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
                      <TableCell colSpan={5} className="h-24 text-center text-muted-foreground">
                        Nenhuma peça encontrada.
                      </TableCell>
                    </TableRow>
                  )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>

        <Card>
          <CardHeader>
            <div>
              <CardTitle>Catálogo de Serviços</CardTitle>
              <CardDescription>Mão de obra e serviços recorrentes.</CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            <div className="max-h-[500px] overflow-y-auto rounded-md border" style={{ contentVisibility: 'auto' as const, containIntrinsicSize: '500px' }}>
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortableHeader column="name" label="Serviço / Mão de Obra" sortConfig={sortConfig} onSort={cycleSort} />
                    <SortableHeader column="costPrice" label="Custo Estimado" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell text-right" align="right" />
                    <SortableHeader column="salePrice" label="Preço de Venda" sortConfig={sortConfig} onSort={cycleSort} className="text-right" align="right" />
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {isLoading ? (
                    Array.from({ length: 2 }).map((_, i) => (
                      <TableRow key={i}>
                        <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                        <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      </TableRow>
                    ))
                  ) : filteredServices.length > 0 ? (
                    filteredServices.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell>
                          <div className="flex flex-col gap-1">
                            <span className="font-medium">{item.name}</span>
                            <span className="text-xs text-muted-foreground line-clamp-1">{item.description}</span>
                          </div>
                        </TableCell>
                        <TableCell className="hidden md:table-cell text-right font-medium text-muted-foreground">
                          {formatCurrency(item.costPrice)}
                        </TableCell>
                        <TableCell className="text-right font-bold text-primary">
                          {formatCurrency(item.salePrice)}
                        </TableCell>
                        <TableCell className="text-right">
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuLabel>Ações</DropdownMenuLabel>
                              <DropdownMenuItem onClick={() => handleEditItem(item)}>
                                <Edit className="mr-2 h-4 w-4" /> Editar
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                className="text-destructive focus:text-destructive"
                                onClick={() => handleDeleteItem(item.id)}
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
                      <TableCell colSpan={4} className="h-24 text-center text-muted-foreground">
                        Nenhum serviço encontrado.
                      </TableCell>
                    </TableRow>
                  )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>
          </div>

      {/* Restock Dialog */}
      <Dialog open={!!restockItem} onOpenChange={(open) => { if (!open) setRestockItem(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Adicionar ao Estoque</DialogTitle>
            <DialogDescription>
              Adicione unidades ao estoque de <strong>{restockItem?.name}</strong>.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="restock-qty">Quantidade a adicionar</Label>
            <Input
              id="restock-qty"
              type="number"
              min="1"
              className="mt-2"
              value={restockQuantity}
              onChange={(e) => setRestockQuantity(Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRestockItem(null)}>Cancelar</Button>
            {restockErrors.quantity && <p className="text-xs text-destructive">{restockErrors.quantity}</p>}
            <Button
              onClick={() => {
                const r = quantitySchema.safeParse({ quantity: restockQuantity });
                const fe = parseErrors(r);
                if (fe) { setRestockErrors(fe); return; }
                setRestockErrors({});
                restockItem && restockMutation.mutate({ id: restockItem.id, quantity: restockQuantity });
              }}
              disabled={restockMutation.isPending}
              className="gap-2"
            >
              <PackagePlus className="h-4 w-4" /> {restockMutation.isPending ? "Adicionando..." : "Adicionar"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Remove Dialog */}
      <Dialog open={!!removeItem} onOpenChange={(open) => { if (!open) setRemoveItem(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remover do Estoque</DialogTitle>
            <DialogDescription>
              Remova unidades do estoque de <strong>{removeItem?.name}</strong>. Estoque atual: {removeItem?.currentQuantity} un.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="remove-qty">Quantidade a remover</Label>
            <Input
              id="remove-qty"
              type="number"
              min="1"
              max={removeItem?.currentQuantity ?? 1}
              className="mt-2"
              value={removeQuantity}
              onChange={(e) => setRemoveQuantity(Math.max(1, Math.min(removeItem?.currentQuantity ?? 1, parseInt(e.target.value) || 1)))}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRemoveItem(null)}>Cancelar</Button>
            {removeErrors.quantity && <p className="text-xs text-destructive">{removeErrors.quantity}</p>}
            <Button
              onClick={() => {
                const r = quantitySchema.safeParse({ quantity: removeQuantity });
                const fe = parseErrors(r);
                if (fe) { setRemoveErrors(fe); return; }
                setRemoveErrors({});
                removeItem && removeStockMutation.mutate({ id: removeItem.id, quantity: removeQuantity });
              }}
              disabled={removeStockMutation.isPending}
              variant="destructive"
              className="gap-2"
            >
              <Package className="h-4 w-4" /> {removeStockMutation.isPending ? "Removendo..." : "Remover"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* History Sheet */}
      <Sheet open={isHistoryOpen} onOpenChange={setIsHistoryOpen}>
        <SheetContent className="sm:max-w-lg">
          <SheetHeader>
            <SheetTitle>Histórico de Movimentações</SheetTitle>
            <SheetDescription>
              {historyItem ? `Movimentações de ${historyItem.name}` : ""}
            </SheetDescription>
          </SheetHeader>
          <div className="py-4">
            {movements.length === 0 ? (
              <p className="text-center text-muted-foreground py-8">Nenhuma movimentação encontrada.</p>
            ) : (
              <div className="rounded-md border">
                <Table>
                  <TableHeader>
                    <TableRow>
                    <TableHead>Tipo</TableHead>
                    <TableHead className="text-center">Quantidade</TableHead>
                    <TableHead>OS</TableHead>
                    <TableHead className="text-right">Data</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {movements.map((mov) => (
                      <TableRow key={mov.id}>
                        <TableCell>
                          <Badge variant={mov.type === "entrada" ? "default" : "destructive"}>
                            {mov.type === "entrada" ? "Entrada" : "Saída"}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-center font-medium">{mov.quantity} un.</TableCell>
                        <TableCell className="font-mono text-xs">
                          {mov.osDisplayId ?? "-"}
                        </TableCell>
                        <TableCell className="text-right text-xs text-muted-foreground">
                          {mov.createdAt ? new Date(mov.createdAt).toLocaleString("pt-BR") : "-"}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
          <SheetFooter>
            <Button variant="outline" className="w-full" onClick={() => setIsHistoryOpen(false)}>
              Fechar
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* Sheet para Cadastro/Edição de Item */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>
              {selectedItem ? "Editar" : "Novo"} {formData.type === "part" ? "Item no Estoque" : "Serviço"}
            </SheetTitle>
            <SheetDescription>
              {formData.type === "part"
                ? "Cadastre peças e insumos para gerenciar seu estoque."
                : "Cadastre serviços e mão de obra para suas ordens de serviço."}
            </SheetDescription>
          </SheetHeader>

          <div className="grid gap-4 py-6">
            <div className="grid gap-2">
              <Label htmlFor="name">Nome do {formData.type === "part" ? "Produto" : "Serviço"}</Label>
              <Input
                id="name"
                value={formData.name}
                placeholder={formData.type === "part" ? "Ex: Tela iPhone 11" : "Ex: Mão de obra Drone"}
                onChange={(e) => { setFormData({ ...formData, name: e.target.value }); setErrors(clearFieldError(errors, "name")); }}
              />
              {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="description">Descrição</Label>
              <Textarea
                id="description"
                value={formData.description}
                placeholder="Ex: Detalhes adicionais..."
                onChange={(e) => { setFormData({ ...formData, description: e.target.value }); setErrors(clearFieldError(errors, "description")); }}
              />
              {errors.description && <p className="text-xs text-destructive">{errors.description}</p>}
            </div>

            <Separator />

            {formData.type === "part" && (
              <div className="grid gap-2">
                <Label htmlFor="min">Qtd. Mínima (Alerta)</Label>
                <div className="relative">
                  <Box className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    id="min"
                    type="number"
                    className="pl-9"
                    value={formData.minQuantity}
                    onChange={(e) => { setFormData({ ...formData, minQuantity: parseInt(e.target.value) || 0 }); setErrors(clearFieldError(errors, "minQuantity")); }}
                  />
                </div>
                {errors.minQuantity && <p className="text-xs text-destructive">{errors.minQuantity}</p>}
              </div>
            )}

            <div className="grid grid-cols-2 gap-4">
              <div className="grid gap-2">
                <Label htmlFor="cost">{formData.type === "part" ? "Preço de Custo" : "Custo Estimado"}</Label>
                <div className="relative">
                  <DollarSign className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    id="cost"
                    type="number"
                    step="0.01"
                    className="pl-9"
                    value={formData.costPrice}
                    onChange={(e) => { setFormData({ ...formData, costPrice: parseFloat(e.target.value) || 0 }); setErrors(clearFieldError(errors, "costPrice")); }}
                  />
                </div>
                {errors.costPrice && <p className="text-xs text-destructive">{errors.costPrice}</p>}
              </div>
              <div className="grid gap-2">
                <Label htmlFor="sale">Preço de Venda</Label>
                <div className="relative">
                  <TrendingUp className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-primary" />
                  <Input
                    id="sale"
                    type="number"
                    step="0.01"
                    className="pl-9"
                    value={formData.salePrice}
                    onChange={(e) => { setFormData({ ...formData, salePrice: parseFloat(e.target.value) || 0 }); setErrors(clearFieldError(errors, "salePrice")); }}
                  />
                </div>
              </div>
            </div>
          </div>

          <SheetFooter>
            <Button variant="outline" className="w-full" onClick={() => setIsSheetOpen(false)}>
              Cancelar
            </Button>
            <Button className="w-full gap-2" onClick={handleSave}>
              <Save className="h-4 w-4" /> {selectedItem ? "Salvar Alterações" : "Cadastrar"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      <AlertDialog open={!!confirmDeleteId} onOpenChange={() => setConfirmDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Excluir item</AlertDialogTitle>
            <AlertDialogDescription>
              Esta ação não pode ser desfeita. Deseja realmente excluir este item?
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancelar</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDeleteItem}>Excluir</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
