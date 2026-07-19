import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
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
  Copy,
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
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow
} from "@/components/ui/table";
import { toastSuccess, toastError, copyToClipboard } from "@/lib/errors";
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
import { formatCurrency, formatBRPhone } from "@/lib/formatters";
import { customerSchema, parseErrors, clearFieldError, ValidationErrors } from "@/lib/validation";
import { useSort } from "@/hooks/useSort";
import { SortableHeader } from "@/components/shared/SortableHeader";
import { ServiceOrderDetailSheet } from "@/components/shared/ServiceOrderDetailSheet";

const fetchCustomers = async (): Promise<Customer[]> => {
  return await invoke<Customer[]>("get_customers");
};

const fetchCustomerOrders = async (customerId: string): Promise<ServiceOrder[]> => {
  return await invoke<ServiceOrder[]>("get_service_orders_by_customer_id", { customerId });
};

const initialFormData = {
  name: "",
  phone: "",
  email: "",
  address: "",
};

export function Customers() {
  const [searchTerm, setSearchTerm] = useState("");
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [isInternational, setIsInternational] = useState(false);
  const [selectedCustomerId, setSelectedCustomerId] = useState<string | null>(null);
  const [formData, setFormData] = useState(initialFormData);
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [isHistorySheetOpen, setIsHistorySheetOpen] = useState(false);
  const [viewingCustomer, setViewingCustomer] = useState<Customer | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [selectedOrderId, setSelectedOrderId] = useState<string | null>(null);
  const { sortConfig, cycleSort } = useSort();

  const { data: customers = [], isLoading, error, refetch } = useQuery({
    queryKey: ["customers"],
    queryFn: fetchCustomers,
  });

  const queryClient = useQueryClient();

  const createCustomerMutation = useMutation({
    mutationFn: async (data: { name: string; phone: string; email: string; address: string }) => {
      return await invoke("create_customer", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      setIsSheetOpen(false);
      setFormData(initialFormData);
      toastSuccess("Cliente criado com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao criar cliente."),
  });

  const updateCustomerMutation = useMutation({
    mutationFn: async (data: { id: string; name: string; phone: string; email: string; address: string }) => {
      return await invoke("update_customer", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      setIsSheetOpen(false);
      setFormData(initialFormData);
      toastSuccess("Cliente atualizado com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao atualizar cliente."),
  });

  const deleteCustomerMutation = useMutation({
    mutationFn: async (id: string) => {
      return await invoke("delete_customer", { id });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      toastSuccess("Cliente excluído com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao excluir cliente."),
  });

  const { data: customerOrders = [], isLoading: isLoadingOrders } = useQuery({
    queryKey: ["customer-orders", viewingCustomer?.id],
    queryFn: () => fetchCustomerOrders(viewingCustomer!.id),
    enabled: !!viewingCustomer,
  });

  const sortedOrders = useMemo(() => {
    return [...customerOrders].sort((a, b) =>
      new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    );
  }, [customerOrders]);

  const getCustomerSortValue = (customer: Customer, column: string): string | number => {
    switch (column) {
      case "name": return customer.name;
      case "phone": return customer.phone;
      case "email": return customer.email;
      case "address": return customer.address;
      case "createdAt": return customer.createdAt || "";
      default: return "";
    }
  };

  const filteredCustomers = useMemo(() => {
    let result = customers.filter(customer =>
      customer.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.email.toLowerCase().includes(searchTerm.toLowerCase()) ||
      customer.phone.includes(searchTerm)
    );

    if (sortConfig.direction && sortConfig.column) {
      const dir = sortConfig.direction;
      const col = sortConfig.column;
      result = [...result].sort((a, b) => {
        const valA = getCustomerSortValue(a, col);
        const valB = getCustomerSortValue(b, col);
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
  }, [customers, searchTerm, sortConfig]);

  const handleAddCustomer = () => {
    setIsEditing(false);
    setSelectedCustomerId(null);
    setErrors({});
    setFormData(initialFormData);
    setIsInternational(false);
    setIsSheetOpen(true);
  };

  const handleEditCustomer = (customer: Customer) => {
    setIsEditing(true);
    setSelectedCustomerId(customer.id);
    setErrors({});
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

  const updateField = (field: string, value: string) => {
    setFormData({ ...formData, [field]: value });
    setErrors(clearFieldError(errors, field));
  };

  const handleSaveCustomer = () => {
    if (createCustomerMutation.isPending || updateCustomerMutation.isPending) return;
    const result = customerSchema.safeParse(formData);
    const fieldErrors = parseErrors(result);
    if (fieldErrors) {
      setErrors(fieldErrors);
      return;
    }
    setErrors({});

    const payload = {
      ...formData,
      phone: formData.phone.replace(/\D/g, ""),
    };

    if (isEditing && selectedCustomerId) {
      updateCustomerMutation.mutate({ id: selectedCustomerId, ...payload });
    } else {
      createCustomerMutation.mutate(payload);
    }
  };

  const handleDeleteCustomer = (id: string) => {
    setConfirmDeleteId(id);
  };

  const confirmDeleteCustomer = () => {
    if (confirmDeleteId && !deleteCustomerMutation.isPending) {
      deleteCustomerMutation.mutate(confirmDeleteId);
      setConfirmDeleteId(null);
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

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-[50vh] gap-4">
        <History className="h-12 w-12 text-destructive" />
        <h3 className="text-xl font-bold">Erro ao carregar clientes</h3>
        <p className="text-muted-foreground text-center max-w-sm">Não foi possível carregar os clientes. Tente novamente.</p>
        <Button onClick={() => refetch()}>Tentar Novamente</Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200">
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
            <div className="max-h-[500px] overflow-y-auto rounded-md border" style={{ contentVisibility: 'auto' as const, containIntrinsicSize: '500px' }}>
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortableHeader column="name" label="Cliente" sortConfig={sortConfig} onSort={cycleSort} />
                    <SortableHeader column="phone" label="Contato" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell" />
                    <SortableHeader column="address" label="Endereço" sortConfig={sortConfig} onSort={cycleSort} className="hidden lg:table-cell" />
                    <SortableHeader column="createdAt" label="Cadastrado em" sortConfig={sortConfig} onSort={cycleSort} className="hidden md:table-cell" />
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
                        {customer.createdAt ? new Date(customer.createdAt).toLocaleDateString('pt-BR') : '-'}
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
                onChange={(e) => updateField("name", e.target.value)}
              />
              {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
            </div>

            <div className="grid gap-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="phone" className="flex items-center gap-2">
                  <Phone className="h-3.5 w-3.5" /> Telefone / WhatsApp
                </Label>
                <div className="flex items-center gap-2">
                  <Checkbox
                    checked={isInternational}
                    onChange={(e) => setIsInternational(e.target.checked)}
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
              {errors.phone && <p className="text-xs text-destructive">{errors.phone}</p>}
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
                onChange={(e) => updateField("email", e.target.value)}
              />
              {errors.email && <p className="text-xs text-destructive">{errors.email}</p>}
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
                onChange={(e) => updateField("address", e.target.value)}
              />
              {errors.address && <p className="text-xs text-destructive">{errors.address}</p>}
            </div>
          </div>

          <SheetFooter className="mt-6 flex-col sm:flex-row gap-2">
            <Button variant="outline" onClick={() => setIsSheetOpen(false)} className="w-full sm:w-auto">
              Cancelar
            </Button>
            <Button onClick={handleSaveCustomer} disabled={createCustomerMutation.isPending || updateCustomerMutation.isPending} className="w-full sm:w-auto gap-2">
              <Save className="h-4 w-4" />
              {createCustomerMutation.isPending || updateCustomerMutation.isPending ? "Salvando..." : isEditing ? "Salvar Alterações" : "Cadastrar Cliente"}
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
                          <span className="text-sm font-bold text-primary">{os.displayId || os.id.slice(0, 8)}</span>
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
                            <p className="text-sm">{new Date(os.createdAt).toLocaleDateString('pt-BR')}</p>
                          </div>
                          <div className="space-y-1 col-span-2">
                            <p className="text-[10px] uppercase font-bold text-muted-foreground tracking-wider">Descrição</p>
                            <p className="text-xs text-muted-foreground line-clamp-2">{os.description}</p>
                          </div>
                        </div>
                      </CardContent>
                      <div className="px-4 py-2 bg-primary/5 flex items-center justify-between border-t">
                        <span className="text-sm font-bold">{formatCurrency(os.totalPrice || 0)}</span>
                        <Button variant="ghost" size="sm" className="h-8 text-xs gap-1.5" onClick={async () => {
                          const text = os.displayId || os.id.slice(0, 8);
                          const ok = await copyToClipboard(text);
                          if (ok) toastSuccess(`ID ${text} copiado.`);
                          else toastError("Erro ao copiar ID.");
                        }}>
                          Copiar ID<Copy className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="sm" className="h-8 text-xs" onClick={() => {
                          setIsHistorySheetOpen(false);
                          window.setTimeout(() => setSelectedOrderId(os.id), 0);
                        }}>
                          Ver detalhes
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

      <AlertDialog open={!!confirmDeleteId} onOpenChange={() => setConfirmDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Excluir cliente</AlertDialogTitle>
            <AlertDialogDescription>
              Esta ação não pode ser desfeita. Deseja realmente excluir este cliente?
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancelar</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDeleteCustomer} disabled={deleteCustomerMutation.isPending}>{deleteCustomerMutation.isPending ? "Excluindo..." : "Excluir"}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <ServiceOrderDetailSheet orderId={selectedOrderId} open={!!selectedOrderId} onClose={() => setSelectedOrderId(null)} />
    </div>
  );
}
