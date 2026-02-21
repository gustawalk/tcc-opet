import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
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
  Wrench,
  Save,
  Trash,
  ChevronRight
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
import { ServiceOrder, OSStatus, InventoryItem } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { formatCurrency } from "@/lib/formatters";
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

// Mock para simular busca de itens de uma OS
const fetchOSItems = async (osId: string): Promise<any[]> => {
  console.log(`Buscando itens da OS: ${osId}`);
  await new Promise(resolve => setTimeout(resolve, 300));
  return [
    { id: "1", name: "Tela LCD iPhone 13", quantity: 1, unit_price: 850.00 },
    { id: "2", name: "Mão de Obra", quantity: 1, unit_price: 150.00 },
  ];
};

// Mock para simular busca de estoque (para seleção)
const fetchInventory = async (): Promise<InventoryItem[]> => {
  return [
    { id: "1", name: "Tela iPhone 13 Pro", description: "...", min_quantity: 3, current_quantity: 2, cost_price: 850, sale_price: 1450 },
    { id: "2", name: "Bateria MacBook Air M1", description: "...", min_quantity: 2, current_quantity: 5, cost_price: 320, sale_price: 580 },
    { id: "3", name: "SSD 1TB NVMe Kingston", description: "...", min_quantity: 5, current_quantity: 8, cost_price: 280, sale_price: 450 },
    { id: "4", name: "Conector Carga USB-C G15", description: "...", min_quantity: 10, current_quantity: 12, cost_price: 15, sale_price: 85 },
    { id: "5", name: "Pasta Térmica Arctic MX-4", description: "...", min_quantity: 5, current_quantity: 1, cost_price: 35, sale_price: 75 },
    { id: "serv-1", name: "Mão de Obra Técnica", description: "Serviço", min_quantity: 0, current_quantity: 999, cost_price: 0, sale_price: 150 },
    { id: "serv-2", name: "Limpeza Preventiva", description: "Serviço", min_quantity: 0, current_quantity: 999, cost_price: 0, sale_price: 100 },
  ];
};

// Mock para simular busca de ordens de serviço
const fetchServiceOrders = async (): Promise<ServiceOrder[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { 
      id: "OS-2023-001", 
      customer_id: "1", 
      customer_name: "Maria Silva", 
      equipment: "Notebook Dell G15", 
      description: "Troca de tela e limpeza preventiva", 
      status: "Em Manutenção", 
      created_at: "2023-10-15",
      total_price: 1000.00
    },
    { 
      id: "OS-2023-002", 
      customer_id: "2", 
      customer_name: "João Pereira", 
      equipment: "iPhone 13 Pro", 
      description: "Troca de bateria", 
      status: "Aguardando Peça", 
      created_at: "2023-10-16",
      total_price: 1200.00
    },
    { 
      id: "OS-2023-003", 
      customer_id: "3", 
      customer_name: "Empresa ABC", 
      equipment: "Impressora HP LaserJet", 
      description: "Limpeza de roletes e troca de toner", 
      status: "Finalizada", 
      created_at: "2023-10-17",
      total_price: 320.00
    },
    { 
      id: "OS-2023-004", 
      customer_id: "4", 
      customer_name: "Carlos Oliveira", 
      equipment: "PlayStation 5", 
      description: "Superaquecimento - Limpeza e troca de metal líquido", 
      status: "Orçamento", 
      created_at: "2023-10-18",
      total_price: 0.00
    },
    { 
      id: "OS-2023-005", 
      customer_id: "5", 
      customer_name: "Ana Santos", 
      equipment: "Samsung Galaxy S22", 
      description: "Reparo no conector de carga", 
      status: "Cancelada", 
      created_at: "2023-10-19",
      total_price: 0.00
    },
  ];
};

