import { lazy, Suspense, useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
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
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  ServiceOrder,
  ChecklistItem,
  ServiceOrderAttachment,
  ServiceOrderEvent,
  PdfPreview,
} from "@/lib/types";
import { formatCurrency } from "@/lib/formatters";
import { toastError, toastSuccess } from "@/lib/errors";
import {
  User,
  Smartphone,
  Calendar,
  Wrench,
  ClipboardCheck,
  CheckSquare,
  Download,
  FileText,
  ImageIcon,
  LoaderCircle,
  Paperclip,
  Trash2,
} from "lucide-react";

const PdfPreviewDialog = lazy(() =>
  import("@/components/shared/PdfPreviewDialog").then(({ PdfPreviewDialog }) => ({
    default: PdfPreviewDialog,
  })),
);

interface ServiceOrderDetailSheetProps {
  orderId: string | null;
  open: boolean;
  onClose: () => void;
}

const fetchOrder = async (id: string): Promise<ServiceOrder | null> => {
  return await invoke<ServiceOrder | null>("get_service_order", { id });
};

const fetchParts = async (orderId: string) => {
  return await invoke<
    {
      id: string;
      serviceOrderId: string;
      inventoryItemId: string;
      inventoryItemName: string;
      quantity: number;
      unitCost: number;
      unitPrice: number;
    }[]
  >("get_service_order_parts", { serviceOrderId: orderId });
};

const fetchChecklist = async (orderId: string): Promise<ChecklistItem[]> => {
  return await invoke<ChecklistItem[]>("get_service_order_checklist", {
    osId: orderId,
  });
};

const fetchEvents = async (orderId: string): Promise<ServiceOrderEvent[]> => {
  return await invoke<ServiceOrderEvent[]>("get_service_order_events", {
    serviceOrderId: orderId,
  });
};

const fetchAttachments = async (
  orderId: string,
): Promise<ServiceOrderAttachment[]> => {
  return await invoke<ServiceOrderAttachment[]>(
    "get_service_order_attachments",
    { serviceOrderId: orderId },
  );
};

const formatFileSize = (sizeBytes: number) => {
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
};

const parseEventDetails = (details: string): Record<string, unknown> | null => {
  try {
    const parsed: unknown = JSON.parse(details);
    return parsed !== null &&
      typeof parsed === "object" &&
      !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
};

const getEventContent = (event: ServiceOrderEvent) => {
  const details = parseEventDetails(event.details);
  const value = (key: string) =>
    typeof details?.[key] === "string" || typeof details?.[key] === "number"
      ? String(details[key])
      : undefined;

  switch (event.eventType) {
    case "created":
      return {
        label: "Ordem de serviço criada",
        detail: value("status")
          ? `Status inicial: ${value("status")}.`
          : undefined,
      };
    case "updated":
      return {
        label: "Dados da ordem atualizados",
        detail: value("discountPercent")
          ? `Desconto: ${value("discountPercent")}%`
          : undefined,
      };
    case "status_changed":
      return {
        label: "Status alterado",
        detail:
          value("from") && value("to")
            ? `${value("from")} para ${value("to")}.`
            : undefined,
      };
    case "item_added":
      return {
        label: "Item adicionado",
        detail: value("quantity")
          ? `Quantidade: ${value("quantity")}.`
          : undefined,
      };
    case "item_removed":
      return {
        label: "Item removido",
        detail: value("quantity")
          ? `Quantidade: ${value("quantity")}.`
          : undefined,
      };
    case "item_updated":
      return {
        label: "Quantidade do item atualizada",
        detail: value("quantity")
          ? `Nova quantidade: ${value("quantity")}.`
          : undefined,
      };
    case "checklist_updated":
      return {
        label: "Checklist atualizado",
        detail: value("itemCount")
          ? `${value("itemCount")} item(ns) salvo(s).`
          : undefined,
      };
    case "attachment_added":
      return { label: "Anexo adicionado", detail: value("fileName") };
    case "attachment_removed":
      return { label: "Anexo removido", detail: value("fileName") };
    case "deleted":
      return { label: "Ordem de serviço excluída", detail: undefined };
    default:
      return {
        label: "Evento registrado",
        detail: details
          ? Object.entries(details)
              .map(([key, entry]) => `${key}: ${String(entry)}`)
              .join(" · ")
          : undefined,
      };
  }
};

function AttachmentItem({
  attachment,
  onDelete,
}: {
  attachment: ServiceOrderAttachment;
  onDelete?: (attachment: ServiceOrderAttachment) => void;
}) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const isImage = attachment.mimeType.startsWith("image/");
  const previewQuery = useQuery({
    queryKey: ["service-order-attachment-preview", attachment.id],
    queryFn: () =>
      invoke<string>("read_service_order_attachment", { id: attachment.id }),
    enabled: previewOpen && isImage,
  });

  const handleExport = async () => {
    const destination = await save({ defaultPath: attachment.fileName });
    if (!destination) return;

    try {
      await invoke("export_service_order_attachment", {
        id: attachment.id,
        destination,
      });
      toastSuccess("Anexo exportado com sucesso.");
    } catch (error) {
      toastError(error, "Erro ao exportar anexo.");
    }
  };

  return (
    <div className="rounded-md border p-3 space-y-3">
      <div className="flex items-start gap-3">
        {isImage ? (
          <ImageIcon className="h-4 w-4 mt-0.5 text-primary" />
        ) : (
          <FileText className="h-4 w-4 mt-0.5 text-muted-foreground" />
        )}
        <div className="min-w-0 flex-1">
          <p
            className="text-sm font-medium truncate"
            title={attachment.fileName}
          >
            {attachment.fileName}
          </p>
          <p className="text-xs text-muted-foreground">
            {attachment.mimeType || "Tipo desconhecido"} ·{" "}
            {formatFileSize(attachment.sizeBytes)}
          </p>
        </div>
      </div>
      {isImage && (
        <div className="space-y-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPreviewOpen((open) => !open)}
          >
            {previewOpen ? "Ocultar visualização" : "Visualizar imagem"}
          </Button>
          {previewOpen && previewQuery.isLoading && (
            <p className="text-xs text-muted-foreground">
              Carregando imagem...
            </p>
          )}
          {previewOpen && previewQuery.isError && (
            <p className="text-xs text-destructive">
              Não foi possível carregar a imagem.
            </p>
          )}
          {previewOpen && previewQuery.data && (
            <img
              src={previewQuery.data}
              alt={attachment.fileName}
              className="max-h-64 w-full rounded-md border object-contain"
            />
          )}
        </div>
      )}
      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          className="gap-1.5"
          onClick={handleExport}
        >
          <Download className="h-3.5 w-3.5" /> Baixar
        </Button>
        {onDelete && (
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5 text-destructive hover:text-destructive"
            onClick={() => onDelete(attachment)}
          >
            <Trash2 className="h-3.5 w-3.5" /> Excluir
          </Button>
        )}
      </div>
    </div>
  );
}

