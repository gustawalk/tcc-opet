import { createContext, ReactNode, useContext, useState } from "react";
import { InventoryItemSheet } from "@/components/shared/InventoryItemSheet";
import type { InventoryItem } from "@/lib/types";

const InventoryItemCreateContext = createContext<{
  openInventoryItemCreate: (type: InventoryItem["type"]) => void;
} | null>(null);

export function InventoryItemCreateProvider({ children }: { children: ReactNode }) {
  const [type, setType] = useState<InventoryItem["type"] | null>(null);

  return (
    <InventoryItemCreateContext.Provider value={{ openInventoryItemCreate: setType }}>
      {children}
      <InventoryItemSheet
        open={type !== null}
        onOpenChange={(open) => !open && setType(null)}
        initialType={type ?? "part"}
      />
    </InventoryItemCreateContext.Provider>
  );
}

export function useInventoryItemCreate() {
  const context = useContext(InventoryItemCreateContext);
  if (!context) {
    throw new Error("useInventoryItemCreate must be used within InventoryItemCreateProvider");
  }
  return context;
}
