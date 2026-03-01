import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  UserPlus,
  Search,
  MoreVertical,
  Phone,
  Mail,
  MapPin,
  FileText,
  Edit,
  Trash2,
  Save,
  User as UserIcon,
  Globe,
  History,
  Copy
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
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
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
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { Customer, ServiceOrder } from "@/lib/types";
import { formatCurrency } from "@/lib/formatters";

// Mock para simular busca de clientes
const fetchCustomers = async (): Promise<Customer[]> => {
  await new Promise(resolve => setTimeout(resolve, 800));
  return [
    { id: "1", name: "Maria Silva", phone: "(41) 99999-1111", email: "maria@email.com", address: "Rua das Flores, 123 - Curitiba", created_at: "2023-01-15" },
    { id: "2", name: "João Pereira", phone: "(41) 98888-2222", email: "joao@email.com", address: "Av. Principal, 500 - Araucária", created_at: "2023-02-20" },
    { id: "3", name: "Empresa ABC", phone: "(41) 3333-4444", email: "contato@abc.com", address: "Rua Industrial, 10 - Curitiba", created_at: "2023-03-10" },
    { id: "4", name: "Carlos Oliveira", phone: "(41) 97777-3333", email: "carlos@email.com", address: "Rua das Palmeiras, 45 - São José dos Pinhais", created_at: "2023-04-05" },
    { id: "5", name: "Ana Santos", phone: "(41) 96666-4444", email: "ana@email.com", address: "Rua XV de Novembro, 1000 - Curitiba", created_at: "2023-05-12" },
  ];
};

// Mock para simular busca de OS por cliente
const fetchCustomerOrders = async (customerId: string): Promise<ServiceOrder[]> => {
  await new Promise(resolve => setTimeout(resolve, 500));
  const mockOrders: Record<string, ServiceOrder[]> = {
    "1": [
      { id: "OS-1001", customer_id: "1", equipment: "iPhone 13", status: "Finalizada", created_at: "2023-11-01", total_price: 450.00, description: "Troca de tela" },
      { id: "OS-1025", customer_id: "1", equipment: "MacBook Pro", status: "Em Manutenção", created_at: "2024-02-10", total_price: 1200.00, description: "Limpeza interna e pasta térmica" },
      { id: "OS-1040", customer_id: "1", equipment: "iPad Air 4", status: "Orçamento", created_at: "2024-03-01", total_price: 350.00, description: "Bateria estufada" },
      { id: "OS-1055", customer_id: "1", equipment: "Apple Watch S7", status: "Aguardando Peça", created_at: "2024-03-15", total_price: 600.00, description: "Vidro quebrado" },
    ],
    "2": [
      { id: "OS-1005", customer_id: "2", equipment: "Samsung S22", status: "Aguardando Peça", created_at: "2023-12-15", total_price: 850.00, description: "Troca de conector" },
    ]
  };
  return mockOrders[customerId] || [];
};

const initialFormData = {
  name: "",
  phone: "",
  email: "",
  address: "",
};

// Utilitário para formatar telefone BR
const formatBRPhone = (value: string) => {
  const digits = value.replace(/\D/g, "");
  if (digits.length <= 2) return digits;
  if (digits.length <= 7) return `(${digits.slice(0, 2)}) ${digits.slice(2)}`;
  return `(${digits.slice(0, 2)}) ${digits.slice(2, 7)}-${digits.slice(7, 11)}`;
};

