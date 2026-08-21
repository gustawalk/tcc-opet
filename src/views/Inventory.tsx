import { useEffect, useRef, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { dataCommand } from "@/lib/data-client";
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
  Copy,
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
import { InventoryInsights, InventoryItem, InventoryMovement, InventorySummary, Page } from "@/lib/types";
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
import { quantitySchema, parseErrors, ValidationErrors } from "@/lib/validation";
import { Pagination } from "@/components/shared/Pagination";
import { useDebounce } from "@/hooks/use-debounce";
import { Copyable } from "@/components/shared/Copyable";
import { toastSuccess, toastError } from "@/lib/errors";
import { InventoryItemSheet } from "@/components/shared/InventoryItemSheet";
import {
  currencyInputToNumber,
  formatCurrencyInput,
  integerInputToNumber,
  normalizeCurrencyInput,
  normalizeIntegerInput,
  sanitizeIntegerInput,
} from "@/lib/numeric-input";

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const SEARCH_DEBOUNCE_MS = 300;

const fetchInventoryPage = (args: {
  itemType: "part" | "service";
  limit: number;
  offset: number;
  search: string;
}): Promise<Page<InventoryItem>> => {
  return dataCommand<Page<InventoryItem>>("get_inventory_items_page", args);
};

const fetchMovements = async (itemId: string): Promise<InventoryMovement[]> => {
  return await dataCommand<InventoryMovement[]>("get_inventory_movements", { id: itemId });
};

const deleteInventoryItem = async (id: string) => {
  return await dataCommand("delete_inventory_item", { id });
};

export function Inventory() {
  const partsListRef = useRef<HTMLDivElement>(null);
  const servicesListRef = useRef<HTMLDivElement>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const search = useDebounce(searchTerm, SEARCH_DEBOUNCE_MS);
  const [partsPage, setPartsPage] = useState(1);
  const [partsPageSize, setPartsPageSize] = useState(20);
  const [servicesPage, setServicesPage] = useState(1);
  const [servicesPageSize, setServicesPageSize] = useState(20);
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedItem, setSelectedItem] = useState<InventoryItem | null>(null);
  const [duplicateItem, setDuplicateItem] = useState<InventoryItem | null>(null);
  const [createType, setCreateType] = useState<InventoryItem["type"]>("part");
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [restockErrors, setRestockErrors] = useState<ValidationErrors>({});
  const [removeErrors, setRemoveErrors] = useState<ValidationErrors>({});

  // Restock state
  const [restockItem, setRestockItem] = useState<InventoryItem | null>(null);
  const [restockQuantity, setRestockQuantity] = useState("1");
  const [restockUnitCost, setRestockUnitCost] = useState("");
  const [restockReason, setRestockReason] = useState("");
  const [inactiveDays, setInactiveDays] = useState("90");

  // Remove state
  const [removeItem, setRemoveItem] = useState<InventoryItem | null>(null);
  const [removeQuantity, setRemoveQuantity] = useState("1");

  // History state
  const [historyItem, setHistoryItem] = useState<InventoryItem | null>(null);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);

  const queryClient = useQueryClient();

  const partsQuery = useQuery({
    queryKey: ["inventoryItemsPage", "part", partsPage, partsPageSize, search],
    queryFn: () =>
      fetchInventoryPage({
        itemType: "part",
        limit: partsPageSize,
        offset: (partsPage - 1) * partsPageSize,
        search,
      }),
    placeholderData: (previousData) => previousData,
  });

  const servicesQuery = useQuery({
    queryKey: ["inventoryItemsPage", "service", servicesPage, servicesPageSize, search],
    queryFn: () =>
      fetchInventoryPage({
        itemType: "service",
        limit: servicesPageSize,
        offset: (servicesPage - 1) * servicesPageSize,
        search,
      }),
    placeholderData: (previousData) => previousData,
  });

  const partsTotal = partsQuery.data?.total ?? 0;
  const partsTotalPages = Math.max(1, Math.ceil(partsTotal / partsPageSize));
  const servicesTotal = servicesQuery.data?.total ?? 0;
  const servicesTotalPages = Math.max(
    1,
    Math.ceil(servicesTotal / servicesPageSize),
  );

  useEffect(() => {
    setPartsPage(1);
    setServicesPage(1);
  }, [search, partsPageSize, servicesPageSize]);

  useEffect(() => {
    if (partsQuery.data && partsPage > partsTotalPages) setPartsPage(partsTotalPages);
  }, [partsQuery.data, partsTotalPages, partsPage]);

  useEffect(() => {
    if (servicesQuery.data && servicesPage > servicesTotalPages) {
      setServicesPage(servicesTotalPages);
    }
  }, [servicesQuery.data, servicesTotalPages, servicesPage]);

  const { data: summary, isLoading: isSummaryLoading } = useQuery({
    queryKey: ["inventorySummary"],
    queryFn: () => dataCommand<InventorySummary>("get_inventory_summary"),
  });

  const deleteMutation = useMutation({
    mutationFn: deleteInventoryItem,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventoryItemsPage"] });
      queryClient.invalidateQueries({ queryKey: ["inventorySummary"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-insights"] });
      toastSuccess("Item excluído com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao excluir item."),
  });

  const restockMutation = useMutation({
    mutationFn: async ({ id, quantity, unitCost, reason }: { id: string; quantity: number; unitCost?: number; reason?: string }) => {
      return await dataCommand("restock_inventory_item", { id, quantity, unitCost, reason });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventoryItemsPage"] });
      queryClient.invalidateQueries({ queryKey: ["inventorySummary"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-movements"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-insights"] });
      setRestockItem(null);
      setRestockQuantity("1");
      setRestockUnitCost("");
      setRestockReason("");
    },
    onError: (err) => toastError(err, "Erro ao adicionar estoque."),
  });

  const removeStockMutation = useMutation({
    mutationFn: async ({ id, quantity }: { id: string; quantity: number }) => {
      return await dataCommand("remove_stock_inventory_item", { id, quantity });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["inventoryItemsPage"] });
      queryClient.invalidateQueries({ queryKey: ["inventorySummary"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-movements"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-insights"] });
      setRemoveItem(null);
      setRemoveQuantity("1");
    },
    onError: (err) => toastError(err, "Erro ao remover estoque."),
  });

  const { data: movements = [] } = useQuery({
    queryKey: ["inventory-movements", historyItem?.id],
    queryFn: () => fetchMovements(historyItem!.id),
    enabled: isHistoryOpen && !!historyItem,
  });

  const { data: insights, isLoading: isInsightsLoading, error: insightsError } = useQuery({
    queryKey: ["inventory-insights", inactiveDays],
    queryFn: () =>
      dataCommand<InventoryInsights>("get_inventory_insights", {
        inactiveDays: integerInputToNumber(inactiveDays) ?? 0,
      }),
  });

  const handleAddItem = (type: "part" | "service" = "part") => {
    setSelectedItem(null);
    setDuplicateItem(null);
    setCreateType(type);
    setIsSheetOpen(true);
  };

  const handleEditItem = (item: InventoryItem) => {
    setDuplicateItem(null);
    setSelectedItem(item);
    setIsSheetOpen(true);
  };

  const handleDuplicateItem = (item: InventoryItem) => {
    setSelectedItem(null);
    setDuplicateItem(item);
    setCreateType(item.type);
    setIsSheetOpen(true);
  };

  const handleDeleteItem = (id: string) => {
    setConfirmDeleteId(id);
  };

  const confirmDeleteItem = async () => {
    if (!confirmDeleteId || deleteMutation.isPending) return;
    try {
      await deleteMutation.mutateAsync(confirmDeleteId);
      setConfirmDeleteId(null);
    } catch {
      // The mutation displays the error and keeps the confirmation dialog open.
    }
  };

  const getMovementReasonLabel = (reason: string) => {
    const labels: Record<string, string> = {
      manual_restock: "Reposição manual",
      manual_removal: "Retirada manual",
      service_order_add: "Peça adicionada à OS",
      service_order_remove: "Peça removida da OS",
    };
    return (labels[reason] ?? reason) || "Movimentação do sistema";
  };

  const getAbcDescription = (classification: string) => {
    const descriptions: Record<string, string> = {
      A: "até 80% do valor em estoque",
      B: "de 80% a 95% do valor em estoque",
      C: "restante do valor em estoque",
    };
    return descriptions[classification] ?? "valor em estoque";
  };

  const partsError = partsQuery.error;

  if (partsError) {
    return (
      <div className="flex flex-col items-center justify-center h-[50vh] gap-4">
        <AlertTriangle className="h-12 w-12 text-destructive" />
        <h3 className="text-xl font-bold">Erro ao carregar inventário</h3>
        <p className="text-muted-foreground text-center max-w-sm">Não foi possível carregar o inventário. Tente novamente.</p>
        <Button onClick={() => partsQuery.refetch()}>Tentar Novamente</Button>
      </div>
    );
  }

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
                {isSummaryLoading ? "…" : summary?.lowStock ?? 0}
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
                {isSummaryLoading ? "…" : summary?.outOfStock ?? 0}
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
                {isSummaryLoading ? "…" : formatCurrency(summary?.totalStockValue ?? 0)}
              </div>
              <span className="text-xs text-muted-foreground mt-1">custo médio</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div><CardTitle className="text-base">Insights de Estoque</CardTitle><CardDescription>Inatividade e curva ABC pelo valor em estoque: quantidade atual × custo médio.</CardDescription></div>
            <div className="flex items-center gap-2"><Label htmlFor="inactive-days" className="text-xs whitespace-nowrap">Sem movimento há</Label><Input id="inactive-days" inputMode="numeric" className="h-8 w-20" value={inactiveDays} onChange={(event) => setInactiveDays(sanitizeIntegerInput(event.target.value))} onBlur={(event) => setInactiveDays(normalizeIntegerInput(event.target.value))} /><span className="text-xs text-muted-foreground">dias</span></div>
          </div>
        </CardHeader>
        <CardContent>
          {isInsightsLoading ? <div className="h-16 animate-pulse rounded bg-muted" /> : insightsError ? <p className="text-sm text-destructive">Não foi possível carregar os insights de estoque.</p> : insights ? (
            <div className="grid gap-3 md:grid-cols-4">
              <div className="rounded-md border p-3"><p className="text-xs text-muted-foreground">Itens inativos</p><p className="mt-1 text-2xl font-bold">{insights.inactiveItems.length}</p><p className="mt-1 text-xs text-muted-foreground line-clamp-1">{insights.inactiveItems.length ? insights.inactiveItems.map((item) => item.name).join(", ") : "Nenhum item no período."}</p></div>
              {insights.abcGroups.map((group) => <div key={group.classification} className="rounded-md border p-3"><p className="text-xs text-muted-foreground">Classe {group.classification}</p><p className="mt-1 text-2xl font-bold">{group.itemCount} itens</p><p className="mt-1 text-xs text-muted-foreground">{formatCurrency(group.inventoryValue)}</p><p className="mt-1 text-xs text-muted-foreground">{getAbcDescription(group.classification)}</p></div>)}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <div className="space-y-6">
        <Card ref={partsListRef} className="scroll-mt-20">
          <CardHeader>
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div>
                <CardTitle>Estoque de Peças</CardTitle>
                <CardDescription>Peças e componentes físicos cadastrados.</CardDescription>
              </div>
              <div className="relative w-full md:w-72">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Buscar peça ou serviço..."
                  className="pl-9"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Peça / Descrição</TableHead>
                    <TableHead className="text-center">Estoque Atual</TableHead>
                    <TableHead className="hidden md:table-cell text-right">Custo Médio</TableHead>
                    <TableHead className="text-right">Preço de Venda</TableHead>
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {partsQuery.isLoading ? (
                    Array.from({ length: 3 }).map((_, i) => (
                      <TableRow key={i}>
                        <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                        <TableCell><div className="h-5 w-16 bg-muted animate-pulse rounded mx-auto" /></TableCell>
                        <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      </TableRow>
                    ))
                  ) : partsQuery.data && partsQuery.data.items.length > 0 ? (
                    partsQuery.data.items.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell>
                          <div className="flex flex-col gap-1">
                            <span className="font-medium">{item.name}</span>
                            <span className="text-xs text-muted-foreground line-clamp-1">{item.description}</span>
                            {item.supplierName && <span className="text-xs text-muted-foreground">Fornecedor: {item.supplierName}</span>}
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
                          {formatCurrency(item.averageCost || item.costPrice)}
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
                              <DropdownMenuItem onClick={() => { setRestockItem(item); setRestockQuantity("1"); }}>
                              <PackagePlus className="mr-2 h-4 w-4" /> Adicionar ao estoque
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => { setRemoveItem(item); setRemoveQuantity("1"); }} disabled={item.currentQuantity < 1}>
                              <Package className="mr-2 h-4 w-4" /> Retirar do estoque
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => { setHistoryItem(item); setIsHistoryOpen(true); }}>
                                <History className="mr-2 h-4 w-4" /> Histórico
                              </DropdownMenuItem>
                               <DropdownMenuItem onClick={() => handleEditItem(item)}>
                                 <Edit className="mr-2 h-4 w-4" /> Editar
                               </DropdownMenuItem>
                               <DropdownMenuItem onClick={() => handleDuplicateItem(item)}>
                                 <Copy className="mr-2 h-4 w-4" /> Duplicar
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
              <CardFooter className="border-t px-6 py-4">
                <Pagination
                  className="w-full"
                  totalItems={partsTotal}
                  page={partsPage}
                  pageSize={partsPageSize}
                  onPageChange={setPartsPage}
                  onPageSizeChange={setPartsPageSize}
                  pageSizeOptions={PAGE_SIZE_OPTIONS}
                  scrollTargetRef={partsListRef}
                />
              </CardFooter>
            </Card>

        <Card ref={servicesListRef} className="scroll-mt-20">
          <CardHeader>
            <div>
              <CardTitle>Catálogo de Serviços</CardTitle>
              <CardDescription>Mão de obra e serviços recorrentes.</CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Serviço / Mão de obra</TableHead>
                    <TableHead className="hidden md:table-cell text-right">Custo Estimado</TableHead>
                    <TableHead className="text-right">Preço de Venda</TableHead>
                    <TableHead className="text-right w-[100px]">Ações</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {servicesQuery.isLoading ? (
                    Array.from({ length: 2 }).map((_, i) => (
                      <TableRow key={i}>
                        <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                        <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                        <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      </TableRow>
                    ))
                  ) : servicesQuery.data && servicesQuery.data.items.length > 0 ? (
                    servicesQuery.data.items.map((item) => (
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
                               <DropdownMenuItem onClick={() => handleDuplicateItem(item)}>
                                 <Copy className="mr-2 h-4 w-4" /> Duplicar
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
              <CardFooter className="border-t px-6 py-4">
                <Pagination
                  className="w-full"
                  totalItems={servicesTotal}
                  page={servicesPage}
                  pageSize={servicesPageSize}
                  onPageChange={setServicesPage}
                  onPageSizeChange={setServicesPageSize}
                  pageSizeOptions={PAGE_SIZE_OPTIONS}
                  scrollTargetRef={servicesListRef}
                />
              </CardFooter>
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
          <div className="grid gap-4 py-4">
            <Label htmlFor="restock-qty">Quantidade a adicionar</Label>
            <Input
              id="restock-qty"
              className="mt-2"
              value={restockQuantity}
              inputMode="numeric"
              onChange={(event) => setRestockQuantity(sanitizeIntegerInput(event.target.value))}
              onBlur={(event) => setRestockQuantity(normalizeIntegerInput(event.target.value))}
            />
            <div className="grid gap-2"><Label htmlFor="restock-cost">Custo unitário (opcional)</Label><Input id="restock-cost" inputMode="decimal" value={restockUnitCost} placeholder={`Atual: ${formatCurrency(restockItem?.costPrice ?? 0)}`} onChange={(event) => setRestockUnitCost(formatCurrencyInput(event.target.value))} onBlur={(event) => setRestockUnitCost(normalizeCurrencyInput(event.target.value))} /></div>
            <div className="grid gap-2"><Label htmlFor="restock-reason">Motivo (opcional)</Label><Input id="restock-reason" value={restockReason} placeholder="Ex.: Nota fiscal 123" onChange={(event) => setRestockReason(event.target.value)} /></div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRestockItem(null)} disabled={restockMutation.isPending}>Cancelar</Button>
            {restockErrors.quantity && <p className="text-xs text-destructive">{restockErrors.quantity}</p>}
            {restockErrors.unitCost && <p className="text-xs text-destructive">{restockErrors.unitCost}</p>}
            <Button
              onClick={() => {
                const r = quantitySchema.safeParse({ quantity: restockQuantity });
                if (!r.success) { setRestockErrors(parseErrors(r) ?? {}); return; }
                const unitCostValue = currencyInputToNumber(restockUnitCost);
                const unitCost = unitCostValue && unitCostValue > 0 ? unitCostValue : undefined;
                setRestockErrors({});
                if (restockItem) restockMutation.mutate({ id: restockItem.id, quantity: r.data.quantity, unitCost, reason: restockReason.trim() || undefined });
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
            <Label htmlFor="remove-qty">Quantidade a retirar</Label>
            <Input
              id="remove-qty"
              className="mt-2"
              value={removeQuantity}
              inputMode="numeric"
              onChange={(event) => setRemoveQuantity(sanitizeIntegerInput(event.target.value))}
              onBlur={(event) => setRemoveQuantity(normalizeIntegerInput(event.target.value))}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRemoveItem(null)} disabled={removeStockMutation.isPending}>Cancelar</Button>
            {removeErrors.quantity && <p className="text-xs text-destructive">{removeErrors.quantity}</p>}
            <Button
              onClick={() => {
                const r = quantitySchema.safeParse({ quantity: removeQuantity });
                if (!r.success) { setRemoveErrors(parseErrors(r) ?? {}); return; }
                setRemoveErrors({});
                if (removeItem) removeStockMutation.mutate({ id: removeItem.id, quantity: r.data.quantity });
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
                    <TableHead>Motivo</TableHead>
                    <TableHead className="text-center">Quantidade</TableHead>
                     <TableHead className="text-right">Custo un.</TableHead>
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
                        <TableCell className="text-xs text-muted-foreground">{getMovementReasonLabel(mov.reason)}</TableCell>
                        <TableCell className="text-center font-medium">{mov.quantity} un.</TableCell>
                        <TableCell className="text-right text-xs">{mov.unitCost == null ? "-" : formatCurrency(mov.unitCost)}</TableCell>
                        <TableCell className="font-mono text-xs">
                          <Copyable label={mov.osDisplayId ?? ""} />
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

      <InventoryItemSheet
        open={isSheetOpen}
        onOpenChange={(open) => {
          setIsSheetOpen(open);
          if (!open) {
            setSelectedItem(null);
            setDuplicateItem(null);
          }
        }}
        initialType={createType}
        item={selectedItem}
        duplicateItem={duplicateItem}
      />

      {confirmDeleteId && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() => !deleteMutation.isPending && setConfirmDeleteId(null)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Excluir item</h3>
            <p className="text-sm text-muted-foreground">
              Esta ação não pode ser desfeita. Deseja realmente excluir este item?
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmDeleteId(null)}>
                Cancelar
              </Button>
              <Button variant="destructive" onClick={confirmDeleteItem} disabled={deleteMutation.isPending}>
                {deleteMutation.isPending ? "Excluindo..." : "Excluir"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