const getStatusBadge = (status: string) => {
  switch (status) {
    case "Orçamento":
      return <Badge variant="outline">{status}</Badge>;
    case "Em Manutenção":
      return (
        <Badge variant="default" className="bg-blue-600">
          {status}
        </Badge>
      );
    case "Aguardando Peça":
      return <Badge variant="destructive">{status}</Badge>;
    case "Finalizada":
      return (
        <Badge variant="secondary" className="bg-green-600 text-white">
          {status}
        </Badge>
      );
    case "Cancelada":
      return (
        <Badge variant="outline" className="opacity-50">
          {status}
        </Badge>
      );
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
};

export function ServiceOrderDetailSheet({
  orderId,
  open,
  onClose,
}: ServiceOrderDetailSheetProps) {
  const [eventsExpanded, setEventsExpanded] = useState(false);
  const {
    data: order,
    isLoading: orderLoading,
    isError: orderError,
  } = useQuery({
    queryKey: ["service-order", orderId],
    queryFn: () => fetchOrder(orderId!),
    enabled: !!orderId && open,
  });

  const {
    data: parts,
    isLoading: partsLoading,
    isError: partsError,
  } = useQuery({
    queryKey: ["service-order-parts", orderId],
    queryFn: () => fetchParts(orderId!),
    enabled: !!orderId && open,
  });

  const {
    data: checklistItems,
    isLoading: checklistLoading,
    isError: checklistError,
  } = useQuery({
    queryKey: ["service-order-checklist", orderId],
    queryFn: () => fetchChecklist(orderId!),
    enabled: !!orderId && open,
  });

  const {
    data: events,
    isLoading: eventsLoading,
    isError: eventsError,
  } = useQuery({
    queryKey: ["service-order-events", orderId],
    queryFn: () => fetchEvents(orderId!),
    enabled: !!orderId && open,
  });

  const {
    data: attachments,
    isLoading: attachmentsLoading,
    isError: attachmentsError,
  } = useQuery({
    queryKey: ["service-order-attachments", orderId],
    queryFn: () => fetchAttachments(orderId!),
    enabled: !!orderId && open,
  });

  const [pdfPreview, setPdfPreview] = useState<PdfPreview | null>(null);
  const pdfMutation = useMutation({
    mutationFn: () =>
      invoke<PdfPreview>("preview_service_order_pdf", {
        serviceOrderId: orderId!,
      }),
    onSuccess: setPdfPreview,
    onError: (error) =>
      toastError(error, "Erro ao gerar PDF da ordem de serviço."),
  });

  const handleGeneratePdf = () => {
    if (order) pdfMutation.mutate();
  };

  useEffect(() => {
    setEventsExpanded(false);
  }, [orderId, open]);

  const visibleEvents = eventsExpanded ? events : events?.slice(0, 1);
  const timelineItems = visibleEvents?.map((event) => {
    const content = getEventContent(event);
    return (
      <div
        key={event.id}
        className="border-l-2 border-primary/30 pl-3 py-1"
      >
        <p className="text-sm font-medium">{content.label}</p>
        {content.detail && (
          <p className="text-xs text-muted-foreground">{content.detail}</p>
        )}
        <p className="text-xs text-muted-foreground mt-1">
          {new Date(event.createdAt).toLocaleString("pt-BR")}
        </p>
      </div>
    );
  });

  return (
    <Sheet
      open={open}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <SheetContent className="sm:max-w-xl overflow-y-auto">
        <SheetHeader>
          <div className="flex justify-between items-start pt-4">
            <div className="space-y-1">
              <SheetTitle className="text-2xl font-bold flex items-center gap-2">
                <Wrench className="h-6 w-6 text-primary" />{" "}
                {(order?.displayId || order?.id.slice(0, 8)) ?? "Carregando..."}
              </SheetTitle>
              <SheetDescription>
                Detalhamento completo da ordem de serviço.
              </SheetDescription>
            </div>
            {order && getStatusBadge(order.status)}
          </div>
        </SheetHeader>

        {orderLoading ? (
          <div className="flex items-center justify-center py-16">
            <p className="text-muted-foreground">Carregando...</p>
          </div>
        ) : orderError ? (
          <div className="flex items-center justify-center py-16">
            <p className="text-destructive">
              Não foi possível carregar a ordem de serviço.
            </p>
          </div>
        ) : order ? (
          <div className="mt-8 space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">
                  Cliente
                </p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <User className="h-4 w-4" /> {order.customerName}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">
                  Equipamento
                </p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Smartphone className="h-4 w-4" /> {order.equipment}
                </p>
              </div>
              {order.imei && (
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground font-semibold uppercase">
                    IMEI / Serial
                  </p>
                  <p className="text-sm font-medium flex items-center gap-1.5">
                    <Smartphone className="h-4 w-4 text-primary" /> {order.imei}
                  </p>
                </div>
              )}
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">
                  Responsável
                </p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <User className="h-4 w-4" />{" "}
                  {order.userName || "Não atribuído"}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">
                  Abertura
                </p>
                <p className="text-sm font-medium flex items-center gap-1.5">
                  <Calendar className="h-4 w-4" />{" "}
                  {new Date(order.createdAt).toLocaleDateString("pt-BR")}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground font-semibold uppercase">
                  Total Previsto
                </p>
                {order.discountPercent > 0 ? (
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-bold text-primary">
                      {formatCurrency(
                        (order.totalPrice || 0) *
                          (1 - order.discountPercent / 100),
                      )}
                    </p>
                    <span className="text-xs line-through text-muted-foreground">
                      {formatCurrency(order.totalPrice || 0)}
                    </span>
                    <Badge variant="outline" className="text-[10px]">
                      -{order.discountPercent}%
                    </Badge>
                  </div>
                ) : (
                  <p className="text-sm font-bold text-primary">
                    {formatCurrency(order.totalPrice || 0)}
                  </p>
                )}
              </div>
            </div>

            <Separator />

            {checklistLoading ? (
              <p className="text-xs text-muted-foreground">
                Carregando checklist...
              </p>
            ) : checklistError ? (
              <p className="text-xs text-destructive">
                Não foi possível carregar o checklist.
              </p>
            ) : checklistItems && checklistItems.length > 0 ? (
              <>
                <div className="space-y-3">
                  <p className="text-xs text-muted-foreground font-semibold uppercase flex items-center gap-2">
                    <ClipboardCheck className="h-4 w-4" /> Checklist
                  </p>
                  <div className="grid grid-cols-2 gap-y-2 gap-x-4 border rounded-md p-3 bg-muted/20">
                    {checklistItems.map((item) => (
                      <div key={item.id} className="flex items-center gap-2">
                        {item.checked ? (
                          <CheckSquare className="h-4 w-4 text-primary" />
                        ) : (
                          <div className="h-4 w-4 border border-muted-foreground rounded-sm" />
                        )}
                        <span
                          className={`text-xs ${item.checked ? "font-medium" : "text-muted-foreground"}`}
                        >
                          {item.label}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
                <Separator />
              </>
            ) : (
              <p className="text-xs text-muted-foreground">
                Nenhum checklist vinculado.
              </p>
            )}

            <div className="space-y-2">
              <p className="text-xs text-muted-foreground font-semibold uppercase">
                Relato / Problema
              </p>
              <div className="p-3 bg-muted/50 rounded-md border text-sm">
                {order.description}
              </div>
            </div>

            <Separator />

            <div className="space-y-4">
              <p className="text-xs text-muted-foreground font-semibold uppercase">
                Peças & Mão de Obra
              </p>
              <div className="rounded-md border overflow-hidden">
                <Table>
                  <TableHeader className="bg-muted/50">
                    <TableRow>
                      <TableHead className="h-8 text-[10px]">Item</TableHead>
                      <TableHead className="h-8 text-center text-[10px]">
                        Qtd
                      </TableHead>
                      <TableHead className="h-8 text-right text-[10px]">
                        Valor
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {parts?.map((item) => (
                      <TableRow key={item.id}>
                        <TableCell className="py-2 text-xs font-medium">
                          {item.inventoryItemName}
                        </TableCell>
                        <TableCell className="py-2 text-center text-xs">
                          {item.quantity}
                        </TableCell>
                        <TableCell className="py-2 text-right text-xs">
                          {formatCurrency(item.unitPrice * item.quantity)}
                        </TableCell>
                      </TableRow>
                    ))}
                    {partsLoading && (
                      <TableRow>
                        <TableCell
                          colSpan={3}
                          className="h-12 text-center text-xs text-muted-foreground"
                        >
                          Carregando itens...
                        </TableCell>
                      </TableRow>
                    )}
                    {partsError && (
                      <TableRow>
                        <TableCell
                          colSpan={3}
                          className="h-12 text-center text-xs text-destructive"
                        >
                          Não foi possível carregar os itens.
                        </TableCell>
                      </TableRow>
                    )}
                    {!partsLoading && !partsError && parts?.length === 0 && (
                      <TableRow>
                        <TableCell
                          colSpan={3}
                          className="h-12 text-center text-xs text-muted-foreground italic"
                        >
                          Nenhuma peça ou serviço adicionado.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
            </div>

            <Separator />

            <div className="space-y-3">
              <p className="text-xs text-muted-foreground font-semibold uppercase flex items-center gap-2">
                <Paperclip className="h-4 w-4" /> Anexos
              </p>
              {attachmentsLoading && (
                <p className="text-sm text-muted-foreground">
                  Carregando anexos...
                </p>
              )}
              {attachmentsError && (
                <p className="text-sm text-destructive">
                  Não foi possível carregar os anexos.
                </p>
              )}
              {!attachmentsLoading &&
                !attachmentsError &&
                attachments?.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    Nenhum anexo adicionado.
                  </p>
                )}
              {attachments?.map((attachment) => (
                <AttachmentItem
                  key={attachment.id}
                  attachment={attachment}
                />
              ))}
            </div>

            <Separator />

            <div className="space-y-3">
              <p className="text-xs text-muted-foreground font-semibold uppercase">
                Linha do tempo
              </p>
              {eventsLoading && (
                <p className="text-sm text-muted-foreground">
                  Carregando eventos...
                </p>
              )}
              {eventsError && (
                <p className="text-sm text-destructive">
                  Não foi possível carregar os eventos.
                </p>
              )}
              {!eventsLoading && !eventsError && events?.length === 0 && (
                <p className="text-sm text-muted-foreground">
                  Nenhum evento registrado.
                </p>
              )}
              {eventsExpanded && events && events.length > 4 ? (
                <ScrollArea className="h-64">
                  <div className="space-y-2 pr-3">{timelineItems}</div>
                </ScrollArea>
              ) : (
                <div className="space-y-2">{timelineItems}</div>
              )}
              {!eventsLoading && !eventsError && (events?.length ?? 0) > 1 && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="px-0"
                  aria-expanded={eventsExpanded}
                  onClick={() => setEventsExpanded((expanded) => !expanded)}
                >
                  {eventsExpanded ? "Ver menos" : "Ver mais"}
                </Button>
              )}
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center py-16">
            <p className="text-muted-foreground">
              Ordem de serviço não encontrada.
            </p>
          </div>
        )}

        <SheetFooter className="mt-8">
          <Button
            variant="default"
            className="w-full gap-2"
            onClick={handleGeneratePdf}
            disabled={!order || pdfMutation.isPending}
          >
            {pdfMutation.isPending && (
              <LoaderCircle className="h-4 w-4 animate-spin" />
            )}
            {pdfMutation.isPending ? "Gerando PDF..." : "Gerar PDF da OS"}
          </Button>
        </SheetFooter>
      </SheetContent>
      {pdfPreview && (
        <Suspense fallback={null}>
          <PdfPreviewDialog preview={pdfPreview} onClose={() => setPdfPreview(null)} />
        </Suspense>
      )}
    </Sheet>
  );
}
