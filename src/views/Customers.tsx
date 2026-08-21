import { useEffect, useRef, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { dataCommand } from "@/lib/data-client";
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
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toastSuccess, toastError } from "@/lib/errors";
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
import { Customer, Page } from "@/lib/types";
import { formatBRPhone } from "@/lib/formatters";
import {
  customerSchema,
  parseErrors,
  clearFieldError,
  ValidationErrors,
} from "@/lib/validation";
import { Pagination } from "@/components/shared/Pagination";
import { useDebounce } from "@/hooks/use-debounce";
import { useCustomerDrawer } from "@/components/shared/CustomerDrawerProvider";

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const SEARCH_DEBOUNCE_MS = 300;

const fetchCustomersPage = (args: {
  limit: number;
  offset: number;
  search: string;
}): Promise<Page<Customer>> => {
  return dataCommand<Page<Customer>>("get_customers_page", args);
};

const initialFormData = {
  name: "",
  phone: "",
  email: "",
  address: "",
};

export function Customers() {
  const { openCustomerHistory } = useCustomerDrawer();
  const listRef = useRef<HTMLDivElement>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const search = useDebounce(searchTerm, SEARCH_DEBOUNCE_MS);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [isSheetOpen, setIsSheetOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [isInternational, setIsInternational] = useState(false);
  const [selectedCustomerId, setSelectedCustomerId] = useState<string | null>(
    null,
  );
  const [formData, setFormData] = useState(initialFormData);
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  const queryClient = useQueryClient();

  const {
    data,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["customersPage", page, pageSize, search],
    queryFn: () =>
      fetchCustomersPage({
        limit: pageSize,
        offset: (page - 1) * pageSize,
        search,
      }),
    placeholderData: (previousData) => previousData,
  });

  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  useEffect(() => {
    setPage(1);
  }, [search, pageSize]);

  useEffect(() => {
    if (data && page > totalPages) setPage(totalPages);
  }, [data, totalPages, page]);

  const handlePageSizeChange = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };

  const createCustomerMutation = useMutation({
    mutationFn: async (data: {
      name: string;
      phone: string;
      email: string;
      address: string;
    }) => {
      return await dataCommand("create_customer", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customersPage"] });
      setIsSheetOpen(false);
      setFormData(initialFormData);
      toastSuccess("Cliente criado com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao criar cliente."),
  });

  const updateCustomerMutation = useMutation({
    mutationFn: async (data: {
      id: string;
      name: string;
      phone: string;
      email: string;
      address: string;
    }) => {
      return await dataCommand("update_customer", data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customersPage"] });
      setIsSheetOpen(false);
      setFormData(initialFormData);
      toastSuccess("Cliente atualizado com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao atualizar cliente."),
  });

  const deleteCustomerMutation = useMutation({
    mutationFn: async (id: string) => {
      return await dataCommand("delete_customer", { id });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["customersPage"] });
      toastSuccess("Cliente excluído com sucesso.");
    },
    onError: (err) => toastError(err, "Erro ao excluir cliente."),
  });

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
    const isIntl =
      onlyDigits.length > 11 ||
      (onlyDigits.length > 0 && onlyDigits.length < 10);
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
    if (createCustomerMutation.isPending || updateCustomerMutation.isPending)
      return;
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
    openCustomerHistory(customer.id);
  };

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-[50vh] gap-4">
        <History className="h-12 w-12 text-destructive" />
        <h3 className="text-xl font-bold">Erro ao carregar clientes</h3>
        <p className="text-muted-foreground text-center max-w-sm">
          Não foi possível carregar os clientes. Tente novamente.
        </p>
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

      <Card ref={listRef} className="scroll-mt-20">
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Base de Clientes</CardTitle>
              <CardDescription>
                Consulte e gerencie as informações de contato dos seus clientes.
              </CardDescription>
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
                  <TableHead className="hidden md:table-cell">
                    Contato
                  </TableHead>
                  <TableHead className="hidden lg:table-cell">
                    Endereço
                  </TableHead>
                  <TableHead className="hidden md:table-cell">
                    Cadastrado em
                  </TableHead>
                  <TableHead className="text-right w-[100px]">Ações</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell>
                        <div className="h-5 w-32 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="h-5 w-40 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="hidden lg:table-cell">
                        <div className="h-5 w-48 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="h-5 w-24 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell>
                        <div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" />
                      </TableCell>
                    </TableRow>
                  ))
                ) : data && data.items.length > 0 ? (
                  data.items.map((customer) => (
                    <TableRow key={customer.id}>
                      <TableCell className="font-medium">
                        <div className="flex flex-col">
                          {customer.name}
                          <span className="text-xs text-muted-foreground md:hidden">
                            {customer.phone}
                          </span>
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
                        {customer.createdAt
                          ? new Date(customer.createdAt).toLocaleDateString(
                              "pt-BR",
                            )
                          : "-"}
                      </TableCell>
                      <TableCell className="text-right">
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                            >
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuLabel>Ações</DropdownMenuLabel>
                            <DropdownMenuItem
                              onClick={() => handleViewOS(customer)}
                            >
                              <FileText className="mr-2 h-4 w-4" /> Ver ordens
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => handleEditCustomer(customer)}
                            >
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
                    <TableCell
                      colSpan={5}
                      className="h-24 text-center text-muted-foreground"
                    >
                      Nenhum cliente encontrado.
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
            totalItems={total}
            page={page}
            pageSize={pageSize}
            onPageChange={setPage}
            onPageSizeChange={handlePageSizeChange}
            pageSizeOptions={PAGE_SIZE_OPTIONS}
            scrollTargetRef={listRef}
          />
        </CardFooter>
      </Card>

      {/* Sheet para Adicionar/Editar Cliente */}
      <Sheet open={isSheetOpen} onOpenChange={setIsSheetOpen}>
        <SheetContent className="sm:max-w-md">
          <SheetHeader>
            <SheetTitle className="flex items-center gap-2">
              {isEditing ? (
                <Edit className="h-5 w-5" />
              ) : (
                <UserPlus className="h-5 w-5" />
              )}
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
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name}</p>
              )}
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
                  <Label
                    htmlFor="intl"
                    className="text-xs font-normal cursor-pointer flex items-center gap-1"
                  >
                    <Globe className="h-3 w-3" /> Internacional
                  </Label>
                </div>
              </div>
              <Input
                id="phone"
                placeholder={
                  isInternational ? "Ex: +1 555-0123" : "(00) 00000-0000"
                }
                value={formData.phone}
                onChange={handlePhoneChange}
              />
              {errors.phone && (
                <p className="text-xs text-destructive">{errors.phone}</p>
              )}
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
              {errors.email && (
                <p className="text-xs text-destructive">{errors.email}</p>
              )}
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
              {errors.address && (
                <p className="text-xs text-destructive">{errors.address}</p>
              )}
            </div>
          </div>

          <SheetFooter className="mt-6 flex-col sm:flex-row gap-2">
            <Button
              variant="outline"
              onClick={() => setIsSheetOpen(false)}
              className="w-full sm:w-auto"
            >
              Cancelar
            </Button>
            <Button
              onClick={handleSaveCustomer}
              disabled={
                createCustomerMutation.isPending ||
                updateCustomerMutation.isPending
              }
              className="w-full sm:w-auto gap-2"
            >
              <Save className="h-4 w-4" />
              {createCustomerMutation.isPending ||
              updateCustomerMutation.isPending
                ? "Salvando..."
                : isEditing
                  ? "Salvar Alterações"
                  : "Cadastrar Cliente"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {confirmDeleteId && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 pointer-events-auto"
          onClick={() =>
            !deleteCustomerMutation.isPending && setConfirmDeleteId(null)
          }
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4 pointer-events-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-semibold">Excluir cliente</h3>
            <p className="text-sm text-muted-foreground">
              Esta ação não pode ser desfeita. Deseja realmente excluir este
              cliente?
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setConfirmDeleteId(null)}
              >
                Cancelar
              </Button>
              <Button
                variant="destructive"
                onClick={confirmDeleteCustomer}
                disabled={deleteCustomerMutation.isPending}
              >
                {deleteCustomerMutation.isPending ? "Excluindo..." : "Excluir"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