export function ServiceOrders() {
  const navigate = useNavigate();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  
  // Estados para Controle de Sheet
  const [selectedOS, setSelectedOS] = useState<ServiceOrder | null>(null);
  const [osItems, setOsItems] = useState<any[]>([]);
  const [isDetailOpen, setIsDetailOpen] = useState(false);
  const [isEditOpen, setIsEditOpen] = useState(false);
  
  // Estados de Edição Temporários
  const [editStatus, setEditStatus] = useState<OSStatus>("Orçamento");
  const [editDescription, setEditDescription] = useState("");

  // Estado para busca de itens no estoque
  const [inventorySearch, setInventorySearch] = useState("");
  const [showInventoryResults, setShowInventoryResults] = useState(false);

  const { data: orders = [], isLoading } = useQuery({
    queryKey: ["service-orders"],
    queryFn: fetchServiceOrders,
  });

  const { data: inventory = [] } = useQuery({
    queryKey: ["inventory-lookup"],
    queryFn: fetchInventory,
  });

  const filteredOrders = useMemo(() => {
    return orders.filter(order => {
      const matchesSearch = 
        order.id.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (order.customer_name && order.customer_name.toLowerCase().includes(searchTerm.toLowerCase())) ||
        order.equipment.toLowerCase().includes(searchTerm.toLowerCase());
      
      const matchesStatus = statusFilter === "all" || order.status === statusFilter;
      
      return matchesSearch && matchesStatus;
    });
  }, [orders, searchTerm, statusFilter]);

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

  const handleEditOS = async (order: ServiceOrder) => {
    console.log("Abrindo Edição para OS:", order.id);
    setSelectedOS(order);
    setEditStatus(order.status);
    setEditDescription(order.description);
    const items = await fetchOSItems(order.id);
    setOsItems(items);
    setIsEditOpen(true);
  };

  const handleViewOS = async (order: ServiceOrder) => {
    console.log("Abrindo Visualização para OS:", order.id);
    setSelectedOS(order);
    const items = await fetchOSItems(order.id);
    setOsItems(items);
    setIsDetailOpen(true);
  };

  const handleDeleteOS = (id: string) => {
    const confirmDelete = window.confirm(`Deseja realmente excluir a OS ${id}? Esta ação não pode ser desfeita.`);
    if (confirmDelete) {
      console.log("Ação: Deletar OS", id);
      alert("Ordem de serviço excluída (Simulado)");
    }
  };

  const handleSaveEdit = () => {
    if (!selectedOS) return;
    const updatedOS = {
      ...selectedOS,
      status: editStatus,
      description: editDescription
    };
    console.log("Salvando alterações da OS:", updatedOS);
    alert("Alterações salvas com sucesso!");
    setIsEditOpen(false);
  };

  const handleAddInventoryItem = (item: InventoryItem) => {
    console.log("Adicionando item do estoque à OS:", item.name);
    const newItem = {
      id: Math.random().toString(), 
      product_id: item.id,
      name: item.name,
      quantity: 1,
      unit_price: item.sale_price
    };
    setOsItems([...osItems, newItem]);
    setInventorySearch("");
    setShowInventoryResults(false);
  };

  const handleRemoveItem = (itemId: string) => {
    console.log("Removendo item da OS:", itemId);
    setOsItems(osItems.filter(i => i.id !== itemId));
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
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[120px]">ID</TableHead>
                  <TableHead>Cliente & Equipamento</TableHead>
                  <TableHead className="hidden md:table-cell">Status</TableHead>
                  <TableHead className="hidden lg:table-cell">Abertura</TableHead>
                  <TableHead className="text-right">Valor</TableHead>
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
                        {order.id}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          <div className="flex items-center gap-1.5 font-medium">
                            <User className="h-3 w-3 text-muted-foreground" />
                            {order.customer_name}
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
                          {new Date(order.created_at).toLocaleDateString('pt-BR')}
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-medium">
                        {order.total_price ? formatCurrency(order.total_price) : formatCurrency(0)}
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
          </CardContent>
        </Card>
      </div>

      {/* SHEET DE DETALHES (VIEW) */}
      <Sheet open={isDetailOpen} onOpenChange={setIsDetailOpen}>
        <SheetContent className="sm:max-w-xl overflow-y-auto">
          <SheetHeader>
            <div className="flex justify-between items-start">
              <div className="space-y-1">
                <SheetTitle className="text-2xl font-bold flex items-center gap-2">
                  <Wrench className="h-6 w-6 text-primary" /> {selectedOS?.id}
                </SheetTitle>
                <SheetDescription>
                  Detalhamento completo da ordem de serviço.
                </SheetDescription>
              </div>
              {selectedOS && getStatusBadge(selectedOS.status)}
            </div>
          </SheetHeader>

          <div className="mt-8 space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Cliente</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <User className="h-4 w-4" /> {selectedOS?.customer_name}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Equipamento</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Smartphone className="h-4 w-4" /> {selectedOS?.equipment}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Abertura</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Calendar className="h-4 w-4" /> {selectedOS && new Date(selectedOS.created_at).toLocaleDateString('pt-BR')}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Total Previsto</p>
                <p className="text-sm font-bold text-primary">
                  {formatCurrency(selectedOS?.total_price || 0)}
                </p>
              </div>
            </div>

            <Separator />

            <div className="space-y-2">
              <p className="text-xs text-muted-foreground font-semibold uppercase">Relato / Problema</p>
              <div className="p-3 bg-muted/50 rounded-md border text-sm">
                {selectedOS?.description}
              </div>
            </div>

            <Separator />

            <div className="space-y-4">
              <p className="text-xs text-muted-foreground font-semibold uppercase">Peças & Mão de Obra</p>
              <div className="rounded-md border overflow-hidden">
                <Table>
                  <TableHeader className="bg-muted/50">
                    <TableRow>
                      <TableHead className="h-8 text-[10px]">Item</TableHead>
                      <TableHead className="h-8 text-center text-[10px]">Qtd</TableHead>
                      <TableHead className="h-8 text-right text-[10px]">Valor</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {osItems.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell className="py-2 text-xs font-medium">{item.name}</TableCell>
                        <TableCell className="py-2 text-center text-xs">{item.quantity}</TableCell>
                        <TableCell className="py-2 text-right text-xs">{formatCurrency(item.unit_price)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </div>
          </div>

          <SheetFooter className="mt-8">
            <Button variant="outline" className="w-full gap-2" onClick={() => console.log("Gerando PDF...")}>
              Gerar PDF da OS
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* SHEET DE EDIÇÃO (EDIT) */}
      <Sheet open={isEditOpen} onOpenChange={setIsEditOpen}>
        <SheetContent className="sm:max-w-xl overflow-y-auto">
          <SheetHeader>
            <SheetTitle>Editar OS {selectedOS?.id}</SheetTitle>
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
                onChange={(e) => setEditDescription(e.target.value)}
                className="min-h-[100px] text-sm"
              />
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
                              <span className="text-[10px] text-muted-foreground">Estoque: {item.current_quantity} un.</span>
                            </div>
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-bold text-primary">{formatCurrency(item.sale_price)}</span>
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
                        <TableCell className="py-2 text-xs font-medium">{item.name}</TableCell>
                        <TableCell className="py-2 text-right text-xs font-bold">{formatCurrency(item.unit_price)}</TableCell>
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
              <p className="text-right text-sm font-bold">
                Total: {formatCurrency(osItems.reduce((acc, i) => acc + i.unit_price, 0))}
              </p>
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
