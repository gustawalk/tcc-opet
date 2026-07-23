import { useMemo, useState } from "react";
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
import { Card, CardContent } from "@/components/ui/card";
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
import { SortableHeader } from "@/components/shared/SortableHeader";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { useSort } from "@/hooks/useSort";
import { formatCurrency } from "@/lib/formatters";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  Customer,
  OSStatus,
  ServiceOrder,
  User as UserType,
} from "@/lib/types";

const fetchOrders = () => invoke<ServiceOrder[]>("get_service_orders");
const fetchUsers = () => invoke<UserType[]>("get_users");
const fetchCustomers = () => invoke<Customer[]>("get_customers");
const EMPTY_ORDERS: ServiceOrder[] = [];

export function ServiceOrders() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { openServiceOrder } = useServiceOrderDrawer();
  const { sortConfig, cycleSort } = useSort();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [userFilter, setUserFilter] = useState<string | null>(null);
  const [customerFilter, setCustomerFilter] = useState<string | null>(null);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const ordersQuery = useQuery({
    queryKey: ["service-orders"],
    queryFn: fetchOrders,
  });
  const usersQuery = useQuery({ queryKey: ["users"], queryFn: fetchUsers });
  const customersQuery = useQuery({
    queryKey: ["customers"],
    queryFn: fetchCustomers,
  });
  const orders = ordersQuery.data ?? EMPTY_ORDERS;
  const users = usersQuery.data ?? [];
  const customers = customersQuery.data ?? [];
  const filteredOrders = useMemo(() => {
    const result = orders.filter(
      (order) =>
        (order.displayId.toLowerCase().includes(searchTerm.toLowerCase()) ||
          order.customerName
            ?.toLowerCase()
            .includes(searchTerm.toLowerCase()) ||
          order.equipment.toLowerCase().includes(searchTerm.toLowerCase())) &&
        (statusFilter === "all" || order.status === statusFilter) &&
        (!userFilter || order.userId === userFilter) &&
        (!customerFilter || order.customerId === customerFilter),
    );
    if (!sortConfig.column || !sortConfig.direction) return result;
    return [...result].sort((a, b) => {
      const av = a[sortConfig.column as keyof ServiceOrder] ?? "";
      const bv = b[sortConfig.column as keyof ServiceOrder] ?? "";
      return sortConfig.direction === "asc"
        ? String(av).localeCompare(String(bv), "pt-BR")
        : String(bv).localeCompare(String(av), "pt-BR");
    });
  }, [
    orders,
    searchTerm,
    statusFilter,
    sortConfig,
    userFilter,
    customerFilter,
  ]);
  const deleteOrder = async () => {
    if (!deleteId || isDeleting) return;
    setIsDeleting(true);
    try {
      await invoke("delete_service_order", { id: deleteId });
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["service-orders"] }),
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
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">
            Ordens de Serviço
          </h2>
          <p className="text-muted-foreground mt-1">
            Gerencie atendimentos, ordens de serviço e orçamentos.
          </p>
        </div>
        <Button className="gap-2" onClick={() => navigate("/os/new")}>
          <Plus className="h-4 w-4" />
          Nova OS
        </Button>
      </div>
      <Tabs defaultValue="all" onValueChange={setStatusFilter}>
        <div className="flex flex-col md:flex-row justify-between gap-4">
          <TabsList>
            <TabsTrigger value="all">Todas</TabsTrigger>
            <TabsTrigger value="Orçamento">Orçamentos</TabsTrigger>
            <TabsTrigger value="Em Manutenção">Em Manutenção</TabsTrigger>
            <TabsTrigger value="Aguardando Peça">Pendentes</TabsTrigger>
            <TabsTrigger value="Finalizada">Finalizadas</TabsTrigger>
            <TabsTrigger value="Cancelada">Canceladas</TabsTrigger>
          </TabsList>
          <div className="flex gap-2 items-center w-full md:w-auto">
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
            <div className="relative w-full md:w-72">
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
      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <SortableHeader
                  column="displayId"
                  label="ID"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                />
                <SortableHeader
                  column="customerName"
                  label="Cliente & Equipamento"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                />
                <SortableHeader
                  column="status"
                  label="Status"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                  className="hidden md:table-cell"
                />
                <SortableHeader
                  column="createdAt"
                  label="Abertura"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                  className="hidden lg:table-cell"
                />
                <SortableHeader
                  column="totalPrice"
                  label="Valor"
                  sortConfig={sortConfig}
                  onSort={cycleSort}
                  align="right"
                />
                <TableHead className="text-right">Ações</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {ordersQuery.isLoading ? (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="h-24 text-center text-muted-foreground"
                  >
                    Carregando ordens de serviço...
                  </TableCell>
                </TableRow>
              ) : filteredOrders.length ? (
                filteredOrders.map((order) => (
                  <TableRow
                    key={order.id}
                    className="cursor-pointer"
                    onClick={() => openServiceOrder(order.id)}
                  >
                    <TableCell className="font-mono text-xs font-bold">
                      {order.displayId}
                    </TableCell>
                    <TableCell>
                      <p className="flex gap-1 text-sm font-medium">
                        <UserIcon className="h-3 w-3" />
                        {order.customerName}
                      </p>
                      <p className="flex gap-1 text-xs text-muted-foreground">
                        <Smartphone className="h-3 w-3" />
                        {order.equipment}
                      </p>
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
                        {order.discountPercent > 0 && (
                          <span className="text-xs text-muted-foreground line-through">
                            {formatCurrency(order.totalPrice || 0)}
                          </span>
                        )}
                        <span>
                          {formatCurrency(
                            (order.totalPrice || 0) *
                              (1 - order.discountPercent / 100),
                          )}
                        </span>
                        {order.discountPercent > 0 && (
                          <Badge variant="outline" className="text-[10px]">
                            -{order.discountPercent}%
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
