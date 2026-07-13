import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetFooter,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ServiceOrder, ChecklistItem } from "@/lib/types";
import { formatCurrency } from "@/lib/formatters";
import {
  User,
  Smartphone,
  Calendar,
  Wrench,
  ClipboardCheck,
  CheckSquare,
} from "lucide-react";

interface ServiceOrderDetailSheetProps {
  orderId: string | null;
  open: boolean;
  onClose: () => void;
}

const fetchOrder = async (id: string): Promise<ServiceOrder | null> => {
  return await invoke<ServiceOrder | null>("get_service_order", { id });
};

const fetchParts = async (orderId: string) => {
  return await invoke<{ id: string; serviceOrderId: string; inventoryItemId: string; inventoryItemName: string; quantity: number; unitCost: number; unitPrice: number }[]>(
    "get_service_order_parts",
    { serviceOrderId: orderId }
  );
};

const fetchChecklist = async (orderId: string): Promise<ChecklistItem[]> => {
  return await invoke<ChecklistItem[]>("get_service_order_checklist", { osId: orderId });
};

const getStatusBadge = (status: string) => {
  switch (status) {
    case "Orçamento": return <Badge variant="outline">{status}</Badge>;
    case "Em Manutenção": return <Badge variant="default" className="bg-blue-600">{status}</Badge>;
    case "Aguardando Peça": return <Badge variant="destructive">{status}</Badge>;
    case "Finalizada": return <Badge variant="secondary" className="bg-green-600 text-white">{status}</Badge>;
    case "Cancelada": return <Badge variant="outline" className="opacity-50">{status}</Badge>;
    default: return <Badge variant="outline">{status}</Badge>;
  }
};

export function ServiceOrderDetailSheet({ orderId, open, onClose }: ServiceOrderDetailSheetProps) {
  const { data: order, isLoading: orderLoading } = useQuery({
    queryKey: ["service-order", orderId],
    queryFn: () => fetchOrder(orderId!),
    enabled: !!orderId && open,
  });

  const { data: parts = [] } = useQuery({
    queryKey: ["service-order-parts", orderId],
    queryFn: () => fetchParts(orderId!),
    enabled: !!orderId && open,
  });

  const { data: checklistItems = [] } = useQuery({
    queryKey: ["service-order-checklist", orderId],
    queryFn: () => fetchChecklist(orderId!),
    enabled: !!orderId && open,
  });

  const lastOrder = useRef(order);
  const lastParts = useRef(parts);
  const lastChecklist = useRef(checklistItems);

  useEffect(() => {
    if (order) lastOrder.current = order;
  }, [order]);

  useEffect(() => {
    if (parts.length > 0) lastParts.current = parts;
  }, [parts]);

  useEffect(() => {
    if (checklistItems.length > 0) lastChecklist.current = checklistItems;
  }, [checklistItems]);

  const displayOrder = order ?? lastOrder.current;
  const displayParts = parts.length > 0 ? parts : lastParts.current;
  const displayChecklist = checklistItems.length > 0 ? checklistItems : lastChecklist.current;
  const hasNoCachedData = !displayOrder;

  return (
    <Sheet open={open} onOpenChange={(open) => { if (!open) onClose(); }}>
      <SheetContent className="sm:max-w-xl overflow-y-auto">
        <SheetHeader>
          <div className="flex justify-between items-start pt-4">
            <div className="space-y-1">
              <SheetTitle className="text-2xl font-bold flex items-center gap-2">
                <Wrench className="h-6 w-6 text-primary" /> {(displayOrder?.displayId || displayOrder?.id.slice(0, 8)) ?? "Carregando..."}
              </SheetTitle>
              <SheetDescription>
                Detalhamento completo da ordem de serviço.
              </SheetDescription>
            </div>
            {displayOrder && getStatusBadge(displayOrder.status)}
          </div>
        </SheetHeader>

        {hasNoCachedData && orderLoading ? (
          <div className="flex items-center justify-center py-16">
            <p className="text-muted-foreground">Carregando...</p>
          </div>
        ) : displayOrder ? (
          <div className="mt-8 space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Cliente</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <User className="h-4 w-4" /> {displayOrder.customerName}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Equipamento</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Smartphone className="h-4 w-4" /> {displayOrder.equipment}
                </p>
              </div>
              {displayOrder.imei && (
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground font-semibold uppercase">IMEI / Serial</p>
                  <p className="text-sm font-medium flex items-center gap-1.5">
                    <Smartphone className="h-4 w-4 text-primary" /> {displayOrder.imei}
                  </p>
                </div>
              )}
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Responsável</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <User className="h-4 w-4" /> {displayOrder.userName || "Não atribuído"}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Abertura</p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Calendar className="h-4 w-4" /> {new Date(displayOrder.createdAt).toLocaleDateString('pt-BR')}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">Total Previsto</p>
                {displayOrder.discountPercent > 0 ? (
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-bold text-primary">
                      {formatCurrency((displayOrder.totalPrice || 0) * (1 - displayOrder.discountPercent / 100))}
                    </p>
                    <span className="text-xs line-through text-muted-foreground">{formatCurrency(displayOrder.totalPrice || 0)}</span>
                    <Badge variant="outline" className="text-[10px]">-{displayOrder.discountPercent}%</Badge>
                  </div>
                ) : (
                  <p className="text-sm font-bold text-primary">
                    {formatCurrency(displayOrder.totalPrice || 0)}
                  </p>
                )}
              </div>
            </div>

            <Separator />

            {displayChecklist.length > 0 && (
              <>
                <div className="space-y-3">
                  <p className="text-xs text-muted-foreground font-semibold uppercase flex items-center gap-2">
                    <ClipboardCheck className="h-4 w-4" /> Checklist
                  </p>
                  <div className="grid grid-cols-2 gap-y-2 gap-x-4 border rounded-md p-3 bg-muted/20">
                    {displayChecklist.map((item) => (
                      <div key={item.id} className="flex items-center gap-2">
                        {item.checked ? (
                          <CheckSquare className="h-4 w-4 text-primary" />
                        ) : (
                          <div className="h-4 w-4 border border-muted-foreground rounded-sm" />
                        )}
                        <span className={`text-xs ${item.checked ? "font-medium" : "text-muted-foreground"}`}>
                          {item.label}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
                <Separator />
              </>
            )}

            <div className="space-y-2">
              <p className="text-xs text-muted-foreground font-semibold uppercase">Relato / Problema</p>
              <div className="p-3 bg-muted/50 rounded-md border text-sm">
                {displayOrder.description}
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
                    {displayParts.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell className="py-2 text-xs font-medium">{item.inventoryItemName}</TableCell>
                        <TableCell className="py-2 text-center text-xs">{item.quantity}</TableCell>
                        <TableCell className="py-2 text-right text-xs">{formatCurrency(item.unitPrice * item.quantity)}</TableCell>
                      </TableRow>
                    ))}
                    {displayParts.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={3} className="h-12 text-center text-xs text-muted-foreground italic">
                          Nenhuma peça ou serviço adicionado.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center py-16">
            <p className="text-muted-foreground">Ordem de serviço não encontrada.</p>
          </div>
        )}

        <SheetFooter className="mt-8">
          <Button variant="default" className="w-full gap-2" onClick={() => console.log("Gerar PDF da OS...")}>
            Gerar PDF da OS
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
