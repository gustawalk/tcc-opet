import { createContext, ReactNode, useContext, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Package } from "lucide-react";
import { dataCommand } from "@/lib/data-client";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { formatCurrency } from "@/lib/formatters";
import type { InventoryItem } from "@/lib/types";

const InventoryDrawerContext = createContext<{ openInventoryItem: (id: string) => void } | null>(null);

function InventoryDetailSheet({ id, onClose }: { id: string; onClose: () => void }) {
  const query = useQuery({ queryKey: ["inventory-item", id], queryFn: () => dataCommand<InventoryItem | null>("get_inventory_item", { id }) });
  const item = query.data;
  return <Sheet open onOpenChange={(open) => !open && onClose()}><SheetContent className="sm:max-w-lg"><SheetHeader><SheetTitle className="flex items-center gap-2"><Package className="h-5 w-5" />{item?.name ?? "Item de estoque"}</SheetTitle><SheetDescription>{item?.type === "part" ? "Peça" : "Serviço"}</SheetDescription></SheetHeader>{item && <dl className="mt-6 grid grid-cols-2 gap-4 text-sm"><div><dt className="text-muted-foreground">Estoque atual</dt><dd className="font-medium">{item.currentQuantity}</dd></div><div><dt className="text-muted-foreground">Estoque mínimo</dt><dd className="font-medium">{item.minQuantity}</dd></div><div><dt className="text-muted-foreground">Preço de venda</dt><dd className="font-medium">{formatCurrency(item.salePrice)}</dd></div><div><dt className="text-muted-foreground">Fornecedor</dt><dd className="font-medium">{item.supplierName || "Não informado"}</dd></div><div className="col-span-2"><dt className="text-muted-foreground">Descrição</dt><dd className="mt-1">{item.description || "Sem descrição."}</dd></div></dl>}</SheetContent></Sheet>;
}

export function InventoryDrawerProvider({ children }: { children: ReactNode }) {
  const [id, setId] = useState<string | null>(null);
  return <InventoryDrawerContext.Provider value={{ openInventoryItem: setId }}>{children}{id && <InventoryDetailSheet id={id} onClose={() => setId(null)} />}</InventoryDrawerContext.Provider>;
}

export function useInventoryDrawer() {
  const context = useContext(InventoryDrawerContext);
  if (!context) throw new Error("useInventoryDrawer must be used within InventoryDrawerProvider");
  return context;
}
