import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { FlaskConical, Search } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Pagination } from "@/components/shared/Pagination";
import { useDebounce } from "@/hooks/use-debounce";
import type { OSStatus, Page, ServiceOrder } from "@/lib/types";
import { formatCurrency, formatDate } from "@/lib/formatters";

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const SEARCH_DEBOUNCE_MS = 300;

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

const fetchServiceOrdersPage = (args: {
  limit: number;
  offset: number;
  search: string;
}): Promise<Page<ServiceOrder>> => {
  return invoke<Page<ServiceOrder>>("get_service_orders_page", args);
};

export function PaginationPreview() {
  const listRef = useRef<HTMLDivElement>(null);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [searchTerm, setSearchTerm] = useState("");
  const search = useDebounce(searchTerm, SEARCH_DEBOUNCE_MS);

  useEffect(() => {
    setPage(1);
  }, [search, pageSize]);

  const { data, isLoading } = useQuery({
    queryKey: ["serviceOrdersPage", page, pageSize, search],
    queryFn: () =>
      fetchServiceOrdersPage({
        limit: pageSize,
        offset: (page - 1) * pageSize,
        search,
      }),
    placeholderData: (previousData) => previousData,
  });

  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  useEffect(() => {
    if (data && page > totalPages) setPage(totalPages);
  }, [data, totalPages, page]);

  const handlePageSizeChange = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setPage(1);
  };

  return (
    <div className="flex flex-col gap-6 animate-in fade-in duration-200 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="text-3xl font-bold tracking-tight">Paginação</h2>
            <Badge variant="secondary" className="gap-1.5">
              <FlaskConical className="h-3 w-3" /> Preview
            </Badge>
          </div>
          <p className="text-muted-foreground mt-1">
            Demo consumindo <code>get_service_orders_page</code> (LIMIT/OFFSET) com busca por
            substring (LIKE) no backend e debounce de {SEARCH_DEBOUNCE_MS}ms.
          </p>
        </div>
      </div>

      <Card ref={listRef} className="scroll-mt-20">
        <CardHeader>
          <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div>
              <CardTitle>Ordens de Serviço</CardTitle>
              <CardDescription>
                Lista paginada no backend com o contrato <code>{"{ items, total }"}</code>.
              </CardDescription>
            </div>
            <div className="relative w-full md:w-72">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Buscar por cliente, equipamento ou descrição..."
                className="pl-9"
                value={searchTerm}
                onChange={(event) => setSearchTerm(event.target.value)}
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableCell className="text-xs font-medium text-muted-foreground w-[110px]">
                    OS
                  </TableCell>
                  <TableCell className="text-xs font-medium text-muted-foreground">
                    Cliente
                  </TableCell>
                  <TableCell className="text-xs font-medium text-muted-foreground">
                    Equipamento
                  </TableCell>
                  <TableCell className="text-xs font-medium text-muted-foreground hidden md:table-cell">
                    Status
                  </TableCell>
                  <TableCell className="text-xs font-medium text-muted-foreground hidden md:table-cell">
                    Data
                  </TableCell>
                  <TableCell className="text-xs font-medium text-muted-foreground text-right">
                    Valor
                  </TableCell>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  Array.from({ length: 5 }).map((_, i) => (
                    <TableRow key={i}>
                      <TableCell>
                        <div className="h-4 w-16 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell>
                        <div className="h-4 w-40 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell>
                        <div className="h-4 w-32 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="h-5 w-24 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                        <div className="h-4 w-24 bg-muted animate-pulse rounded" />
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="h-4 w-20 bg-muted animate-pulse rounded ml-auto" />
                      </TableCell>
                    </TableRow>
                  ))
                ) : data && data.items.length > 0 ? (
                  data.items.map((order) => (
                    <TableRow key={order.id}>
                      <TableCell className="text-xs font-medium">{order.displayId}</TableCell>
                      <TableCell>{order.customerName || "—"}</TableCell>
                      <TableCell className="text-sm">{order.equipment}</TableCell>
                      <TableCell className="hidden md:table-cell">
                        {statusBadge(order.status)}
                      </TableCell>
                      <TableCell className="hidden md:table-cell text-xs text-muted-foreground">
                        {formatDate(order.createdAt)}
                      </TableCell>
                      <TableCell className="text-right text-sm font-medium">
                        {order.totalPrice != null ? formatCurrency(order.totalPrice) : "—"}
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

      <p className="text-xs text-muted-foreground">
        A busca é enviada ao backend a cada digitação, com debounce de {SEARCH_DEBOUNCE_MS}ms —
        a contagem e a página atual são recalculadas no servidor.
      </p>
    </div>
  );
}
