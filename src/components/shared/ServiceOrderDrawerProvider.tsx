import {
  createContext,
  lazy,
  ReactNode,
  Suspense,
  useContext,
  useState,
} from "react";

const ServiceOrderDetailSheet = lazy(() =>
  import("@/components/shared/ServiceOrderDetailSheet").then(
    ({ ServiceOrderDetailSheet }) => ({ default: ServiceOrderDetailSheet }),
  ),
);
const ServiceOrderEditorSheet = lazy(() =>
  import("@/components/shared/ServiceOrderEditorSheet").then(
    ({ ServiceOrderEditorSheet }) => ({ default: ServiceOrderEditorSheet }),
  ),
);
const CustomerHistorySheet = lazy(() =>
  import("@/components/shared/CustomerHistorySheet").then(
    ({ CustomerHistorySheet }) => ({ default: CustomerHistorySheet }),
  ),
);

type ServiceOrderDrawerMode = "view" | "edit";

interface ServiceOrderDrawerContextValue {
  openServiceOrder: (id: string, mode?: ServiceOrderDrawerMode) => void;
  closeServiceOrder: () => void;
  openCustomerHistory: (id: string) => void;
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
  const [customerId, setCustomerId] = useState<string | null>(null);

  return (
    <ServiceOrderDrawerContext.Provider
      value={{
        openServiceOrder: (id, mode = "view") => setDrawer({ id, mode }),
        closeServiceOrder,
        openCustomerHistory: setCustomerId,
      }}
      >
        {children}
      {drawer && (
        <Suspense fallback={null}>
          {drawer.mode === "view" ? (
            <ServiceOrderDetailSheet
              orderId={drawer.id}
              open
              onClose={closeServiceOrder}
              onEdit={() =>
                setDrawer((current) =>
                  current ? { ...current, mode: "edit" } : current,
                )
              }
            />
          ) : (
            <ServiceOrderEditorSheet
              orderId={drawer.id}
              open
              onClose={closeServiceOrder}
              onView={() =>
                setDrawer((current) =>
                  current ? { ...current, mode: "view" } : current,
                )
              }
            />
          )}
        </Suspense>
      )}
      {customerId && (
        <Suspense fallback={null}>
          <CustomerHistorySheet
            customerId={customerId}
            open
            onClose={() => setCustomerId(null)}
          />
        </Suspense>
      )}
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
