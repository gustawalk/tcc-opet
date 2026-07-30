import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Copy, FileText, History, Mail, MapPin, Phone } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
} from "@/components/ui/card";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useServiceOrderDrawer } from "@/components/shared/ServiceOrderDrawerProvider";
import { copyToClipboard, toastError, toastSuccess } from "@/lib/errors";
import { formatCurrency } from "@/lib/formatters";
import { Customer, ServiceOrder } from "@/lib/types";

interface CustomerHistorySheetProps {
  customerId: string | null;
  open: boolean;
  onClose: () => void;
}

const fetchCustomer = (id: string) => invoke<Customer | null>("get_customer", { id });
const fetchCustomerOrders = (customerId: string) =>
  invoke<ServiceOrder[]>("get_service_orders_by_customer_id", { customerId });

export function CustomerHistorySheet({
  customerId,
  open,
  onClose,
}: CustomerHistorySheetProps) {
  const { openServiceOrder } = useServiceOrderDrawer();
  const customerQuery = useQuery({
    queryKey: ["customer", customerId],
    queryFn: () => fetchCustomer(customerId!),
    enabled: open && !!customerId,
  });
  const ordersQuery = useQuery({
    queryKey: ["customer-orders", customerId],
    queryFn: () => fetchCustomerOrders(customerId!),
    enabled: open && !!customerId,
  });
  const orders = useMemo(
    () =>
      [...(ordersQuery.data ?? [])].sort(
        (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
      ),
    [ordersQuery.data],
  );
  const customer = customerQuery.data;

  const statusBadge = (status: ServiceOrder["status"]) => {
    const variants: Record<
      ServiceOrder["status"],
      "default" | "secondary" | "destructive" | "outline"
    > = {
      Finalizada: "secondary",
      "Em Manutenção": "default",
      "Aguardando Peça": "destructive",
      Orçamento: "outline",
      Cancelada: "outline",
    };
    return <Badge variant={variants[status]}>{status}</Badge>;
  };

  return (
    <Sheet open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
      <SheetContent className="flex h-full flex-col sm:max-w-lg">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <History className="h-5 w-5 text-primary" />
            Histórico de Ordens de Serviço
          </SheetTitle>
          <SheetDescription>
            {customer ? (
              <>Serviços realizados para <strong>{customer.name}</strong>.</>
            ) : (
              "Carregando informações do cliente..."
            )}
          </SheetDescription>
        </SheetHeader>

        {customer && (
          <div className="mt-4 grid gap-2 rounded-md border bg-muted/30 p-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-2"><Phone className="h-3.5 w-3.5" />{customer.phone}</span>
            <span className="flex items-center gap-2"><Mail className="h-3.5 w-3.5" />{customer.email}</span>
            <span className="flex items-center gap-2"><MapPin className="h-3.5 w-3.5" />{customer.address}</span>
          </div>
        )}
        {customerQuery.isError && (
          <p className="mt-4 text-sm text-destructive">Não foi possível carregar o cliente.</p>
        )}

        <div className="mt-6 flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            {ordersQuery.isLoading ? (
              <div className="space-y-4">
                {[1, 2, 3].map((index) => (
                  <div key={index} className="h-24 w-full animate-pulse rounded-lg bg-muted" />
                ))}
              </div>
            ) : ordersQuery.isError ? (
              <p className="text-sm text-destructive">Não foi possível carregar o histórico.</p>
            ) : orders.length ? (
              <div className="space-y-4 pr-4">
                {orders.map((order) => (
                  <Card
                    key={order.id}
                    className="overflow-hidden border-primary/10 transition-colors hover:border-primary/30"
                  >
                    <CardHeader className="bg-muted/30 p-4 pb-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-bold text-primary">
                          {order.displayId || order.id.slice(0, 8)}
                        </span>
                        {statusBadge(order.status)}
                      </div>
                    </CardHeader>
                    <CardContent className="p-4 pt-2">
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-1">
                          <p className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Equipamento</p>
                          <p className="text-sm font-medium">{order.equipment}</p>
                        </div>
                        <div className="space-y-1 text-right">
                          <p className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Abertura</p>
                          <p className="text-sm">{new Date(order.createdAt).toLocaleDateString("pt-BR")}</p>
                        </div>
                        <div className="col-span-2 space-y-1">
                          <p className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Descrição</p>
                          <p className="line-clamp-2 text-xs text-muted-foreground">{order.description}</p>
                        </div>
                      </div>
                    </CardContent>
                    <div className="flex items-center justify-between border-t bg-primary/5 px-4 py-2">
                      <span className="text-sm font-bold">{formatCurrency(order.totalPrice || 0)}</span>
                      <div className="flex items-center gap-1">
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-8 gap-1.5 text-xs"
                          onClick={async () => {
                            const text = order.displayId || order.id.slice(0, 8);
                            if (await copyToClipboard(text)) toastSuccess(`ID ${text} copiado.`);
                            else toastError("Erro ao copiar ID.");
                          }}
                        >
                          Copiar ID <Copy className="h-3 w-3" />
                        </Button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-8 text-xs"
                          onClick={() => {
                            onClose();
                            window.setTimeout(() => openServiceOrder(order.id), 0);
                          }}
                        >
                          Ver detalhes
                        </Button>
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <FileText className="mb-4 h-12 w-12 text-muted-foreground/30" />
                <p className="text-muted-foreground">Este cliente ainda não possui ordens de serviço.</p>
              </div>
            )}
          </ScrollArea>
        </div>

        <SheetFooter className="mt-6 border-t pt-6">
          <Button type="button" variant="outline" onClick={onClose} className="w-full">
            Fechar histórico
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