export function Customers() {
  const [searchTerm, setSearchTerm] = useState("");

  // Estados para o Sheet de Cadastro/Edição
  const [isSheetOpen, setIsSheetOpen] = useState<boolean>(false);
  const [isEditing, setIsEditing] = useState<boolean>(false);
  const [isInternational, setIsInternational] = useState<boolean>(false);
  const [selectedCustomerId, setSelectedCustomerId] = useState<string | null>(null);
  const [formData, setFormData] = useState(initialFormData);

  // Estados para o Sheet de Histórico de OS
  const [isHistorySheetOpen, setIsHistorySheetOpen] = useState(false);
  const [viewingCustomer, setViewingCustomer] = useState<Customer | null>(null);

  const { data: customers = [], isLoading } = useQuery({
    queryKey: ["customers"],
    queryFn: fetchCustomers,
  });

  const { data: customerOrders = [], isLoading: isLoadingOrders } = useQuery({
    queryKey: ["customer-orders", viewingCustomer?.id],
    queryFn: () => fetchCustomerOrders(viewingCustomer!.id),
    enabled: !!viewingCustomer,
  });

  // Ordenação: Mais recentes primeiro
  const sortedOrders = useMemo(() => {
    return [...customerOrders].sort((a, b) =>
      new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
  }, [customerOrders]);

  const filteredCustomers = useMemo(() => {
    return customers.filter(customer =>
      customer.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.email.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.phone.includes(searchTerm)
    );
  }, [customers, searchTerm]);

  const handleAddCustomer = () => {
    setIsEditing(false);
    setSelectedCustomerId(null);
    setFormData(initialFormData);
    setIsInternational(false);
    setIsSheetOpen(true);
  };

  const handleEditCustomer = (customer: Customer) => {
    setIsEditing(true);
    setSelectedCustomerId(customer.id);

    // Tenta detectar se é internacional (se não segue o padrão BR de dígitos)
    const onlyDigits = customer.phone.replace(/\D/g, "");
    const isIntl = onlyDigits.length > 11 || (onlyDigits.length > 0 && onlyDigits.length < 10);

    setIsInternational(isIntl);
    setFormData({
      name: customer.name,
      phone: customer.phone,
      email: customer.email,
      address: customer.address,
    });
    setIsSheetOpen(true);
  };

  const handlePhoneChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (isInternational) {
      setFormData({ ...formData, phone: value });
    } else {
      setFormData({ ...formData, phone: formatBRPhone(value) });
    }
  };

  const handleSaveCustomer = () => {
    // Payload contém apenas números no telefone
    const phoneDigitsOnly = formData.phone.replace(/\D/g, "");

    const payload = {
      ...(isEditing ? { id: selectedCustomerId } : {}),
      ...formData,
      phone: phoneDigitsOnly,
      is_international: isInternational, // Metadado útil
      updated_at: new Date().toISOString(),
      ...(isEditing ? {} : { created_at: new Date().toISOString() })
    };

    console.log(isEditing ? "Ação: Atualizar cliente" : "Ação: Criar novo cliente", payload);

    setIsSheetOpen(false);
    setFormData(initialFormData);
  };

  const handleDeleteCustomer = (id: string) => {
    const customer = customers.find(c => c.id === id);
    if (confirm(`Deseja realmente excluir o cliente ${customer?.name}?`)) {
      console.log("Ação: Deletar cliente (Soft Delete)", { id, deleted_at: new Date().toISOString() });
    }
  };

  const handleViewOS = (customer: Customer) => {
    setViewingCustomer(customer);
    setIsHistorySheetOpen(true);
  };

  const getStatusBadge = (status: string) => {
    const variants: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
      "Finalizada": "secondary",
      "Em Manutenção": "default",
      "Aguardando Peça": "destructive",
      "Orçamento": "outline",
      "Cancelada": "outline"
    };
    return <Badge variant={variants[status] || "outline"}>{status}</Badge>;
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Clientes</h2>
          <p className="text-muted-foreground mt-1">
            Gerencie o cadastro de clientes da sua assistência.
          </p>
        </div>
        <Button onClick={handleAddCustomer} className="gap-2">
          <UserPlus className="h-4 w-4" /> Novo Cliente
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Base de Clientes</CardTitle>
              <CardDescription>Consulte e gerencie as informações de contato dos seus clientes.</CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar cliente..."
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
                  <TableHead>Cliente</TableHead>
                  <TableHead className="hidden md:table-cell">Contato</TableHead>
                  <TableHead className="hidden lg:table-cell">Endereço</TableHead>
                  <TableHead className="hidden md:table-cell">Cadastrado em</TableHead>
                  <TableHead className="text-right w-[100px]">Ações</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell><div className="h-5 w-32 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-40 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden lg:table-cell"><div className="h-5 w-48 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell className="hidden md:table-cell"><div className="h-5 w-24 bg-muted animate-pulse rounded" /></TableCell>
                      <TableCell><div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" /></TableCell>
                    </TableRow>
                  ))
                ) : filteredCustomers.length > 0 ? (
                  filteredCustomers.map((customer) => (
                    <TableRow key={customer.id}>
                      <TableCell className="font-medium">
                        <div className="flex flex-col">
                          {customer.name}
                          <span className="text-xs text-muted-foreground md:hidden">{customer.phone}</span>
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="flex flex-col gap-1">
                          <div className="flex items-center gap-1.5 text-xs">
                            <Phone className="h-3 w-3 text-muted-foreground" />
                            {customer.phone}
                          </div>
                          <div className="flex items-center gap-1.5 text-xs">
                            <Mail className="h-3 w-3 text-muted-foreground" />
                            {customer.email}
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="hidden lg:table-cell max-w-[250px] truncate text-xs">
                        <div className="flex items-center gap-1.5">
                          <MapPin className="h-3 w-3 text-muted-foreground shrink-0" />
                          {customer.address}
                        </div>
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs">
                        {customer.created_at ? new Date(customer.created_at).toLocaleDateString('pt-BR') : '-'}
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
                            <DropdownMenuItem onClick={() => handleViewOS(customer)}>
                              <FileText className="mr-2 h-4 w-4" /> Ver OS
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleEditCustomer(customer)}>
                              <Edit className="mr-2 h-4 w-4" /> Editar
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              className="text-destructive focus:text-destructive"
                              onClick={() => handleDeleteCustomer(customer.id)}
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
                      Nenhum cliente encontrado.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {/* Sheet para Adicionar/Editar Cliente */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent className="sm:max-w-md">
          <SheetHeader>
            <SheetTitle className="flex items-center gap-2">
              {isEditing ? <Edit className="h-5 w-5" /> : <UserPlus className="h-5 w-5" />}
              {isEditing ? "Editar Cliente" : "Novo Cliente"}
            </SheetTitle>
            <SheetDescription>
              {isEditing
                ? "Altere as informações do cliente selecionado."
                : "Preencha os dados abaixo para cadastrar um novo cliente."}
            </SheetDescription>
          </SheetHeader>

          <div className="grid gap-4 py-6">
            <div className="grid gap-2">
              <Label htmlFor="name" className="flex items-center gap-2">
                <UserIcon className="h-3.5 w-3.5" /> Nome Completo
              </Label>
              <Input
                id="name"
                placeholder="Ex: Maria Silva"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              />
            </div>

            <div className="grid gap-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="phone" className="flex items-center gap-2">
                  <Phone className="h-3.5 w-3.5" /> Telefone / WhatsApp
                </Label>
                <div className="flex items-center gap-2">
                  <Checkbox
                    checked={isInternational}
                    onChange={(check) => { setIsInternational(check.target.checked) }}
                  />
                  <Label htmlFor="intl" className="text-xs font-normal cursor-pointer flex items-center gap-1">
                    <Globe className="h-3 w-3" /> Internacional
                  </Label>
                </div>
              </div>
              <Input
                id="phone"
                placeholder={isInternational ? "Ex: +1 555-0123" : "(00) 00000-0000"}
                value={formData.phone}
                onChange={handlePhoneChange}
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="email" className="flex items-center gap-2">
                <Mail className="h-3.5 w-3.5" /> E-mail
              </Label>
              <Input
                id="email"
                type="email"
                placeholder="cliente@email.com"
                value={formData.email}
                onChange={(e) => setFormData({ ...formData, email: e.target.value })}
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="address" className="flex items-center gap-2">
                <MapPin className="h-3.5 w-3.5" /> Endereço
              </Label>
              <Textarea
                id="address"
                placeholder="Rua, número, bairro, cidade..."
                className="min-h-[100px]"
                value={formData.address}
                onChange={(e) => setFormData({ ...formData, address: e.target.value })}
              />
            </div>
          </div>

          <SheetFooter className="mt-6 flex-col sm:flex-row gap-2">
            <Button variant="outline" onClick={() => setIsSheetOpen(false)} className="w-full sm:w-auto">
              Cancelar
            </Button>
            <Button onClick={handleSaveCustomer} className="w-full sm:w-auto gap-2">
              <Save className="h-4 w-4" />
              {isEditing ? "Salvar Alterações" : "Cadastrar Cliente"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* Sheet para Histórico de OS */}
      <Sheet open={isHistorySheetOpen} onOpenChange={setIsHistorySheetOpen}>
        <SheetContent className="sm:max-w-2xl overflow-hidden flex flex-col">
          <SheetHeader>
            <SheetTitle className="flex items-center gap-2">
              <History className="h-5 w-5 text-primary" />
              Histórico de Ordens de Serviço
            </SheetTitle>
            <SheetDescription>
              Mostrando serviços realizados para <strong>{viewingCustomer?.name}</strong>.
            </SheetDescription>
          </SheetHeader>

          <div className="flex-1 overflow-hidden mt-6">
            <ScrollArea className="h-full">
              {isLoadingOrders ? (
                <div className="space-y-4">
                  {[1, 2, 3].map(i => (
                    <div key={i} className="h-24 w-full bg-muted animate-pulse rounded-lg" />
                  ))}
                </div>
              ) : sortedOrders.length > 0 ? (
                <div className="space-y-4 pr-4">
                  {sortedOrders.map((os) => (
                    <Card key={os.id} className="overflow-hidden border-primary/10 hover:border-primary/30 transition-colors">
                      <CardHeader className="p-4 pb-2 bg-muted/30">
                        <div className="flex items-center justify-between">
                          <span className="text-sm font-bold text-primary">{os.id}</span>
                          {getStatusBadge(os.status)}
                        </div>
                      </CardHeader>
                      <CardContent className="p-4 pt-2">
                        <div className="grid grid-cols-2 gap-4">
                          <div className="space-y-1">
                            <p className="text-[10px] uppercase font-bold text-muted-foreground tracking-wider">Equipamento</p>
                            <p className="text-sm font-medium">{os.equipment}</p>
                          </div>
                          <div className="space-y-1 text-right">
                            <p className="text-[10px] uppercase font-bold text-muted-foreground tracking-wider">Abertura</p>
                            <p className="text-sm">{new Date(os.created_at).toLocaleDateString('pt-BR')}</p>
                          </div>
                          <div className="space-y-1 col-span-2">
                            <p className="text-[10px] uppercase font-bold text-muted-foreground tracking-wider">Descrição</p>
                            <p className="text-xs text-muted-foreground line-clamp-2">{os.description}</p>
                          </div>
                        </div>
                      </CardContent>
                      <div className="px-4 py-2 bg-primary/5 flex items-center justify-between border-t">
                        <span className="text-sm font-bold">{formatCurrency(os.total_price || 0)}</span>
                        <Button variant="ghost" size="sm" className="h-8 text-xs gap-1.5" onClick={() => alert(`ID copiado: ${os.id}`)}>
                          Copiar ID da ordem<Copy className="h-3 w-3" />
                        </Button>
                      </div>
                    </Card>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center py-12 text-center">
                  <FileText className="h-12 w-12 text-muted-foreground/30 mb-4" />
                  <p className="text-muted-foreground">Este cliente ainda não possui ordens de serviço.</p>
                </div>
              )}
            </ScrollArea>
          </div>

          <SheetFooter className="mt-6 pt-6 border-t">
            <Button variant="outline" onClick={() => setIsHistorySheetOpen(false)} className="w-full">
              Fechar Histórico
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
