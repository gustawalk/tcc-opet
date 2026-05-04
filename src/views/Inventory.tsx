import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { 
  Plus, 
  Search, 
  MoreVertical, 
  Package, 
  TrendingUp, 
  AlertTriangle,
  Edit,
  Trash2,
  History,
  Save,
  DollarSign,
  Box
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
import { InventoryItem } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { formatCurrency } from "@/lib/formatters";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { Separator } from "@/components/ui/separator";

const fetchInventory = async (): Promise<InventoryItem[]> => {
  return await invoke<InventoryItem[]>("get_inventory_items");
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
  
  // Form State
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    type: "part" as "part" | "service",
    minQuantity: 0,
    costPrice: 0,
    salePrice: 0
  });

  const { data: items = [], isLoading } = useQuery({
    queryKey: ["inventory"],
    queryFn: fetchInventory,
  });

  const queryClient = useQueryClient();

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
    },
  });

  const parts = useMemo(() => items.filter(i => i.type === "part"), [items]);
  const services = useMemo(() => items.filter(i => i.type === "service"), [items]);

  const filteredParts = useMemo(() => {
    return parts.filter(item => 
      item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.description.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [parts, searchTerm]);

  const filteredServices = useMemo(() => {
    return services.filter(item => 
      item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.description.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [services, searchTerm]);

  const handleAddItem = (type: "part" | "service" = "part") => {
    setSelectedItem(null);
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

  const handleDeleteItem = async (id: string) => {
    if (window.confirm("Deseja realmente excluir este item?")) {
      await deleteMutation.mutateAsync(id);
    }
  };

  const handleMovement = (item: InventoryItem) => {
    console.log("Ação: Ver histórico de movimentação de", item.name);
    alert(`Histórico de movimentações para: ${item.name} (Simulado)`);
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Estoque & Serviços</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie peças, insumos e serviços de mão de obra.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" className="gap-2" onClick={() => console.log("Ação: Exportar inventário")}>
            <History className="h-4 w-4" /> Movimentações
          </Button>
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
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Peça / Descrição</TableHead>
                    <TableHead className="text-center">Estoque Atual</TableHead>
                    <TableHead className="hidden md:table-cell text-right">Preço de Custo</TableHead>
                    <TableHead className="text-right">Preço de Venda</TableHead>
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
                              <DropdownMenuItem onClick={() => handleMovement(item)}>
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
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Serviço / Mão de Obra</TableHead>
                    <TableHead className="hidden md:table-cell text-right">Custo Estimado</TableHead>
                    <TableHead className="text-right">Preço de Venda</TableHead>
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
                onChange={(e) => setFormData({...formData, name: e.target.value})}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="description">Descrição</Label>
              <Textarea 
                id="description" 
                value={formData.description}
                placeholder="Ex: Detalhes adicionais..."
                onChange={(e) => setFormData({...formData, description: e.target.value})}
              />
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
                    onChange={(e) => setFormData({...formData, minQuantity: parseInt(e.target.value)})}
                  />
                </div>
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
                    onChange={(e) => setFormData({...formData, costPrice: parseFloat(e.target.value)})}
                  />
                </div>
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
                    onChange={(e) => setFormData({...formData, salePrice: parseFloat(e.target.value)})}
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
    </div>
  );
}
