import { useEffect, useRef, useState } from "react";
import { Check, DollarSign, Search, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { formatCurrency } from "@/lib/formatters";
import { InventoryItem } from "@/lib/types";

export type ServiceOrderItemLine = {
  id: string;
  inventoryItemId: string;
  inventoryItemName: string;
  itemType: InventoryItem["type"];
  quantity: number;
  unitPrice: number;
  maxQuantity?: number;
};

interface ServiceOrderItemsEditorProps {
  inventory: InventoryItem[];
  lines: ServiceOrderItemLine[];
  isLoading?: boolean;
  isBusy?: boolean;
  onSelectItem: (item: InventoryItem) => void | Promise<void>;
  onQuantityChange: (
    line: ServiceOrderItemLine,
    quantity: number,
  ) => void | Promise<void>;
  onRemove: (line: ServiceOrderItemLine) => void | Promise<void>;
}

function TruncatedItemName({
  name,
  side,
}: {
  name: string;
  side?: "top" | "right" | "bottom" | "left";
}) {
  const nameRef = useRef<HTMLSpanElement>(null);
  const [isTruncated, setIsTruncated] = useState(false);

  useEffect(() => {
    const element = nameRef.current;
    if (!element) return;

    const updateTruncation = () => {
      setIsTruncated(element.scrollWidth > element.clientWidth);
    };

    updateTruncation();
    const observer = new ResizeObserver(updateTruncation);
    observer.observe(element);
    return () => observer.disconnect();
  }, [name]);

  const label = (
    <span ref={nameRef} className="block truncate text-sm font-medium">
      {name}
    </span>
  );

  if (!isTruncated) return label;

  return (
    <Tooltip>
      <TooltipTrigger asChild>{label}</TooltipTrigger>
      <TooltipContent side={side}>{name}</TooltipContent>
    </Tooltip>
  );
}

export function ServiceOrderItemsEditor({
  inventory,
  lines,
  isLoading = false,
  isBusy = false,
  onSelectItem,
  onQuantityChange,
  onRemove,
}: ServiceOrderItemsEditorProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const [search, setSearch] = useState("");
  const [showItems, setShowItems] = useState(false);
  const filteredItems = inventory.filter((item) =>
    item.name.toLowerCase().includes(search.toLowerCase()),
  );

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (editorRef.current && !editorRef.current.contains(event.target as Node))
        setShowItems(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);

  const selectItem = (item: InventoryItem) => {
    setSearch("");
    setShowItems(false);
    void onSelectItem(item);
  };

  const changeQuantity = (line: ServiceOrderItemLine, value: string) => {
    const requested = Math.trunc(Number(value)) || 1;
    const quantity = Math.max(
      1,
      Math.min(line.maxQuantity ?? requested, requested),
    );
    void onQuantityChange(line, quantity);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex gap-2 text-lg">
          <DollarSign className="h-5 w-5 text-primary" />
          Peças e Serviços
        </CardTitle>
        <CardDescription>
          Peças consomem estoque; serviços não possuem limite.
        </CardDescription>
      </CardHeader>
      <TooltipProvider delayDuration={300}>
        <CardContent className="space-y-4">
          <div ref={editorRef} className="relative">
            <div className="relative">
              <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Buscar peça ou serviço..."
                value={search}
                onFocus={() => setShowItems(true)}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setShowItems(true);
                }}
                disabled={isBusy}
              />
            </div>
            {showItems && (
              <Card className="absolute z-10 mt-1 w-full shadow-lg">
                <CardContent className="max-h-48 overflow-auto p-1">
                  {filteredItems.map((item) => {
                    const unavailable =
                      item.type === "part" && item.currentQuantity <= 0;
                    const isSelected = lines.some(
                      (line) => line.inventoryItemId === item.id,
                    );
                    return (
                      <button
                        type="button"
                        disabled={isBusy || (unavailable && !isSelected)}
                        key={item.id}
                        onClick={() => selectItem(item)}
                        aria-pressed={isSelected}
                        className={`flex w-full items-center justify-between gap-3 rounded p-2 text-left transition-colors hover:bg-accent disabled:opacity-50 ${
                          isSelected
                            ? "bg-primary/10 text-primary ring-1 ring-primary/30"
                            : ""
                        }`}
                      >
                        <span className="min-w-0 flex-1">
                          <TruncatedItemName name={item.name} />
                          <span className="text-xs text-muted-foreground">
                            {item.type === "part"
                              ? `Peça | Estoque: ${item.currentQuantity}`
                              : "Serviço"}
                          </span>
                        </span>
                        <span className="flex shrink-0 items-center gap-2">
                          <span className="text-xs font-bold text-primary">
                            {formatCurrency(item.salePrice)}
                          </span>
                          {isSelected && (
                            <Badge variant="secondary" className="gap-1 text-[10px]">
                              <Check className="h-3 w-3" />
                              Selecionado
                            </Badge>
                          )}
                        </span>
                      </button>
                    );
                  })}
                </CardContent>
              </Card>
            )}
          </div>
          <div className="divide-y rounded-md border">
            {isLoading ? (
              <p className="p-6 text-center text-xs text-muted-foreground">
                Carregando itens...
              </p>
            ) : lines.length ? (
              lines.map((line) => (
                <div className="flex items-center gap-2 p-3" key={line.id}>
                  <div className="min-w-0 flex-1">
                    <div className="flex min-w-0 items-center gap-1">
                      <TruncatedItemName
                        name={line.inventoryItemName}
                        side="top"
                      />
                      <Badge variant="outline" className="text-[10px]">
                        {line.itemType === "part" ? "Peça" : "Serviço"}
                      </Badge>
                    </div>
                    <p className="text-xs text-primary">
                      {formatCurrency(line.unitPrice)}
                    </p>
                  </div>
                  <Input
                    className="h-8 w-16 text-center"
                    type="number"
                    min="1"
                    max={line.maxQuantity}
                    value={line.quantity}
                    aria-label={`Quantidade de ${line.inventoryItemName}`}
                    onChange={(event) => changeQuantity(line, event.target.value)}
                    disabled={isBusy}
                  />
                  <span className="w-20 text-right text-xs font-bold">
                    {formatCurrency(line.unitPrice * line.quantity)}
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7 text-destructive"
                    onClick={() => void onRemove(line)}
                    disabled={isBusy}
                    aria-label={`Remover ${line.inventoryItemName}`}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              ))
            ) : (
              <p className="p-6 text-center text-xs text-muted-foreground">
                Nenhum item adicionado.
              </p>
            )}
          </div>
        </CardContent>
      </TooltipProvider>
    </Card>
  );
}
