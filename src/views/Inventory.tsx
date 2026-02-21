import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
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

// Mock para simular busca de estoque
const fetchInventory = async (): Promise<InventoryItem[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { 
      id: "1", 
      name: "Tela iPhone 13 Pro", 
      description: "Tela original compatível com iPhone 13 Pro", 
      min_quantity: 3, 
      current_quantity: 2, 
      cost_price: 850.00, 
      sale_price: 1450.00 
    },
    { 
      id: "2", 
      name: "Bateria MacBook Air M1", 
      description: "Bateria interna para MacBook Air M1 (A2337)", 
      min_quantity: 2, 
      current_quantity: 5, 
      cost_price: 320.00, 
      sale_price: 580.00 
    },
    { 
      id: "3", 
      name: "SSD 1TB NVMe Kingston", 
      description: "SSD NVMe M.2 2280 NV2 1TB", 
      min_quantity: 5, 
      current_quantity: 8, 
      cost_price: 280.00, 
      sale_price: 450.00 
    },
    { 
      id: "4", 
      name: "Conector Carga USB-C G15", 
      description: "Conector de carga para Dell G15 series", 
      min_quantity: 10, 
      current_quantity: 12, 
      cost_price: 15.00, 
      sale_price: 85.00 
    },
    { 
      id: "5", 
      name: "Pasta Térmica Arctic MX-4", 
      description: "Seringa de 4g de pasta térmica de alta performance", 
      min_quantity: 5, 
      current_quantity: 1, 
      cost_price: 35.00, 
      sale_price: 75.00 
    },
  ];
};

export function Inventory() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [selectedItem, setSelectedItem] = useState<InventoryItem | null>(null);
  
  // Form State
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    min_quantity: 0,
    cost_price: 0,
    sale_price: 0
  });

  const { data: items = [], isLoading } = useQuery({
    queryKey: ["inventory"],
    queryFn: fetchInventory,
  });

  const filteredItems = useMemo(() => {
    return items.filter(item => 
      item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.description.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [items, searchTerm]);

  const handleAddItem = () => {
    setSelectedItem(null);
    setFormData({
      name: "",
      description: "",
      min_quantity: 5,
      cost_price: 0,
      sale_price: 0
    });
    setIsSheetOpen(true);
  };

  const handleEditItem = (item: InventoryItem) => {
    setSelectedItem(item);
    setFormData({
      name: item.name,
      description: item.description,
      min_quantity: item.min_quantity,
      cost_price: item.cost_price,
      sale_price: item.sale_price
    });
    setIsSheetOpen(true);
  };

  const handleSave = () => {
    if (selectedItem) {
      console.log("Ação: Atualizar item no estoque", { id: selectedItem.id, ...formData });
      alert("Item atualizado!");
    } else {
      console.log("Ação: Criar novo item no estoque", formData);
      alert("Item criado e adicionado!");
    }
    setIsSheetOpen(false);
  };

  const handleDeleteItem = (id: string) => {
    if (window.confirm("Deseja realmente excluir este item?")) {
      console.log("Ação: Deletar item do estoque", id);
      alert("Item removido do estoque!");
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
          <h2 className="text-3xl font-bold tracking-tight">Estoque</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie peças, insumos e visualize alertas de estoque mínimo.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" className="gap-2" onClick={() => console.log("Ação: Exportar inventário")}>
            <History className="h-4 w-4" /> Movimentações
          </Button>
          <Button onClick={handleAddItem} className="gap-2">
            <Plus className="h-4 w-4" /> Novo Item
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Itens em Alerta</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-amber-500" />
              <div className="text-2xl font-bold">
                {items.filter(i => i.current_quantity <= i.min_quantity && i.current_quantity > 0).length}
              </div>
              <span className="text-xs text-muted-foreground mt-1">abaixo do mínimo</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Esgotados</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Package className="h-5 w-5 text-destructive" />
              <div className="text-2xl font-bold">
                {items.filter(i => i.current_quantity === 0).length}
              </div>
              <span className="text-xs text-muted-foreground mt-1">itens sem estoque</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Valor Total em Estoque</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-primary" />
              <div className="text-2xl font-bold">
                {formatCurrency(items.reduce((acc, i) => acc + (i.cost_price * i.current_quantity), 0))}
              </div>
              <span className="text-xs text-muted-foreground mt-1">preço de custo</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Inventário de Peças</CardTitle>
              <CardDescription>Peças e componentes cadastrados no sistema.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar item..."
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
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-5 w-16 bg-muted animate-pulse rounded mx-auto" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      <TableCell><div className="h-5 w-24 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : filteredItems.length > 0 ? (
                  filteredItems.map((item) => (
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
                            item.current_quantity === 0 ? "destructive" :
                            item.current_quantity <= item.min_quantity ? "default" : "secondary"
                          }>
                            {item.current_quantity} un.
                          </Badge>
                          <span className="text-[10px] text-muted-foreground">Mín: {item.min_quantity}</span>
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-right font-medium text-muted-foreground">
                        {formatCurrency(item.cost_price)}
                      </TableCell>
                      <TableCell className="text-right font-bold text-primary">
                        {formatCurrency(item.sale_price)}
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
                      Nenhum item encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {/* Sheet para Cadastro/Edição de Item */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>{selectedItem ? "Editar Item" : "Novo Item no Estoque"}</SheetTitle>
            <SheetDescription>
              Cadastre peças e insumos para gerenciar seu estoque e adicionar em ordens de serviço.
            </SheetDescription>
          </SheetHeader>

          <div className="grid gap-4 py-6">
            <div className="grid gap-2">
              <Label htmlFor="name">Nome da Peça / Produto</Label>
              <Input 
                id="name" 
                value={formData.name}
                placeholder="Ex: Tela iPhone 11"
                onChange={(e) => setFormData({...formData, name: e.target.value})}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="description">Descrição</Label>
              <Textarea 
                id="description" 
                value={formData.description}
                placeholder="Ex: Peça original importada..."
                onChange={(e) => setFormData({...formData, description: e.target.value})}
              />
            </div>
            
            <Separator />

            <div className="grid grid-cols-2 gap-4">
              <div className="grid gap-2">
                <Label htmlFor="min">Qtd. Mínima (Alerta)</Label>
                <div className="relative">
                  <Box className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input 
                    id="min" 
                    type="number"
                    className="pl-9"
                    value={formData.min_quantity}
                    onChange={(e) => setFormData({...formData, min_quantity: parseInt(e.target.value)})}
                  />
                </div>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="cost">Preço de Custo</Label>
                <div className="relative">
                  <DollarSign className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input 
                    id="cost" 
                    type="number"
                    step="0.01"
                    className="pl-9"
                    value={formData.cost_price}
                    onChange={(e) => setFormData({...formData, cost_price: parseFloat(e.target.value)})}
                  />
                </div>
              </div>
            </div>

            <div className="grid gap-2">
              <Label htmlFor="sale">Preço de Venda sugerido</Label>
              <div className="relative">
                <TrendingUp className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-primary" />
                <Input 
                  id="sale" 
                  type="number"
                  step="0.01"
                  className="pl-9"
                  value={formData.sale_price}
                  onChange={(e) => setFormData({...formData, sale_price: parseFloat(e.target.value)})}
                />
              </div>
            </div>
          </div>

          <SheetFooter>
            <Button variant="outline" className="w-full" onClick={() => setIsSheetOpen(false)}>
              Cancelar
            </Button>
            <Button className="w-full gap-2" onClick={handleSave}>
              <Save className="h-4 w-4" /> {selectedItem ? "Salvar Alterações" : "Cadastrar Peça"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
