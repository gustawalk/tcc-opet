import { createContext, ReactNode, useContext, useState } from "react";
import { ServiceOrderDetailSheet } from "@/components/shared/ServiceOrderDetailSheet";
import { ServiceOrderEditorSheet } from "@/components/shared/ServiceOrderEditorSheet";

type ServiceOrderDrawerMode = "view" | "edit";

interface ServiceOrderDrawerContextValue {
  openServiceOrder: (id: string, mode?: ServiceOrderDrawerMode) => void;
  closeServiceOrder: () => void;
}

const ServiceOrderDrawerContext =
  createContext<ServiceOrderDrawerContextValue | null>(null);

export function ServiceOrderDrawerProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [drawer, setDrawer] = useState<{
    id: string;
    mode: ServiceOrderDrawerMode;
  } | null>(null);
  const closeServiceOrder = () => setDrawer(null);

  return (
    <ServiceOrderDrawerContext.Provider
      value={{
        openServiceOrder: (id, mode = "view") => setDrawer({ id, mode }),
        closeServiceOrder,
      }}
    >
      {children}
      <ServiceOrderDetailSheet
        orderId={drawer?.id ?? null}
        open={drawer?.mode === "view"}
        onClose={closeServiceOrder}
        onEdit={() =>
          setDrawer((current) =>
            current ? { ...current, mode: "edit" } : current,
          )
        }
      />
      <ServiceOrderEditorSheet
        orderId={drawer?.id ?? null}
        open={drawer?.mode === "edit"}
        onClose={closeServiceOrder}
        onView={() =>
          setDrawer((current) =>
            current ? { ...current, mode: "view" } : current,
          )
        }
      />
    </ServiceOrderDrawerContext.Provider>
  );
}

export function useServiceOrderDrawer() {
  const context = useContext(ServiceOrderDrawerContext);
  if (!context)
    throw new Error(
      "useServiceOrderDrawer must be used within ServiceOrderDrawerProvider",
    );
  return context;
}
