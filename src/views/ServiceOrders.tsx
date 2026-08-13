import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import {
  Calendar,
  Edit,
  Eye,
  MoreVertical,
  Plus,
  Search,
  Smartphone,
  Trash2,
  User as UserIcon,
  X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SearchableSelect } from "@/components/shared/SearchableSelect";
import { Pagination } from "@/components/shared/Pagination";
import { useDebounce } from "@/hooks/use-debounce";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { useCustomerDrawer } from "@/components/shared/CustomerDrawerProvider";
import { applyDiscount, formatCurrency } from "@/lib/formatters";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  Customer,
  OSStatus,
  Page,
  ServiceOrder,
  User as UserType,
} from "@/lib/types";

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const SEARCH_DEBOUNCE_MS = 300;

const fetchOrdersPage = (args: {
  limit: number;
  offset: number;
  search: string;
  status?: string;
  userId?: string;
  customerId?: string;
}): Promise<Page<ServiceOrder>> => {
  return invoke<Page<ServiceOrder>>("get_service_orders_page", args);
};
const fetchUsers = () => invoke<UserType[]>("get_users");
const fetchCustomers = () => invoke<Customer[]>("get_customers");

export function ServiceOrders() {
  const listRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { openServiceOrder } = useServiceOrderDrawer();
  const { openCustomerHistory } = useCustomerDrawer();
  const [searchTerm, setSearchTerm] = useState("");
  const search = useDebounce(searchTerm, SEARCH_DEBOUNCE_MS);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [statusFilter, setStatusFilter] = useState("all");
  const [userFilter, setUserFilter] = useState<string | null>(null);
  const [customerFilter, setCustomerFilter] = useState<string | null>(null);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const ordersQuery = useQuery({
    queryKey: [
      "serviceOrdersPage",
      page,
      pageSize,
      search,
      statusFilter,
      userFilter,
      customerFilter,
    ],
    queryFn: () =>
      fetchOrdersPage({
        limit: pageSize,
        offset: (page - 1) * pageSize,
        search,
        status: statusFilter === "all" ? undefined : statusFilter,
        userId: userFilter ?? undefined,
        customerId: customerFilter ?? undefined,
      }),
    placeholderData: (previousData) => previousData,
  });
  const usersQuery = useQuery({ queryKey: ["users"], queryFn: fetchUsers });
  const customersQuery = useQuery({
    queryKey: ["customers"],
    queryFn: fetchCustomers,
  });
  const users = usersQuery.data ?? [];
  const customers = customersQuery.data ?? [];
  const total = ordersQuery.data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  useEffect(() => {
    setPage(1);
  }, [search, pageSize, statusFilter, userFilter, customerFilter]);

  useEffect(() => {
    if (ordersQuery.data && page > totalPages) setPage(totalPages);
  }, [ordersQuery.data, totalPages, page]);

  const handlePageSizeChange = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };

  const deleteOrder = async () => {
    if (!deleteId || isDeleting) return;
    setIsDeleting(true);
    try {
      await invoke("delete_service_order", { id: deleteId });
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["serviceOrdersPage"] }),
        queryClient.invalidateQueries({ queryKey: ["dashboard-data"] }),
      ]);
      toastSuccess("Ordem de serviço excluída.");
    } catch (error) {
      toastError(error, "Erro ao excluir ordem de serviço.");
    } finally {
      setDeleteId(null);
      setIsDeleting(false);
    }
  };
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

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight sm:text-3xl">
            Ordens
          </h2>
          <p className="text-muted-foreground mt-1">
            Acompanhe atendimentos, ordens e orçamentos.
          </p>
        </div>
        <Button className="w-full gap-2 sm:w-auto" onClick={() => navigate("/os/new")}>
          <Plus className="h-4 w-4" />
          Nova Ordem
        </Button>
      </div>
      <Tabs defaultValue="all" onValueChange={setStatusFilter}>
        <div className="flex flex-col gap-4 xl:flex-row xl:justify-between">
          <div className="-mx-1 overflow-x-auto px-1 pb-1">
          <TabsList className="w-max">
            <TabsTrigger value="all">Todas</TabsTrigger>
            <TabsTrigger value="Orçamento">Orçamentos</TabsTrigger>
            <TabsTrigger value="Em Manutenção">Em Manutenção</TabsTrigger>
            <TabsTrigger value="Aguardando Peça">Aguardando peça</TabsTrigger>
            <TabsTrigger value="Finalizada">Finalizadas</TabsTrigger>
            <TabsTrigger value="Cancelada">Canceladas</TabsTrigger>
          </TabsList>
          </div>
          <div className="grid w-full gap-2 sm:grid-cols-2 xl:flex xl:w-auto xl:items-center">
            <div className="flex items-center gap-1">
              <SearchableSelect
                options={customers}
                value={customerFilter}
                onSelect={(customer) =>
                  setCustomerFilter(
                    customerFilter === customer.id ? null : customer.id,
                  )
                }
                placeholder="Clientes"
                searchPlaceholder="Buscar cliente..."
                getKey={(customer) => customer.id}
                getLabel={(customer) => customer.name}
                className="w-full md:w-44"
              />
              {customerFilter && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 shrink-0"
                  title="Limpar filtro de cliente"
                  onClick={() => setCustomerFilter(null)}
                >
                  <X className="h-3 w-3" />
                </Button>
              )}
            </div>
            <div className="flex items-center gap-1">
              <SearchableSelect
                options={users}
                value={userFilter}
                onSelect={(user) =>
                  setUserFilter(userFilter === user.id ? null : user.id)
                }
                placeholder="Funcionários"
                searchPlaceholder="Buscar funcionário..."
                getKey={(user) => user.id}
                getLabel={(user) => user.name}
                className="w-full md:w-44"
              />
              {userFilter && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 shrink-0"
                  title="Limpar filtro de funcionário"
                  onClick={() => setUserFilter(null)}
                >
                  <X className="h-3 w-3" />
                </Button>
              )}
            </div>
            <div className="relative w-full sm:col-span-2 xl:w-72">
              <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Buscar por ID, Cliente ou Equipamento..."
                value={searchTerm}
                onChange={(event) => setSearchTerm(event.target.value)}
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
      <Card ref={listRef} className="scroll-mt-20">
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Nº da ordem</TableHead>
                <TableHead>Cliente & Equipamento</TableHead>
                <TableHead className="hidden md:table-cell">Status</TableHead>
                <TableHead className="hidden lg:table-cell">Abertura</TableHead>
                <TableHead className="text-right">Valor</TableHead>
                <TableHead className="text-right">Ações</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {ordersQuery.isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell>
                      <div className="h-4 w-16 bg-muted animate-pulse rounded" />
                    </TableCell>
                    <TableCell>
                      <div className="h-4 w-40 bg-muted animate-pulse rounded" />
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                      <div className="h-5 w-24 bg-muted animate-pulse rounded" />
                    </TableCell>
                    <TableCell className="hidden lg:table-cell">
                      <div className="h-4 w-24 bg-muted animate-pulse rounded" />
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="h-4 w-20 bg-muted animate-pulse rounded ml-auto" />
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="h-8 w-8 bg-muted animate-pulse rounded ml-auto" />
                    </TableCell>
                  </TableRow>
                ))
              ) : ordersQuery.data && ordersQuery.data.items.length ? (
                ordersQuery.data.items.map((order) => (
                  <TableRow
                    key={order.id}
                    className="cursor-pointer"
                    onClick={() => openServiceOrder(order.id)}
                  >
                    <TableCell className="font-mono text-xs font-bold">
                      {order.displayId}
                    </TableCell>
                      <TableCell>
                      <div>
                        <span className="flex gap-1 text-sm font-medium">
                          <UserIcon className="h-3 w-3" />
                          <button
                            type="button"
                            className="rounded-sm hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            onClick={(event) => {
                              event.stopPropagation();
                              openCustomerHistory(order.customerId);
                            }}
                          >
                            {order.customerName}
                          </button>
                        </span>
                        <span className="flex gap-1 text-xs text-muted-foreground">
                          <Smartphone className="h-3 w-3" />
                          {order.equipment}
                        </span>
                        <span className="mt-2 flex flex-wrap items-center gap-2 md:hidden">
                          {statusBadge(order.status)}
                          <span className="flex items-center gap-1 text-xs text-muted-foreground">
                            <Calendar className="h-3 w-3" />
                            {new Date(order.createdAt).toLocaleDateString("pt-BR")}
                          </span>
                        </span>
                      </div>
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
                        {order.discountBasisPoints > 0 && (
                          <span className="text-xs text-muted-foreground line-through">
                            {formatCurrency(order.totalPrice || 0)}
                          </span>
                        )}
                        <span>
                          {formatCurrency(
                            applyDiscount(
                              order.totalPrice || 0,
                              order.discountBasisPoints,
                            ),
                          )}
                        </span>
                        {order.discountBasisPoints > 0 && (
                          <Badge variant="outline" className="text-[10px]">
                            -{order.discountBasisPoints / 100}%
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell
                      className="text-right"
                      onClick={(event) => event.stopPropagation()}
                    >
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuLabel>Ações</DropdownMenuLabel>
                          <DropdownMenuItem
                            onClick={() => openServiceOrder(order.id)}
                          >
                            <Eye className="mr-2 h-4 w-4" />
                            Visualizar
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => openServiceOrder(order.id, "edit")}
                          >
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
      {deleteId && (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
          onClick={() => !isDeleting && setDeleteId(null)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md space-y-4"
            onClick={(event) => event.stopPropagation()}
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
    </div>
  );
}
